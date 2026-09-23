use crate::Parser;
use vut_ast::{Block, Expr, For, ForKind, If, Stmt};
use vut_diagnostics::codes;
use vut_lexer::TokenKind;

impl Parser<'_> {
    pub(crate) fn statement(&mut self) -> Stmt {
        match self.kind() {
            TokenKind::If => {
                let start = self.advance().span.start();
                Stmt::If(self.if_after_keyword(start))
            }
            TokenKind::For => self.for_statement(),
            TokenKind::Unsafe => {
                let start = self.advance().span.start();
                Stmt::Unsafe(self.block_from(start, "expected indented `unsafe` body"))
            }
            TokenKind::Break => {
                let span = self.advance().span;
                self.finish_line();
                Stmt::Break(span)
            }
            TokenKind::Continue => {
                let span = self.advance().span;
                self.finish_line();
                Stmt::Continue(span)
            }
            TokenKind::Return => self.return_statement(),
            TokenKind::Import => {
                let span = self.advance().span;
                self.error(codes::E3005, span, "imports are only valid at module scope");
                self.recover_line();
                Stmt::Error(span)
            }
            _ => self.expression_or_assignment(),
        }
    }

    fn expression_or_assignment(&mut self) -> Stmt {
        let target = self.expression_allow_trailing();
        let annotation = self.take(TokenKind::Colon).map(|_| self.type_expression());
        if self.take(TokenKind::Equal).is_some() {
            let value = self.expression_or_block();
            let span = self.span_from(target.span().start(), value.span().end());
            if !matches!(
                target,
                Expr::Name(_) | Expr::Member { .. } | Expr::Subscript { .. }
            ) {
                self.error(codes::E0101, target.span(), "invalid assignment target");
            }
            // An indented block already consumed its terminating dedent.
            if !matches!(value, Expr::Block(_)) {
                self.finish_line();
            }
            Stmt::Binding {
                target,
                annotation,
                value,
                span,
            }
        } else {
            if annotation.is_some() {
                self.error(
                    codes::E0101,
                    target.span(),
                    "type annotation requires an assignment",
                );
            }
            self.finish_line();
            Stmt::Expression(target)
        }
    }

    /// Parses an expression, or an indented block used as an expression when the
    /// value starts on the next line.
    pub(crate) fn expression_or_block(&mut self) -> Expr {
        if self.at(TokenKind::Newline) {
            let start = self.current().span.start();
            return Expr::Block(self.indented_block(start, "expected indented block expression"));
        }
        self.expression_allow_trailing()
    }

    fn return_statement(&mut self) -> Stmt {
        let token = self.advance();
        let value = (!self.at_any(&[TokenKind::Newline, TokenKind::Dedent, TokenKind::Eof]))
            .then(|| self.expression(0));
        let end = value
            .as_ref()
            .map_or(token.span.end(), |expression| expression.span().end());
        self.finish_line();
        Stmt::Return {
            value,
            span: self.span_from(token.span.start(), end),
        }
    }

    pub(crate) fn if_after_keyword(&mut self, start: usize) -> If {
        let condition = self.expression_deny_trailing();
        let body = self.block("expected indented `if` body");
        let mut elifs = Vec::new();
        while self.take(TokenKind::Elif).is_some() {
            let condition = self.expression_deny_trailing();
            let body = self.block("expected indented `elif` body");
            elifs.push((condition, body));
        }
        let otherwise = self
            .take(TokenKind::Else)
            .map(|_| self.block("expected indented `else` body"));
        let end = otherwise.as_ref().map_or_else(
            || {
                elifs
                    .last()
                    .map_or(body.span.end(), |(_, block)| block.span.end())
            },
            |block| block.span.end(),
        );
        If {
            condition,
            body,
            elifs,
            otherwise,
            span: self.span_from(start, end),
        }
    }

    fn for_statement(&mut self) -> Stmt {
        let start = self.advance().span.start();
        let kind = if self.at(TokenKind::Colon) {
            ForKind::Infinite
        } else if self.at(TokenKind::Identifier)
            && (self.nth(1).kind == TokenKind::In
                || (self.nth(1).kind == TokenKind::Comma
                    && self.nth(2).kind == TokenKind::Identifier
                    && self.nth(3).kind == TokenKind::In))
        {
            let value = self.name("expected loop value binding");
            let index = if self.take(TokenKind::Comma).is_some() {
                Some(self.name("expected loop index binding"))
            } else {
                None
            };
            self.expect(
                TokenKind::In,
                codes::E5001,
                "expected `in` in iterable loop",
            );
            let iterable = self.expression_deny_trailing();
            ForKind::Iterable {
                value,
                index,
                iterable,
            }
        } else {
            ForKind::Conditional(self.expression_deny_trailing())
        };
        let body = self.block("expected indented `for` body");
        let span = self.span_from(start, body.span.end());
        Stmt::For(For { kind, body, span })
    }

    pub(crate) fn block(&mut self, message: &str) -> Block {
        let start = self.current().span.start();
        self.block_from(start, message)
    }

    pub(crate) fn block_from(&mut self, start: usize, message: &str) -> Block {
        let colon = self.expect(TokenKind::Colon, codes::E0102, "expected `:` before block");
        self.indented_block(start.min(colon.span.start()), message)
    }

    /// Parses one indented block body, assuming its header colon was consumed.
    pub(crate) fn indented_block(&mut self, start: usize, message: &str) -> Block {
        self.expect(
            TokenKind::Newline,
            codes::E0101,
            "expected newline before block",
        );
        // Blank and comment-only lines are not statements and may appear
        // between a block header and its first indented statement.
        self.skip_newlines();
        let indent = self.expect(TokenKind::Indent, codes::E0106, message);
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.at_any(&[TokenKind::Dedent, TokenKind::Eof]) {
            statements.push(self.statement());
            self.skip_newlines();
        }
        let end = self
            .take(TokenKind::Dedent)
            .map_or_else(|| self.current().span.end(), |token| token.span.end());
        Block {
            statements,
            span: self.span_from(start.min(indent.span.start()), end),
        }
    }
}
