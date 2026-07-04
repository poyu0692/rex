use crate::ast::{Ast, BindingKind, ExprId, ExprKind, Ident, ItemId, ItemKind, StmtId, StmtKind};
use crate::diagnostics::Diagnostic;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveOutput {
    pub symbols: Symbols,
    pub resolved_names: ResolvedNames,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Symbols {
    pub functions: Vec<FunctionSymbol>,
    pub locals: Vec<LocalSymbol>,
}

impl Symbols {
    pub fn function(&self, id: FunctionId) -> &FunctionSymbol {
        self.functions
            .get(id.index())
            .expect("invalid function symbol id")
    }

    pub fn local(&self, id: LocalId) -> &LocalSymbol {
        self.locals
            .get(id.index())
            .expect("invalid local symbol id")
    }

    pub fn function_id_for_item(&self, item: ItemId) -> Option<FunctionId> {
        self.functions
            .iter()
            .position(|function| function.item == item)
            .map(FunctionId)
    }

    pub fn local_id_for_stmt(&self, stmt: StmtId) -> Option<LocalId> {
        self.locals
            .iter()
            .position(|local| local.source == LocalSource::Stmt(stmt))
            .map(LocalId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSymbol {
    pub name: Ident,
    pub item: ItemId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSymbol {
    pub name: Ident,
    pub kind: BindingKind,
    pub source: LocalSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalSource {
    Param { item: ItemId, index: usize },
    Stmt(StmtId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNames {
    pub exprs: Vec<Option<ResolvedName>>,
}

impl ResolvedNames {
    fn new(expr_count: usize) -> Self {
        Self {
            exprs: vec![None; expr_count],
        }
    }

    pub fn expr(&self, id: ExprId) -> Option<ResolvedName> {
        self.exprs.get(id.index()).copied().flatten()
    }

    fn set_expr(&mut self, id: ExprId, resolution: ResolvedName) {
        if let Some(slot) = self.exprs.get_mut(id.index()) {
            *slot = Some(resolution);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedName {
    Function(FunctionId),
    Local(LocalId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(usize);

impl FunctionId {
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(usize);

impl LocalId {
    pub const fn index(self) -> usize {
        self.0
    }
}

pub fn resolve(ast: &Ast) -> ResolveOutput {
    Resolver::new(ast).resolve()
}

struct Resolver<'a> {
    ast: &'a Ast,
    symbols: Symbols,
    resolved_names: ResolvedNames,
    diagnostics: Vec<Diagnostic>,
    functions_by_name: HashMap<String, FunctionId>,
    scopes: Vec<Scope>,
}

impl<'a> Resolver<'a> {
    fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            symbols: Symbols::default(),
            resolved_names: ResolvedNames::new(ast.exprs().len()),
            diagnostics: Vec::new(),
            functions_by_name: HashMap::new(),
            scopes: Vec::new(),
        }
    }

    fn resolve(mut self) -> ResolveOutput {
        let Some(module) = self.ast.root() else {
            return self.finish();
        };

        for item in &module.items {
            self.declare_item(*item);
        }

        for item in &module.items {
            self.resolve_item(*item);
        }

        self.finish()
    }

    fn finish(self) -> ResolveOutput {
        ResolveOutput {
            symbols: self.symbols,
            resolved_names: self.resolved_names,
            diagnostics: self.diagnostics,
        }
    }

    fn declare_item(&mut self, item: ItemId) {
        let ItemKind::Function(function) = &self.ast.item(item).kind else {
            return;
        };

        let id = FunctionId(self.symbols.functions.len());
        self.symbols.functions.push(FunctionSymbol {
            name: function.name.clone(),
            item,
        });

        if let Some(previous) = self.functions_by_name.get(&function.name.text) {
            let previous_name = &self.symbols.functions[previous.index()].name;
            self.diagnostics.push(
                Diagnostic::error(
                    "resolve.duplicate_function",
                    format!("function `{}` is already defined", function.name.text),
                    function.name.span,
                )
                .with_note("previous definition is here", Some(previous_name.span)),
            );
            return;
        }

        self.functions_by_name
            .insert(function.name.text.clone(), id);
    }

    fn resolve_item(&mut self, item: ItemId) {
        let ItemKind::Function(function) = &self.ast.item(item).kind else {
            return;
        };

        self.scopes.push(Scope::default());
        for (index, param) in function.params.iter().enumerate() {
            self.declare_param(item, index, &param.name);
        }
        for stmt in &function.body {
            self.resolve_stmt(*stmt);
        }
        self.scopes.pop();
    }

    fn resolve_stmt(&mut self, stmt: StmtId) {
        match &self.ast.stmt(stmt).kind {
            StmtKind::Expr(expr) | StmtKind::Print(expr) => self.resolve_expr(*expr),
            StmtKind::Local(local) => {
                self.resolve_expr(local.initializer);
                self.declare_local(local.kind, &local.name, LocalSource::Stmt(stmt));
            }
            StmtKind::Error(_) => {}
        }
    }

    fn declare_param(&mut self, item: ItemId, index: usize, name: &Ident) {
        self.declare_local(BindingKind::Let, name, LocalSource::Param { item, index });
    }

    fn declare_local(&mut self, kind: BindingKind, name: &Ident, source: LocalSource) {
        let id = LocalId(self.symbols.locals.len());
        self.symbols.locals.push(LocalSymbol {
            name: name.clone(),
            kind,
            source,
        });

        let Some(scope) = self.scopes.last_mut() else {
            return;
        };

        if let Some(previous) = scope.locals_by_name.get(&name.text) {
            let previous_name = &self.symbols.locals[previous.index()].name;
            self.diagnostics.push(
                Diagnostic::error(
                    "resolve.duplicate_local",
                    format!("local `{}` is already defined in this scope", name.text),
                    name.span,
                )
                .with_note("previous definition is here", Some(previous_name.span)),
            );
            return;
        }

        scope.locals_by_name.insert(name.text.clone(), id);
    }

    fn resolve_expr(&mut self, expr: ExprId) {
        match &self.ast.expr(expr).kind {
            ExprKind::Ident(name) => self.resolve_name(expr, name),
            ExprKind::Number(_) | ExprKind::Error(_) => {}
            ExprKind::Unary { expr, .. } => self.resolve_expr(*expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.resolve_expr(*lhs);
                self.resolve_expr(*rhs);
            }
        }
    }

    fn resolve_name(&mut self, expr: ExprId, name: &Ident) {
        if let Some(local) = self.lookup_local(&name.text) {
            self.resolved_names
                .set_expr(expr, ResolvedName::Local(local));
            return;
        }

        if let Some(function) = self.functions_by_name.get(&name.text) {
            self.resolved_names
                .set_expr(expr, ResolvedName::Function(*function));
            return;
        }

        self.diagnostics.push(Diagnostic::error(
            "resolve.undefined_name",
            format!("undefined name `{}`", name.text),
            name.span,
        ));
    }

    fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.locals_by_name.get(name).copied())
    }
}

#[derive(Debug, Default)]
struct Scope {
    locals_by_name: HashMap<String, LocalId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Function, Item, Module, Param, Stmt, TypeRef};
    use crate::diagnostics::Span;
    use crate::lexer::lex;
    use crate::parser::parse;

    #[test]
    fn resolves_local_identifiers() {
        let ast = parse_source("fn main():\n    var x = 1 + 2\n    let y = x * 3\n    print y\n");
        let output = resolve(&ast);

        assert!(output.diagnostics.is_empty());
        assert_eq!(output.symbols.functions.len(), 1);
        assert_eq!(output.symbols.locals.len(), 2);
        assert_eq!(output.symbols.locals[0].name.text, "x");
        assert_eq!(output.symbols.locals[1].name.text, "y");

        let LocalSource::Stmt(stmt) = output.symbols.locals[1].source else {
            panic!("expected statement local");
        };
        let y_initializer = match &ast.stmt(stmt).kind {
            StmtKind::Local(local) => local.initializer,
            _ => panic!("expected local statement"),
        };
        let ExprKind::Binary { lhs, .. } = ast.expr(y_initializer).kind else {
            panic!("expected binary initializer");
        };
        assert_eq!(
            output.resolved_names.expr(lhs),
            Some(ResolvedName::Local(LocalId(0)))
        );
    }

    #[test]
    fn reports_undefined_names() {
        let ast = parse_source("fn main():\n    print missing\n");
        let output = resolve(&ast);

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "resolve.undefined_name")
        );
    }

    #[test]
    fn reports_duplicate_local_names() {
        let ast = parse_source("fn main():\n    var x = 1\n    let x = 2\n    print x\n");
        let output = resolve(&ast);

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "resolve.duplicate_local")
        );
        assert_eq!(output.symbols.locals.len(), 2);
    }

    #[test]
    fn resolves_function_parameters() {
        let mut ast = Ast::new();
        let param = Param::new(
            Ident::new("x", Span::new(8, 9)),
            Some(TypeRef::named("int", Span::new(11, 14))),
        );
        let param_ref = ast.push_expr(Expr::new(
            ExprKind::Ident(Ident::new("x", Span::new(27, 28))),
            Span::new(27, 28),
        ));
        let stmt = ast.push_stmt(Stmt::new(StmtKind::Print(param_ref), Span::new(21, 28)));
        let item = ast.push_item(Item::new(
            ItemKind::Function(Function::new(
                Ident::new("main", Span::new(3, 7)),
                vec![param],
                None,
                vec![stmt],
            )),
            Span::new(0, 28),
        ));
        let module = ast.push_module(Module::new(vec![item], Span::new(0, 28)));
        ast.set_root(module);

        let output = resolve(&ast);

        assert!(output.diagnostics.is_empty());
        assert_eq!(output.symbols.locals.len(), 1);
        assert_eq!(
            output.symbols.locals[0].source,
            LocalSource::Param { item, index: 0 }
        );
        assert_eq!(
            output.resolved_names.expr(param_ref),
            Some(ResolvedName::Local(LocalId(0)))
        );
    }

    fn parse_source(source: &str) -> Ast {
        let output = parse(lex(source));
        assert!(output.diagnostics.is_empty());
        output.ast
    }
}
