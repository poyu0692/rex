use crate::diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    nodes: Vec<Node>,
    root: NodeId,
}

impl Ast {
    pub fn new(root: Node) -> Self {
        Self {
            nodes: vec![root],
            root: NodeId(0),
        }
    }

    pub fn push(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn root_id(&self) -> NodeId {
        self.root
    }

    pub fn set_root(&mut self, root: NodeId) -> bool {
        if self.get_node(root).is_none() {
            return false;
        }

        self.root = root;
        true
    }

    pub fn node(&self, id: NodeId) -> &Node {
        self.get_node(id).expect("invalid AST node id")
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index())
    }

    pub fn root(&self) -> &Node {
        self.node(self.root)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Span,
}

impl Node {
    pub const fn new(kind: NodeKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Module(Module),
    Item(Item),
    Expr(Expr),
    Stmt(Stmt),
    Error(Error),
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
    pub items: Vec<NodeId>,
}

impl Module {
    pub fn new(items: Vec<NodeId>) -> Self {
        Self { items }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
}

impl Item {
    pub const fn new(kind: ItemKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Function(Function),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Vec<NodeId>,
}

impl Function {
    pub fn new(
        name: Ident,
        params: Vec<Param>,
        return_type: Option<TypeRef>,
        body: Vec<NodeId>,
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
pub struct Expr {
    pub kind: ExprKind,
}

impl Expr {
    pub const fn new(kind: ExprKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
}

impl Stmt {
    pub const fn new(kind: StmtKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Expr(NodeId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error;

impl Error {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Number(NumberLiteral),
    Unary {
        op: UnaryOp,
        expr: NodeId,
    },
    Binary {
        lhs: NodeId,
        op: BinaryOp,
        rhs: NodeId,
    },
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

    fn integer(value: &str, span: Span) -> Node {
        Node::new(
            NodeKind::Expr(Expr::new(ExprKind::Number(NumberLiteral::Integer(
                value.into(),
            )))),
            span,
        )
    }

    #[test]
    fn ast_owns_root_node() {
        let span = Span::new(0, 3);
        let ast = Ast::new(integer("123", span));

        assert_eq!(ast.root_id(), NodeId(0));
        assert_eq!(ast.root().span, span);
    }

    #[test]
    fn ast_adds_nodes_and_returns_ids() {
        let mut ast = Ast::new(integer("1", Span::new(0, 1)));
        let rhs = ast.push(integer("2", Span::new(4, 5)));
        let lhs = ast.root_id();
        let root = ast.push(Node::new(
            NodeKind::Expr(Expr::new(ExprKind::Binary {
                lhs,
                op: BinaryOp::Add,
                rhs,
            })),
            Span::new(0, 5),
        ));

        assert_eq!(rhs.index(), 1);
        assert!(ast.set_root(root));
        assert_eq!(ast.root().span, Span::new(0, 5));
    }

    #[test]
    fn ast_rejects_invalid_root_ids() {
        let mut ast = Ast::new(integer("1", Span::new(0, 1)));

        assert!(!ast.set_root(NodeId(1)));
        assert_eq!(ast.root_id(), NodeId(0));
        assert!(ast.get_node(NodeId(1)).is_none());
    }

    #[test]
    fn ast_represents_function_items_with_type_refs() {
        let mut ast = Ast::new(Node::new(
            NodeKind::Module(Module::new(Vec::new())),
            Span::new(0, 0),
        ));
        let param_ty = TypeRef::named("Enemy", Span::new(16, 21));
        let return_ty = TypeRef::named("float", Span::new(26, 31));
        let function = ast.push(Node::new(
            NodeKind::Item(Item::new(ItemKind::Function(Function::new(
                Ident::new("read_hp", Span::new(3, 10)),
                vec![Param::new(
                    Ident::new("enemy", Span::new(11, 16)),
                    Some(param_ty.clone()),
                )],
                Some(return_ty.clone()),
                Vec::new(),
            )))),
            Span::new(0, 32),
        ));
        let module = ast.push(Node::new(
            NodeKind::Module(Module::new(vec![function])),
            Span::new(0, 32),
        ));

        assert!(ast.set_root(module));
        assert_eq!(ast.root_id(), module);
        assert_eq!(
            param_ty.kind,
            TypeRefKind::Named(Ident::new("Enemy", Span::new(16, 21)))
        );
        assert_eq!(
            return_ty.kind,
            TypeRefKind::Named(Ident::new("float", Span::new(26, 31)))
        );

        let NodeKind::Module(module) = &ast.root().kind else {
            panic!("root should be a module");
        };
        assert_eq!(module.items, vec![function]);
    }
}
