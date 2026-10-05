use syn::{
    Block, Expr as SynExpr, ExprBlock, ExprIf,
    visit::{self, Visit},
};

#[derive(Default)]
pub(super) struct ContainsAwait {
    found: bool,
}
impl ContainsAwait {
    pub(super) fn in_expr(expr: &SynExpr) -> bool {
        let mut visitor = Self::default();
        visitor.visit_expr(expr);
        visitor.found
    }

    pub(super) fn in_loop(condition: Option<&SynExpr>, body: &Block) -> bool {
        let mut visitor = Self::default();
        if let Some(condition) = condition {
            visitor.visit_expr(condition);
        }
        visitor.visit_block(body);
        visitor.found
    }

    pub(super) fn in_if(if_expr: &ExprIf) -> bool {
        let mut visitor = Self::default();
        visitor.visit_expr_if(if_expr);
        visitor.found
    }

    pub(super) fn in_block(block_expr: &ExprBlock) -> bool {
        let mut visitor = Self::default();
        visitor.visit_expr_block(block_expr);
        visitor.found
    }
}

impl<'ats> Visit<'ats> for ContainsAwait {
    fn visit_macro(&mut self, node: &'ats syn::Macro) {
        self.found |= super::builtin_macro::contains_await(node);
    }

    fn visit_expr_await(&mut self, node: &'ats syn::ExprAwait) {
        self.found = true;
        visit::visit_expr_await(self, node);
    }

    fn visit_expr_closure(&mut self, _node: &'ats syn::ExprClosure) {}
}
