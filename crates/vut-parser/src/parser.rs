//! Recovering recursive-descent parser with a Pratt expression core.
use vut_ast::{Expr, File, Item, Name};
use vut_diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSink, codes};
use vut_lexer::{Token, TokenKind};
use vut_source::{SourceId, Span};

pub struct Parser<'a> {
    pub(crate) source: SourceId,
    pub(crate) text: &'a str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) position: usize,
    pub(crate) diagnostics: DiagnosticSink,
    /// Set when a block-structured expression (`if`/`match`) just consumed its
    /// own terminating dedent, so the enclosing statement needs no terminator.
    pub(crate) block_line_finished: bool,
    /// True only where a trailing receiver-function body (`Call(): ...`) is a
    /// valid statement/RHS form, so `if cond:` / `for x in it:` / `match v:`
    /// headers keep their block colon.
    pub(crate) receiver_trailing_allowed: bool,
}

impl<'a> Parser<'a> {
    #[must_use]
    pub fn new(source: SourceId, text: &'a str, tokens: Vec<Token>) -> Self {
        let tokens = tokens
            .into_iter()
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::LineComment | TokenKind::MultiLineComment | TokenKind::DocComment
                )
            })
            .collect();
        Self {
            source,
            text,
            tokens,
            position: 0,
            diagnostics: DiagnosticSink::new(),
            block_line_finished: false,
            receiver_trailing_allowed: false,
        }
    }

    #[must_use]
    pub fn parse(mut self) -> (File, DiagnosticSink) {
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(TokenKind::Eof) {
            if let Some(item) = self.item() {
                items.push(item);
            } else {
                let span = self.current().span;
                if self.at(TokenKind::Indent) {
                    self.error(codes::E0105, span, "unexpected indentation");
                } else {
                    self.error(codes::E0104, span, "expected a top-level declaration");
                }
                items.push(Item::Statement(vut_ast::Stmt::Error(span)));
                self.recover_top_level();
            }
            self.skip_newlines();
        }
        (
            File {
                source: self.source,
                items,
            },
            self.diagnostics,
        )
    }

    pub(crate) fn item(&mut self) -> Option<Item> {
        let attributes = self.attributes();
        if !attributes.is_empty()
            && !matches!(
                self.kind(),
                TokenKind::Fn
                    | TokenKind::Async
                    | TokenKind::Data
                    | TokenKind::Opaque
                    | TokenKind::Interface
                    | TokenKind::Enum
                    | TokenKind::Type
                    | TokenKind::Extern
            )
        {
            let span = attributes
                .last()
                .map_or(self.current().span, |item| item.span);
            self.diagnostics.push(
                Diagnostic::error(
                    codes::E0111,
                    "dangling attribute",
                    span,
                    "attribute must be followed by a declaration",
                )
                .with_label(self.current().span, "this is not an attribute target"),
            );
            return Some(Item::Statement(vut_ast::Stmt::Error(span)));
        }
        match self.kind() {
            TokenKind::Import => Some(Item::Import(self.import())),
            TokenKind::Fn => Some(self.function_or_method(attributes, false, false)),
            TokenKind::Async => Some(self.async_function_or_method(attributes, false)),
            TokenKind::Static => Some(self.static_function_or_method(attributes)),
            TokenKind::Extern => Some(Item::ExternFunction(self.extern_function(attributes))),
            TokenKind::Data => Some(Item::Data(self.data(attributes))),
            TokenKind::Opaque => Some(Item::Data(self.opaque_data(attributes))),
            TokenKind::Interface => Some(Item::Interface(self.interface(attributes))),
            TokenKind::Enum => Some(Item::Enum(self.enum_declaration(attributes))),
            TokenKind::Type => Some(Item::TypeAlias(self.type_alias(attributes))),
            TokenKind::Identifier if self.looks_like_removed_method() => {
                Some(self.reject_removed_method())
            }
            TokenKind::If
            | TokenKind::For
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Return
            | TokenKind::Identifier
            | TokenKind::Integer
            | TokenKind::Float
            | TokenKind::StringStart
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
            | TokenKind::Unsafe
            | TokenKind::AtSign
            | TokenKind::LParen
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Not => Some(Item::Statement(self.statement())),
            _ => None,
        }
    }

    /// Parses `async fn ...` at top level. `async` is only a modifier on `fn`.
    fn async_function_or_method(
        &mut self,
        attributes: Vec<vut_ast::Attribute>,
        is_static: bool,
    ) -> Item {
        let start = self.current().span.start();
        let async_token = self.advance();
        if !self.at(TokenKind::Fn) {
            self.error(
                codes::E0112,
                async_token.span,
                "`async` must be followed by `fn`",
            );
            self.recover_top_level();
            return Item::Statement(vut_ast::Stmt::Error(
                self.span_from(start, async_token.span.end()),
            ));
        }
        self.function_or_method(attributes, true, is_static)
    }

    /// `static fn Type.name(...)` — an associated function with no `self`.
    fn static_function_or_method(&mut self, attributes: Vec<vut_ast::Attribute>) -> Item {
        let start = self.current().span.start();
        let static_token = self.advance();
        if self.at(TokenKind::Async) {
            return self.async_function_or_method(attributes, true);
        }
        if !self.at(TokenKind::Fn) {
            self.error(
                codes::E0112,
                static_token.span,
                "`static` must be followed by `fn`",
            );
            self.recover_top_level();
            return Item::Statement(vut_ast::Stmt::Error(
                self.span_from(start, static_token.span.end()),
            ));
        }
        self.function_or_method(attributes, false, true)
    }

    fn looks_like_removed_method(&self) -> bool {
        let mut offset = 1;
        while self.nth(offset).kind == TokenKind::Dot
            && self.nth(offset + 1).kind == TokenKind::Identifier
        {
            if self.nth(offset + 2).kind == TokenKind::LParen {
                return true;
            }
            offset += 2;
        }
        false
    }

    fn reject_removed_method(&mut self) -> Item {
        let span = self.current().span;
        self.diagnostics.push(
            Diagnostic::error(
                vut_diagnostics::codes::E0110,
                "method declarations require `fn`",
                span,
                "the old method declaration syntax has been removed",
            )
            .with_help("write `fn Type.method(...):`"),
        );
        self.recover_top_level();
        Item::Statement(vut_ast::Stmt::Error(span))
    }

    pub(crate) fn name(&mut self, message: &str) -> Name {
        if self.at(TokenKind::Identifier) {
            let token = self.advance();
            Name {
                text: self.slice(token.span).to_owned(),
                span: token.span,
            }
        } else {
            let span = self.current().span;
            self.error(codes::E0101, span, message);
            Name {
                text: String::new(),
                span,
            }
        }
    }

    pub(crate) fn member_name(&mut self, message: &str) -> Name {
        if self.at_any(&[TokenKind::Identifier, TokenKind::At]) {
            let token = self.advance();
            Name {
                text: self.slice(token.span).to_owned(),
                span: token.span,
            }
        } else {
            self.name(message)
        }
    }

    pub(crate) fn slice(&self, span: Span) -> &str {
        &self.text[span.start()..span.end()]
    }
    pub(crate) fn span_from(&self, start: usize, end: usize) -> Span {
        Span::new(self.source, start, end)
    }
    pub(crate) fn finish_line(&mut self) {
        if std::mem::take(&mut self.block_line_finished) {
            return;
        }
        if self.take(TokenKind::Newline).is_none()
            && !self.at_any(&[TokenKind::Dedent, TokenKind::Eof])
        {
            self.error(
                codes::E0101,
                self.current().span,
                "expected end of statement",
            );
            self.recover_line();
        }
    }
    pub(crate) fn skip_newlines(&mut self) {
        while self.take(TokenKind::Newline).is_some() {}
    }
    pub(crate) fn recover_line(&mut self) {
        while !self.at_any(&[TokenKind::Newline, TokenKind::Dedent, TokenKind::Eof]) {
            self.advance();
        }
        self.take(TokenKind::Newline);
    }
    fn recover_top_level(&mut self) {
        self.recover_line();
        while self.at(TokenKind::Indent) {
            self.advance();
            self.recover_line();
        }
        while self.take(TokenKind::Dedent).is_some() {}
    }
    pub(crate) fn expect(&mut self, kind: TokenKind, code: DiagnosticCode, message: &str) -> Token {
        if self.at(kind) {
            self.advance()
        } else {
            let token = self.current();
            self.error(code, token.span, message);
            token
        }
    }
    pub(crate) fn error(&mut self, code: DiagnosticCode, span: Span, message: &str) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span, message));
    }
    pub(crate) fn take(&mut self, kind: TokenKind) -> Option<Token> {
        self.at(kind).then(|| self.advance())
    }
    pub(crate) fn at(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }
    pub(crate) fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.kind())
    }
    pub(crate) fn kind(&self) -> TokenKind {
        self.current().kind
    }
    pub(crate) fn current(&self) -> Token {
        self.nth(0)
    }
    pub(crate) fn nth(&self, offset: usize) -> Token {
        self.tokens
            .get(self.position + offset)
            .copied()
            .unwrap_or(Token {
                kind: TokenKind::Eof,
                span: Span::new(self.source, self.text.len(), self.text.len()),
            })
    }
    pub(crate) fn advance(&mut self) -> Token {
        let token = self.current();
        if token.kind != TokenKind::Eof {
            self.position += 1;
        }
        token
    }

    /// Offset (from the cursor) of the `)` matching the `(` at the cursor.
    pub(crate) fn matching_paren_offset(&self) -> Option<usize> {
        if !self.at(TokenKind::LParen) {
            return None;
        }
        let mut depth = 0_usize;
        let mut offset = 0_usize;
        loop {
            match self.nth(offset).kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(offset);
                    }
                }
                TokenKind::Eof => return None,
                _ => {}
            }
            offset += 1;
        }
    }

    /// True when the parenthesized group at the cursor is immediately followed
    /// by a `:`, which begins a trailing receiver-function body.
    pub(crate) fn group_followed_by_colon(&self) -> bool {
        self.matching_paren_offset()
            .is_some_and(|close| self.nth(close + 1).kind == TokenKind::Colon)
    }

    /// Parses an expression where a trailing receiver body (`Call(): ...`) is a
    /// valid form (statement expressions and assignment right-hand sides).
    pub(crate) fn expression_allow_trailing(&mut self) -> Expr {
        let saved = self.receiver_trailing_allowed;
        self.receiver_trailing_allowed = true;
        let value = self.expression(0);
        self.receiver_trailing_allowed = saved;
        value
    }

    /// Parses an expression where a following `:` still means a block header
    /// (`if`/`elif` conditions, `for` subjects, `match` values).
    pub(crate) fn expression_deny_trailing(&mut self) -> Expr {
        let saved = self.receiver_trailing_allowed;
        self.receiver_trailing_allowed = false;
        let value = self.expression(0);
        self.receiver_trailing_allowed = saved;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vut_ast::{BinaryOp, Expr, ForKind, ImportMode, Stmt, TemplateSegment, TypeExpr};
    use vut_lexer::Lexer;

    fn parse(source: &str) -> (File, DiagnosticSink) {
        let source_id = SourceId::from_index(0);
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(
            !lexical.has_errors(),
            "lexical diagnostics: {:?}",
            lexical.as_slice()
        );
        Parser::new(source_id, source, tokens).parse()
    }

    #[test]
    fn parses_arrow_and_fn_lambda_forms() {
        let single = parse("result = x => x * 2\n");
        assert!(!single.1.has_errors(), "{:?}", single.1.as_slice());
        let Item::Statement(Stmt::Binding {
            value:
                Expr::Lambda {
                    parameters,
                    body: vut_ast::LambdaBody::Expression(_),
                    ..
                },
            ..
        }) = &single.0.items[0]
        else {
            panic!("expected single-parameter arrow lambda")
        };
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name.text, "x");

        let multiple = parse("result = (a, b) => a + b\n");
        assert!(!multiple.1.has_errors(), "{:?}", multiple.1.as_slice());
        let Item::Statement(Stmt::Binding {
            value:
                Expr::Lambda {
                    parameters,
                    body: vut_ast::LambdaBody::Expression(_),
                    ..
                },
            ..
        }) = &multiple.0.items[0]
        else {
            panic!("expected multi-parameter arrow lambda")
        };
        assert_eq!(parameters.len(), 2);

        let none = parse("result = () => out(\"Hello\")\n");
        assert!(!none.1.has_errors(), "{:?}", none.1.as_slice());
        let Item::Statement(Stmt::Binding {
            value: Expr::Lambda { parameters, .. },
            ..
        }) = &none.0.items[0]
        else {
            panic!("expected zero-parameter arrow lambda")
        };
        assert!(parameters.is_empty());

        let block = parse("result = fn(x):\n  x * 2\n");
        assert!(!block.1.has_errors(), "{:?}", block.1.as_slice());
        let Item::Statement(Stmt::Binding {
            value:
                Expr::Lambda {
                    body: vut_ast::LambdaBody::Block(_),
                    ..
                },
            ..
        }) = &block.0.items[0]
        else {
            panic!("expected block lambda")
        };
    }

    #[test]
    fn rejects_extern_function_body_and_accepts_named_callback_parameters() {
        let source_id = SourceId::from_index(0);
        let source =
            "extern \"C\" fn bad():\n  1\ntype Callback = extern \"C\" fn(a: i32, b: ptr(void))\n";
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(!lexical.has_errors(), "{:?}", lexical.as_slice());
        let (file, diagnostics) = Parser::new(source_id, source, tokens).parse();
        assert!(
            diagnostics
                .as_slice()
                .iter()
                .any(|diagnostic| diagnostic.code.as_deref() == Some("E8007")),
            "{:?}",
            diagnostics.as_slice()
        );
        assert!(
            file.items
                .iter()
                .any(|item| matches!(item, Item::TypeAlias(_)))
        );
    }

    #[test]
    fn parses_function_type_annotation_and_lambda_argument() {
        let (file, diagnostics) = parse(
            "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nresult = apply(10, x => x * 2)\n",
        );
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Function(function) = &file.items[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            function.parameters[1].ty,
            TypeExpr::Function {
                ref parameters,
                return_type: Some(_),
                ..
            } if parameters.len() == 1
        ));
        let Item::Statement(Stmt::Binding {
            value: Expr::Call { arguments, .. },
            ..
        }) = &file.items[1]
        else {
            panic!("expected call statement")
        };
        assert!(matches!(arguments[1].value, Expr::Lambda { .. }));
    }

    #[test]
    fn parses_all_declaration_kinds() {
        let source = "import ..shared.ui as ui\n\
type UserId = u64\n\
data User:\n  name: str\n  age: int = 20\n\
interface Named: Printable:\n  name() -> str\n\
enum Status:\n  ready\n  done\n\
fn make(name: str) -> User:\n  User(name = name, age = 20)\n\
fn User.greet(prefix: str):\n  out(\"$prefix $self\")\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        assert!(matches!(
            file.items[0],
            Item::Import(vut_ast::Import {
                parent_depth: 1,
                mode: ImportMode::Alias(_),
                ..
            })
        ));
        assert!(matches!(file.items[1], Item::TypeAlias(_)));
        assert!(matches!(file.items[2], Item::Data(_)));
        assert!(matches!(file.items[3], Item::Interface(_)));
        assert!(matches!(file.items[4], Item::Enum(_)));
        assert!(matches!(file.items[5], Item::Function(_)));
        let Item::Method(method) = &file.items[6] else {
            panic!("qualified `fn` declaration must parse as a method")
        };
        let TypeExpr::Named { path, .. } = &method.receiver else {
            panic!("method receiver must remain structurally separate")
        };
        assert_eq!(path[0].text, "User");
        assert_eq!(method.name.text, "greet");
    }

    #[test]
    fn parses_precedence_postfix_lists_and_ranges() {
        let (file, diagnostics) = parse("value: list(int) = @(user.score + 2 * 3, 0..=10)\n");
        assert!(!diagnostics.has_errors());
        let Item::Statement(Stmt::Binding {
            annotation: Some(TypeExpr::Applied { .. }),
            value: Expr::List { values, .. },
            ..
        }) = &file.items[0]
        else {
            panic!("unexpected AST")
        };
        assert!(matches!(
            values[0],
            Expr::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
        assert!(matches!(
            values[1],
            Expr::Binary {
                op: BinaryOp::RangeInclusive,
                ..
            }
        ));
    }

    #[test]
    fn parses_attributes_on_declarations_without_list_literal_regression() {
        let source = "@repr(C)\ndata Point:\n  x: f32\n  y: f32\nitems = @(1, 2, 3)\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Data(data) = &file.items[0] else {
            panic!("expected attributed data")
        };
        assert_eq!(data.attributes[0].name.text, "repr");
        assert!(matches!(
            data.attributes[0].args[0],
            vut_ast::AttributeArg::Ident(_)
        ));
        assert!(matches!(
            file.items[1],
            Item::Statement(Stmt::Binding {
                value: Expr::List { .. },
                ..
            })
        ));
    }

    #[test]
    fn parses_extern_function_with_link_name_attribute() {
        let source = "@link_name(\"native_add\")\nextern \"C\" fn add(a: i32, b: i32) -> i32\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::ExternFunction(function) = &file.items[0] else {
            panic!("expected extern function")
        };
        assert_eq!(function.abi, "C");
        assert_eq!(function.name.text, "add");
        assert_eq!(function.attributes[0].name.text, "link_name");
    }

    #[test]
    fn parses_template_segments_as_normal_expressions() {
        let (file, diagnostics) =
            parse("out(\"hello $name, next = $(user.age + 1), price \\$100\")\n");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Statement(Stmt::Expression(Expr::Call { arguments, .. })) = &file.items[0] else {
            panic!("expected call")
        };
        let Expr::String { segments, .. } = &arguments[0].value else {
            panic!("expected string")
        };
        assert_eq!(
            segments
                .iter()
                .filter(|segment| matches!(segment, TemplateSegment::Expression(_)))
                .count(),
            2
        );
        assert!(segments.iter().any(
            |segment| matches!(segment, TemplateSegment::Text { text, .. } if text.contains("$100"))
        ));
    }

    #[test]
    fn parses_if_for_and_match() {
        let source = "if active:\n  out(\"yes\")\nelif pending:\n  out(\"wait\")\nelse:\n  out(\"no\")\n\
for value, index in values:\n  out(\"$index: $value\")\n\
text = match status:\n  ready: \"Ready\"\n  done: \"Done\"\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        assert!(matches!(file.items[0], Item::Statement(Stmt::If(_))));
        assert!(matches!(
            file.items[1],
            Item::Statement(Stmt::For(vut_ast::For {
                kind: ForKind::Iterable { index: Some(_), .. },
                ..
            }))
        ));
        assert!(matches!(
            file.items[2],
            Item::Statement(Stmt::Binding {
                value: Expr::Match { .. },
                ..
            })
        ));
    }

    #[test]
    fn allows_statements_after_block_expressions() {
        let source = "fn main():\n  x = \"hi\".to_bytes()\n  match x.to_str():\n    ok(v): out(v)\n    err(e): out(\"bad\")\n  out(\"after\")\n  m = if x.len() > 0:\n    \"yes\"\n  else:\n    \"no\"\n  out(m)\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Function(function) = &file.items[0] else {
            panic!("expected function")
        };
        assert_eq!(function.body.statements.len(), 5);
        assert!(matches!(
            function.body.statements[1],
            Stmt::Expression(Expr::Match { .. })
        ));
        assert!(matches!(function.body.statements[2], Stmt::Expression(_)));
        assert!(matches!(
            function.body.statements[3],
            Stmt::Binding {
                value: Expr::If(_),
                ..
            }
        ));
        assert!(matches!(function.body.statements[4], Stmt::Expression(_)));
    }

    #[test]
    fn parses_result_constructors_patterns_and_question_operator() {
        let source = "fn load() -> result(int, str):\n  ok(1)\nfn main() -> result(int, str):\n  value = load()?\n  match ok(value):\n    ok(number): ok(number)\n    err(message): err(message)\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Function(function) = &file.items[1] else {
            panic!("expected function")
        };
        assert!(matches!(
            function.body.statements[0],
            Stmt::Binding {
                value: Expr::ResultPropagate { .. },
                ..
            }
        ));
        let Stmt::Expression(Expr::Match { arms, .. }) = &function.body.statements[1] else {
            panic!("expected result match")
        };
        assert!(matches!(
            arms[0].pattern,
            vut_ast::MatchPattern::ResultOk { .. }
        ));
        assert!(matches!(
            arms[1].pattern,
            vut_ast::MatchPattern::ResultErr { .. }
        ));
    }

    #[test]
    fn rejects_tuples_explicit_self_and_recovers_to_next_declaration() {
        let source_id = SourceId::from_index(0);
        let source = "value = (1, 2)\nfn Counter.add(self: Counter):\n  return\nfn valid():\n  1\n";
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(!lexical.has_errors());
        let (file, diagnostics) = Parser::new(source_id, source, tokens).parse();
        assert!(diagnostics.has_errors());
        let codes: Vec<_> = diagnostics
            .as_slice()
            .iter()
            .filter_map(|item| item.code.as_deref())
            .collect();
        assert!(codes.contains(&"E0109"));
        assert!(codes.contains(&"E6008"));
        assert!(
            file.items.iter().any(
                |item| matches!(item, Item::Function(function) if function.name.text == "valid")
            )
        );
    }

    #[test]
    fn rejects_removed_method_syntax_and_recovers() {
        let source_id = SourceId::from_index(0);
        let source = "Counter.increment():\n  self.value = self.value + 1\nfn valid():\n  1\n";
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(!lexical.has_errors());
        let (file, diagnostics) = Parser::new(source_id, source, tokens).parse();
        let diagnostic = diagnostics
            .as_slice()
            .iter()
            .find(|item| item.code.as_deref() == Some("E0110"))
            .expect("removed method syntax must have a specific diagnostic");
        assert_eq!(diagnostic.title, "method declarations require `fn`");
        assert_eq!(
            diagnostic.help.as_deref(),
            Some("write `fn Type.method(...):`")
        );
        assert!(
            file.items.iter().any(
                |item| matches!(item, Item::Function(function) if function.name.text == "valid")
            )
        );
    }

    #[test]
    fn parses_all_four_for_forms_and_chained_calls() {
        let source = "for:\n  work()\nfor running:\n  work()\nfor item in items:\n  item.save()\nfor item, index in items:\n  log(value = item, index = index)\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let kinds: Vec<_> = file
            .items
            .iter()
            .map(|item| match item {
                Item::Statement(Stmt::For(loop_)) => &loop_.kind,
                _ => panic!("expected loop"),
            })
            .collect();
        assert!(matches!(kinds[0], ForKind::Infinite));
        assert!(matches!(kinds[1], ForKind::Conditional(_)));
        assert!(matches!(kinds[2], ForKind::Iterable { index: None, .. }));
        assert!(matches!(kinds[3], ForKind::Iterable { index: Some(_), .. }));
    }

    #[test]
    fn reports_specific_common_syntax_diagnostics() {
        let source_id = SourceId::from_index(0);
        let source = "import math as m at add\nif active\n  run()\n";
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(!lexical.has_errors());
        let (_, diagnostics) = Parser::new(source_id, source, tokens).parse();
        let codes: Vec<_> = diagnostics
            .as_slice()
            .iter()
            .filter_map(|item| item.code.as_deref())
            .collect();
        assert!(codes.contains(&"E0108"));
        assert!(codes.contains(&"E0102"));
    }

    #[test]
    fn invalid_template_expression_uses_template_diagnostic() {
        let source_id = SourceId::from_index(0);
        let source = "out(\"bad = $(+)\")\n";
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(!lexical.has_errors());
        let (_, diagnostics) = Parser::new(source_id, source, tokens).parse();
        assert!(
            diagnostics
                .as_slice()
                .iter()
                .any(|item| item.code.as_deref() == Some("E0007"))
        );
    }

    #[test]
    fn arbitrary_valid_utf8_never_panics() {
        let samples = [
            "",
            "\0",
            "🦀 λ 文 العربية",
            "fn (((((((((((((((",
            "\"$(a + (b * c))\"",
            "data X:\n  a:\n    @@@\n",
        ];
        for source in samples {
            let source_id = SourceId::from_index(0);
            let (tokens, _) = Lexer::new(source_id, source).lex();
            let _ = Parser::new(source_id, source, tokens).parse();
        }
    }

    #[test]
    fn allows_comment_and_blank_lines_before_first_block_statement() {
        let (_, diagnostics) = parse("fn main():\n  # explanation\n\n  out(\"ok\")\n");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
    }

    #[test]
    fn parses_async_functions_methods_and_await_precedence() {
        let (file, diagnostics) = parse(
            "async fn fetch() -> int:\n  await load()\n\
async fn User.refresh() -> int:\n  value = await fetch()\n  value\n\
async fn combined() -> int:\n  await operation()?\n",
        );
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Function(function) = &file.items[0] else {
            panic!("expected async function")
        };
        assert!(function.is_async);
        let Item::Method(method) = &file.items[1] else {
            panic!("expected async method")
        };
        assert!(method.is_async);
        assert_eq!(method.name.text, "refresh");
        let Item::Function(combined) = &file.items[2] else {
            panic!("expected async function")
        };
        // `await operation()?` parses as `(await operation())?`.
        let Stmt::Expression(Expr::ResultPropagate { value, .. }) = &combined.body.statements[0]
        else {
            panic!("expected result propagation over await")
        };
        assert!(matches!(value.as_ref(), Expr::Await { .. }));
        let Expr::Await { value, .. } = value.as_ref() else {
            unreachable!()
        };
        assert!(matches!(value.as_ref(), Expr::Call { .. }));
    }

    #[test]
    fn rejects_async_without_fn_and_await_without_operand() {
        let (_, diagnostics) = parse("async data Point:\n  x: int\nasync fn broken():\n  await\n");
        let codes: Vec<_> = diagnostics
            .as_slice()
            .iter()
            .filter_map(|item| item.code.as_deref())
            .collect();
        assert!(codes.contains(&"E0112"), "{codes:?}");
        assert!(codes.contains(&"E0103"), "{codes:?}");
    }

    #[test]
    fn parses_payload_enums_and_full_patterns() {
        let source = "enum Shape:\n  point\n  circle(radius: float)\n  rect(width: float, height: float)\n\
fn classify(shape: Shape) -> str:\n  match shape:\n    point: \"p\"\n    circle(radius = r) if r > 0.0: \"c\"\n    rect(width = w, height = h): \"r\"\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Enum(enumeration) = &file.items[0] else {
            panic!("expected enum")
        };
        assert_eq!(enumeration.variants.len(), 3);
        assert!(enumeration.variants[0].fields.is_empty());
        assert_eq!(enumeration.variants[1].name.text, "circle");
        assert_eq!(enumeration.variants[1].fields[0].name.text, "radius");
        assert_eq!(enumeration.variants[2].fields.len(), 2);
        let Item::Function(function) = &file.items[1] else {
            panic!("expected function")
        };
        let Stmt::Expression(Expr::Match { arms, .. }) = &function.body.statements[0] else {
            panic!("expected match")
        };
        assert!(matches!(
            arms[1].pattern,
            vut_ast::MatchPattern::Variant { .. }
        ));
        assert!(arms[1].guard.is_some());
    }

    #[test]
    fn parses_or_range_string_and_list_patterns() {
        let source = "fn f(v: int) -> int:\n  match v:\n    1 or 2: 1\n    3..=9: 2\n    _: 3\nfn g(s: str) -> int:\n  match s:\n    \"hi\": 1\n    _: 2\nfn h(xs: list(int)) -> int:\n  match xs:\n    @(a, b): a\n    @(rest..): 1\n    @(): 0\nfn i(b: bool) -> int:\n  match b:\n    true: 1\n    false: 0\n";
        let (_, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
    }

    #[test]
    fn parses_vutcon_spawn_expressions() {
        let source = "fn calc() -> int:\n  1\n\nasync fn main():\n  a = vut(() => calc())\n  b = vut(fn():\n    value = calc()\n    value * 2\n  )\n  x = await a\n  y = await b\n  job: vutcon(int) = a\n  out(\"$(x + y)\")\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let spawns = file
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Function(function) => Some(function),
                _ => None,
            })
            .flat_map(|function| function.body.statements.iter())
            .filter(|statement| {
                matches!(
                    statement,
                    Stmt::Binding {
                        value: Expr::Spawn { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(spawns, 2);
    }

    #[test]
    fn parses_multiple_generic_bounds() {
        let source = "interface A:\n  a() -> int\ninterface B:\n  b() -> int\nfn use(T: A + B)(value: T) -> int:\n  value.a()\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let function = file
            .items
            .iter()
            .find_map(|item| match item {
                Item::Function(function) => Some(function),
                _ => None,
            })
            .expect("function");
        assert_eq!(function.type_parameters.len(), 1);
        assert_eq!(function.type_parameters[0].bounds.len(), 2);
    }

    #[test]
    fn parses_generic_declarations_and_bounds() {
        let source = "interface Comparable:\n  compare(other: int) -> int\n\ndata Box(T):\n  value: T\n\ndata Pair(A, B):\n  first: A\n  second: B\n\ninterface Container(T):\n  get() -> T\n\nfn identity(T)(value: T) -> T:\n  value\n\nfn max(T: Comparable)(a: T, b: T) -> T:\n  a\n\nfn plain(value: int) -> int:\n  value\n";
        let (file, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let functions: Vec<_> = file
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Function(function) => Some(function),
                _ => None,
            })
            .collect();
        assert_eq!(functions[0].type_parameters.len(), 1);
        assert!(functions[0].type_parameters[0].bounds.is_empty());
        assert_eq!(functions[1].type_parameters.len(), 1);
        assert_eq!(functions[1].type_parameters[0].bounds.len(), 1);
        // A single parenthesized group stays the value-parameter list.
        assert!(functions[2].type_parameters.is_empty());
        assert_eq!(functions[2].parameters.len(), 1);
    }

    #[test]
    fn parses_block_match_arms_and_block_assignment_values() {
        let source = "fn f(x: int) -> int:\n  match x:\n    0:\n      a = 1\n      a + 2\n    _: 0\n\nfn g() -> str:\n  message: str =\n    name = \"vut\"\n    \"hi $name\"\n  message\n\nfn h(x: int) -> int:\n  match x:\n    n if n > 0:\n      doubled = n * 2\n      doubled\n    _: 0\n";
        let (_, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
    }

    #[test]
    fn parses_variadic_parameters_and_spread_arguments() {
        let source = "fn sum(...values: int) -> int:\n  values.len()\nfn label(prefix: str, ...rest: str) -> str:\n  prefix\nfn main():\n  xs = @(1, 2)\n  out(\"x\")\n  s = sum(...xs)\n  out(\"$(s)\")\n";
        let (_, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
    }

    #[test]
    fn rejects_block_expression_without_indented_body() {
        let (_, diagnostics) = parse("fn f() -> int:\n  match 0:\n    0:\n    1: 1\n");
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn parses_receiver_function_types() {
        for (source, expected_parameters) in [
            ("type H = fn(Scope)() -> void\n", 0_usize),
            ("type H = fn(Scope)(Event) -> bool\n", 1),
            ("type H = fn(Scope)(Event, int) -> str\n", 2),
        ] {
            let (file, diagnostics) = parse(source);
            assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
            let Item::TypeAlias(alias) = &file.items[0] else {
                panic!("expected type alias")
            };
            let TypeExpr::Function {
                receiver,
                parameters,
                return_type,
                ..
            } = &alias.ty
            else {
                panic!("expected function type for `{source}`")
            };
            assert!(receiver.is_some(), "receiver missing for `{source}`");
            assert_eq!(parameters.len(), expected_parameters);
            assert!(return_type.is_some());
        }
    }

    #[test]
    fn single_parenthesized_group_is_not_a_receiver() {
        let (file, diagnostics) = parse("type H = fn(int) -> int\n");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::TypeAlias(alias) = &file.items[0] else {
            panic!("expected type alias")
        };
        let TypeExpr::Function {
            receiver,
            parameters,
            ..
        } = &alias.ty
        else {
            panic!("expected function type")
        };
        assert!(receiver.is_none());
        assert_eq!(parameters.len(), 1);
    }

    #[test]
    fn parses_trailing_receiver_body_without_parameters() {
        let (file, diagnostics) = parse("Column():\n  Button(\"A\")\n");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Statement(Stmt::Expression(Expr::Call { arguments, .. })) = &file.items[0] else {
            panic!("expected call statement")
        };
        assert_eq!(arguments.len(), 1);
        let Expr::Lambda {
            receiver,
            parameters,
            body: vut_ast::LambdaBody::Block(_),
            ..
        } = &arguments[0].value
        else {
            panic!("expected trailing receiver lambda")
        };
        assert!(matches!(receiver, vut_ast::LambdaReceiver::Inferred));
        assert!(parameters.is_empty());
    }

    #[test]
    fn parses_trailing_receiver_body_with_parameters() {
        let (file, diagnostics) = parse("UserCard(user) (event):\n  Text(event)\n");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Statement(Stmt::Expression(Expr::Call { arguments, .. })) = &file.items[0] else {
            panic!("expected call statement")
        };
        assert_eq!(arguments.len(), 2, "callee argument plus trailing lambda");
        let Expr::Lambda {
            receiver,
            parameters,
            ..
        } = &arguments[1].value
        else {
            panic!("expected trailing receiver lambda")
        };
        assert!(matches!(receiver, vut_ast::LambdaReceiver::Inferred));
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name.text, "event");
    }

    #[test]
    fn chained_calls_without_colon_stay_chained() {
        let (file, diagnostics) = parse("result = foo()(10)\n");
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
        let Item::Statement(Stmt::Binding { value, .. }) = &file.items[0] else {
            panic!("expected binding")
        };
        let Expr::Call {
            callee, arguments, ..
        } = value
        else {
            panic!("expected chained call")
        };
        assert!(matches!(**callee, Expr::Call { .. }));
        assert_eq!(arguments.len(), 1);
        assert!(!matches!(arguments[0].value, Expr::Lambda { .. }));
    }

    #[test]
    fn header_colons_are_not_trailing_bodies() {
        let source = "fn f() -> bool:\n  true\n\nfn g() -> int:\n  if f():\n    out(\"yes\")\n  for item in make():\n    out(\"$item\")\n  match pick():\n    n: n\n";
        let (_, diagnostics) = parse(source);
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.as_slice());
    }
}
