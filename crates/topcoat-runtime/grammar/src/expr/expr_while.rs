use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprWhile;
use topcoat_core_grammar::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, NameResolver};

impl Expr {
    pub(super) fn expr_while(
        expr: &ExprWhile,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        Self::expr_while_inner(expr, rust, js, names, true)
    }

    pub(super) fn expr_while_inner(
        expr: &ExprWhile,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
        returns_value: bool,
    ) -> syn::Result<()> {
        if let Some(label) = &expr.label {
            return Err(syn::Error::new_spanned(label, "labels are not supported"));
        }
        let (cond, body) =
            Self::lower_loop(Some(&expr.cond), &expr.body, js, names, returns_value)?;
        let token = &expr.while_token;
        quote! { #token #topcoat_runtime::Surrogate::into_real(#cond) #body }.to_tokens(rust);
        Ok(())
    }
}
