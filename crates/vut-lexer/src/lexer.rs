//! Indentation-aware, allocation-conscious Vut lexer.
use crate::{Token, TokenKind};
use vut_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, codes};
use vut_source::{SourceId, Span};

pub struct Lexer<'a> {
    source: SourceId,
    text: &'a str,
    position: usize,
    line_start: bool,
    indentation: Vec<usize>,
    delimiters: u32,
    /// Indentation column of the line that opened an indented block while
    /// still inside parentheses; `None` when not inside such a block.
    block_floor: Option<usize>,
    /// Leading-space count of the current physical line.
    line_indent: usize,
    tokens: Vec<Token>,
    diagnostics: DiagnosticSink,
    in_import_path: bool,
}
impl<'a> Lexer<'a> {
    #[must_use]
    pub fn new(source: SourceId, text: &'a str) -> Self {
        Self {
            source,
            text,
            position: 0,
            line_start: true,
            indentation: vec![0],
            delimiters: 0,
            block_floor: None,
            line_indent: 0,
            tokens: Vec::new(),
            diagnostics: DiagnosticSink::new(),
            in_import_path: false,
        }
    }
    #[must_use]
    pub fn lex(mut self) -> (Vec<Token>, DiagnosticSink) {
        while self.position < self.text.len() {
            self.scan();
        }
        if self.delimiters != 0 {
            self.error(
                codes::E0001,
                Span::new(self.source, self.position, self.position),
                "unclosed parenthesis",
            );
        }
        while self.indentation.len() > 1 {
            self.indentation.pop();
            self.emit(TokenKind::Dedent, self.position, self.position);
        }
        self.emit(TokenKind::Eof, self.position, self.position);
        (self.tokens, self.diagnostics)
    }
    fn scan(&mut self) {
        if self.line_start && !self.begin_line() {
            return;
        }
        let start = self.position;
        let byte = self.current();
        match byte {
            b' ' | b'\t' | b'\r' => {
                if self.line_start {
                    self.line_indent += 1;
                }
                self.position += 1;
            }
            b'\n' => self.newline(start),
            b'#' => self.comment(),
            b'"' => self.string(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.identifier(),
            b'0'..=b'9' => self.number(),
            b'+' => self.one(TokenKind::Plus),
            b'-' => {
                self.position += 1;
                if self.take(b'>') {
                    self.emit(TokenKind::Arrow, start, self.position);
                } else {
                    self.emit(TokenKind::Minus, start, self.position);
                }
            }
            b'*' => self.one(TokenKind::Star),
            b'/' => self.one(TokenKind::Slash),
            b'%' => self.one(TokenKind::Percent),
            b'=' => self.equal(start),
            b'!' => {
                self.position += 1;
                if self.take(b'=') {
                    self.emit(TokenKind::BangEqual, start, self.position);
                } else {
                    self.error(
                        codes::E0001,
                        Span::new(self.source, start, self.position),
                        "invalid token `!`",
                    );
                    self.emit(TokenKind::Error, start, self.position);
                }
            }
            b'>' => self.pair(b'=', TokenKind::GreaterEqual, TokenKind::Greater),
            b'<' => self.pair(b'=', TokenKind::LessEqual, TokenKind::Less),
            b':' => self.one(TokenKind::Colon),
            b'?' => self.one(TokenKind::Question),
            b',' => self.one(TokenKind::Comma),
            b'.' => {
                self.position += 1;
                if self.in_import_path {
                    while self.take(b'.') {}
                    self.emit(TokenKind::RelativeDots, start, self.position);
                } else if self.take(b'.') {
                    if self.take(b'.') {
                        self.emit(TokenKind::Ellipsis, start, self.position);
                    } else if self.take(b'=') {
                        self.emit(TokenKind::RangeInclusive, start, self.position);
                    } else {
                        self.emit(TokenKind::Range, start, self.position);
                    }
                } else {
                    self.emit(TokenKind::Dot, start, self.position);
                }
            }
            b'(' => {
                self.position += 1;
                self.delimiters += 1;
                self.emit(TokenKind::LParen, start, self.position);
            }
            b')' => {
                self.position += 1;
                if self.delimiters == 0 {
                    self.error(
                        codes::E0001,
                        Span::new(self.source, start, self.position),
                        "unmatched `)`",
                    );
                } else {
                    self.delimiters -= 1;
                }
                self.emit(TokenKind::RParen, start, self.position);
            }
            b'@' => self.one(TokenKind::AtSign),
            _ => {
                self.position += self.char_len();
                self.error(
                    codes::E0001,
                    Span::new(self.source, start, self.position),
                    "invalid character",
                );
                self.emit(TokenKind::Error, start, self.position);
            }
        }
    }
    /// Handles leading indentation when a physical line begins. Returns
    /// `false` when the input has been exhausted.
    fn begin_line(&mut self) -> bool {
        if self.delimiters != 0 && self.block_floor.is_none() {
            return true;
        }
        let width = self.indentation();
        if self.block_floor.is_some_and(|floor| width <= floor) {
            self.block_floor = None;
        }
        self.position != self.text.len()
    }
    /// Consumes the current line terminator, emitting `Newline` and, when
    /// inside parentheses, opening an indented block after a trailing `:`.
    fn newline(&mut self, start: usize) {
        self.position += 1;
        if self.delimiters == 0 || self.block_floor.is_some() {
            self.emit(TokenKind::Newline, start, self.position);
        } else if self.last_significant() == Some(TokenKind::Colon) {
            self.block_floor = Some(self.line_indent);
            self.emit(TokenKind::Newline, start, self.position);
        }
        self.line_start = true;
        self.line_indent = 0;
    }
    /// Scans `=`, `==`, or the fat arrow `=>`.
    fn equal(&mut self, start: usize) {
        self.position += 1;
        if self.take(b'=') {
            self.emit(TokenKind::EqualEqual, start, self.position);
        } else if self.take(b'>') {
            self.emit(TokenKind::FatArrow, start, self.position);
        } else {
            self.emit(TokenKind::Equal, start, self.position);
        }
    }
    fn indentation(&mut self) -> usize {
        let start = self.position;
        let mut width = 0;
        while self.position < self.text.len() && self.current() == b' ' {
            self.position += 1;
            width += 1;
        }
        self.line_indent = width;
        if self.position < self.text.len() && self.current() == b'\t' {
            let end = self.position + 1;
            self.position = end;
            self.error(
                codes::E0107,
                Span::new(self.source, start, end),
                "tabs are not valid indentation",
            );
            self.line_start = false;
            return width;
        }
        let blank_crlf = self.position + 1 < self.text.len()
            && self.current() == b'\r'
            && self.text.as_bytes()[self.position + 1] == b'\n';
        if self.position == self.text.len()
            || self.current() == b'\n'
            || blank_crlf
            || self.current() == b'#'
        {
            self.line_start = false;
            return width;
        }
        let current = *self.indentation.last().unwrap_or(&0);
        if width > current {
            self.indentation.push(width);
            self.emit(TokenKind::Indent, start, self.position);
        } else if width < current {
            while self.indentation.last().is_some_and(|level| *level > width) {
                self.indentation.pop();
                self.emit(TokenKind::Dedent, start, self.position);
            }
            if self.indentation.last() != Some(&width) {
                self.error(
                    codes::E0107,
                    Span::new(self.source, start, self.position),
                    "inconsistent indentation",
                );
            }
        }
        self.line_start = false;
        width
    }
    fn identifier(&mut self) {
        let start = self.position;
        while self.position < self.text.len()
            && (self.current().is_ascii_alphanumeric() || self.current() == b'_')
        {
            self.position += 1;
        }
        let kind = match &self.text[start..self.position] {
            "fn" => TokenKind::Fn,
            "data" => TokenKind::Data,
            "interface" => TokenKind::Interface,
            "enum" => TokenKind::Enum,
            "type" => TokenKind::Type,
            "extern" => TokenKind::Extern,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "static" => TokenKind::Static,
            "vut" => TokenKind::Vut,
            "if" => TokenKind::If,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "at" => TokenKind::At,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "dyn" => TokenKind::Dyn,
            "opaque" => TokenKind::Opaque,
            "unsafe" => TokenKind::Unsafe,
            _ => TokenKind::Identifier,
        };
        if kind == TokenKind::Import {
            self.in_import_path = true;
        } else if self.in_import_path && kind == TokenKind::Identifier {
            self.in_import_path = false;
        }
        self.emit(kind, start, self.position);
    }
    fn number(&mut self) {
        let start = self.position;
        while self.position < self.text.len() && self.current().is_ascii_digit() {
            self.position += 1;
        }
        let kind = if self.position + 1 < self.text.len()
            && self.current() == b'.'
            && self.text.as_bytes()[self.position + 1].is_ascii_digit()
        {
            self.position += 1;
            while self.position < self.text.len() && self.current().is_ascii_digit() {
                self.position += 1;
            }
            TokenKind::Float
        } else {
            TokenKind::Integer
        };
        if self.position < self.text.len()
            && (self.current().is_ascii_alphabetic() || self.current() == b'_')
        {
            while self.position < self.text.len()
                && (self.current().is_ascii_alphanumeric() || self.current() == b'_')
            {
                self.position += 1;
            }
            self.error(
                codes::E0004,
                Span::new(self.source, start, self.position),
                "invalid numeric literal",
            );
            self.emit(TokenKind::Error, start, self.position);
            return;
        }
        self.emit(kind, start, self.position);
    }
    fn string(&mut self) {
        let start = self.position;
        self.position += 1;
        self.emit(TokenKind::StringStart, start, self.position);
        let mut text_start = self.position;
        while self.position < self.text.len() {
            let byte = self.current();
            if byte == b'\\' {
                self.position += 1;
                if self.position == self.text.len() || self.current() == b'\n' {
                    break;
                }
                let escape_start = self.position - 1;
                let escaped = self.current();
                self.position += self.char_len();
                if !matches!(escaped, b'\\' | b'"' | b'n' | b'r' | b't' | b'0' | b'$') {
                    self.error(
                        codes::E0003,
                        Span::new(self.source, escape_start, self.position),
                        "invalid string escape",
                    );
                }
            } else if byte == b'"' {
                self.emit_text(text_start, self.position);
                let quote = self.position;
                self.position += 1;
                self.emit(TokenKind::StringEnd, quote, self.position);
                return;
            } else if byte == b'\n' {
                break;
            } else if byte == b'$' {
                self.emit_text(text_start, self.position);
                self.interpolation();
                text_start = self.position;
            } else {
                self.position += self.char_len();
            }
        }
        self.emit_text(text_start, self.position);
        self.error(
            codes::E0002,
            Span::new(self.source, start, self.position),
            "unterminated string",
        );
        self.emit(TokenKind::Error, start, self.position);
    }

    fn interpolation(&mut self) {
        let start = self.position;
        self.position += 1;
        if self.position < self.text.len()
            && (self.current().is_ascii_alphabetic() || self.current() == b'_')
        {
            while self.position < self.text.len()
                && (self.current().is_ascii_alphanumeric() || self.current() == b'_')
            {
                self.position += 1;
            }
            self.emit(TokenKind::InterpolationIdentifier, start, self.position);
            return;
        }
        if !self.take(b'(') {
            self.error(
                codes::E0005,
                Span::new(self.source, start, self.position),
                "invalid template interpolation",
            );
            self.emit(TokenKind::Error, start, self.position);
            return;
        }
        self.emit(TokenKind::InterpolationStart, start, self.position);
        let expression_start = self.position;
        let expression_token_start = self.tokens.len();
        let baseline = self.delimiters;
        self.delimiters += 1;
        while self.position < self.text.len() {
            if self.current() == b'"' || self.current() == b'\n' {
                self.error(
                    codes::E0006,
                    Span::new(self.source, start, self.position),
                    "unclosed template interpolation",
                );
                self.delimiters = baseline;
                return;
            }
            if self.current() == b')' && self.delimiters == baseline + 1 {
                if self.tokens.len() == expression_token_start {
                    self.error(
                        codes::E0007,
                        Span::new(self.source, expression_start, self.position),
                        "template interpolation requires an expression",
                    );
                }
                let end = self.position;
                self.position += 1;
                self.delimiters = baseline;
                self.emit(TokenKind::InterpolationEnd, end, self.position);
                return;
            }
            self.scan_code();
        }
        self.error(
            codes::E0006,
            Span::new(self.source, start, self.position),
            "unclosed template interpolation",
        );
        self.delimiters = baseline;
    }

    fn scan_code(&mut self) {
        let was_line_start = self.line_start;
        self.line_start = false;
        self.scan();
        self.line_start = was_line_start;
    }

    fn emit_text(&mut self, start: usize, end: usize) {
        if start < end {
            self.emit(TokenKind::StringText, start, end);
        }
    }
    fn comment(&mut self) {
        let start = self.position;
        if self.text[start..].starts_with("###") {
            self.position += 3;
            while self.position < self.text.len() && self.current() != b'\n' {
                self.position += self.char_len();
            }
            self.emit(TokenKind::DocComment, start, self.position);
        } else if self.text[start..].starts_with("##") {
            self.position += 2;
            while self.position < self.text.len() && !self.text[self.position..].starts_with("##") {
                self.position += self.char_len();
            }
            if self.position == self.text.len() {
                self.error(
                    codes::E0001,
                    Span::new(self.source, start, self.position),
                    "unterminated multiline comment",
                );
            } else {
                self.position += 2;
            }
            self.emit(TokenKind::MultiLineComment, start, self.position);
            self.line_start = self.position > 0 && self.text.as_bytes()[self.position - 1] == b'\n';
        } else {
            self.position += 1;
            while self.position < self.text.len() && self.current() != b'\n' {
                self.position += self.char_len();
            }
            self.emit(TokenKind::LineComment, start, self.position);
        }
    }
    fn pair(&mut self, wanted: u8, paired: TokenKind, single: TokenKind) {
        let start = self.position;
        self.position += 1;
        let kind = if self.take(wanted) { paired } else { single };
        self.emit(kind, start, self.position);
    }
    fn one(&mut self, kind: TokenKind) {
        let start = self.position;
        self.position += 1;
        self.emit(kind, start, self.position);
    }
    fn take(&mut self, byte: u8) -> bool {
        if self.position < self.text.len() && self.current() == byte {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn current(&self) -> u8 {
        self.text.as_bytes()[self.position]
    }
    fn char_len(&self) -> usize {
        self.text[self.position..]
            .chars()
            .next()
            .map_or(1, char::len_utf8)
    }
    fn emit(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(self.source, start, end),
        });
    }
    fn last_significant(&self) -> Option<TokenKind> {
        self.tokens
            .iter()
            .rev()
            .map(|token| token.kind)
            .find(|kind| {
                !matches!(
                    kind,
                    TokenKind::LineComment | TokenKind::MultiLineComment | TokenKind::DocComment
                )
            })
    }
    fn error(&mut self, code: DiagnosticCode, span: Span, message: &str) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span, message));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn kinds(source: &str) -> (Vec<TokenKind>, DiagnosticSink) {
        let (tokens, diagnostics) = Lexer::new(SourceId::from_index(0), source).lex();
        (
            tokens.into_iter().map(|token| token.kind).collect(),
            diagnostics,
        )
    }
    fn codes(diagnostics: &DiagnosticSink) -> Vec<&str> {
        diagnostics
            .as_slice()
            .iter()
            .filter_map(|item| item.code.as_deref())
            .collect()
    }
    #[test]
    fn lexes_indentation_and_range() {
        let (tokens, diagnostics) =
            Lexer::new(SourceId::from_index(0), "if x:\n  a = 1..=2\n").lex();
        assert!(!diagnostics.has_errors());
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Indent));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::RangeInclusive)
        );
    }

    #[test]
    fn lexes_fat_arrow_distinctly_from_thin_arrow() {
        let (tokens, diagnostics) = kinds("a => b\nx -> y\n");
        assert!(!diagnostics.has_errors(), "{:?}", codes(&diagnostics));
        assert!(tokens.contains(&TokenKind::FatArrow));
        assert!(tokens.contains(&TokenKind::Arrow));
    }

    #[test]
    fn emits_block_tokens_for_lambda_inside_parentheses() {
        let source = "apply(10, fn(x):\n  value = x * 2\n  value\n)\n";
        let (tokens, diagnostics) = kinds(source);
        assert!(!diagnostics.has_errors(), "{:?}", codes(&diagnostics));
        assert!(tokens.contains(&TokenKind::Indent));
        assert!(tokens.contains(&TokenKind::Dedent));
        let colon = tokens
            .iter()
            .position(|kind| *kind == TokenKind::Colon)
            .expect("colon");
        assert!(tokens[colon + 1..].contains(&TokenKind::Newline));
    }

    #[test]
    fn blank_lines_do_not_change_indentation() {
        for source in [
            "fn main():\n  first = 1\n\n  second = 2\n",
            "fn main():\r\n  first = 1\r\n\r\n  second = 2\r\n",
            "fn main():\r\n  first = 1\r\n  \r\n  second = 2\r\n",
            "fn main():\n  first = 1\n  # keep the block open\n  second = 2\n",
        ] {
            let (actual, diagnostics) = kinds(source);
            assert!(!diagnostics.has_errors(), "{source:?}");
            assert_eq!(
                actual
                    .iter()
                    .filter(|kind| **kind == TokenKind::Indent)
                    .count(),
                1,
                "{source:?}"
            );
            assert_eq!(
                actual
                    .iter()
                    .filter(|kind| **kind == TokenKind::Dedent)
                    .count(),
                1,
                "{source:?}"
            );
        }
    }
    #[test]
    fn rejects_unescaped_bad_characters() {
        let (_, diagnostics) = Lexer::new(SourceId::from_index(0), "[]").lex();
        assert!(diagnostics.has_errors());
    }
    #[test]
    fn recognizes_every_keyword_without_reserving_future_words() {
        let source = "fn data interface enum type extern async await static if elif else match for in break continue return import as at and or not true false null dyn opaque unsafe";
        let (actual, diagnostics) = kinds(source);
        assert!(!diagnostics.has_errors());
        assert_eq!(actual.last(), Some(&TokenKind::Eof));
        assert!(
            actual[..actual.len() - 1]
                .iter()
                .all(|kind| *kind != TokenKind::Identifier),
            "reserved words must not lex as identifiers: {actual:?}"
        );
        let (future, diagnostics) = kinds("record generator");
        assert!(!diagnostics.has_errors());
        assert_eq!(future[0], TokenKind::Identifier);
        assert_eq!(future[1], TokenKind::Identifier);
    }
    #[test]
    fn tokenizes_attributes_without_changing_list_literal_prefix() {
        let (actual, diagnostics) = kinds("@repr(C)\nitems = @(1, 2, 3)\n");
        assert!(!diagnostics.has_errors());
        assert_eq!(
            &actual[..12],
            &[
                TokenKind::AtSign,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::Identifier,
                TokenKind::RParen,
                TokenKind::Newline,
                TokenKind::Identifier,
                TokenKind::Equal,
                TokenKind::AtSign,
                TokenKind::LParen,
                TokenKind::Integer,
                TokenKind::Comma,
            ]
        );
    }
    #[test]
    fn tokenizes_unified_fn_method_header() {
        let (actual, diagnostics) = kinds("fn Counter.increment(amount: int):\n");
        assert!(!diagnostics.has_errors());
        assert_eq!(
            actual,
            vec![
                TokenKind::Fn,
                TokenKind::Identifier,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::RParen,
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }
    #[test]
    fn uses_longest_operator_and_disambiguates_float_from_range() {
        let (actual, diagnostics) = kinds("1.0 1..10 1..=10 -> == != >= <=");
        assert!(!diagnostics.has_errors());
        assert_eq!(
            actual,
            vec![
                TokenKind::Float,
                TokenKind::Integer,
                TokenKind::Range,
                TokenKind::Integer,
                TokenKind::Integer,
                TokenKind::RangeInclusive,
                TokenKind::Integer,
                TokenKind::Arrow,
                TokenKind::EqualEqual,
                TokenKind::BangEqual,
                TokenKind::GreaterEqual,
                TokenKind::LessEqual,
                TokenKind::Eof
            ]
        );
    }
    #[test]
    fn preserves_comment_kinds_and_priority() {
        let (actual, diagnostics) = kinds("# line\n### docs\n## multi\nline ##\n");
        assert!(!diagnostics.has_errors());
        assert!(actual.contains(&TokenKind::LineComment));
        assert!(actual.contains(&TokenKind::DocComment));
        assert!(actual.contains(&TokenKind::MultiLineComment));
    }
    #[test]
    fn template_tokens_preserve_identifier_expression_and_nested_parentheses() {
        let (actual, diagnostics) = kinds("\"hi $name $((a + b) * (c - d))\"");
        assert!(!diagnostics.has_errors());
        assert_eq!(actual.first(), Some(&TokenKind::StringStart));
        assert!(actual.contains(&TokenKind::InterpolationIdentifier));
        assert!(actual.contains(&TokenKind::InterpolationStart));
        assert!(actual.contains(&TokenKind::InterpolationEnd));
        assert_eq!(actual[actual.len() - 2], TokenKind::StringEnd);
    }
    #[test]
    fn escaped_dollar_is_plain_string_text() {
        let (actual, diagnostics) = kinds("\"price: \\$100\"");
        assert!(!diagnostics.has_errors());
        assert!(!actual.contains(&TokenKind::InterpolationIdentifier));
        assert!(!actual.contains(&TokenKind::InterpolationStart));
    }
    #[test]
    fn reports_precise_template_failures_and_recovers_to_string_end() {
        let (_, invalid) = kinds("\"hello $1\"");
        assert!(codes(&invalid).contains(&"E0005"));
        let (_, empty) = kinds("\"hello $()\"");
        assert!(codes(&empty).contains(&"E0007"));
        let (actual, unclosed) = kinds("\"hello $(name\"");
        assert!(codes(&unclosed).contains(&"E0006"));
        assert!(actual.contains(&TokenKind::StringEnd));
    }
    #[test]
    fn diagnoses_invalid_escapes_tabs_dedents_and_unclosed_delimiters() {
        let (_, escape) = kinds("\"\\q\"");
        assert!(codes(&escape).contains(&"E0003"));
        let (_, tab) = kinds("if x:\n\tvalue\n");
        assert!(codes(&tab).contains(&"E0107"));
        let (_, dedent) = kinds("if x:\n  value\n other\n");
        assert!(codes(&dedent).contains(&"E0107"));
        let (_, parenthesis) = kinds("call(");
        assert!(codes(&parenthesis).contains(&"E0001"));
    }
    #[test]
    fn ignores_layout_newlines_inside_parentheses_and_flushes_eof_dedents() {
        let (actual, diagnostics) = kinds("value = @(\n  1,\n  2\n)\nif true:\n  value");
        assert!(!diagnostics.has_errors());
        assert_eq!(
            actual
                .iter()
                .filter(|kind| **kind == TokenKind::Indent)
                .count(),
            1
        );
        assert_eq!(
            actual
                .iter()
                .filter(|kind| **kind == TokenKind::Dedent)
                .count(),
            1
        );
    }
    #[test]
    fn preserves_relative_import_prefix_as_one_token() {
        let (actual, diagnostics) = kinds("import ...math\n");
        assert!(!diagnostics.has_errors());
        assert!(actual.contains(&TokenKind::RelativeDots));
        assert!(!actual.contains(&TokenKind::Range));
    }
    #[test]
    fn arbitrary_valid_utf8_never_panics() {
        for source in ["", "猫", "💥$", "##", "\"$(())\"", "\0", "((((", "\r\n"] {
            let _ = Lexer::new(SourceId::from_index(0), source).lex();
        }
    }
    #[test]
    fn malformed_number_recovers_at_one_error_token() {
        let (actual, diagnostics) = kinds("123abc + 1");
        assert!(codes(&diagnostics).contains(&"E0004"));
        assert_eq!(actual[0], TokenKind::Error);
        assert!(actual.contains(&TokenKind::Plus));
    }
    #[test]
    fn anonymous_record_prefix_is_invalid_outside_strings() {
        let (actual, diagnostics) = kinds("$()");
        assert!(codes(&diagnostics).contains(&"E0001"));
        assert_eq!(actual[0], TokenKind::Error);
        assert!(!actual.contains(&TokenKind::InterpolationStart));
    }
    #[test]
    fn interpolation_diagnostic_span_points_at_the_dollar() {
        let (_, diagnostics) = Lexer::new(SourceId::from_index(7), "\"hi $1\"").lex();
        let diagnostic = diagnostics
            .as_slice()
            .iter()
            .find(|item| item.code.as_deref() == Some("E0005"))
            .unwrap();
        let span = diagnostic.primary.as_ref().unwrap().span;
        assert_eq!(span.source(), SourceId::from_index(7));
        assert_eq!((span.start(), span.end()), (4, 5));
    }
    #[test]
    fn recovery_reports_independent_errors_and_reaches_eof() {
        let (actual, diagnostics) = kinds("[] ! 12bad ok");
        assert!(diagnostics.as_slice().len() >= 4);
        assert_eq!(actual.last(), Some(&TokenKind::Eof));
        assert!(actual.contains(&TokenKind::Identifier));
    }
}
