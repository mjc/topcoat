use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprBreak;

use super::js::Js;
use crate::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_break(
        expr: &ExprBreak,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        if let Some(label) = &expr.label {
            return Err(syn::Error::new_spanned(label, "labels are not supported"));
        }
        let jump = names
            .control_flow
            .loop_jump()
            .ok_or_else(|| syn::Error::new_spanned(expr, "break requires an enclosing loop"))?;
        let mut value = TokenStream::new();
        if let Some(marker) = &jump.marker {
            js.push_str(&format!("{marker}.value = ("));
        } else if jump.returns_value {
            js.push_str("return ");
        } else if expr.expr.is_some() {
            js.push_str("0, ");
        }
        if let Some(expr) = &expr.expr {
            Self::dispatch(expr, &mut value, js, names)?;
        } else if jump.marker.is_some() || jump.returns_value {
            js.push_str("undefined");
        }
        if let Some(marker) = &jump.marker {
            js.push_str(&format!("); {marker}.continuing = false; throw {marker}"));
        } else if !jump.returns_value {
            if expr.expr.is_some() {
                js.push_str("; ");
            }
            js.push_str("break");
        }
        let token = &expr.break_token;
        quote! { #token #value }.to_tokens(rust);
        Ok(())
    }
}
