use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprClosure};
use topcoat_core_grammar::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{
    Expr,
    name_resolver::{LocalBindingKind, NameResolver},
};

impl Expr {
    pub(super) fn expr_closure(
        closure: &ExprClosure,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        names.control_flow.enter_function(false);
        let asyncness = &closure.asyncness;
        let nested = names.has_local_scope();
        let deferred = asyncness.is_some() && nested;
        if asyncness.is_some() && !deferred {
            js.push_str("async ");
        }

        js.push('(');
        names.push_scope();
        let mut inputs = Vec::with_capacity(closure.inputs.len());
        for (i, input) in closure.inputs.iter().enumerate() {
            if i > 0 {
                js.push_str(", ");
            }
            let mut tokens = TokenStream::new();
            let (ident, name) = Self::pat(input, &mut tokens, js, names)?;
            if let syn::Pat::Type(input) = input {
                let pat = &input.pat;
                let ty = &input.ty;
                tokens = quote! { #pat: <#ty as #topcoat_runtime::Surrogated>::Surrogate };
            }
            // Top-level handlers receive facade events, which raw! can borrow
            // directly. Typed local closure parameters use vocabulary values.
            let kind = if nested && matches!(input, syn::Pat::Type(_)) {
                LocalBindingKind::Surrogate
            } else {
                LocalBindingKind::Plain
            };
            names.bind_local(&ident, name, kind)?;
            inputs.push(tokens);
        }
        js.push_str(") => ");
        if deferred {
            js.push_str("cx.future(async () => ");
        }

        let mut body = TokenStream::new();
        let mut body_js = Js::default();
        match &*closure.body {
            // A block body maps directly onto the arrow function body without
            // the IIFE wrapper that a block expression would need.
            SynExpr::Block(block) => {
                Self::block(&block.block, &mut body, &mut body_js, names, true)?;
            }
            other if Self::is_statement_only(other) => {
                body_js.push_str("{ ");
                Self::stmt_expr(other, &mut body, &mut body_js, names)?;
                body_js.push_str("; }");
            }
            other => Self::dispatch(other, &mut body, &mut body_js, names)?,
        }
        let is_block =
            matches!(&*closure.body, SynExpr::Block(_)) || Self::is_statement_only(&closure.body);
        let target = names.control_flow.leave_function();
        target.function_body(body_js, js, is_block);
        names.pop_scope();
        if deferred {
            js.push(')');
        }

        let output = match &closure.output {
            syn::ReturnType::Default => TokenStream::new(),
            syn::ReturnType::Type(arrow, ty) => {
                quote! { #arrow <#ty as #topcoat_runtime::Surrogated>::Surrogate }
            }
        };
        quote! { #asyncness move |#(#inputs),*| #output #body }.to_tokens(rust);
        Ok(())
    }
}
