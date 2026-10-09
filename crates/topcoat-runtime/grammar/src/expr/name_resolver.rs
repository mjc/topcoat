use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use topcoat_core_grammar::paths::topcoat_runtime;

use super::control_flow::ControlFlow;

pub(super) enum ResolvedIdent {
    Local { js_name: String, rust_ident: Ident },
    External { rust_ident: Ident },
}

#[derive(Clone)]
struct LocalBinding {
    js_name: String,
    rust_ident: Ident,
    kind: LocalBindingKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalBindingKind {
    Plain,
    Surrogate,
    Closure,
}

impl LocalBindingKind {
    pub(super) fn for_expr(expr: &syn::Expr, names: &NameResolver) -> Self {
        if names.is_closure(expr) {
            Self::Closure
        } else {
            Self::Surrogate
        }
    }
}

pub(super) struct ExternalBinding {
    pub(super) value: TokenStream,
    pub(super) rust_ident: Ident,
}

#[derive(Default)]
pub(super) struct NameResolver {
    pub(super) control_flow: ControlFlow,
    scopes: Vec<HashMap<String, LocalBinding>>,
    externals: Vec<ExternalBinding>,
    external_by_name: HashMap<String, usize>,
    next_local: usize,
}

impl NameResolver {
    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn allocate_local(&mut self) -> String {
        let name = format!("__local{}", self.next_local);
        self.next_local += 1;
        name
    }

    pub(super) fn bind_local(
        &mut self,
        ident: &Ident,
        name: String,
        kind: LocalBindingKind,
    ) -> syn::Result<()> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            syn::Error::new_spanned(ident, "local binding requires an active scope")
        })?;
        scope.insert(
            ident.to_string(),
            LocalBinding {
                js_name: name,
                rust_ident: ident.clone(),
                kind,
            },
        );
        Ok(())
    }

    pub(super) fn resolve(&mut self, ident: &Ident) -> ResolvedIdent {
        let original = ident.to_string();
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(&original) {
                return ResolvedIdent::Local {
                    js_name: binding.js_name.clone(),
                    rust_ident: binding.rust_ident.clone(),
                };
            }
        }

        if let Some(index) = self.external_by_name.get(&original) {
            let binding = &self.externals[*index];
            return ResolvedIdent::External {
                rust_ident: binding.rust_ident.clone(),
            };
        }

        let index = self.externals.len();
        let rust_ident = self.capture_value(
            quote! {
                ::core::convert::Into::<#topcoat_runtime::Expr<_>>::into(
                    ::core::clone::Clone::clone(&#ident),
                )
            },
            ident.span(),
        );
        self.external_by_name.insert(original, index);
        ResolvedIdent::External { rust_ident }
    }

    /// Binds a value serialized by the Rust target when the expression is built.
    pub(super) fn capture_value(&mut self, value: TokenStream, span: Span) -> Ident {
        let index = self.externals.len();
        let rust_ident = Ident::new(&format!("__topcoat_external{index}"), span);
        self.externals.push(ExternalBinding {
            value,
            rust_ident: rust_ident.clone(),
        });
        rust_ident
    }

    pub(super) fn is_surrogate_local(&self, ident: &Ident) -> bool {
        let original = ident.to_string();
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(&original) {
                return binding.kind == LocalBindingKind::Surrogate;
            }
        }
        false
    }

    pub(super) fn has_local_scope(&self) -> bool {
        !self.scopes.is_empty()
    }

    pub(super) fn is_closure(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Closure(_) => true,
            syn::Expr::Path(path) => path.path.get_ident().is_some_and(|ident| {
                let original = ident.to_string();
                self.scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(&original))
                    .is_some_and(|binding| binding.kind == LocalBindingKind::Closure)
            }),
            syn::Expr::Paren(paren) => self.is_closure(&paren.expr),
            syn::Expr::If(expr) => {
                self.block_is_closure(&expr.then_branch)
                    && expr
                        .else_branch
                        .as_ref()
                        .is_some_and(|(_, expr)| self.is_closure(expr))
            }
            syn::Expr::Block(expr) => self.block_is_closure(&expr.block),
            _ => false,
        }
    }

    fn block_is_closure(&self, block: &syn::Block) -> bool {
        let mut names = Self {
            scopes: self.scopes.clone(),
            ..Self::default()
        };
        names.push_scope();
        for stmt in &block.stmts {
            if let syn::Stmt::Local(local) = stmt {
                let pat = match &local.pat {
                    syn::Pat::Type(pat) => &*pat.pat,
                    pat => pat,
                };
                if let syn::Pat::Ident(pat) = pat {
                    let kind = local
                        .init
                        .as_ref()
                        .map_or(LocalBindingKind::Surrogate, |init| {
                            LocalBindingKind::for_expr(&init.expr, &names)
                        });
                    names.bind_local(&pat.ident, String::new(), kind).unwrap();
                }
            }
        }
        matches!(block.stmts.last(), Some(syn::Stmt::Expr(expr, None)) if names.is_closure(expr))
    }

    pub(super) fn externals(&self) -> &[ExternalBinding] {
        &self.externals
    }
}
