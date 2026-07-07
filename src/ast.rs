use crate::diagnostics::Span;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ast {
    modules: Vec<Module>,
    items: Vec<Item>,
    stmts: Vec<Stmt>,
    exprs: Vec<Expr>,
    root: Option<ModuleId>,
}

impl Ast {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(root: Module) -> Self {
        let mut ast = Self::new();
        let root = ast.push_module(root);
        ast.set_root(root);
        ast
    }

    pub fn push_module(&mut self, module: Module) -> ModuleId {
        let id = ModuleId(self.modules.len());
        self.modules.push(module);
        id
    }

    pub fn push_item(&mut self, item: Item) -> ItemId {
        let id = ItemId(self.items.len());
        self.items.push(item);
        id
    }

    pub fn push_stmt(&mut self, stmt: Stmt) -> StmtId {
        let id = StmtId(self.stmts.len());
        self.stmts.push(stmt);
        id
    }

    pub fn push_expr(&mut self, expr: Expr) -> ExprId {
        let id = ExprId(self.exprs.len());
        self.exprs.push(expr);
        id
    }

    pub fn modules(&self) -> &[Module] {
        &self.modules
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn stmts(&self) -> &[Stmt] {
        &self.stmts
    }

    pub fn exprs(&self) -> &[Expr] {
        &self.exprs
    }

    pub fn root_id(&self) -> Option<ModuleId> {
        self.root
    }

    pub fn set_root(&mut self, root: ModuleId) -> bool {
        if self.get_module(root).is_none() {
            return false;
        }

        self.root = Some(root);
        true
    }

    pub fn module(&self, id: ModuleId) -> &Module {
        self.get_module(id).expect("invalid AST module id")
    }

    pub fn get_module(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(id.index())
    }

    pub fn item(&self, id: ItemId) -> &Item {
        self.get_item(id).expect("invalid AST item id")
    }

    pub fn get_item(&self, id: ItemId) -> Option<&Item> {
        self.items.get(id.index())
    }

    pub fn stmt(&self, id: StmtId) -> &Stmt {
        self.get_stmt(id).expect("invalid AST statement id")
    }

    pub fn get_stmt(&self, id: StmtId) -> Option<&Stmt> {
        self.stmts.get(id.index())
    }

    pub fn expr(&self, id: ExprId) -> &Expr {
        self.get_expr(id).expect("invalid AST expression id")
    }

    pub fn get_expr(&self, id: ExprId) -> Option<&Expr> {
        self.exprs.get(id.index())
    }

    pub fn root(&self) -> Option<&Module> {
        self.root.map(|root| self.module(root))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(usize);

impl ModuleId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(usize);

impl ItemId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StmtId(usize);

impl StmtId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(usize);

impl ExprId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub text: String,
    pub span: Span,
}

impl Ident {
    pub fn new(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub items: Vec<ItemId>,
    pub span: Span,
}

impl Module {
    pub fn new(items: Vec<ItemId>, span: Span) -> Self {
        Self { items, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

impl Item {
    pub const fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Function(Function),
    Error(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Vec<StmtId>,
}

impl Function {
    pub fn new(
        name: Ident,
        params: Vec<Param>,
        return_type: Option<TypeRef>,
        body: Vec<StmtId>,
    ) -> Self {
        Self {
            name,
            params,
            return_type,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Ident,
    pub ty: Option<TypeRef>,
}

impl Param {
    pub fn new(name: Ident, ty: Option<TypeRef>) -> Self {
        Self { name, ty }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub kind: TypeRefKind,
    pub span: Span,
}

impl TypeRef {
    pub fn named(name: impl Into<String>, span: Span) -> Self {
        Self::named_ident(Ident::new(name, span))
    }

    pub fn named_ident(name: Ident) -> Self {
        let span = name.span;
        Self {
            kind: TypeRefKind::Named(name),
            span,
        }
    }

    pub const fn new(kind: TypeRefKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRefKind {
    Named(Ident),
    Generic {
        base: Box<TypeRef>,
        args: Vec<TypeRef>,
    },
    Function {
        params: Vec<TypeRef>,
        return_type: Option<Box<TypeRef>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub const fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Expr(ExprId),
    Local(Local),
    Print(ExprId),
    Error(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub kind: BindingKind,
    pub name: Ident,
    pub ty: Option<TypeRef>,
    pub initializer: ExprId,
}

impl Local {
    pub fn new(kind: BindingKind, name: Ident, ty: Option<TypeRef>, initializer: ExprId) -> Self {
        Self {
            kind,
            name,
            ty,
            initializer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Let,
    Var,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub const fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Ident(Ident),
    Number(NumberLiteral),
    Unary {
        op: UnaryOp,
        expr: ExprId,
    },
    Binary {
        lhs: ExprId,
        op: BinaryOp,
        rhs: ExprId,
    },
    Error(Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error;

impl Error {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberLiteral {
    Integer(String),
    Float(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integer(value: &str, span: Span) -> Expr {
        Expr::new(ExprKind::Number(NumberLiteral::Integer(value.into())), span)
    }

    #[test]
    fn ast_starts_without_a_root() {
        let ast = Ast::new();

        assert_eq!(ast.root_id(), None);
        assert!(ast.root().is_none());
        assert!(ast.modules().is_empty());
    }

    #[test]
    fn ast_adds_typed_nodes_and_returns_ids() {
        let mut ast = Ast::new();
        let lhs = ast.push_expr(integer("1", Span::new(0, 1)));
        let rhs = ast.push_expr(integer("2", Span::new(4, 5)));
        let root = ast.push_expr(Expr::new(
            ExprKind::Binary {
                lhs,
                op: BinaryOp::Add,
                rhs,
            },
            Span::new(0, 5),
        ));

        assert_eq!(lhs.index(), 0);
        assert_eq!(rhs.index(), 1);
        assert_eq!(root.index(), 2);
        assert_eq!(ast.expr(root).span, Span::new(0, 5));
    }

    #[test]
    fn ast_rejects_invalid_root_ids() {
        let mut ast = Ast::new();

        assert!(!ast.set_root(ModuleId(0)));
        assert_eq!(ast.root_id(), None);
        assert!(ast.get_module(ModuleId(0)).is_none());
    }

    #[test]
    fn ast_represents_function_items_with_type_refs() {
        let mut ast = Ast::new();
        let param_ty = TypeRef::named("Enemy", Span::new(16, 21));
        let return_ty = TypeRef::named("float", Span::new(26, 31));
        let body_expr = ast.push_expr(integer("1", Span::new(37, 38)));
        let body_stmt = ast.push_stmt(Stmt::new(StmtKind::Print(body_expr), Span::new(31, 38)));
        let function = ast.push_item(Item::new(
            ItemKind::Function(Function::new(
                Ident::new("read_hp", Span::new(3, 10)),
                vec![Param::new(
                    Ident::new("enemy", Span::new(11, 16)),
                    Some(param_ty.clone()),
                )],
                Some(return_ty.clone()),
                vec![body_stmt],
            )),
            Span::new(0, 38),
        ));
        let module = ast.push_module(Module::new(vec![function], Span::new(0, 38)));

        assert!(ast.set_root(module));
        assert_eq!(ast.root_id(), Some(module));
        assert_eq!(
            param_ty.kind,
            TypeRefKind::Named(Ident::new("Enemy", Span::new(16, 21)))
        );
        assert_eq!(
            return_ty.kind,
            TypeRefKind::Named(Ident::new("float", Span::new(26, 31)))
        );
        assert_eq!(ast.root().expect("root module").items, vec![function]);

        let ItemKind::Function(function) = &ast.item(function).kind else {
            panic!("item should be a function");
        };
        assert_eq!(function.body, vec![body_stmt]);
        assert_eq!(ast.stmt(body_stmt).kind, StmtKind::Print(body_expr));
    }
}
