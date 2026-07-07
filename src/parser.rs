use crate::ast::{
    Ast, BinaryOp, BindingKind, Error, Expr, ExprId, ExprKind, Function, Ident, Item, ItemId,
    ItemKind, Local, Module, ModuleId, NumberLiteral, Param, Stmt, StmtId, StmtKind, TypeRef,
    TypeRefKind, UnaryOp,
};
use crate::diagnostics::{Diagnostic, Span};
use crate::lexer::{Keyword, LexOutput, Symbol, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    pub ast: Ast,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(input: LexOutput) -> ParseOutput {
    Parser::new(input.tokens, input.diagnostics).parse()
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    ast: Ast,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(tokens: Vec<Token>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            tokens,
            position: 0,
            ast: Ast::new(),
            diagnostics,
        }
    }

    fn parse(mut self) -> ParseOutput {
        let root = self.parse_module();
        self.ast.set_root(root);

        ParseOutput {
            ast: self.ast,
            diagnostics: self.diagnostics,
        }
    }

    fn parse_module(&mut self) -> ModuleId {
        self.skip_newlines();
        let start = self.current().span.start;
        let mut items = Vec::new();

        while !self.at_eof() {
            self.skip_newlines();
            if self.at_eof() {
                break;
            }

            items.push(self.parse_item());
        }

        let end = self.current().span.end;
        self.ast
            .push_module(Module::new(items, Span::new(start, end)))
    }

    fn parse_item(&mut self) -> ItemId {
        if self.at_keyword(Keyword::Fn) {
            return self.parse_function_item();
        }

        let span = self.current().span;
        self.error("parse.expected_item", "expected top-level item", span);
        self.synchronize_to_item_boundary();
        self.ast
            .push_item(Item::new(ItemKind::Error(Error::new()), span))
    }

    fn parse_function_item(&mut self) -> ItemId {
        let start = self.expect_keyword(Keyword::Fn).span.start;
        let name = self.expect_identifier("expected function name");

        self.expect_symbol(Symbol::LParen, "expected `(` after function name");
        let params = self.parse_param_list();
        self.expect_symbol(Symbol::RParen, "expected `)` after function parameters");
        let return_type =
            if self.match_token(|kind| matches!(kind, TokenKind::Symbol(Symbol::ThinArrow))) {
                Some(self.parse_type_ref())
            } else {
                None
            };
        self.expect_symbol(Symbol::Colon, "expected `:` after function signature");
        self.expect_newline("expected newline after function signature");

        let mut body = Vec::new();
        if self.match_token(|kind| matches!(kind, TokenKind::Indent)) {
            while !self.at_eof() && !self.at_token(|kind| matches!(kind, TokenKind::Dedent)) {
                self.skip_newlines();
                if self.at_eof() || self.at_token(|kind| matches!(kind, TokenKind::Dedent)) {
                    break;
                }
                body.push(self.parse_stmt());
            }
            self.expect_dedent("expected dedent after function body");
        } else {
            self.error(
                "parse.expected_indent",
                "expected indented function body",
                self.current().span,
            );
        }

        let end = body
            .last()
            .map(|stmt| self.ast.stmt(*stmt).span.end)
            .unwrap_or(name.span.end);
        self.ast.push_item(Item::new(
            ItemKind::Function(Function::new(name, params, return_type, body)),
            Span::new(start, end),
        ))
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        let mut params = Vec::new();

        while !self.at_param_list_end() {
            let name = self.expect_identifier("expected parameter name");
            let ty = if self.match_token(|kind| matches!(kind, TokenKind::Symbol(Symbol::Colon))) {
                Some(self.parse_type_ref())
            } else {
                self.error(
                    "parse.expected_param_type",
                    "expected `:` and parameter type after parameter name",
                    self.current().span,
                );
                None
            };
            params.push(Param::new(name, ty));

            if !self.match_token(|kind| matches!(kind, TokenKind::Symbol(Symbol::Comma))) {
                break;
            }
        }

        params
    }

    fn parse_stmt(&mut self) -> StmtId {
        self.skip_newlines();
        let stmt = if self.at_keyword(Keyword::Let) || self.at_keyword(Keyword::Var) {
            self.parse_local_stmt()
        } else if self.at_identifier("print") {
            let start = self.advance().span.start;
            let expr = self.parse_expr();
            let span = Span::new(start, self.ast.expr(expr).span.end);
            Stmt::new(StmtKind::Print(expr), span)
        } else {
            let expr = self.parse_expr();
            let span = self.ast.expr(expr).span;
            Stmt::new(StmtKind::Expr(expr), span)
        };

        let id = self.ast.push_stmt(stmt);
        self.finish_stmt();
        id
    }

    fn parse_local_stmt(&mut self) -> Stmt {
        let token = self.advance();
        let kind = match token.kind {
            TokenKind::Keyword(Keyword::Let) => BindingKind::Let,
            TokenKind::Keyword(Keyword::Var) => BindingKind::Var,
            _ => unreachable!("caller checked for a local binding keyword"),
        };
        let name = self.expect_identifier("expected variable name");
        let ty = if self.match_token(|kind| matches!(kind, TokenKind::Symbol(Symbol::Colon))) {
            Some(self.parse_type_ref())
        } else {
            None
        };
        self.expect_symbol(Symbol::Equal, "expected `=` after variable name");
        let initializer = self.parse_expr();
        let span = Span::new(token.span.start, self.ast.expr(initializer).span.end);

        Stmt::new(
            StmtKind::Local(Local::new(kind, name, ty, initializer)),
            span,
        )
    }

    fn parse_type_ref(&mut self) -> TypeRef {
        let token = self.current().clone();
        let mut ty = match token.kind {
            TokenKind::Identifier(text) => {
                self.advance();
                TypeRef::named_ident(Ident::new(text, token.span))
            }
            TokenKind::Keyword(Keyword::SelfType) => {
                self.advance();
                TypeRef::named_ident(Ident::new("Self", token.span))
            }
            _ => {
                self.error("parse.expected_type", "expected type", token.span);
                if !matches!(
                    token.kind,
                    TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
                ) {
                    self.advance();
                }
                TypeRef::named("<error>", token.span)
            }
        };

        while self.match_token(|kind| matches!(kind, TokenKind::Symbol(Symbol::LBracket))) {
            let mut args = Vec::new();
            while !self.at_type_arg_list_end() {
                args.push(self.parse_type_ref());
                if !self.match_token(|kind| matches!(kind, TokenKind::Symbol(Symbol::Comma))) {
                    break;
                }
            }

            let end = self
                .expect_symbol(Symbol::RBracket, "expected `]` after type arguments")
                .map(|token| token.span.end)
                .or_else(|| args.last().map(|arg| arg.span.end))
                .unwrap_or(ty.span.end);
            let span = Span::new(ty.span.start, end);
            ty = TypeRef::new(
                TypeRefKind::Generic {
                    base: Box::new(ty),
                    args,
                },
                span,
            );
        }

        ty
    }

    fn parse_expr(&mut self) -> ExprId {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> ExprId {
        let mut lhs = self.parse_prefix();

        loop {
            let Some((left_bp, right_bp, op)) = self.current_infix() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }

            self.advance();
            let rhs = self.parse_expr_bp(right_bp);
            let span = merge_spans(self.ast.expr(lhs).span, self.ast.expr(rhs).span);
            lhs = self
                .ast
                .push_expr(Expr::new(ExprKind::Binary { lhs, op, rhs }, span));
        }

        lhs
    }

    fn parse_prefix(&mut self) -> ExprId {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Integer(value) => {
                self.advance();
                self.ast.push_expr(Expr::new(
                    ExprKind::Number(NumberLiteral::Integer(value)),
                    token.span,
                ))
            }
            TokenKind::Float(value) => {
                self.advance();
                self.ast.push_expr(Expr::new(
                    ExprKind::Number(NumberLiteral::Float(value)),
                    token.span,
                ))
            }
            TokenKind::Identifier(text) => {
                self.advance();
                self.ast.push_expr(Expr::new(
                    ExprKind::Ident(Ident::new(text, token.span)),
                    token.span,
                ))
            }
            TokenKind::Symbol(Symbol::Minus) => {
                self.advance();
                let expr = self.parse_expr_bp(30);
                let span = merge_spans(token.span, self.ast.expr(expr).span);
                self.ast.push_expr(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr,
                    },
                    span,
                ))
            }
            TokenKind::Symbol(Symbol::LParen) => {
                self.advance();
                let expr = self.parse_expr();
                self.expect_symbol(Symbol::RParen, "expected `)` after expression");
                expr
            }
            _ => {
                self.error("parse.expected_expr", "expected expression", token.span);
                if !matches!(
                    token.kind,
                    TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
                ) {
                    self.advance();
                }
                self.ast
                    .push_expr(Expr::new(ExprKind::Error(Error::new()), token.span))
            }
        }
    }

    fn current_infix(&self) -> Option<(u8, u8, BinaryOp)> {
        let TokenKind::Symbol(symbol) = self.current().kind else {
            return None;
        };

        match symbol {
            Symbol::Plus => Some((10, 11, BinaryOp::Add)),
            Symbol::Minus => Some((10, 11, BinaryOp::Sub)),
            Symbol::Star => Some((20, 21, BinaryOp::Mul)),
            Symbol::Slash => Some((20, 21, BinaryOp::Div)),
            Symbol::Percent => Some((20, 21, BinaryOp::Rem)),
            _ => None,
        }
    }

    fn finish_stmt(&mut self) {
        if self.match_token(|kind| matches!(kind, TokenKind::Newline)) {
            return;
        }

        if self.at_eof() || self.at_token(|kind| matches!(kind, TokenKind::Dedent)) {
            return;
        }

        self.error(
            "parse.expected_newline",
            "expected newline after statement",
            self.current().span,
        );
        self.synchronize_to_stmt_boundary();
        self.match_token(|kind| matches!(kind, TokenKind::Newline));
    }

    fn skip_newlines(&mut self) {
        while self.match_token(|kind| matches!(kind, TokenKind::Newline)) {}
    }

    fn synchronize_to_stmt_boundary(&mut self) {
        while !self.at_eof()
            && !self.at_token(|kind| matches!(kind, TokenKind::Newline | TokenKind::Dedent))
        {
            self.advance();
        }
    }

    fn synchronize_to_item_boundary(&mut self) {
        while !self.at_eof()
            && !self.at_token(|kind| matches!(kind, TokenKind::Newline | TokenKind::Dedent))
        {
            self.advance();
        }
        self.skip_newlines();
        self.match_token(|kind| matches!(kind, TokenKind::Dedent));
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Token {
        if self.at_keyword(keyword) {
            return self.advance();
        }

        let token = self.current().clone();
        self.error(
            "parse.expected_keyword",
            format!("expected `{}`", keyword.as_str()),
            token.span,
        );
        token
    }

    fn expect_identifier(&mut self, message: &'static str) -> Ident {
        let token = self.current().clone();
        if let TokenKind::Identifier(text) = token.kind {
            self.advance();
            return Ident::new(text, token.span);
        }

        self.error("parse.expected_identifier", message, token.span);
        Ident::new("<error>", token.span)
    }

    fn expect_symbol(&mut self, symbol: Symbol, message: &'static str) -> Option<Token> {
        if self.at_symbol(symbol) {
            return Some(self.advance());
        }

        self.error("parse.expected_symbol", message, self.current().span);
        None
    }

    fn expect_newline(&mut self, message: &'static str) -> Option<Token> {
        if self.at_token(|kind| matches!(kind, TokenKind::Newline)) {
            return Some(self.advance());
        }

        self.error("parse.expected_newline", message, self.current().span);
        None
    }

    fn expect_dedent(&mut self, message: &'static str) -> Option<Token> {
        if self.at_token(|kind| matches!(kind, TokenKind::Dedent)) {
            return Some(self.advance());
        }

        self.error("parse.expected_dedent", message, self.current().span);
        None
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.current().kind, TokenKind::Keyword(current) if current == keyword)
    }

    fn at_identifier(&self, expected: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(current) if current == expected)
    }

    fn at_symbol(&self, symbol: Symbol) -> bool {
        matches!(self.current().kind, TokenKind::Symbol(current) if current == symbol)
    }

    fn at_param_list_end(&self) -> bool {
        self.at_eof()
            || self.at_symbol(Symbol::RParen)
            || self.at_token(|kind| matches!(kind, TokenKind::Newline))
    }

    fn at_type_arg_list_end(&self) -> bool {
        self.at_eof()
            || self.at_symbol(Symbol::RBracket)
            || self.at_token(|kind| matches!(kind, TokenKind::Newline))
    }

    fn at_token(&self, predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
        predicate(&self.current().kind)
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn match_token(&mut self, predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
        if self.at_token(predicate) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.position += 1;
        }
        token
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.position)
            .or_else(|| self.tokens.last())
            .expect("lexer always emits EOF")
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }
}

fn merge_spans(lhs: Span, rhs: Span) -> Span {
    Span::new(lhs.start, rhs.end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn parses_function_with_temporary_print_statement() {
        let output = parse_source("fn main():\n    print 1 + 2 * 3\n");

        assert!(output.diagnostics.is_empty());
        let module = output.ast.root().expect("root module");
        assert_eq!(module.items.len(), 1);

        let ItemKind::Function(function) = &output.ast.item(module.items[0]).kind else {
            panic!("expected function item");
        };
        assert_eq!(function.name.text, "main");
        assert_eq!(function.body.len(), 1);

        let StmtKind::Print(expr) = output.ast.stmt(function.body[0]).kind else {
            panic!("expected print statement");
        };
        let ExprKind::Binary {
            lhs,
            op: BinaryOp::Add,
            rhs,
        } = output.ast.expr(expr).kind
        else {
            panic!("expected addition expression");
        };

        assert!(matches!(
            output.ast.expr(lhs).kind,
            ExprKind::Number(NumberLiteral::Integer(ref value)) if value == "1"
        ));
        assert!(matches!(
            output.ast.expr(rhs).kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn parses_unary_minus_with_higher_precedence_than_addition() {
        let output = parse_source("fn main():\n    print -1 + 2\n");

        assert!(output.diagnostics.is_empty());
        let function = only_function(&output);
        let StmtKind::Print(expr) = output.ast.stmt(function.body[0]).kind else {
            panic!("expected print statement");
        };
        let ExprKind::Binary {
            lhs,
            op: BinaryOp::Add,
            ..
        } = output.ast.expr(expr).kind
        else {
            panic!("expected addition expression");
        };

        assert!(matches!(
            output.ast.expr(lhs).kind,
            ExprKind::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn parses_local_bindings_and_identifier_expressions() {
        let output =
            parse_source("fn main():\n    var x = 1 + 2\n    let y = x * 3\n    print y\n");

        assert!(output.diagnostics.is_empty());
        let function = only_function(&output);
        assert_eq!(function.body.len(), 3);

        let StmtKind::Local(local) = &output.ast.stmt(function.body[0]).kind else {
            panic!("expected local binding");
        };
        assert_eq!(local.kind, BindingKind::Var);
        assert_eq!(local.name.text, "x");
        assert_eq!(local.ty, None);
        assert!(matches!(
            output.ast.expr(local.initializer).kind,
            ExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));

        let StmtKind::Local(local) = &output.ast.stmt(function.body[1]).kind else {
            panic!("expected local binding");
        };
        assert_eq!(local.kind, BindingKind::Let);
        assert_eq!(local.name.text, "y");
        assert_eq!(local.ty, None);

        let StmtKind::Print(expr) = output.ast.stmt(function.body[2]).kind else {
            panic!("expected print statement");
        };
        assert!(matches!(
            &output.ast.expr(expr).kind,
            ExprKind::Ident(ident) if ident.text == "y"
        ));
    }

    #[test]
    fn parses_function_parameter_and_return_types() {
        let output = parse_source("fn add(x: int, y: float) -> float:\n    print x\n");

        assert!(output.diagnostics.is_empty());
        let function = only_function(&output);

        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].name.text, "x");
        assert_eq!(
            named_type_text(function.params[0].ty.as_ref().expect("param type")),
            "int"
        );
        assert_eq!(function.params[1].name.text, "y");
        assert_eq!(
            named_type_text(function.params[1].ty.as_ref().expect("param type")),
            "float"
        );
        assert_eq!(
            named_type_text(function.return_type.as_ref().expect("return type")),
            "float"
        );
    }

    #[test]
    fn parses_local_type_annotations() {
        let output = parse_source("fn main():\n    var enemy: Enemy = 1\n");

        assert!(output.diagnostics.is_empty());
        let function = only_function(&output);
        let StmtKind::Local(local) = &output.ast.stmt(function.body[0]).kind else {
            panic!("expected local binding");
        };

        assert_eq!(local.name.text, "enemy");
        assert_eq!(
            named_type_text(local.ty.as_ref().expect("local type")),
            "Enemy"
        );
    }

    #[test]
    fn parses_generic_type_refs() {
        let output = parse_source("fn main(values: Vec[int]):\n    print values\n");

        assert!(output.diagnostics.is_empty());
        let function = only_function(&output);
        let ty = function.params[0].ty.as_ref().expect("param type");

        let TypeRefKind::Generic { base, args } = &ty.kind else {
            panic!("expected generic type");
        };
        assert_eq!(named_type_text(base), "Vec");
        assert_eq!(args.len(), 1);
        assert_eq!(named_type_text(&args[0]), "int");
    }

    #[test]
    fn reports_invalid_top_level_tokens() {
        let output = parse_source("1 + 2\n");

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "parse.expected_item")
        );
    }

    fn parse_source(source: &str) -> ParseOutput {
        parse(lex(source))
    }

    fn only_function(output: &ParseOutput) -> &Function {
        let module = output.ast.root().expect("root module");
        let ItemKind::Function(function) = &output.ast.item(module.items[0]).kind else {
            panic!("expected function item");
        };
        function
    }

    fn named_type_text(ty: &TypeRef) -> &str {
        let TypeRefKind::Named(name) = &ty.kind else {
            panic!("expected named type");
        };
        &name.text
    }
}
