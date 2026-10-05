use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprReturn;
use topcoat_core_grammar::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_return(
        expr: &ExprReturn,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let jump = names.control_flow.return_jump();
        let mut value = TokenStream::new();
        if let Some(marker) = &jump.marker {
            js.push_str(&format!("{marker}.value = ("));
        } else {
            js.push_str("return ");
        }
        if let Some(expr) = &expr.expr {
            Self::dispatch(expr, &mut value, js, names)?;
        } else {
            js.push_str("undefined");
            quote! { () }.to_tokens(&mut value);
        }
        if let Some(marker) = &jump.marker {
            js.push_str(&format!("); throw {marker}"));
        }
        let token = &expr.return_token;
        if jump.root {
            quote! { #token #topcoat_runtime::Surrogate::into_real(#value) }.to_tokens(rust);
        } else {
            quote! { #token #value }.to_tokens(rust);
        }
        Ok(())
    }
}
