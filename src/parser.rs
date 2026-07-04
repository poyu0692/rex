use crate::ast::{
    Ast, BinaryOp, Error, Expr, ExprId, ExprKind, Function, Ident, Item, ItemKind, Module,
    NumberLiteral, Stmt, StmtId, StmtKind, UnaryOp,
};
use crate::diagnostics::{Diagnostic, Span};
use crate::lexer::{Keyword, Symbol, Token, TokenKind, lex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    pub ast: Ast,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &str) -> ParseOutput {
    let output = lex(source);
    Parser::new(output.tokens, output.diagnostics).parse()
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

    fn parse_module(&mut self) -> crate::ast::ModuleId {
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

    fn parse_item(&mut self) -> crate::ast::ItemId {
        if self.at_keyword(Keyword::Fn) {
            return self.parse_function_item();
        }

        let span = self.current().span;
        self.error("parse.expected_item", "expected top-level item", span);
        self.synchronize_to_item_boundary();
        self.ast
            .push_item(Item::new(ItemKind::Error(Error::new()), span))
    }

    fn parse_function_item(&mut self) -> crate::ast::ItemId {
        let start = self.expect_keyword(Keyword::Fn).span.start;
        let name = self.expect_identifier("expected function name");

        self.expect_symbol(Symbol::LParen, "expected `(` after function name");
        if !self.at_symbol(Symbol::RParen) {
            self.error(
                "parse.unsupported_params",
                "function parameters are not parsed yet",
                self.current().span,
            );
            self.synchronize_until(&[
                TokenStop::Symbol(Symbol::RParen),
                TokenStop::Symbol(Symbol::Colon),
                TokenStop::Newline,
                TokenStop::Eof,
            ]);
        }
        self.expect_symbol(Symbol::RParen, "expected `)` after function parameters");
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
            ItemKind::Function(Function::new(name, Vec::new(), None, body)),
            Span::new(start, end),
        ))
    }

    fn parse_stmt(&mut self) -> StmtId {
        self.skip_newlines();
        let stmt = if self.at_identifier("print") {
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

    fn synchronize_until(&mut self, stops: &[TokenStop]) {
        while !self.at_eof() && !stops.iter().any(|stop| stop.matches(self.current())) {
            self.advance();
        }
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

#[derive(Debug, Clone, Copy)]
enum TokenStop {
    Symbol(Symbol),
    Newline,
    Eof,
}

impl TokenStop {
    fn matches(self, token: &Token) -> bool {
        match self {
            Self::Symbol(expected) => {
                matches!(token.kind, TokenKind::Symbol(current) if current == expected)
            }
            Self::Newline => matches!(token.kind, TokenKind::Newline),
            Self::Eof => matches!(token.kind, TokenKind::Eof),
        }
    }
}

fn merge_spans(lhs: Span, rhs: Span) -> Span {
    Span::new(lhs.start, rhs.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_function_with_temporary_print_statement() {
        let output = parse("fn main():\n    print 1 + 2 * 3\n");

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
        let output = parse("fn main():\n    print -1 + 2\n");

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
    fn reports_invalid_top_level_tokens() {
        let output = parse("1 + 2\n");

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "parse.expected_item")
        );
    }

    fn only_function(output: &ParseOutput) -> &Function {
        let module = output.ast.root().expect("root module");
        let ItemKind::Function(function) = &output.ast.item(module.items[0]).kind else {
            panic!("expected function item");
        };
        function
    }
}
