use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprBreak;

use super::{control_flow::ReturnValue, js::Js};
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
            write!(js, "{marker}.value = (").unwrap();
        } else if jump.value != ReturnValue::None {
            Self::return_value(
                expr.expr.as_deref(),
                &mut value,
                js,
                names,
                jump.value == ReturnValue::Boxed,
            )?;
        } else if expr.expr.is_some() {
            js.push_str("0, ");
        }
        if jump.marker.is_some() || jump.value == ReturnValue::None {
            if let Some(expr) = &expr.expr {
                Self::dispatch(expr, &mut value, js, names)?;
            } else if jump.marker.is_some() {
                js.push_str("undefined");
            }
        }
        if let Some(marker) = &jump.marker {
            write!(js, "); {marker}.continuing = false; throw {marker}").unwrap();
        } else if jump.value == ReturnValue::None {
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
