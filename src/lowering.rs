use crate::ast::{Ast, ExprId, ExprKind, ItemId, ItemKind, StmtId, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::hir::{
    Hir, HirBlock, HirBlockId, HirExpr, HirExprId, HirExprKind, HirFunction, HirStmt, HirStmtId,
    HirStmtKind,
};
use crate::name_resolver::{ResolveOutput, ResolvedName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerOutput {
    pub hir: Hir,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lower(ast: &Ast, resolved: &ResolveOutput) -> LowerOutput {
    Lowerer::new(ast, resolved).lower()
}

struct Lowerer<'a> {
    ast: &'a Ast,
    resolved: &'a ResolveOutput,
    hir: Hir,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lowerer<'a> {
    fn new(ast: &'a Ast, resolved: &'a ResolveOutput) -> Self {
        Self {
            ast,
            resolved,
            hir: Hir::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lower(mut self) -> LowerOutput {
        if let Some(module) = self.ast.root() {
            for item in &module.items {
                self.lower_item(*item);
            }
        }

        LowerOutput {
            hir: self.hir,
            diagnostics: self.diagnostics,
        }
    }

    fn lower_item(&mut self, item: ItemId) {
        let ItemKind::Function(function) = &self.ast.item(item).kind else {
            return;
        };
        let Some(function_id) = self.resolved.symbols.function_id_for_item(item) else {
            return;
        };

        let body = self.lower_block(&function.body, self.ast.item(item).span);
        self.hir.push_function(HirFunction::new(
            function_id,
            body,
            self.ast.item(item).span,
        ));
    }

    fn lower_block(&mut self, stmts: &[StmtId], span: crate::diagnostics::Span) -> HirBlockId {
        let stmts = stmts
            .iter()
            .map(|stmt| self.lower_stmt(*stmt))
            .collect::<Vec<_>>();
        self.hir.push_block(HirBlock::new(stmts, None, span))
    }

    fn lower_stmt(&mut self, stmt: StmtId) -> HirStmtId {
        let ast_stmt = self.ast.stmt(stmt);
        let kind = match &ast_stmt.kind {
            StmtKind::Expr(expr) => HirStmtKind::Expr(self.lower_expr(*expr)),
            StmtKind::Print(expr) => HirStmtKind::Print(self.lower_expr(*expr)),
            StmtKind::Local(local) => {
                let initializer = self.lower_expr(local.initializer);
                let Some(local_id) = self.resolved.symbols.local_id_for_stmt(stmt) else {
                    return self
                        .hir
                        .push_stmt(HirStmt::new(HirStmtKind::Error, ast_stmt.span));
                };
                HirStmtKind::Local {
                    local: local_id,
                    initializer,
                }
            }
            StmtKind::Error(_) => HirStmtKind::Error,
        };

        self.hir.push_stmt(HirStmt::new(kind, ast_stmt.span))
    }

    fn lower_expr(&mut self, expr: ExprId) -> HirExprId {
        let ast_expr = self.ast.expr(expr);
        let kind = match &ast_expr.kind {
            ExprKind::Ident(_) => match self.resolved.resolved_names.expr(expr) {
                Some(ResolvedName::Local(local)) => HirExprKind::Local(local),
                Some(ResolvedName::Function(function)) => HirExprKind::Function(function),
                None => HirExprKind::Error,
            },
            ExprKind::Number(number) => HirExprKind::Number(number.clone()),
            ExprKind::Unary { op, expr } => HirExprKind::Unary {
                op: *op,
                expr: self.lower_expr(*expr),
            },
            ExprKind::Binary { lhs, op, rhs } => HirExprKind::Binary {
                lhs: self.lower_expr(*lhs),
                op: *op,
                rhs: self.lower_expr(*rhs),
            },
            ExprKind::Error(_) => HirExprKind::Error,
        };

        self.hir.push_expr(HirExpr::new(kind, ast_expr.span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, NumberLiteral};
    use crate::lexer::lex;
    use crate::name_resolver::resolve;
    use crate::parser::parse;

    #[test]
    fn lowers_locals_and_resolved_identifier_expressions() {
        let ast = parse_source("fn main():\n    var x = 1 + 2\n    print x\n");
        let resolved = resolve(&ast);
        assert!(resolved.diagnostics.is_empty());

        let output = lower(&ast, &resolved);

        assert!(output.diagnostics.is_empty());
        assert_eq!(output.hir.functions().len(), 1);
        let function = &output.hir.functions()[0];
        let body = output.hir.block(function.body);
        assert_eq!(body.stmts.len(), 2);
        assert_eq!(body.tail, None);

        let HirStmtKind::Local { local, initializer } = output.hir.stmt(body.stmts[0]).kind else {
            panic!("expected local statement");
        };
        assert_eq!(local.index(), 0);
        assert!(matches!(
            output.hir.expr(initializer).kind,
            HirExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));

        let HirStmtKind::Print(expr) = output.hir.stmt(body.stmts[1]).kind else {
            panic!("expected print statement");
        };
        assert!(matches!(
            output.hir.expr(expr).kind,
            HirExprKind::Local(local) if local.index() == 0
        ));
    }

    #[test]
    fn lowers_unresolved_identifiers_to_error_expressions() {
        let ast = parse_source("fn main():\n    print missing\n");
        let resolved = resolve(&ast);
        let output = lower(&ast, &resolved);

        let function = &output.hir.functions()[0];
        let body = output.hir.block(function.body);
        let HirStmtKind::Print(expr) = output.hir.stmt(body.stmts[0]).kind else {
            panic!("expected print statement");
        };
        assert_eq!(output.hir.expr(expr).kind, HirExprKind::Error);
    }

    #[test]
    fn lowers_number_literals() {
        let ast = parse_source("fn main():\n    print 42\n");
        let resolved = resolve(&ast);
        let output = lower(&ast, &resolved);

        let function = &output.hir.functions()[0];
        let body = output.hir.block(function.body);
        let HirStmtKind::Print(expr) = output.hir.stmt(body.stmts[0]).kind else {
            panic!("expected print statement");
        };
        assert!(matches!(
            &output.hir.expr(expr).kind,
            HirExprKind::Number(NumberLiteral::Integer(value)) if value == "42"
        ));
    }

    fn parse_source(source: &str) -> Ast {
        let output = parse(lex(source));
        assert!(output.diagnostics.is_empty());
        output.ast
    }
}
