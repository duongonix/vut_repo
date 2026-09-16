use crate::Parser;
use vut_ast::{
    Argument, BinaryOp, Expr, FieldPattern, LambdaBody, LambdaParameter, LambdaReceiver,
    LiteralPattern, MapEntry, MatchArm, MatchPattern, TemplateSegment, UnaryOp,
};
use vut_diagnostics::codes;
use vut_lexer::TokenKind;

impl Parser<'_> {
    pub(crate) fn expression(&mut self, minimum: u8) -> Expr {
        let mut left = self.postfix();
        if minimum == 0
            && self.at(TokenKind::FatArrow)
            && let Expr::Name(name) = &left
        {
            self.advance();
            let parameter = LambdaParameter {
                name: name.clone(),
                ty: None,
                span: name.span,
            };
            let body = self.expression(0);
            let span = self.span_from(name.span.start(), body.span().end());
            return Expr::Lambda {
                receiver: LambdaReceiver::None,
                parameters: vec![parameter],
                body: LambdaBody::Expression(Box::new(body)),
                is_async: false,
                span,
            };
        }
        while let Some((operator, power)) = infix(self.kind()) {
            if power < minimum {
                break;
            }
            self.advance();
            let right = self.expression(power + 1);
            let span = self.span_from(left.span().start(), right.span().end());
            left = Expr::Binary {
                left: Box::new(left),
                op: operator,
                right: Box::new(right),
                span,
            };
        }
        left
    }

    fn postfix(&mut self) -> Expr {
        self.postfix_with_question(true)
    }

    fn postfix_with_question(&mut self, allow_question: bool) -> Expr {
        let mut expression = self.prefix();
        loop {
            if self.take(TokenKind::Dot).is_some() {
                let member = self.member_name("expected member name after `.`");
                let span = self.span_from(expression.span().start(), member.span.end());
                expression = Expr::Member {
                    object: Box::new(expression),
                    member,
                    span,
                };
            } else if self.receiver_trailing_allowed
                && matches!(&expression, Expr::Call { .. })
                && self.group_followed_by_colon()
            {
                // `Call(args) (params):` — this group is the trailing receiver
                // function's parameter list, not a chained call.
                let parameters = self.trailing_receiver_parameters();
                expression = self.attach_trailing_receiver(expression, parameters);
                break;
            } else if self.take(TokenKind::LParen).is_some() {
                let callee_start = expression.span().start();
                let mut arguments = Vec::new();
                while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
                    let start = self.current().span.start();
                    let spread = self.take(TokenKind::Ellipsis).is_some();
                    let name = if !spread
                        && self.at(TokenKind::Identifier)
                        && self.nth(1).kind == TokenKind::Equal
                    {
                        let name = self.name("expected argument name");
                        self.advance();
                        Some(name)
                    } else {
                        None
                    };
                    let value = self.expression(0);
                    arguments.push(Argument {
                        name,
                        spread,
                        span: self.span_from(start, value.span().end()),
                        value,
                    });
                    if self.take(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                let end = self
                    .expect(
                        TokenKind::RParen,
                        codes::E0101,
                        "expected `)` after arguments",
                    )
                    .span
                    .end();
                let span = self.span_from(callee_start, end);
                if let Expr::Name(name) = &expression
                    && matches!(name.text.as_str(), "ok" | "err")
                {
                    expression = if arguments.len() == 1 && arguments[0].name.is_none() {
                        let value = Box::new(arguments.remove(0).value);
                        if name.text == "ok" {
                            Expr::ResultOk { value, span }
                        } else {
                            Expr::ResultErr { value, span }
                        }
                    } else {
                        self.error(
                            codes::E0101,
                            span,
                            "`ok` and `err` constructors take exactly one positional argument",
                        );
                        Expr::Error(span)
                    };
                    continue;
                }
                expression = Expr::Call {
                    callee: Box::new(expression),
                    arguments,
                    span,
                };
                // `Call(args):` — a colon directly after the call begins a
                // trailing receiver function with no ordinary parameters.
                if self.receiver_trailing_allowed && self.at(TokenKind::Colon) {
                    expression = self.attach_trailing_receiver(expression, Vec::new());
                    break;
                }
            } else if allow_question && let Some(question) = self.take(TokenKind::Question) {
                expression = Expr::ResultPropagate {
                    span: self.span_from(expression.span().start(), question.span.end()),
                    value: Box::new(expression),
                };
            } else {
                break;
            }
        }
        expression
    }

    /// Parses the parenthesized parameter list of a trailing receiver function:
    /// `(event, index: int)`.
    fn trailing_receiver_parameters(&mut self) -> Vec<LambdaParameter> {
        self.expect(
            TokenKind::LParen,
            codes::E0110,
            "expected `(` before receiver function parameters",
        );
        let mut parameters = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            let name = self.name("expected receiver function parameter name");
            let param_start = name.span.start();
            let mut end = name.span.end();
            let ty = if self.take(TokenKind::Colon).is_some() {
                let ty = self.type_expression();
                end = ty.span().end();
                Some(ty)
            } else {
                None
            };
            parameters.push(LambdaParameter {
                name,
                ty,
                span: self.span_from(param_start, end),
            });
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(
            TokenKind::RParen,
            codes::E0110,
            "expected `)` after receiver function parameters",
        );
        parameters
    }

    /// Appends an inferred receiver lambda (built from the trailing `:` body)
    /// as the final argument of `call`.
    fn attach_trailing_receiver(&mut self, call: Expr, parameters: Vec<LambdaParameter>) -> Expr {
        let Expr::Call {
            callee,
            mut arguments,
            span: call_span,
        } = call
        else {
            return call;
        };
        let lambda_start = self.current().span.start();
        let block = self.block_from(lambda_start, "expected indented receiver function body");
        self.block_line_finished = true;
        let end = block.span.end();
        let lambda_span = self.span_from(lambda_start, end);
        arguments.push(Argument {
            name: None,
            spread: false,
            span: lambda_span,
            value: Expr::Lambda {
                receiver: LambdaReceiver::Inferred,
                parameters,
                body: LambdaBody::Block(block),
                is_async: false,
                span: lambda_span,
            },
        });
        Expr::Call {
            callee,
            arguments,
            span: self.span_from(call_span.start(), end),
        }
    }

    /// Parses the operand of a prefix `await`.
    ///
    /// The operand is a member/call chain without a trailing `?`, so that
    /// `await operation()?` parses as `(await operation())?`.
    fn await_expression(&mut self, token: vut_lexer::Token) -> Expr {
        if self.at_any(&[
            TokenKind::Newline,
            TokenKind::Dedent,
            TokenKind::Eof,
            TokenKind::RParen,
            TokenKind::Comma,
        ]) {
            self.error(
                codes::E0103,
                token.span,
                "expected expression after `await`",
            );
            return Expr::Error(token.span);
        }
        let value = self.postfix_with_question(false);
        let span = self.span_from(token.span.start(), value.span().end());
        Expr::Await {
            value: Box::new(value),
            span,
        }
    }

    /// Parses `vut(<callable>)`, which spawns a Vutcon.
    fn spawn_expression(&mut self, token: vut_lexer::Token) -> Expr {
        let start = token.span.start();
        if !self.at(TokenKind::LParen) {
            self.error(
                codes::E6020,
                token.span,
                "`vut(...)` requires an anonymous callable",
            );
            return Expr::Error(token.span);
        }
        self.advance();
        if self.at(TokenKind::RParen) {
            self.error(
                codes::E6020,
                token.span,
                "`vut(...)` requires an anonymous callable",
            );
            let end = self.advance().span.end();
            return Expr::Error(self.span_from(start, end));
        }
        let callable = self.expression(0);
        let end = self
            .expect(
                TokenKind::RParen,
                codes::E0101,
                "expected `)` after the `vut` callable",
            )
            .span
            .end();
        let span = self.span_from(start, end);
        Expr::Spawn {
            callable: Box::new(callable),
            span,
        }
    }

    fn prefix(&mut self) -> Expr {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier => {
                let text = self.slice(token.span).to_owned();
                if text == "array" && self.take(TokenKind::LParen).is_some() {
                    self.array(token.span.start())
                } else if text == "map" && self.take(TokenKind::LParen).is_some() {
                    self.map(token.span.start())
                } else {
                    Expr::Name(vut_ast::Name {
                        text,
                        span: token.span,
                    })
                }
            }
            TokenKind::Integer => Expr::Integer {
                text: self.slice(token.span).to_owned(),
                span: token.span,
            },
            TokenKind::Float => Expr::Float {
                text: self.slice(token.span).to_owned(),
                span: token.span,
            },
            TokenKind::StringStart => self.template_string(token.span.start()),
            TokenKind::True | TokenKind::False => Expr::Bool {
                value: token.kind == TokenKind::True,
                span: token.span,
            },
            TokenKind::Null => Expr::Null(token.span),
            TokenKind::AtSign => self.list(token.span.start()),
            TokenKind::If => {
                let value = Expr::If(Box::new(self.if_after_keyword(token.span.start())));
                self.block_line_finished = true;
                value
            }
            TokenKind::Match => {
                let value = self.match_expression(token.span.start());
                self.block_line_finished = true;
                value
            }
            TokenKind::Await => self.await_expression(token),
            TokenKind::Vut => self.spawn_expression(token),
            TokenKind::Minus | TokenKind::Plus | TokenKind::Not => {
                let op = match token.kind {
                    TokenKind::Minus => UnaryOp::Negate,
                    TokenKind::Plus => UnaryOp::Positive,
                    _ => UnaryOp::Not,
                };
                let value = self.expression(7);
                let span = self.span_from(token.span.start(), value.span().end());
                Expr::Unary {
                    op,
                    value: Box::new(value),
                    span,
                }
            }
            TokenKind::Fn => self.fn_keyword_lambda(token.span, false),
            TokenKind::Async => self.fn_keyword_lambda(token.span, true),
            TokenKind::LParen => {
                if self.looks_like_arrow_lambda() {
                    return self.arrow_lambda(token.span.start());
                }
                let value = self.expression(0);
                if self.at(TokenKind::Comma) {
                    self.error(
                        codes::E0109,
                        self.current().span,
                        "tuple expressions do not exist in Vut",
                    );
                    while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
                        self.advance();
                    }
                }
                let end = self
                    .expect(
                        TokenKind::RParen,
                        codes::E0109,
                        "expected `)` after expression",
                    )
                    .span
                    .end();
                Expr::Group {
                    span: self.span_from(token.span.start(), end),
                    value: Box::new(value),
                }
            }
            _ => {
                self.error(codes::E0103, token.span, "expected expression");
                Expr::Error(token.span)
            }
        }
    }

    /// Parses `fn(...): ...` and `async fn(...): ...`. `async` is only a
    /// modifier on `fn`; the arrow lambda form stays synchronous.
    fn fn_keyword_lambda(&mut self, span: vut_source::Span, is_async: bool) -> Expr {
        if is_async && self.take(TokenKind::Fn).is_none() {
            self.error(codes::E0112, span, "`async` must be followed by `fn`");
            return Expr::Error(span);
        }
        if self.at(TokenKind::LParen) {
            self.fn_lambda(span.start(), is_async)
        } else {
            self.error(codes::E0103, span, "expected expression");
            Expr::Error(span)
        }
    }

    fn looks_like_arrow_lambda(&self) -> bool {
        let mut offset = 0;
        if self.nth(offset).kind == TokenKind::RParen {
            return self.nth(offset + 1).kind == TokenKind::FatArrow;
        }
        loop {
            if self.nth(offset).kind != TokenKind::Identifier {
                return false;
            }
            offset += 1;
            match self.nth(offset).kind {
                TokenKind::Comma => offset += 1,
                TokenKind::RParen => return self.nth(offset + 1).kind == TokenKind::FatArrow,
                _ => return false,
            }
        }
    }

    fn arrow_lambda(&mut self, start: usize) -> Expr {
        let mut parameters = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            let name = self.name("expected lambda parameter name");
            parameters.push(LambdaParameter {
                span: name.span,
                name,
                ty: None,
            });
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(
            TokenKind::RParen,
            codes::E0101,
            "expected `)` after lambda parameters",
        );
        self.expect(
            TokenKind::FatArrow,
            codes::E0101,
            "expected `=>` after lambda parameters",
        );
        let body = self.expression(0);
        let span = self.span_from(start, body.span().end());
        Expr::Lambda {
            receiver: LambdaReceiver::None,
            parameters,
            body: LambdaBody::Expression(Box::new(body)),
            is_async: false,
            span,
        }
    }

    fn fn_lambda(&mut self, start: usize, is_async: bool) -> Expr {
        self.expect(
            TokenKind::LParen,
            codes::E0110,
            "expected `(` before lambda parameters",
        );
        let mut parameters = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            let name = self.name("expected lambda parameter name");
            let param_start = name.span.start();
            let mut end = name.span.end();
            let ty = if self.take(TokenKind::Colon).is_some() {
                let ty = self.type_expression();
                end = ty.span().end();
                Some(ty)
            } else {
                None
            };
            parameters.push(LambdaParameter {
                name,
                ty,
                span: self.span_from(param_start, end),
            });
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(
            TokenKind::RParen,
            codes::E0110,
            "expected `)` after lambda parameters",
        );
        let body = self.block("expected indented lambda body");
        self.block_line_finished = true;
        let span = self.span_from(start, body.span.end());
        Expr::Lambda {
            receiver: LambdaReceiver::None,
            parameters,
            body: LambdaBody::Block(body),
            is_async,
            span,
        }
    }

    fn list(&mut self, start: usize) -> Expr {
        self.expect(TokenKind::LParen, codes::E0101, "expected `(` after `@`");
        let mut values = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            values.push(self.expression(0));
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self
            .expect(TokenKind::RParen, codes::E0101, "expected `)` after list")
            .span
            .end();
        Expr::List {
            values,
            span: self.span_from(start, end),
        }
    }

    fn array(&mut self, start: usize) -> Expr {
        let mut values = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            values.push(self.expression(0));
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self
            .expect(TokenKind::RParen, codes::E0101, "expected `)` after array")
            .span
            .end();
        Expr::Array {
            values,
            span: self.span_from(start, end),
        }
    }

    fn map(&mut self, start: usize) -> Expr {
        let mut entries = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            let entry_start = self
                .expect(
                    TokenKind::LParen,
                    codes::E0101,
                    "expected `(key, value)` map entry",
                )
                .span
                .start();
            let key = self.expression(0);
            self.expect(
                TokenKind::Comma,
                codes::E0101,
                "expected `,` between map key and value",
            );
            let value = self.expression(0);
            let entry_end = self
                .expect(
                    TokenKind::RParen,
                    codes::E0101,
                    "expected `)` after map entry",
                )
                .span
                .end();
            entries.push(MapEntry {
                key,
                value,
                span: self.span_from(entry_start, entry_end),
            });
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self
            .expect(TokenKind::RParen, codes::E0101, "expected `)` after map")
            .span
            .end();
        Expr::Map {
            entries,
            span: self.span_from(start, end),
        }
    }

    fn template_string(&mut self, start: usize) -> Expr {
        let mut segments = Vec::new();
        while !self.at_any(&[TokenKind::StringEnd, TokenKind::Eof, TokenKind::Error]) {
            match self.kind() {
                TokenKind::StringText => {
                    let token = self.advance();
                    segments.push(TemplateSegment::Text {
                        text: decode(self.slice(token.span)),
                        span: token.span,
                    });
                }
                TokenKind::InterpolationIdentifier => {
                    let token = self.advance();
                    let span = self.span_from(token.span.start() + 1, token.span.end());
                    segments.push(TemplateSegment::Expression(Expr::Name(vut_ast::Name {
                        text: self.slice(span).to_owned(),
                        span,
                    })));
                }
                TokenKind::InterpolationStart => {
                    self.advance();
                    let value = self.expression(0);
                    self.expect(
                        TokenKind::InterpolationEnd,
                        codes::E0007,
                        "invalid template expression",
                    );
                    segments.push(TemplateSegment::Expression(value));
                }
                _ => {
                    let span = self.advance().span;
                    self.error(codes::E0007, span, "invalid token in template expression");
                }
            }
        }
        let end = self
            .take(TokenKind::StringEnd)
            .map_or_else(|| self.current().span.end(), |token| token.span.end());
        Expr::String {
            segments,
            span: self.span_from(start, end),
        }
    }

    fn match_expression(&mut self, start: usize) -> Expr {
        let value = self.expression_deny_trailing();
        self.expect(
            TokenKind::Colon,
            codes::E0102,
            "expected `:` after match value",
        );
        self.expect(
            TokenKind::Newline,
            codes::E0101,
            "expected newline after match header",
        );
        self.expect(
            TokenKind::Indent,
            codes::E0106,
            "expected indented match arms",
        );
        let mut arms = Vec::new();
        self.skip_newlines();
        while !self.at_any(&[TokenKind::Dedent, TokenKind::Eof]) {
            let pattern = self.match_pattern();
            let guard = self.take(TokenKind::If).map(|_| self.expression(0));
            self.expect(
                TokenKind::Colon,
                codes::E0102,
                "expected `:` after match pattern",
            );
            let arm_value = self.expression_or_block();
            let block_arm = matches!(arm_value, Expr::Block(_));
            let start = pattern.span().start();
            let span = self.span_from(start, arm_value.span().end());
            arms.push(MatchArm {
                pattern,
                guard,
                value: arm_value,
                span,
            });
            // An indented block already consumed its terminating dedent.
            if !block_arm {
                self.finish_line();
            }
            self.skip_newlines();
        }
        let end = self
            .expect(
                TokenKind::Dedent,
                codes::E0106,
                "expected end of match block",
            )
            .span
            .end();
        Expr::Match {
            value: Box::new(value),
            arms,
            span: self.span_from(start, end),
        }
    }

    fn match_pattern(&mut self) -> MatchPattern {
        self.or_pattern()
    }

    fn or_pattern(&mut self) -> MatchPattern {
        let first = self.primary_pattern();
        if !self.at(TokenKind::Or) {
            return first;
        }
        let start = first.span().start();
        let mut alternatives = vec![first];
        while self.take(TokenKind::Or).is_some() {
            alternatives.push(self.primary_pattern());
        }
        let end = alternatives.last().map_or(start, |item| item.span().end());
        MatchPattern::Or {
            alternatives,
            span: self.span_from(start, end),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "recursive pattern parsing keeps the pattern kinds together"
    )]
    fn primary_pattern(&mut self) -> MatchPattern {
        let token = self.current();
        match token.kind {
            TokenKind::Identifier => {
                self.advance();
                if self.slice(token.span) == "_" {
                    return MatchPattern::Wildcard(token.span);
                }
                let name = vut_ast::Name {
                    text: self.slice(token.span).to_owned(),
                    span: token.span,
                };
                if self.at(TokenKind::LParen) {
                    self.advance();
                    let fields = self.variant_pattern_fields();
                    let close = self
                        .expect(
                            TokenKind::RParen,
                            codes::E0101,
                            "expected `)` after variant pattern fields",
                        )
                        .span
                        .end();
                    let span = self.span_from(token.span.start(), close);
                    if name.text == "ok" || name.text == "err" {
                        let pattern = fields.into_iter().next().map_or_else(
                            || {
                                self.error(codes::E0103, span, "expected a result payload pattern");
                                MatchPattern::Error(span)
                            },
                            |field| match field {
                                FieldPattern::Positional(pattern)
                                | FieldPattern::Named { pattern, .. } => pattern,
                            },
                        );
                        if name.text == "ok" {
                            MatchPattern::ResultOk {
                                pattern: Box::new(pattern),
                                span,
                            }
                        } else {
                            MatchPattern::ResultErr {
                                pattern: Box::new(pattern),
                                span,
                            }
                        }
                    } else {
                        MatchPattern::Variant { name, fields, span }
                    }
                } else {
                    MatchPattern::Binding(name)
                }
            }
            TokenKind::Integer => {
                self.advance();
                let text = self.slice(token.span).replace('_', "");
                if let Some(inclusive) = self.range_operator() {
                    let end = self.expect(
                        TokenKind::Integer,
                        codes::E7111,
                        "expected an integer range end",
                    );
                    let end_text = self.slice(end.span).replace('_', "");
                    MatchPattern::Range {
                        start: LiteralPattern::Integer(text),
                        end: LiteralPattern::Integer(end_text),
                        inclusive,
                        span: self.span_from(token.span.start(), end.span.end()),
                    }
                } else {
                    MatchPattern::Literal {
                        value: LiteralPattern::Integer(text),
                        span: token.span,
                    }
                }
            }
            TokenKind::Float => {
                self.advance();
                MatchPattern::Literal {
                    value: LiteralPattern::Float(self.slice(token.span).replace('_', "")),
                    span: token.span,
                }
            }
            TokenKind::StringStart => {
                self.advance();
                let (value, span) = self.pattern_string(token.span.start());
                MatchPattern::Literal {
                    value: LiteralPattern::Str(value),
                    span,
                }
            }
            TokenKind::True | TokenKind::False => {
                self.advance();
                MatchPattern::Literal {
                    value: LiteralPattern::Bool(token.kind == TokenKind::True),
                    span: token.span,
                }
            }
            TokenKind::Null => {
                self.advance();
                MatchPattern::Literal {
                    value: LiteralPattern::Null,
                    span: token.span,
                }
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.match_pattern();
                let close = self
                    .expect(
                        TokenKind::RParen,
                        codes::E0101,
                        "expected `)` after pattern",
                    )
                    .span
                    .end();
                MatchPattern::Group {
                    pattern: Box::new(inner),
                    span: self.span_from(token.span.start(), close),
                }
            }
            TokenKind::AtSign => {
                self.advance();
                self.expect(
                    TokenKind::LParen,
                    codes::E7112,
                    "expected `(` after `@` in a list pattern",
                );
                let mut items = Vec::new();
                let mut rest = None;
                while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
                    if self.at(TokenKind::Range) {
                        let dots = self.advance();
                        rest = Some(vut_ast::Name {
                            text: "_".into(),
                            span: dots.span,
                        });
                    } else if self.at(TokenKind::Identifier) && self.nth(1).kind == TokenKind::Range
                    {
                        let name = self.name("expected rest binding");
                        self.advance();
                        rest = Some(name);
                    } else {
                        items.push(self.match_pattern());
                    }
                    if self.take(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                let close = self
                    .expect(
                        TokenKind::RParen,
                        codes::E7112,
                        "expected `)` after list pattern",
                    )
                    .span
                    .end();
                MatchPattern::List {
                    items,
                    rest,
                    span: self.span_from(token.span.start(), close),
                }
            }
            _ => {
                self.error(codes::E0103, token.span, "expected a match pattern");
                self.advance();
                MatchPattern::Error(token.span)
            }
        }
    }

    fn variant_pattern_fields(&mut self) -> Vec<FieldPattern> {
        let mut fields = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            if self.at(TokenKind::Identifier) && self.nth(1).kind == TokenKind::Equal {
                let field = self.name("expected variant field name");
                self.advance();
                let pattern = self.match_pattern();
                fields.push(FieldPattern::Named { field, pattern });
            } else {
                fields.push(FieldPattern::Positional(self.match_pattern()));
            }
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        fields
    }

    fn range_operator(&mut self) -> Option<bool> {
        if self.take(TokenKind::Range).is_some() {
            Some(false)
        } else if self.take(TokenKind::RangeInclusive).is_some() {
            Some(true)
        } else {
            None
        }
    }

    fn pattern_string(&mut self, start: usize) -> (String, vut_source::Span) {
        let mut value = String::new();
        while !self.at_any(&[TokenKind::StringEnd, TokenKind::Eof, TokenKind::Error]) {
            if self.kind() == TokenKind::StringText {
                let token = self.advance();
                value.push_str(&decode(self.slice(token.span)));
            } else {
                self.error(
                    codes::E0101,
                    self.current().span,
                    "string patterns cannot contain interpolation",
                );
                self.advance();
            }
        }
        let end = self
            .take(TokenKind::StringEnd)
            .map_or_else(|| self.current().span.end(), |token| token.span.end());
        (value, self.span_from(start, end))
    }
}

fn infix(kind: TokenKind) -> Option<(BinaryOp, u8)> {
    Some(match kind {
        TokenKind::Or => (BinaryOp::Or, 1),
        TokenKind::And => (BinaryOp::And, 2),
        TokenKind::EqualEqual => (BinaryOp::Equal, 3),
        TokenKind::BangEqual => (BinaryOp::NotEqual, 3),
        TokenKind::Less => (BinaryOp::Less, 3),
        TokenKind::LessEqual => (BinaryOp::LessEqual, 3),
        TokenKind::Greater => (BinaryOp::Greater, 3),
        TokenKind::GreaterEqual => (BinaryOp::GreaterEqual, 3),
        TokenKind::Range => (BinaryOp::RangeExclusive, 4),
        TokenKind::RangeInclusive => (BinaryOp::RangeInclusive, 4),
        TokenKind::Plus => (BinaryOp::Add, 5),
        TokenKind::Minus => (BinaryOp::Subtract, 5),
        TokenKind::Star => (BinaryOp::Multiply, 6),
        TokenKind::Slash => (BinaryOp::Divide, 6),
        TokenKind::Percent => (BinaryOp::Modulo, 6),
        _ => return None,
    })
}

fn decode(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        if let Some(escaped) = chars.next() {
            output.push(match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                '\\' => '\\',
                '"' => '"',
                '$' => '$',
                other => other,
            });
        }
    }
    output
}
