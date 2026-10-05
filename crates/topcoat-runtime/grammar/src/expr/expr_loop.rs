use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Block, Expr as SynExpr, ExprLoop};

use super::js::Js;
use crate::expr::{Expr, NameResolver, contains_await::ContainsAwait};

impl Expr {
    pub(super) fn expr_loop(
        expr: &ExprLoop,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        Self::loop_expr(expr, rust, js, names, true)
    }

    pub(super) fn loop_expr(
        expr: &ExprLoop,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        returns_value: bool,
    ) -> syn::Result<()> {
        if let Some(label) = &expr.label {
            return Err(syn::Error::new_spanned(label, "labels are not supported"));
        }
        let (_, body) = Self::lower_loop(None, &expr.body, js, names, returns_value)?;
        let token = &expr.loop_token;
        quote! { #token #body }.to_tokens(rust);
        Ok(())
    }

    pub(super) fn lower_loop(
        condition: Option<&SynExpr>,
        body: &Block,
        js: &mut Js,
        names: &mut NameResolver,
        returns_value: bool,
    ) -> syn::Result<(TokenStream, TokenStream)> {
        let is_async = ContainsAwait::in_loop(condition, body);
        if returns_value {
            names.control_flow.enter_value();
        }
        names.control_flow.enter_loop(returns_value);
        let mut loop_js = Js::default();
        let mut cond = TokenStream::new();
        loop_js.push_str("while (");
        if let Some(condition) = condition {
            Self::dispatch(condition, &mut cond, &mut loop_js, names)?;
            loop_js.push_str(".dehydrate()");
        } else {
            loop_js.push_str("true");
        }
        loop_js.push_str(") ");
        let mut body_js = Js::default();
        let mut body_rust = TokenStream::new();
        Self::block(body, &mut body_rust, &mut body_js, names, false)?;
        let target = names.control_flow.leave_loop();
        target.loop_body(body_js, &mut loop_js);
        if returns_value {
            names.control_flow.leave_value();
            js.push_str(if is_async {
                "(await (async () => { "
            } else {
                "(() => { "
            });
            target.declaration(js);
            js.append(loop_js);
            js.push_str(if is_async { " })())" } else { " })()" });
        } else if target.escapes {
            js.push_str("{ ");
            target.declaration(js);
            js.append(loop_js);
            js.push_str(" }");
        } else {
            js.append(loop_js);
        }
        Ok((cond, body_rust))
    }
}
