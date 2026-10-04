use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Stmt;

use super::js::Js;
use crate::expr::{
    Expr,
    name_resolver::{LocalBindingKind, NameResolver},
};

impl Expr {
    pub(super) fn stmt(
        stmt: &Stmt,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        is_last: bool,
    ) -> syn::Result<()> {
        match stmt {
            Stmt::Local(local) => {
                let init = local.init.as_ref().ok_or_else(|| {
                    syn::Error::new_spanned(local, "let binding requires an initializer")
                })?;
                if let Some((_, diverge)) = &init.diverge {
                    return Err(syn::Error::new_spanned(
                        diverge,
                        "let-else is not supported",
                    ));
                }

                js.push_str("let ");
                let mut pat = TokenStream::new();
                let (ident, name) = Self::pat(&local.pat, &mut pat, js, names)?;
                js.push_str(" = ");
                let mut value = TokenStream::new();
                Self::dispatch(&init.expr, &mut value, js, names)?;
                js.push_str("; ");
                names.bind_local(&ident, name, LocalBindingKind::Surrogate)?;

                quote! { let #pat = #value; }.to_tokens(rust);
            }
            Stmt::Expr(expr, semi) => {
                // A trailing expression (no semicolon) is the block's value, so
                // it becomes the JavaScript `return`.
                let returns = is_last
                    && semi.is_none()
                    && !matches!(
                        expr,
                        syn::Expr::Break(_)
                            | syn::Expr::Continue(_)
                            | syn::Expr::Return(_)
                            | syn::Expr::Loop(_)
                            | syn::Expr::While(_)
                    );
                if returns {
                    js.push_str("return ");
                }

                let mut value = TokenStream::new();
                if returns {
                    Self::dispatch(expr, &mut value, js, names)?;
                } else {
                    Self::statement_expr(expr, &mut value, js, names)?;
                }

                if returns {
                    js.push(';');
                    value.to_tokens(rust);
                } else {
                    js.push_str("; ");
                    quote! { #value; }.to_tokens(rust);
                }
            }
            Stmt::Macro(stmt_macro) => Self::stmt_macro(stmt_macro, rust, js, names)?,
            other @ Stmt::Item(_) => {
                return Err(syn::Error::new_spanned(other, "unsupported statement"));
            }
        }
        Ok(())
    }

    /// Emits statement blocks and conditionals in the enclosing function so
    /// their jumps still target the surrounding loop or closure.
    fn statement_expr(
        expr: &syn::Expr,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match expr {
            syn::Expr::If(inner) => {
                Self::expr_if_inner(inner, js, names, false)?.to_tokens(rust);
                Ok(())
            }
            syn::Expr::Block(inner) => Self::block(&inner.block, rust, js, names, false),
            syn::Expr::Paren(inner) => Self::statement_expr(&inner.expr, rust, js, names),
            other => Self::dispatch(other, rust, js, names),
        }
    }
}
