use crate::Parser;
use vut_ast::{
    Attribute, AttributeArg, Data, DataField, Enum, ExternFunction, Function, Import, ImportMode,
    Interface, Method, Parameter, Signature, TypeAlias, TypeExpr, TypeParameter,
};
use vut_diagnostics::codes;
use vut_lexer::TokenKind;

/// One parenthesized comma-separated type list produced while parsing a
/// function type, together with the span covering its parentheses.
struct TypeList {
    types: Vec<TypeExpr>,
    span: vut_source::Span,
    end: usize,
}

impl Parser<'_> {
    pub(crate) fn attributes(&mut self) -> Vec<Attribute> {
        let mut attributes = Vec::new();
        while self.at(TokenKind::AtSign) && self.nth(1).kind == TokenKind::Identifier {
            attributes.push(self.attribute());
            self.finish_line();
            self.skip_newlines();
        }
        attributes
    }

    fn attribute(&mut self) -> Attribute {
        let start = self.advance().span.start();
        let name = self.name("expected attribute name");
        let mut args = Vec::new();
        if self.take(TokenKind::LParen).is_some() {
            while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
                args.push(self.attribute_arg());
                if self.take(TokenKind::Comma).is_none() {
                    break;
                }
            }
            let end = self
                .expect(
                    TokenKind::RParen,
                    codes::E0101,
                    "expected `)` after attribute arguments",
                )
                .span
                .end();
            return Attribute {
                name,
                args,
                span: self.span_from(start, end),
            };
        }
        Attribute {
            span: self.span_from(start, name.span.end()),
            name,
            args,
        }
    }

    fn attribute_arg(&mut self) -> AttributeArg {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier => AttributeArg::Ident(vut_ast::Name {
                text: self.slice(token.span).to_owned(),
                span: token.span,
            }),
            TokenKind::Integer => AttributeArg::Integer {
                text: self.slice(token.span).to_owned(),
                span: token.span,
            },
            TokenKind::True | TokenKind::False => AttributeArg::Bool {
                value: token.kind == TokenKind::True,
                span: token.span,
            },
            TokenKind::StringStart => {
                let (value, span) = self.simple_string(token.span.start());
                AttributeArg::String { value, span }
            }
            _ => {
                self.error(
                    codes::E0101,
                    token.span,
                    "expected identifier, string, integer, or boolean attribute argument",
                );
                AttributeArg::Ident(vut_ast::Name {
                    text: String::new(),
                    span: token.span,
                })
            }
        }
    }

    fn simple_string(&mut self, start: usize) -> (String, vut_source::Span) {
        let mut value = String::new();
        while !self.at_any(&[TokenKind::StringEnd, TokenKind::Eof, TokenKind::Error]) {
            if self.kind() == TokenKind::StringText {
                let token = self.advance();
                value.push_str(self.slice(token.span));
            } else {
                self.error(
                    codes::E0101,
                    self.current().span,
                    "attribute strings cannot contain interpolation",
                );
                self.advance();
            }
        }
        let end = self
            .take(TokenKind::StringEnd)
            .map_or_else(|| self.current().span.end(), |token| token.span.end());
        (value, self.span_from(start, end))
    }

    pub(crate) fn import(&mut self) -> Import {
        let start = self.advance().span.start();
        let relative_dots = self.take(TokenKind::RelativeDots);
        let relative = relative_dots.is_some();
        let parent_depth =
            relative_dots.map_or(0, |token| self.slice(token.span).len().saturating_sub(1));
        let mut path = vec![self.name("expected module path")];
        while self.take(TokenKind::Dot).is_some() {
            path.push(self.name("expected module path segment"));
        }
        let mode = if self.take(TokenKind::As).is_some() {
            let alias = self.name("expected import alias");
            if self.at(TokenKind::At) {
                self.error(
                    codes::E0108,
                    self.current().span,
                    "an import cannot combine `as` and `at`",
                );
                while !self.at_any(&[TokenKind::Newline, TokenKind::Eof]) {
                    self.advance();
                }
            }
            ImportMode::Alias(alias)
        } else if self.take(TokenKind::At).is_some() {
            let mut names = vec![self.name("expected selected import")];
            while self.take(TokenKind::Comma).is_some() {
                names.push(self.name("expected selected import"));
            }
            if self.at(TokenKind::As) {
                self.error(
                    codes::E0108,
                    self.current().span,
                    "an import cannot combine `at` and `as`",
                );
                while !self.at_any(&[TokenKind::Newline, TokenKind::Eof]) {
                    self.advance();
                }
            }
            ImportMode::Selected(names)
        } else {
            ImportMode::Whole
        };
        let end = path.last().map_or(start, |name| name.span.end());
        self.finish_line();
        Import {
            relative,
            parent_depth,
            path,
            mode,
            span: self.span_from(start, end),
        }
    }

    pub(crate) fn function_or_method(
        &mut self,
        attributes: Vec<Attribute>,
        is_async: bool,
        is_static: bool,
    ) -> vut_ast::Item {
        let start = self.advance().span.start();
        let mut declaration_path = vec![self.name("expected function or receiver type name")];
        while self.take(TokenKind::Dot).is_some() {
            declaration_path.push(self.member_name("expected method name after `.`"));
        }
        let type_parameters = if self.type_parameter_list_ahead() {
            self.type_parameters()
        } else {
            Vec::new()
        };
        let parameters = self.parameters();
        let explicit_self = parameters
            .iter()
            .find(|parameter| parameter.name.text == "self")
            .map(|parameter| parameter.span);
        let return_type = self.take(TokenKind::Arrow).map(|_| self.type_expression());
        if declaration_path.len() == 1 {
            if is_static {
                self.error(
                    codes::E0101,
                    self.span_from(start, start),
                    "static function requires a receiver type (`static fn Type.name(...)`)",
                );
            }
            let body = self.block("expected indented function body");
            return vut_ast::Item::Function(Function {
                attributes,
                is_async,
                name: declaration_path.remove(0),
                type_parameters,
                parameters,
                return_type,
                span: self.span_from(start, body.span.end()),
                body,
            });
        }
        let name = declaration_path.pop().unwrap();
        let receiver_start = declaration_path.first().unwrap().span.start();
        let receiver_end = declaration_path.last().unwrap().span.end();
        let receiver = TypeExpr::Named {
            path: declaration_path,
            span: self.span_from(receiver_start, receiver_end),
        };
        if let Some(span) = explicit_self {
            self.error(
                codes::E6008,
                span,
                "`self` is compiler-injected and must not be declared",
            );
        }
        let body = self.block("expected indented method body");
        vut_ast::Item::Method(Method {
            attributes,
            is_async,
            is_static,
            receiver,
            name,
            type_parameters,
            parameters,
            return_type,
            span: self.span_from(start, body.span.end()),
            body,
        })
    }

    pub(crate) fn extern_function(&mut self, attributes: Vec<Attribute>) -> ExternFunction {
        let start = self.advance().span.start();
        let abi_start = self
            .expect(TokenKind::StringStart, codes::E0101, "expected ABI string")
            .span
            .start();
        let (abi, abi_span) = self.simple_string(abi_start);
        let is_async = self.take(TokenKind::Async).is_some();
        self.expect(
            TokenKind::Fn,
            codes::E0110,
            "expected `fn` after extern ABI",
        );
        let name = self.name("expected extern function name");
        let type_parameters = if self.type_parameter_list_ahead() {
            self.type_parameters()
        } else {
            Vec::new()
        };
        let parameters = self.parameters();
        let return_type = self.take(TokenKind::Arrow).map(|_| self.type_expression());
        let end = return_type.as_ref().map_or_else(
            || {
                parameters
                    .last()
                    .map_or(name.span.end(), |parameter| parameter.span.end())
            },
            |ty| ty.span().end(),
        );
        if self.at(TokenKind::Colon) {
            self.error(
                codes::E8007,
                self.current().span,
                "extern functions cannot have a Vut function body",
            );
            let _ = self.block("extern declarations identify a foreign symbol");
        } else {
            self.finish_line();
        }
        ExternFunction {
            attributes,
            abi,
            abi_span,
            is_async,
            name,
            type_parameters,
            parameters,
            return_type,
            span: self.span_from(start, end),
        }
    }

    pub(crate) fn data(&mut self, attributes: Vec<Attribute>) -> Data {
        let start = self.advance().span.start();
        let name = self.name("expected data name");
        let type_parameters = if self.at(TokenKind::LParen) {
            self.type_parameters()
        } else {
            Vec::new()
        };
        self.expect(
            TokenKind::Colon,
            codes::E0102,
            "expected `:` after data name",
        );
        self.expect(
            TokenKind::Newline,
            codes::E0101,
            "expected newline after data header",
        );
        self.expect(
            TokenKind::Indent,
            codes::E0106,
            "expected indented data fields",
        );
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at_any(&[TokenKind::Dedent, TokenKind::Eof]) {
            let field_start = self.current().span.start();
            let field_name = self.name("expected field name");
            self.expect(
                TokenKind::Colon,
                codes::E0102,
                "expected `:` after field name",
            );
            let ty = self.type_expression();
            let default = self.take(TokenKind::Equal).map(|_| self.expression(0));
            let end = default
                .as_ref()
                .map_or(ty.span().end(), |value| value.span().end());
            fields.push(DataField {
                name: field_name,
                ty,
                default,
                span: self.span_from(field_start, end),
            });
            self.finish_line();
            self.skip_newlines();
        }
        let end = self
            .expect(
                TokenKind::Dedent,
                codes::E0106,
                "expected end of data block",
            )
            .span
            .end();
        Data {
            attributes,
            opaque: false,
            name,
            type_parameters,
            fields,
            span: self.span_from(start, end),
        }
    }

    pub(crate) fn opaque_data(&mut self, attributes: Vec<Attribute>) -> Data {
        let start = self.advance().span.start();
        self.expect(
            TokenKind::Data,
            codes::E0101,
            "expected `data` after `opaque`",
        );
        let name = self.name("expected opaque data name");
        self.finish_line();
        Data {
            attributes,
            opaque: true,
            name: name.clone(),
            type_parameters: Vec::new(),
            fields: Vec::new(),
            span: self.span_from(start, name.span.end()),
        }
    }

    pub(crate) fn interface(&mut self, attributes: Vec<Attribute>) -> Interface {
        let start = self.advance().span.start();
        let name = self.name("expected interface name");
        let type_parameters = if self.at(TokenKind::LParen) {
            self.type_parameters()
        } else {
            Vec::new()
        };
        self.expect(
            TokenKind::Colon,
            codes::E0102,
            "expected `:` after interface name",
        );
        let mut parents = Vec::new();
        if !self.at(TokenKind::Newline) {
            parents.push(self.name("expected parent interface"));
            while self.take(TokenKind::Comma).is_some() {
                parents.push(self.name("expected parent interface"));
            }
            self.expect(
                TokenKind::Colon,
                codes::E0102,
                "expected `:` after interface parents",
            );
        }
        self.expect(
            TokenKind::Newline,
            codes::E0101,
            "expected newline after interface header",
        );
        self.expect(
            TokenKind::Indent,
            codes::E0106,
            "expected interface requirements",
        );
        let mut methods = Vec::new();
        self.skip_newlines();
        while !self.at_any(&[TokenKind::Dedent, TokenKind::Eof]) {
            let method_start = self.current().span.start();
            let is_static = self.take(TokenKind::Static).is_some();
            let method_name = self.name("expected interface method name");
            let parameters = self.parameters();
            let return_type = self.take(TokenKind::Arrow).map(|_| self.type_expression());
            let end = return_type.as_ref().map_or_else(
                || {
                    parameters
                        .last()
                        .map_or(method_name.span.end(), |parameter| parameter.span.end())
                },
                |ty| ty.span().end(),
            );
            methods.push(Signature {
                name: method_name,
                is_static,
                parameters,
                return_type,
                span: self.span_from(method_start, end),
            });
            self.finish_line();
            self.skip_newlines();
        }
        let end = self
            .expect(
                TokenKind::Dedent,
                codes::E0106,
                "expected end of interface block",
            )
            .span
            .end();
        Interface {
            attributes,
            name,
            type_parameters,
            parents,
            methods,
            span: self.span_from(start, end),
        }
    }

    pub(crate) fn enum_declaration(&mut self, attributes: Vec<Attribute>) -> Enum {
        let start = self.advance().span.start();
        let name = self.name("expected enum name");
        let type_parameters = if self.at(TokenKind::LParen) {
            self.type_parameters()
        } else {
            Vec::new()
        };
        self.expect(
            TokenKind::Colon,
            codes::E0102,
            "expected `:` after enum name",
        );
        self.expect(
            TokenKind::Newline,
            codes::E0101,
            "expected newline after enum header",
        );
        self.expect(TokenKind::Indent, codes::E0106, "expected enum variants");
        let mut variants = Vec::new();
        self.skip_newlines();
        while !self.at_any(&[TokenKind::Dedent, TokenKind::Eof]) {
            let variant_start = self.current().span.start();
            let variant_name = self.name("expected enum variant");
            let mut fields = Vec::new();
            if self.take(TokenKind::LParen).is_some() {
                while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
                    let field_start = self.current().span.start();
                    let field_name = self.name("expected variant field name");
                    self.expect(
                        TokenKind::Colon,
                        codes::E0110,
                        "expected `:` after variant field name",
                    );
                    let ty = self.type_expression();
                    fields.push(vut_ast::VariantField {
                        name: field_name,
                        span: self.span_from(field_start, ty.span().end()),
                        ty,
                    });
                    if self.take(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                self.expect(
                    TokenKind::RParen,
                    codes::E0101,
                    "expected `)` after variant fields",
                );
            }
            let variant_end = fields
                .last()
                .map_or_else(|| variant_name.span.end(), |field| field.span.end());
            variants.push(vut_ast::EnumVariant {
                name: variant_name,
                fields,
                span: self.span_from(variant_start, variant_end),
            });
            self.finish_line();
            self.skip_newlines();
        }
        let end = self
            .expect(
                TokenKind::Dedent,
                codes::E0106,
                "expected end of enum block",
            )
            .span
            .end();
        Enum {
            attributes,
            name,
            type_parameters,
            variants,
            span: self.span_from(start, end),
        }
    }

    pub(crate) fn type_alias(&mut self, attributes: Vec<Attribute>) -> TypeAlias {
        let start = self.advance().span.start();
        let name = self.name("expected alias name");
        self.expect(TokenKind::Equal, codes::E0110, "expected `=` in type alias");
        let ty = self.type_expression();
        let span = self.span_from(start, ty.span().end());
        self.finish_line();
        TypeAlias {
            attributes,
            name,
            ty,
            span,
        }
    }

    pub(crate) fn type_expression(&mut self) -> TypeExpr {
        if self.at(TokenKind::Fn) {
            return self.function_type();
        }
        if self.at(TokenKind::Extern) {
            return self.extern_function_type();
        }
        let first = self.type_name();
        let start = first.span.start();
        let mut path = vec![first];
        while self.take(TokenKind::Dot).is_some() {
            path.push(self.type_name());
        }
        let mut ty = if self.take(TokenKind::LParen).is_some() {
            if path.len() != 1 {
                self.error(
                    codes::E0101,
                    self.current().span,
                    "parameterized type name must be unqualified",
                );
            }
            let name = path.remove(0);
            if name.text == "array" {
                let element = self.type_expression();
                self.expect(
                    TokenKind::Comma,
                    codes::E0101,
                    "expected `,` before array length",
                );
                let length_token = self.expect(
                    TokenKind::Integer,
                    codes::E0101,
                    "array length must be a compile-time integer",
                );
                let length = self
                    .slice(length_token.span)
                    .replace('_', "")
                    .parse()
                    .unwrap_or(0);
                let end = self
                    .expect(
                        TokenKind::RParen,
                        codes::E0101,
                        "expected `)` after array type",
                    )
                    .span
                    .end();
                TypeExpr::Array {
                    element: Box::new(element),
                    length,
                    length_span: length_token.span,
                    span: self.span_from(start, end),
                }
            } else {
                let mut arguments = Vec::new();
                while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
                    arguments.push(self.type_expression());
                    if self.take(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                let end = self
                    .expect(
                        TokenKind::RParen,
                        codes::E0101,
                        "expected `)` after type arguments",
                    )
                    .span
                    .end();
                TypeExpr::Applied {
                    name,
                    arguments,
                    span: self.span_from(start, end),
                }
            }
        } else {
            let end = path.last().unwrap().span.end();
            TypeExpr::Named {
                path,
                span: self.span_from(start, end),
            }
        };
        if let Some(question) = self.take(TokenKind::Question) {
            ty = TypeExpr::Optional {
                inner: Box::new(ty),
                span: self.span_from(start, question.span.end()),
            };
        }
        ty
    }

    fn function_type(&mut self) -> TypeExpr {
        let start = self.advance().span.start();
        let first = self.parenthesized_type_list(
            "expected `(` before function type parameters",
            "expected `)` after function type parameters",
        );
        // `fn(Receiver)(Args...) -> T`: two adjacent parenthesized groups mean
        // the first one is the receiver and the second the ordinary parameters.
        // `fn(Scope) -> fn(Event) -> T` puts `->` between the groups, so an
        // adjacent `(` uniquely marks a receiver.
        let (receiver, parameters, end_after_parameters) = if self.at(TokenKind::LParen) {
            let receiver = if first.types.len() == 1 {
                first.types.into_iter().next().map(Box::new)
            } else {
                self.error(
                    codes::E0110,
                    first.span,
                    "a receiver function type must declare exactly one receiver type",
                );
                None
            };
            let second = self.parenthesized_type_list(
                "expected `(` before receiver function parameters",
                "expected `)` after receiver function parameters",
            );
            (receiver, second.types, second.end)
        } else {
            (None, first.types, first.end)
        };
        let mut end = end_after_parameters;
        let return_type = if self.take(TokenKind::Arrow).is_some() {
            let ty = self.type_expression();
            end = ty.span().end();
            Some(Box::new(ty))
        } else {
            None
        };
        TypeExpr::Function {
            receiver,
            parameters,
            return_type,
            span: self.span_from(start, end),
        }
    }

    /// Parses one parenthesized, comma-separated type list. Returns the types
    /// together with the span covering the enclosing parentheses.
    fn parenthesized_type_list(&mut self, open_message: &str, close_message: &str) -> TypeList {
        let open = self.expect(TokenKind::LParen, codes::E0110, open_message);
        let mut types = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            types.push(self.type_expression());
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        let close = self.expect(TokenKind::RParen, codes::E0110, close_message);
        TypeList {
            types,
            span: self.span_from(open.span.start(), close.span.end()),
            end: close.span.end(),
        }
    }

    fn extern_function_type(&mut self) -> TypeExpr {
        let start = self.advance().span.start();
        let abi_start = self
            .expect(TokenKind::StringStart, codes::E0101, "expected ABI string")
            .span
            .start();
        let (abi, abi_span) = self.simple_string(abi_start);
        self.expect(
            TokenKind::Fn,
            codes::E0110,
            "expected `fn` after extern ABI",
        );
        self.expect(
            TokenKind::LParen,
            codes::E0110,
            "expected `(` before extern function pointer parameters",
        );
        let mut parameters = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            if self.at(TokenKind::Identifier) && self.nth(1).kind == TokenKind::Colon {
                self.advance();
                self.advance();
            }
            parameters.push(self.type_expression());
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        let close = self.expect(
            TokenKind::RParen,
            codes::E0110,
            "expected `)` after extern function pointer parameters",
        );
        let return_type = self
            .take(TokenKind::Arrow)
            .map(|_| Box::new(self.type_expression()));
        let end = return_type
            .as_ref()
            .map_or_else(|| close.span.end(), |ty| ty.span().end());
        TypeExpr::ExternFunction {
            abi,
            abi_span,
            parameters,
            return_type,
            span: self.span_from(start, end),
        }
    }

    fn type_name(&mut self) -> vut_ast::Name {
        if self.at(TokenKind::Dyn) {
            let token = self.advance();
            vut_ast::Name {
                text: "dyn".into(),
                span: token.span,
            }
        } else {
            self.name("expected type name")
        }
    }

    /// Parses a generic type-parameter list: `(T, U: Bound)`.
    fn type_parameters(&mut self) -> Vec<TypeParameter> {
        self.expect(
            TokenKind::LParen,
            codes::E0110,
            "expected `(` before type parameters",
        );
        let mut parameters = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            let start = self.current().span.start();
            let name = self.name("expected type parameter");
            let mut bounds = Vec::new();
            if self.take(TokenKind::Colon).is_some() {
                bounds.push(self.type_expression());
                while self.take(TokenKind::Plus).is_some() {
                    bounds.push(self.type_expression());
                }
            }
            let end = bounds
                .last()
                .map_or(name.span.end(), |bound| bound.span().end());
            parameters.push(TypeParameter {
                name,
                bounds,
                span: self.span_from(start, end),
            });
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(
            TokenKind::RParen,
            codes::E0110,
            "expected `)` after type parameters",
        );
        parameters
    }

    /// Returns true when the parenthesized group starting at the current `(` is
    /// immediately followed by another `(`. Generic functions/declarations put
    /// the type-parameter list before the value-parameter list, so two adjacent
    /// groups disambiguate `fn f(T)(x: T)` from `fn f(x: T)`.
    fn type_parameter_list_ahead(&self) -> bool {
        if !self.at(TokenKind::LParen) {
            return false;
        }
        let mut depth = 0_usize;
        let mut offset = 0_usize;
        loop {
            match self.nth(offset).kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                    if depth == 0 {
                        return self.nth(offset + 1).kind == TokenKind::LParen;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            offset += 1;
        }
    }

    fn parameters(&mut self) -> Vec<Parameter> {
        self.expect(
            TokenKind::LParen,
            codes::E0110,
            "expected `(` before parameters",
        );
        let mut parameters = Vec::new();
        while !self.at_any(&[TokenKind::RParen, TokenKind::Eof]) {
            let start = self.current().span.start();
            let variadic = self.take(TokenKind::Ellipsis).is_some();
            let name = self.name("expected parameter name");
            self.expect(
                TokenKind::Colon,
                codes::E0110,
                "expected `:` after parameter name",
            );
            let ty = self.type_expression();
            parameters.push(Parameter {
                name,
                span: self.span_from(start, ty.span().end()),
                ty,
                variadic,
            });
            if self.take(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(
            TokenKind::RParen,
            codes::E0110,
            "expected `)` after parameters",
        );
        parameters
    }
}
