use crate::ast::{BinaryOp, NumberLiteral, UnaryOp};
use crate::diagnostics::Span;
use crate::name_resolver::{FunctionId, LocalId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hir {
    functions: Vec<HirFunction>,
    blocks: Vec<HirBlock>,
    stmts: Vec<HirStmt>,
    exprs: Vec<HirExpr>,
}

impl Hir {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_function(&mut self, function: HirFunction) -> HirFunctionId {
        let id = HirFunctionId(self.functions.len());
        self.functions.push(function);
        id
    }

    pub fn push_block(&mut self, block: HirBlock) -> HirBlockId {
        let id = HirBlockId(self.blocks.len());
        self.blocks.push(block);
        id
    }

    pub fn push_stmt(&mut self, stmt: HirStmt) -> HirStmtId {
        let id = HirStmtId(self.stmts.len());
        self.stmts.push(stmt);
        id
    }

    pub fn push_expr(&mut self, expr: HirExpr) -> HirExprId {
        let id = HirExprId(self.exprs.len());
        self.exprs.push(expr);
        id
    }

    pub fn functions(&self) -> &[HirFunction] {
        &self.functions
    }

    pub fn blocks(&self) -> &[HirBlock] {
        &self.blocks
    }

    pub fn stmts(&self) -> &[HirStmt] {
        &self.stmts
    }

    pub fn exprs(&self) -> &[HirExpr] {
        &self.exprs
    }

    pub fn function(&self, id: HirFunctionId) -> &HirFunction {
        self.functions
            .get(id.index())
            .expect("invalid HIR function id")
    }

    pub fn block(&self, id: HirBlockId) -> &HirBlock {
        self.blocks.get(id.index()).expect("invalid HIR block id")
    }

    pub fn stmt(&self, id: HirStmtId) -> &HirStmt {
        self.stmts
            .get(id.index())
            .expect("invalid HIR statement id")
    }

    pub fn expr(&self, id: HirExprId) -> &HirExpr {
        self.exprs
            .get(id.index())
            .expect("invalid HIR expression id")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirFunctionId(usize);

impl HirFunctionId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirBlockId(usize);

impl HirBlockId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirStmtId(usize);

impl HirStmtId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirExprId(usize);

impl HirExprId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirFunction {
    pub function: FunctionId,
    pub body: HirBlockId,
    pub span: Span,
}

impl HirFunction {
    pub const fn new(function: FunctionId, body: HirBlockId, span: Span) -> Self {
        Self {
            function,
            body,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirBlock {
    pub stmts: Vec<HirStmtId>,
    pub tail: Option<HirExprId>,
    pub span: Span,
}

impl HirBlock {
    pub fn new(stmts: Vec<HirStmtId>, tail: Option<HirExprId>, span: Span) -> Self {
        Self { stmts, tail, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirStmt {
    pub kind: HirStmtKind,
    pub span: Span,
}

impl HirStmt {
    pub const fn new(kind: HirStmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirStmtKind {
    Expr(HirExprId),
    Local {
        local: LocalId,
        initializer: HirExprId,
    },
    Print(HirExprId),
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub span: Span,
}

impl HirExpr {
    pub const fn new(kind: HirExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirExprKind {
    Local(LocalId),
    Function(FunctionId),
    Number(NumberLiteral),
    Unary {
        op: UnaryOp,
        expr: HirExprId,
    },
    Binary {
        lhs: HirExprId,
        op: BinaryOp,
        rhs: HirExprId,
    },
    Block(HirBlockId),
    Error,
}
