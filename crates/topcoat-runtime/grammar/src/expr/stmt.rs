use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, Stmt};

use super::{builtin_macro::RawReturn, js::Js};
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
        is_tail: bool,
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
                let is_tail_value = is_tail && semi.is_none();
                let returns = is_tail_value && !Self::is_statement_only(expr);
                let mut value = TokenStream::new();
                if returns {
                    Self::return_value(
                        Some(expr),
                        &mut value,
                        js,
                        names,
                        names.control_flow.boxes_value(),
                    )?;
                } else {
                    Self::stmt_expr(expr, &mut value, js, names)?;
                }

                if returns {
                    js.push(';');
                } else {
                    js.push_str("; ");
                }

                value.to_tokens(rust);
                semi.to_tokens(rust);
            }
            Stmt::Macro(stmt_macro) => Self::stmt_macro(stmt_macro, rust, js, names)?,
            other @ Stmt::Item(_) => {
                return Err(syn::Error::new_spanned(other, "unsupported statement"));
            }
        }
        Ok(())
    }

    /// Emits a return, boxing values returned by async wrappers.
    pub(super) fn return_value(
        expr: Option<&SynExpr>,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        boxed: bool,
    ) -> syn::Result<()> {
        if !boxed {
            js.push_str("return ");
            if let Some(expr) = expr {
                Self::dispatch(expr, rust, js, names)?;
            } else {
                js.push_str("undefined");
            }
            return Ok(());
        }

        if let Some(SynExpr::Macro(raw)) = expr {
            return Self::expr_macro_value(raw, rust, js, names, RawReturn::Boxed);
        }

        // Promise resolution checks for an inherited `then` property.
        js.push_str("return { __proto__: null, value: (");
        match expr {
            Some(expr) => Self::dispatch(expr, rust, js, names)?,
            None => js.push_str("undefined"),
        }
        js.push_str(") }");
        Ok(())
    }

    /// Lowers a statement expression, keeping jumps in the enclosing scope.
    pub(super) fn stmt_expr(
        expr: &SynExpr,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match expr {
            SynExpr::If(inner) => {
                Self::expr_if_inner(inner, js, names, false)?.to_tokens(rust);
            }
            SynExpr::Block(inner) => Self::block(&inner.block, rust, js, names, false)?,
            SynExpr::Macro(inner) => Self::stmt_macro_expr(inner, rust, js, names)?,
            SynExpr::Loop(inner) => Self::expr_loop_inner(inner, rust, js, names, false)?,
            SynExpr::While(inner) => Self::expr_while_inner(inner, rust, js, names, false)?,
            SynExpr::Break(inner) => Self::expr_break(inner, rust, js, names)?,
            SynExpr::Continue(inner) => Self::expr_continue(inner, rust, js, names)?,
            SynExpr::Return(inner) => Self::expr_return(inner, rust, js, names)?,
            SynExpr::Paren(inner) => {
                let mut unwrapped = inner.expr.as_ref();
                while let SynExpr::Paren(paren) = unwrapped {
                    unwrapped = paren.expr.as_ref();
                }
                if matches!(
                    unwrapped,
                    SynExpr::If(_) | SynExpr::Block(_) | SynExpr::Loop(_) | SynExpr::While(_)
                ) || Self::is_statement_only(unwrapped)
                {
                    let mut nested = TokenStream::new();
                    Self::stmt_expr(&inner.expr, &mut nested, js, names)?;
                    quote! { (#nested) }.to_tokens(rust);
                } else {
                    Self::expr_paren(inner, rust, js, names)?;
                }
            }
            other => Self::dispatch(other, rust, js, names)?,
        }
        Ok(())
    }

    /// Whether the lowered JavaScript requires statement position.
    pub(super) fn is_statement_only(expr: &SynExpr) -> bool {
        match expr {
            SynExpr::Paren(inner) => Self::is_statement_only(&inner.expr),
            SynExpr::Break(_) | SynExpr::Continue(_) | SynExpr::Return(_) => true,
            _ => false,
        }
    }
}
