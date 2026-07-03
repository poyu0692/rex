use crate::diagnostics::{Diagnostic, Severity, Span};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for LexOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "tokens:")?;
        for token in &self.tokens {
            writeln!(f, "  {token}")?;
        }

        if self.diagnostics.is_empty() {
            return f.write_str("diagnostics: none");
        }

        writeln!(f, "diagnostics:")?;
        for diagnostic in &self.diagnostics {
            writeln!(
                f,
                "  {}[{}] {} at {}..{}",
                severity_label(diagnostic.severity),
                diagnostic.code,
                diagnostic.message,
                diagnostic.span.start,
                diagnostic.span.end
            )?;

            for note in &diagnostic.notes {
                match note.span {
                    Some(span) => {
                        writeln!(
                            f,
                            "    note: {} at {}..{}",
                            note.message, span.start, span.end
                        )?;
                    }
                    None => {
                        writeln!(f, "    note: {}", note.message)?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn lex(source: &str) -> LexOutput {
    Lexer::new(source).lex()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{} {}", self.span.start, self.span.end, self.kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Integer(String),
    Float(String),
    String(String),
    Keyword(Keyword),
    Symbol(Symbol),
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    And,
    Async,
    Await,
    Break,
    Continue,
    Elif,
    Else,
    Enum,
    False,
    Fn,
    For,
    If,
    Impl,
    In,
    Let,
    Loop,
    Match,
    Not,
    Or,
    Return,
    SelfType,
    SelfValue,
    Struct,
    Trait,
    True,
    Var,
    While,
    Yield,
}

impl Keyword {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Async => "async",
            Self::Await => "await",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Elif => "elif",
            Self::Else => "else",
            Self::Enum => "enum",
            Self::False => "false",
            Self::Fn => "fn",
            Self::For => "for",
            Self::If => "if",
            Self::Impl => "impl",
            Self::In => "in",
            Self::Let => "let",
            Self::Loop => "loop",
            Self::Match => "match",
            Self::Not => "not",
            Self::Or => "or",
            Self::Return => "return",
            Self::SelfType => "Self",
            Self::SelfValue => "self",
            Self::Struct => "struct",
            Self::Trait => "trait",
            Self::True => "true",
            Self::Var => "var",
            Self::While => "while",
            Self::Yield => "yield",
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Ampersand,
    Bang,
    BangEqual,
    Caret,
    Colon,
    Comma,
    Dot,
    Equal,
    EqualEqual,
    FatArrow,
    Greater,
    GreaterEqual,
    LBrace,
    LBracket,
    Less,
    LessEqual,
    LParen,
    Minus,
    MinusEqual,
    Percent,
    PercentEqual,
    Pipe,
    Plus,
    PlusEqual,
    Question,
    RBrace,
    RBracket,
    RParen,
    Semicolon,
    Slash,
    SlashEqual,
    Star,
    StarEqual,
    ThinArrow,
}

impl Symbol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ampersand => "&",
            Self::Bang => "!",
            Self::BangEqual => "!=",
            Self::Caret => "^",
            Self::Colon => ":",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::Equal => "=",
            Self::EqualEqual => "==",
            Self::FatArrow => "=>",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::LBrace => "{",
            Self::LBracket => "[",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::LParen => "(",
            Self::Minus => "-",
            Self::MinusEqual => "-=",
            Self::Percent => "%",
            Self::PercentEqual => "%=",
            Self::Pipe => "|",
            Self::Plus => "+",
            Self::PlusEqual => "+=",
            Self::Question => "?",
            Self::RBrace => "}",
            Self::RBracket => "]",
            Self::RParen => ")",
            Self::Semicolon => ";",
            Self::Slash => "/",
            Self::SlashEqual => "/=",
            Self::Star => "*",
            Self::StarEqual => "*=",
            Self::ThinArrow => "->",
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier(name) => write!(f, "identifier `{name}`"),
            Self::Integer(value) => write!(f, "integer `{value}`"),
            Self::Float(value) => write!(f, "float `{value}`"),
            Self::String(value) => write!(f, "string {value:?}"),
            Self::Keyword(keyword) => write!(f, "`{keyword}`"),
            Self::Symbol(symbol) => write!(f, "`{symbol}`"),
            Self::Newline => f.write_str("newline"),
            Self::Indent => f.write_str("indent"),
            Self::Dedent => f.write_str("dedent"),
            Self::Eof => f.write_str("end of file"),
        }
    }
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
    at_line_start: bool,
    indents: Vec<usize>,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            line: 1,
            column: 1,
            at_line_start: true,
            indents: vec![0],
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lex(mut self) -> LexOutput {
        while !self.is_eof() {
            if self.at_line_start {
                self.consume_line_start();
                if self.at_line_start || self.is_eof() {
                    continue;
                }
            }

            let start = self.offset;
            match self.peek_char() {
                Some('\n') => self.consume_newline(true),
                Some(' ' | '\t') => self.consume_inline_whitespace(),
                Some('#') => self.consume_comment(),
                Some('"') => self.consume_string(),
                Some(ch) if ch.is_ascii_digit() => self.consume_number(),
                Some(ch) if is_identifier_start(ch) => self.consume_identifier_or_keyword(),
                Some(_) => self.consume_symbol_or_unknown(start),
                None => break,
            }
        }

        while self.indents.len() > 1 {
            self.indents.pop();
            self.push_token(TokenKind::Dedent, Span::at(self.offset));
        }

        self.push_token(TokenKind::Eof, Span::at(self.offset));

        LexOutput {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn consume_line_start(&mut self) {
        let start = self.offset;
        let mut indent = 0;
        let mut tab_start = None;

        loop {
            match self.peek_char() {
                Some(' ') => {
                    indent += 1;
                    self.next_char();
                }
                Some('\t') => {
                    tab_start.get_or_insert(self.offset);
                    indent += 4;
                    self.next_char();
                }
                _ => break,
            }
        }

        if let Some(tab_start) = tab_start {
            self.diagnostics.push(Diagnostic::error(
                "lex.tab_indent",
                "tabs are not allowed for indentation; use spaces",
                Span::new(tab_start, self.offset),
            ));
        }

        match self.peek_char() {
            Some('\n') => {
                self.consume_newline(false);
                return;
            }
            Some('#') => {
                self.consume_comment();
                if matches!(self.peek_char(), Some('\n')) {
                    self.consume_newline(false);
                }
                return;
            }
            None => return,
            _ => {}
        }

        self.apply_indentation(indent, Span::new(start, self.offset));
        self.at_line_start = false;
    }

    fn apply_indentation(&mut self, indent: usize, span: Span) {
        let current = *self.indents.last().expect("indent stack is never empty");

        if indent > current {
            self.indents.push(indent);
            self.push_token(TokenKind::Indent, span);
            return;
        }

        while indent < *self.indents.last().expect("indent stack is never empty") {
            self.indents.pop();
            self.push_token(TokenKind::Dedent, Span::at(span.end));
        }

        if indent != *self.indents.last().expect("indent stack is never empty") {
            self.diagnostics.push(Diagnostic::error(
                "lex.inconsistent_indent",
                "indentation does not match any previous indentation level",
                span,
            ));
            self.indents.push(indent);
        }
    }

    fn consume_inline_whitespace(&mut self) {
        while matches!(self.peek_char(), Some(' ' | '\t')) {
            let start = self.offset;
            let ch = self.next_char().expect("peeked whitespace");
            if ch == '\t' {
                self.diagnostics.push(Diagnostic::error(
                    "lex.tab",
                    "tabs are not allowed; use spaces",
                    Span::new(start, self.offset),
                ));
            }
        }
    }

    fn consume_comment(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.next_char();
        }
    }

    fn consume_newline(&mut self, emit_token: bool) {
        let start = self.offset;
        self.next_char();
        self.at_line_start = true;

        if emit_token {
            self.push_token(TokenKind::Newline, Span::new(start, self.offset));
        }
    }

    fn consume_identifier_or_keyword(&mut self) {
        let start = self.offset;
        self.next_char();

        while matches!(self.peek_char(), Some(ch) if is_identifier_continue(ch)) {
            self.next_char();
        }

        let text = &self.source[start..self.offset];
        let kind = match keyword(text) {
            Some(keyword) => TokenKind::Keyword(keyword),
            None => TokenKind::Identifier(text.to_owned()),
        };
        self.push_token(kind, Span::new(start, self.offset));
    }

    fn consume_number(&mut self) {
        let start = self.offset;

        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            self.next_char();
        }

        let is_float = matches!(self.peek_char(), Some('.'))
            && matches!(self.peek_next_char(), Some(ch) if ch.is_ascii_digit());

        if is_float {
            self.next_char();
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                self.next_char();
            }
        }

        let text = &self.source[start..self.offset];
        let kind = if is_float {
            TokenKind::Float(text.to_owned())
        } else {
            TokenKind::Integer(text.to_owned())
        };
        self.push_token(kind, Span::new(start, self.offset));
    }

    fn consume_string(&mut self) {
        let start = self.offset;
        self.next_char();
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            match ch {
                '"' => {
                    self.next_char();
                    self.push_token(TokenKind::String(value), Span::new(start, self.offset));
                    return;
                }
                '\n' => {
                    self.diagnostics.push(Diagnostic::error(
                        "lex.unterminated_string",
                        "string literal is missing a closing quote",
                        Span::new(start, self.offset),
                    ));
                    self.push_token(TokenKind::String(value), Span::new(start, self.offset));
                    return;
                }
                '\\' => {
                    let escape_start = self.offset;
                    self.next_char();
                    match self.next_char() {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('n') => value.push('\n'),
                        Some('r') => value.push('\r'),
                        Some('t') => value.push('\t'),
                        Some(other) => {
                            self.diagnostics.push(Diagnostic::warning(
                                "lex.unknown_escape",
                                format!("unknown escape sequence '\\{other}'"),
                                Span::new(escape_start, self.offset),
                            ));
                            value.push(other);
                        }
                        None => break,
                    }
                }
                _ => {
                    value.push(ch);
                    self.next_char();
                }
            }
        }

        self.diagnostics.push(Diagnostic::error(
            "lex.unterminated_string",
            "string literal is missing a closing quote",
            Span::new(start, self.offset),
        ));
        self.push_token(TokenKind::String(value), Span::new(start, self.offset));
    }

    fn consume_symbol_or_unknown(&mut self, start: usize) {
        let ch = self.next_char().expect("caller checked for a character");
        let kind = match ch {
            '(' => Some(Symbol::LParen),
            ')' => Some(Symbol::RParen),
            '{' => Some(Symbol::LBrace),
            '}' => Some(Symbol::RBrace),
            '[' => Some(Symbol::LBracket),
            ']' => Some(Symbol::RBracket),
            ':' => Some(Symbol::Colon),
            ',' => Some(Symbol::Comma),
            ';' => Some(Symbol::Semicolon),
            '?' => Some(Symbol::Question),
            '^' => Some(Symbol::Caret),
            '|' => Some(Symbol::Pipe),
            '&' => Some(Symbol::Ampersand),
            '+' => Some(if self.match_char('=') {
                Symbol::PlusEqual
            } else {
                Symbol::Plus
            }),
            '-' => Some(if self.match_char('>') {
                Symbol::ThinArrow
            } else if self.match_char('=') {
                Symbol::MinusEqual
            } else {
                Symbol::Minus
            }),
            '*' => Some(if self.match_char('=') {
                Symbol::StarEqual
            } else {
                Symbol::Star
            }),
            '/' => Some(if self.match_char('=') {
                Symbol::SlashEqual
            } else {
                Symbol::Slash
            }),
            '%' => Some(if self.match_char('=') {
                Symbol::PercentEqual
            } else {
                Symbol::Percent
            }),
            '=' => Some(if self.match_char('>') {
                Symbol::FatArrow
            } else if self.match_char('=') {
                Symbol::EqualEqual
            } else {
                Symbol::Equal
            }),
            '!' => Some(if self.match_char('=') {
                Symbol::BangEqual
            } else {
                Symbol::Bang
            }),
            '<' => Some(if self.match_char('=') {
                Symbol::LessEqual
            } else {
                Symbol::Less
            }),
            '>' => Some(if self.match_char('=') {
                Symbol::GreaterEqual
            } else {
                Symbol::Greater
            }),
            '.' => Some(Symbol::Dot),
            unknown => {
                self.diagnostics.push(Diagnostic::error(
                    "lex.unknown_character",
                    format!("unknown character '{unknown}'"),
                    Span::new(start, self.offset),
                ));
                None
            }
        };

        if let Some(symbol) = kind {
            self.push_token(TokenKind::Symbol(symbol), Span::new(start, self.offset));
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.next_char();
            true
        } else {
            false
        }
    }

    fn push_token(&mut self, kind: TokenKind, span: Span) {
        self.tokens.push(Token { kind, span });
    }

    fn is_eof(&self) -> bool {
        self.offset >= self.source.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.source[self.offset..].chars();
        chars.next()?;
        chars.next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(ch)
    }
}

fn keyword(text: &str) -> Option<Keyword> {
    let keyword = match text {
        "and" => Keyword::And,
        "async" => Keyword::Async,
        "await" => Keyword::Await,
        "break" => Keyword::Break,
        "continue" => Keyword::Continue,
        "elif" => Keyword::Elif,
        "else" => Keyword::Else,
        "enum" => Keyword::Enum,
        "false" => Keyword::False,
        "fn" => Keyword::Fn,
        "for" => Keyword::For,
        "if" => Keyword::If,
        "impl" => Keyword::Impl,
        "in" => Keyword::In,
        "let" => Keyword::Let,
        "loop" => Keyword::Loop,
        "match" => Keyword::Match,
        "not" => Keyword::Not,
        "or" => Keyword::Or,
        "return" => Keyword::Return,
        "Self" => Keyword::SelfType,
        "self" => Keyword::SelfValue,
        "struct" => Keyword::Struct,
        "trait" => Keyword::Trait,
        "true" => Keyword::True,
        "var" => Keyword::Var,
        "while" => Keyword::While,
        "yield" => Keyword::Yield,
        _ => return None,
    };

    Some(keyword)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source)
            .tokens
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn lexes_keywords_symbols_and_indentation() {
        let output = lex("fn main():\n    var a = 1\n    if true:\n        a += 1\n    a\n");

        assert!(output.diagnostics.is_empty());
        assert_eq!(
            output
                .tokens
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::Indent))
                .count(),
            2
        );
        assert_eq!(
            output
                .tokens
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::Dedent))
                .count(),
            2
        );
        assert!(
            output
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Keyword(Keyword::Fn))
        );
        assert!(
            output
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Symbol(Symbol::PlusEqual))
        );
    }

    #[test]
    fn consecutive_dots_stay_separate_symbols() {
        assert_eq!(
            kinds("for i in 1..=10:\n")
                .into_iter()
                .filter(|kind| !matches!(kind, TokenKind::Newline | TokenKind::Eof))
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Keyword(Keyword::For),
                TokenKind::Identifier("i".into()),
                TokenKind::Keyword(Keyword::In),
                TokenKind::Integer("1".into()),
                TokenKind::Symbol(Symbol::Dot),
                TokenKind::Symbol(Symbol::Dot),
                TokenKind::Symbol(Symbol::Equal),
                TokenKind::Integer("10".into()),
                TokenKind::Symbol(Symbol::Colon),
            ]
        );
    }

    #[test]
    fn displays_token_kinds_for_development_output() {
        assert_eq!(Keyword::Struct.to_string(), "struct");
        assert_eq!(Symbol::ThinArrow.to_string(), "->");
        assert_eq!(
            TokenKind::Identifier("enemy".into()).to_string(),
            "identifier `enemy`"
        );
        assert_eq!(TokenKind::Keyword(Keyword::Fn).to_string(), "`fn`");
        assert_eq!(TokenKind::Symbol(Symbol::FatArrow).to_string(), "`=>`");
        assert_eq!(
            TokenKind::String("a\nb".into()).to_string(),
            "string \"a\\nb\""
        );
    }

    #[test]
    fn displays_lex_output_for_development_output() {
        let rendered = lex("var x = \"unterminated\n").to_string();

        assert!(rendered.contains("tokens:"));
        assert!(rendered.contains("0..3 `var`"));
        assert!(rendered.contains("4..5 identifier `x`"));
        assert!(rendered.contains("diagnostics:"));
        assert!(rendered.contains("error[lex.unterminated_string]"));
    }

    #[test]
    fn reports_recoverable_string_errors() {
        let output = lex("println(\"missing)\nvar x = 1\n");

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "lex.unterminated_string")
        );
        assert!(
            output
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Keyword(Keyword::Var))
        );
    }

}
