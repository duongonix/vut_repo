//! Expression and call checking.
use super::context::Context;
#[allow(clippy::wildcard_imports)]
use super::*;

impl Analyzer<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "keeps the exhaustive expression visitor together"
    )]
    pub(super) fn expr(
        &mut self,
        module: &Module,
        expr: &Expr,
        context: &mut Context,
        expected: Option<TypeId>,
    ) -> TypeId {
        let ty = match expr {
            Expr::Name(name) => {
                let ty = context
                    .lookup(&name.text)
                    .or_else(|| {
                        module
                            .symbols
                            .get(&name.text)
                            .copied()
                            .map(|s| self.symbol_type(s))
                    })
                    .or_else(|| {
                        module
                            .imports
                            .get(&name.text)
                            .copied()
                            .map(|s| self.symbol_type(s))
                    })
                    .unwrap_or_else(|| self.intern(Type::Error));
                if let Type::Function(symbol) = self.types[ty.0]
                    && self.async_symbols.contains(&symbol)
                {
                    self.error(
                        codes::E6014,
                        "awaitable value must be awaited before use",
                        name.span,
                        "`async fn` is not a first-class value; call it inside `await`",
                    );
                }
                ty
            }
            Expr::Lambda {
                receiver,
                parameters,
                body,
                is_async,
                span,
            } => {
                if *is_async && !context.in_spawn_callable {
                    self.error(
                        codes::E6024,
                        "anonymous `async fn` outside `vut(...)`",
                        *span,
                        "anonymous async callables are only supported as the argument of `vut(...)`",
                    );
                }
                self.lambda(
                    module, parameters, receiver, body, *is_async, *span, expected, context,
                )
            }
            Expr::Integer { text, span } => {
                let ty = expected
                    .filter(|id| self.is_integer(*id))
                    .unwrap_or_else(|| self.intern(Type::Int));
                if !self.integer_fits(text, ty) {
                    self.error(
                        codes::E1008,
                        "integer literal out of range",
                        *span,
                        "literal does not fit its contextual numeric type",
                    );
                }
                ty
            }
            Expr::Float { .. } => expected
                .filter(|id| self.is_float(*id))
                .unwrap_or_else(|| self.intern(Type::Float)),
            Expr::String { segments, .. } => {
                for segment in segments {
                    if let vut_ast::TemplateSegment::Expression(value) = segment {
                        let ty = self.expr(module, value, context, None);
                        if matches!(self.types[ty.0], Type::Optional(_)) {
                            self.error(
                                codes::E1007,
                                "cannot format an optional value",
                                value.span(),
                                "narrow it with `if value != null` or use `get_or`",
                            );
                        }
                    }
                }
                self.intern(Type::Str)
            }
            Expr::Bool { .. } => self.intern(Type::Bool),
            Expr::Null(_) => self.intern(Type::Null),
            Expr::List { values, span } => {
                let contextual = expected.and_then(|id| match self.types[id.0] {
                    Type::List(inner) => Some(inner),
                    _ => None,
                });
                if values.is_empty() {
                    if let Some(inner) = contextual {
                        self.intern(Type::List(inner))
                    } else {
                        self.error(
                            codes::E1004,
                            "cannot infer empty list type",
                            *span,
                            "add an explicit `list(T)` annotation",
                        );
                        self.intern(Type::Error)
                    }
                } else {
                    let first = self.expr(module, &values[0], context, contextual);
                    for value in &values[1..] {
                        let found = self.expr(module, value, context, contextual);
                        if !self.is_compatible(found, first) {
                            self.error(
                                codes::E1005,
                                "heterogeneous list",
                                value.span(),
                                "all list elements must have one static type",
                            );
                        }
                    }
                    self.intern(Type::List(contextual.unwrap_or(first)))
                }
            }
            Expr::Array { values, span } => {
                let contextual = expected.and_then(|id| match self.types[id.0] {
                    Type::Array(inner, _) => Some(inner),
                    _ => None,
                });
                if values.is_empty() {
                    self.error(
                        codes::E1004,
                        "empty array is not supported",
                        *span,
                        "array literals require at least one element",
                    );
                    self.intern(Type::Error)
                } else {
                    let first = self.expr(module, &values[0], context, contextual);
                    for value in &values[1..] {
                        let found = self.expr(module, value, context, contextual);
                        if !self.is_compatible(found, first) {
                            self.error(
                                codes::E1005,
                                "heterogeneous array",
                                value.span(),
                                "all array elements must have one static type",
                            );
                        }
                    }
                    self.intern(Type::Array(contextual.unwrap_or(first), values.len()))
                }
            }
            Expr::Map { entries, span } => {
                let contextual = expected.and_then(|id| match self.types[id.0] {
                    Type::Map(key, value) => Some((key, value)),
                    _ => None,
                });
                if entries.is_empty() {
                    if let Some((key, value)) = contextual {
                        self.validate_map_key(key, *span);
                        self.intern(Type::Map(key, value))
                    } else {
                        self.error(
                            codes::E1004,
                            "cannot infer empty map type",
                            *span,
                            "add an explicit `map(K, V)` annotation",
                        );
                        self.intern(Type::Error)
                    }
                } else {
                    let first_key =
                        self.expr(module, &entries[0].key, context, contextual.map(|v| v.0));
                    let first_value =
                        self.expr(module, &entries[0].value, context, contextual.map(|v| v.1));
                    for entry in &entries[1..] {
                        let key = self.expr(module, &entry.key, context, contextual.map(|v| v.0));
                        let value =
                            self.expr(module, &entry.value, context, contextual.map(|v| v.1));
                        if !self.is_compatible(key, first_key) {
                            self.error(
                                codes::E1005,
                                "heterogeneous map keys",
                                entry.key.span(),
                                "all map keys must have one static type",
                            );
                        }
                        if !self.is_compatible(value, first_value) {
                            self.error(
                                codes::E1005,
                                "heterogeneous map values",
                                entry.value.span(),
                                "all map values must have one static type",
                            );
                        }
                    }
                    let key = contextual.map_or(first_key, |v| v.0);
                    self.validate_map_key(key, *span);
                    self.intern(Type::Map(key, contextual.map_or(first_value, |v| v.1)))
                }
            }
            Expr::Unary { op, value, span } => {
                let inner = self.expr(module, value, context, None);
                match op {
                    UnaryOp::Not => {
                        let bool_ty = self.intern(Type::Bool);
                        self.compatible(inner, bool_ty, *span, codes::E1009, "`not` requires bool");
                        self.intern(Type::Bool)
                    }
                    UnaryOp::Negate | UnaryOp::Positive if self.is_numeric(inner) => inner,
                    _ => {
                        self.error(
                            codes::E1009,
                            "invalid unary operand",
                            *span,
                            "operator requires a numeric operand",
                        );
                        self.intern(Type::Error)
                    }
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let lhs = self.expr(module, left, context, None);
                let rhs = self.expr(module, right, context, Some(lhs));
                match op {
                    BinaryOp::And | BinaryOp::Or => {
                        let bool_ty = self.intern(Type::Bool);
                        self.compatible(
                            lhs,
                            bool_ty,
                            left.span(),
                            codes::E1009,
                            "logical operand must be bool",
                        );
                        self.compatible(
                            rhs,
                            bool_ty,
                            right.span(),
                            codes::E1009,
                            "logical operand must be bool",
                        );
                        bool_ty
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        self.compatible(
                            rhs,
                            lhs,
                            *span,
                            codes::E1009,
                            "comparison operands differ",
                        );
                        self.intern(Type::Bool)
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
                        if self.is_numeric(lhs) && self.is_numeric(rhs) =>
                    {
                        self.intern(Type::Bool)
                    }
                    BinaryOp::RangeExclusive | BinaryOp::RangeInclusive
                        if self.is_integer(lhs) && self.is_integer(rhs) =>
                    {
                        self.intern(Type::Range(lhs))
                    }
                    BinaryOp::Add
                        if self.types[lhs.0] == Type::Str && self.types[rhs.0] == Type::Str =>
                    {
                        lhs
                    }
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                        if self.is_numeric(lhs) && self.is_compatible(rhs, lhs) =>
                    {
                        lhs
                    }
                    _ => {
                        self.error(
                            codes::E1009,
                            "invalid operator types",
                            *span,
                            "operator does not support these operand types",
                        );
                        self.intern(Type::Error)
                    }
                }
            }
            Expr::Member { object, member, .. } => {
                // A module-qualified type used as a value prefix
                // (`module.Enum.variant`, `module.Type.field`) names the type;
                // the resolver has already resolved the qualifier.
                if let Some(symbol) =
                    self.reference_symbols
                        .get(&member.span)
                        .copied()
                        .filter(|symbol| {
                            matches!(
                                self.symbol_kind(*symbol),
                                vut_resolver::SymbolKind::Data
                                    | vut_resolver::SymbolKind::Enum
                                    | vut_resolver::SymbolKind::TypeAlias
                            )
                        })
                {
                    return self.symbol_type(symbol);
                }
                let owner = self.expr(module, object, context, None);
                let ty = self.field(module.id, owner, member, None);
                // Annotation-driven inference for generic enum variants.
                if let Some(expected) = expected
                    && let Type::Enum(receiver) = self.types[owner.0]
                    && let Type::Enum(instance) = self.types[expected.0]
                    && self.instance_template(instance) == Some(receiver)
                {
                    self.expression_types.insert(expr.span(), expected);
                    return expected;
                }
                ty
            }
            Expr::Call {
                callee,
                arguments,
                span,
            } => self.call(module, callee, arguments, *span, context, expected),
            Expr::ResultOk { value, span } => {
                let Some(expected) = expected else {
                    self.error(
                        codes::E1004,
                        "cannot infer result type",
                        *span,
                        "`ok(...)` requires an expected `result(T, E)` type",
                    );
                    self.expr(module, value, context, None);
                    return self.intern(Type::Error);
                };
                let Type::Result(ok_ty, _) = self.types[expected.0] else {
                    self.error(
                        codes::E1003,
                        "invalid result constructor context",
                        *span,
                        "`ok(...)` can only construct `result(T, E)`",
                    );
                    self.expr(module, value, context, None);
                    return self.intern(Type::Error);
                };
                let actual = self.expr(module, value, context, Some(ok_ty));
                self.compatible(
                    actual,
                    ok_ty,
                    value.span(),
                    codes::E1003,
                    "ok payload mismatch",
                );
                expected
            }
            Expr::ResultErr { value, span } => {
                let Some(expected) = expected else {
                    self.error(
                        codes::E1004,
                        "cannot infer result type",
                        *span,
                        "`err(...)` requires an expected `result(T, E)` type",
                    );
                    self.expr(module, value, context, None);
                    return self.intern(Type::Error);
                };
                let Type::Result(_, err_ty) = self.types[expected.0] else {
                    self.error(
                        codes::E1003,
                        "invalid result constructor context",
                        *span,
                        "`err(...)` can only construct `result(T, E)`",
                    );
                    self.expr(module, value, context, None);
                    return self.intern(Type::Error);
                };
                let actual = self.expr(module, value, context, Some(err_ty));
                self.compatible(
                    actual,
                    err_ty,
                    value.span(),
                    codes::E1003,
                    "err payload mismatch",
                );
                expected
            }
            Expr::ResultPropagate { value, span } => {
                let source = self.expr(module, value, context, None);
                let Type::Result(ok_ty, err_ty) = self.types[source.0] else {
                    self.error(
                        codes::E1003,
                        "invalid `?` operand",
                        *span,
                        "`?` requires a `result(T, E)` value",
                    );
                    return self.intern(Type::Error);
                };
                let Some(return_ty) = context.return_type else {
                    self.error(
                        codes::E6007,
                        "`?` outside function",
                        *span,
                        "`?` can only propagate from a function returning `result(_, E)`",
                    );
                    return ok_ty;
                };
                let Type::Result(_, return_err) = self.types[return_ty.0] else {
                    self.error(
                        codes::E6005,
                        "invalid `?` return type",
                        *span,
                        "current function must return `result(_, E)`",
                    );
                    return ok_ty;
                };
                self.compatible(
                    err_ty,
                    return_err,
                    *span,
                    codes::E6005,
                    "`?` error type mismatch",
                );
                ok_ty
            }
            Expr::Await { value, span } => {
                let inner_span = await_operand_span(value);
                if !context.in_async {
                    self.error(
                        codes::E6011,
                        "`await` outside `async fn`",
                        *span,
                        "`await` is only valid inside an `async fn`",
                    );
                }
                let previous = context.await_span;
                context.await_span = Some(inner_span);
                let operand = self.expr(module, value, context, None);
                context.await_span = previous;
                if matches!(self.types[operand.0], Type::Error) {
                    self.intern(Type::Error)
                } else if let Type::Vutcon(inner) = self.types[operand.0] {
                    inner
                } else if let Type::Future(inner) = self.types[operand.0] {
                    inner
                } else if self.async_calls.contains(&inner_span) {
                    operand
                } else {
                    self.error(
                        codes::E6012,
                        "cannot await non-awaitable value",
                        value.span(),
                        "the operand of `await` must be an async computation, a `future(T)`, or a `vutcon` handle",
                    );
                    self.intern(Type::Error)
                }
            }
            Expr::Spawn { callable, span } => {
                if !context.in_async {
                    self.error(
                        codes::E6022,
                        "`vut(...)` outside `async fn`",
                        *span,
                        "`vut` schedules a Vutcon and requires an async execution context",
                    );
                }
                if !matches!(callable.as_ref(), Expr::Lambda { .. }) {
                    self.error(
                        codes::E6020,
                        "`vut(...)` requires an anonymous callable",
                        callable.span(),
                        "the argument must be `() => ...`, `fn(): ...`, or `async fn(): ...`",
                    );
                    return self.intern(Type::Error);
                }
                let saved_spawn = context.in_spawn_callable;
                context.in_spawn_callable = true;
                let callable_ty = self.expr(module, callable, context, None);
                context.in_spawn_callable = saved_spawn;
                let result = match self.types[callable_ty.0].clone() {
                    Type::Function(symbol) => {
                        let signature = self.signatures.get(&symbol).cloned();
                        match signature {
                            Some(signature) if signature.parameters.is_empty() => signature.result,
                            Some(_) => {
                                self.error(
                                    codes::E6021,
                                    "Vutcon callback cannot have parameters",
                                    callable.span(),
                                    "the callback must take no parameters",
                                );
                                self.intern(Type::Error)
                            }
                            None => self.intern(Type::Error),
                        }
                    }
                    Type::Callable {
                        parameters, result, ..
                    } if parameters.is_empty() => result,
                    Type::Error => return self.intern(Type::Error),
                    _ => {
                        self.error(
                            codes::E6020,
                            "`vut(...)` requires an anonymous callable",
                            callable.span(),
                            "the argument must be `() => ...`, `fn(): ...`, or `async fn(): ...`",
                        );
                        self.intern(Type::Error)
                    }
                };
                self.intern(Type::Vutcon(result))
            }
            Expr::If(value) => {
                self.condition(module, &value.condition, context, codes::E5007);
                let narrowing = self.optional_condition_narrowing(&value.condition, context);

                let then_binding = narrowing
                    .as_ref()
                    .map(|(name, inner, is_eq)| (!*is_eq, name, inner));
                let mut branches =
                    vec![self.block_narrowed(module, &value.body, context, then_binding)];
                let mut last_narrowing = narrowing.clone();
                for (condition, body) in &value.elifs {
                    self.condition(module, condition, context, codes::E5007);
                    let elif_narrowing = self.optional_condition_narrowing(condition, context);
                    let binding = elif_narrowing
                        .as_ref()
                        .map(|(name, inner, is_eq)| (!*is_eq, name, inner));
                    branches.push(self.block_narrowed(module, body, context, binding));
                    last_narrowing = elif_narrowing;
                }
                if let Some(body) = &value.otherwise {
                    let binding = last_narrowing
                        .as_ref()
                        .map(|(name, inner, is_eq)| (*is_eq, name, inner));
                    branches.push(self.block_narrowed(module, body, context, binding));
                }
                // Guard-clause narrowing: when the absent branch always
                // terminates, the present type holds for the code after the if.
                if let Some((name, inner, is_eq)) = &narrowing
                    && *is_eq
                    && Self::block_terminates(&value.body)
                    && let Some(scope) = context.scopes.last_mut()
                {
                    scope.insert(name.clone(), *inner);
                }
                if branches.len() < 2 {
                    self.error(
                        codes::E1010,
                        "if expression requires else",
                        value.span,
                        "a value-producing if must cover all branches",
                    );
                    self.intern(Type::Error)
                } else {
                    let first = branches[0];
                    for branch in &branches[1..] {
                        self.compatible(
                            *branch,
                            first,
                            value.span,
                            codes::E1010,
                            "incompatible if branch types",
                        );
                    }
                    first
                }
            }
            Expr::Match { value, arms, span } => {
                self.check_match(module, value, arms, *span, context, expected)
            }
            Expr::Group { value, .. } => self.expr(module, value, context, expected),
            Expr::Block(block) => self.block_expect(module, block, context, expected),
            Expr::Error(_) => self.intern(Type::Error),
        };
        self.expression_types.insert(expr.span(), ty);
        ty
    }
    /// `out`/`print` accept any number of displayable arguments. The compiler's
    /// display pass rewrites the call into a single `str` argument before final
    /// checking; this accepts both the original and the rewritten form.
    fn try_display_builtin(
        &mut self,
        module: &Module,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let Expr::Name(name) = callee else {
            return None;
        };
        if !matches!(name.text.as_str(), "out" | "print") {
            return None;
        }
        let symbol = self
            .resolution
            .references
            .iter()
            .find(|reference| reference.span == name.span)
            .map(|reference| reference.symbol)
            .or_else(|| module.symbols.get(&name.text).copied())?;
        if !matches!(
            self.resolution.symbols[symbol.0].kind,
            vut_resolver::SymbolKind::Function
        ) {
            return None;
        }
        for argument in arguments {
            self.expr(module, &argument.value, context, None);
        }
        self.call_targets.insert(span, symbol);
        Some(self.intern(Type::Void))
    }

    /// Type-checks `callback.call(receiver, ...args)`, the canonical invocation
    /// of a receiver function. `.call` is reserved for receiver functions; a
    /// plain callable is invoked directly with `callback(args...)`.
    fn try_receiver_call(
        &mut self,
        module: &Module,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let Expr::Member { object, member, .. } = callee else {
            return None;
        };
        if member.text != "call" {
            return None;
        }
        let object_ty = self.expr(module, object, context, None);
        let (receiver, parameters, result) = self.expected_callable(Some(object_ty));
        let Some(parameters) = parameters else {
            self.error(
                codes::E1025,
                "`.call()` requires a receiver function",
                member.span,
                "only a `fn(Receiver)(Args...) -> T` value can be invoked with `.call(receiver, ...)`",
            );
            return Some(self.intern(Type::Error));
        };
        let Some(receiver) = receiver else {
            self.error(
                codes::E1025,
                "`.call()` requires a receiver function",
                member.span,
                "this is a plain function value; invoke it directly as `callback(args...)`",
            );
            return Some(self.intern(Type::Error));
        };
        let Some(result) = result else {
            return Some(self.intern(Type::Error));
        };
        // The receiver is checked against the declared receiver type, then the
        // remaining arguments against the ordinary parameters.
        let Some(receiver_argument) = arguments.first() else {
            self.error(
                codes::E1003,
                "missing receiver argument",
                span,
                "`.call()` takes the receiver as its first argument",
            );
            return Some(self.intern(Type::Error));
        };
        let actual_receiver = self.expr(module, &receiver_argument.value, context, Some(receiver));
        self.compatible(
            actual_receiver,
            receiver,
            receiver_argument.value.span(),
            codes::E1021,
            "receiver function receiver mismatch",
        );
        let rest_parameters = parameters
            .iter()
            .enumerate()
            .map(|(index, ty)| (format!("argument{index}"), *ty))
            .collect();
        let rest_signature = Signature {
            parameters: rest_parameters,
            result,
        };
        self.arguments(
            module,
            &rest_signature,
            &arguments[1..],
            span,
            context,
            false,
        );
        self.receiver_calls.insert(span);
        Some(result)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "call checking keeps builtin dispatch and user resolution together"
    )]
    pub(super) fn call(
        &mut self,
        module: &Module,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
        expected: Option<TypeId>,
    ) -> TypeId {
        if let Some(result) = self.try_display_builtin(module, callee, arguments, span, context) {
            return result;
        }
        if let Some(result) = self.try_receiver_call(module, callee, arguments, span, context) {
            return result;
        }
        if let Some(result) = self.try_builtin_call(module, callee, arguments, span, context) {
            return result;
        }
        if let Some(result) = self.try_bytes_constructor(module, callee, arguments, span, context) {
            return result;
        }
        if let Some(result) = self.try_static_call(module, callee, arguments, span, context) {
            return result;
        }
        if let Expr::Name(name) = callee
            && self
                .resolution
                .implicit_receiver_methods
                .contains_key(&name.span)
        {
            self.implicit_receiver_calls.insert(name.span);
        }
        let target = match callee {
            Expr::Name(name) => self
                .resolution
                .references
                .iter()
                .find(|reference| reference.span == name.span)
                .map(|reference| reference.symbol)
                .or_else(|| module.symbols.get(&name.text).copied()),
            Expr::Member { object, member, .. } => {
                let imported = self
                    .resolution
                    .references
                    .iter()
                    .find(|reference| reference.span == member.span)
                    .map(|reference| reference.symbol)
                    .filter(|symbol| {
                        self.resolution.symbols[symbol.0].module != module.id
                            && matches!(
                                self.resolution.symbols[symbol.0].kind,
                                vut_resolver::SymbolKind::Function
                                    | vut_resolver::SymbolKind::Data
                                    | vut_resolver::SymbolKind::Enum
                                    | vut_resolver::SymbolKind::TypeAlias
                            )
                    });
                if imported.is_some() {
                    imported
                } else {
                    let owner = self.expr(module, object, context, None);
                    if let Type::Enum(receiver) = self.types[owner.0] {
                        // Prefer the concrete enum instance selected by the
                        // expected type, when the annotation names one.
                        let enum_symbol = expected
                            .and_then(|expected| match self.types[expected.0] {
                                Type::Enum(instance)
                                    if self.instance_template(instance) == Some(receiver) =>
                                {
                                    Some(instance)
                                }
                                _ => None,
                            })
                            .unwrap_or(receiver);
                        if let Some(index) = self.variant_index(enum_symbol, &member.text) {
                            let info = self.enum_variants(enum_symbol).unwrap()[index].clone();
                            return self.construct_variant(
                                module,
                                enum_symbol,
                                index,
                                &info,
                                arguments,
                                span,
                                context,
                            );
                        }
                    }
                    if let Type::Data(receiver) | Type::Enum(receiver) = self.types[owner.0] {
                        let receiver_module = self.symbol_module(receiver);
                        self.resolution.modules[receiver_module.0]
                            .methods
                            .get(&MethodKey {
                                receiver,
                                name: member.text.clone(),
                            })
                            .copied()
                            .filter(|symbol| !self.resolution.symbols[symbol.0].is_static)
                    } else if let Type::Interface(interface) = self.types[owner.0] {
                        let requirement = self
                            .interface_shapes
                            .get(&interface)
                            .and_then(|shape| shape.methods.get(&member.text))
                            .cloned();
                        if let Some(requirement) = requirement {
                            let signature = Signature {
                                parameters: requirement
                                    .parameters
                                    .iter()
                                    .enumerate()
                                    .map(|(index, ty)| (format!("argument{index}"), *ty))
                                    .collect(),
                                result: requirement.result,
                            };
                            self.arguments(module, &signature, arguments, span, context, false);
                            return signature.result;
                        }
                        None
                    } else if let Type::Param(_) = self.types[owner.0] {
                        return self
                            .resolve_bound_method(module, owner, member, arguments, span, context);
                    } else {
                        None
                    }
                }
            }
            _ => None,
        };
        if let Some(symbol) = target
            && matches!(
                self.resolution.symbols[symbol.0].kind,
                vut_resolver::SymbolKind::Function
                    | vut_resolver::SymbolKind::Method
                    | vut_resolver::SymbolKind::AnonymousFunction
                    | vut_resolver::SymbolKind::Data
                    | vut_resolver::SymbolKind::Enum
                    | vut_resolver::SymbolKind::TypeAlias
            )
        {
            self.call_targets.insert(span, symbol);
            if self.extern_symbols.contains(&symbol) && context.unsafe_depth == 0 {
                self.error(
                    codes::E8001,
                    "extern call requires unsafe",
                    span,
                    "call raw native functions inside `unsafe:`",
                );
            }
            if self.resolution.symbols[symbol.0].kind == vut_resolver::SymbolKind::Method
                && !self.resolution.symbols[symbol.0].public
                && self.resolution.symbols[symbol.0].module != module.id
            {
                self.error(
                    codes::E2006,
                    "private method",
                    span,
                    "method is private to its module",
                );
                return self.intern(Type::Error);
            }
            // Bare-name construction of a generic data type infers its type
            // arguments from the named field values.
            if self.generic_params.contains_key(&symbol)
                && self.symbol_kind(symbol) == vut_resolver::SymbolKind::Data
            {
                let parameters = self
                    .generic_params
                    .get(&symbol)
                    .cloned()
                    .unwrap_or_default();
                let template_fields = self.fields.get(&symbol).cloned().unwrap_or_default();
                let mut map = HashMap::new();
                for argument in arguments {
                    if let Some(name) = &argument.name
                        && let Some(field) = template_fields.get(&name.text)
                    {
                        let formal = field.ty;
                        let actual = self.expr(module, &argument.value, context, None);
                        self.unify(formal, actual, &mut map);
                    }
                }
                let inferred: Vec<TypeId> = parameters
                    .iter()
                    .map(|parameter| {
                        map.get(parameter)
                            .copied()
                            .unwrap_or_else(|| self.intern(Type::Error))
                    })
                    .collect();
                if inferred
                    .iter()
                    .any(|argument| self.type_contains_param(*argument))
                {
                    self.call_targets.insert(span, symbol);
                    return self.intern(Type::Applied(symbol, inferred));
                }
                let instance = self.instantiate_symbol(symbol, &inferred);
                self.generic_type_applications.push(GenericApplication {
                    span,
                    template: symbol,
                    arguments: inferred,
                });
                self.call_targets.insert(span, instance);
                if let Some(fields) = self.fields.get(&instance).cloned() {
                    return self.construct(module, instance, &fields, arguments, span, context);
                }
            }
            if let Some(fields) = self.fields.get(&symbol).cloned() {
                return self.construct(module, symbol, &fields, arguments, span, context);
            }
            if let Some(parameters) = self.generic_params.get(&symbol).cloned()
                && let Some(signature) = self.signatures.get(&symbol).cloned()
            {
                let awaited = context.await_span == Some(span);
                if awaited {
                    context.await_span = None;
                }
                let result = self.check_generic_call(
                    module,
                    symbol,
                    &parameters,
                    &signature,
                    arguments,
                    expected,
                    span,
                    context,
                );
                if self.async_symbols.contains(&symbol) {
                    self.async_calls.insert(span);
                    if !awaited {
                        self.async_misuse(context, span);
                    }
                }
                return result;
            }
            if let Some(signature) = self.signatures.get(&symbol).cloned() {
                let awaited = context.await_span == Some(span);
                if awaited {
                    context.await_span = None;
                }
                let borrows_resources = self.extern_symbols.contains(&symbol);
                if let Some(element) = self.variadic_functions.get(&symbol).copied() {
                    self.variadic_arguments(module, &signature, element, arguments, span, context);
                } else {
                    self.arguments(
                        module,
                        &signature,
                        arguments,
                        span,
                        context,
                        borrows_resources,
                    );
                }
                if self.async_externs.contains(&symbol) {
                    // Calling a native async extern starts the operation and
                    // yields a `future(T)`; the logical result is awaited later.
                    return self.intern(Type::Future(signature.result));
                }
                if self.async_symbols.contains(&symbol) {
                    self.async_calls.insert(span);
                    if !awaited {
                        self.async_misuse(context, span);
                    }
                }
                return signature.result;
            }
        }
        let callee_type = self.expr(module, callee, context, None);
        self.call_value(module, callee_type, arguments, span, context)
    }

    /// Reports an async call whose result is used without `await`.
    ///
    /// Inside a `return` value this is an invalid async return (`E6013`);
    /// elsewhere it is an awaitable value used as an ordinary value (`E6014`).
    pub(super) fn async_misuse(&mut self, context: &Context, span: Span) {
        if context.return_depth > 0 {
            self.error(
                codes::E6013,
                "invalid async return",
                span,
                "an async computation is not the declared result; add `await`",
            );
        } else {
            self.error(
                codes::E6014,
                "awaitable value must be awaited before use",
                span,
                "this is an async computation, not an ordinary value",
            );
        }
    }

    /// Resolves a static/associated call `T.name(...)` or `Type.name(...)`.
    fn try_static_call(
        &mut self,
        module: &Module,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let Expr::Member { object, member, .. } = callee else {
            return None;
        };
        let Expr::Name(name) = object.as_ref() else {
            return None;
        };
        let symbol = self
            .reference_symbols
            .get(&name.span)
            .copied()
            .or_else(|| module.symbols.get(&name.text).copied())
            .or_else(|| module.imports.get(&name.text).copied())?;
        match self.resolution.symbols[symbol.0].kind {
            vut_resolver::SymbolKind::TypeParameter => {
                self.resolve_static_bound_call(module, symbol, member, arguments, span, context)
            }
            vut_resolver::SymbolKind::Data | vut_resolver::SymbolKind::Enum => {
                self.resolve_concrete_static_call(module, symbol, member, arguments, span, context)
            }
            _ => None,
        }
    }

    /// `T.name(...)` where `T` is a type parameter bounded by an interface with a
    /// static requirement `name`. The result substitutes `Self` with `T`.
    fn resolve_static_bound_call(
        &mut self,
        module: &Module,
        parameter: SymbolId,
        member: &vut_ast::Name,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let bounds = self
            .generic_param_bounds
            .get(&parameter)
            .cloned()
            .unwrap_or_default();
        if bounds.is_empty() {
            return None;
        }
        let mut candidates: Vec<InterfaceMethod> = Vec::new();
        for interface in &bounds {
            if let Some(requirement) = self
                .interface_shapes
                .get(interface)
                .and_then(|shape| shape.methods.get(&member.text))
                .filter(|requirement| requirement.is_static)
            {
                candidates.push(requirement.clone());
            }
        }
        let Some(first) = candidates.first().cloned() else {
            self.error(
                codes::E1018,
                "method is not available on the generic constraint",
                member.span,
                &format!(
                    "no bound of this type parameter declares a static method `{}`",
                    member.text
                ),
            );
            return Some(self.intern(Type::Error));
        };
        if candidates
            .iter()
            .skip(1)
            .any(|other| other.parameters != first.parameters || other.result != first.result)
        {
            self.error(
                codes::E1019,
                "ambiguous generic bound method",
                member.span,
                &format!(
                    "multiple bounds declare `{}` with incompatible signatures",
                    member.text
                ),
            );
            return Some(self.intern(Type::Error));
        }
        let receiver = self.intern(Type::Param(parameter));
        let parameters: Vec<TypeId> = first
            .parameters
            .iter()
            .map(|ty| self.substitute_self(*ty, receiver))
            .collect();
        let result = self.substitute_self(first.result, receiver);
        let signature = Signature {
            parameters: parameters
                .into_iter()
                .enumerate()
                .map(|(index, ty)| (format!("argument{index}"), ty))
                .collect(),
            result,
        };
        self.arguments(module, &signature, arguments, span, context, false);
        self.bound_calls.insert(
            span,
            BoundCall {
                receiver,
                method: member.text.clone(),
                is_static: true,
            },
        );
        Some(result)
    }

    /// `Type.name(...)` where `Type` is a concrete data/enum with an associated
    /// `static fn`.
    fn resolve_concrete_static_call(
        &mut self,
        module: &Module,
        receiver: SymbolId,
        member: &vut_ast::Name,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let receiver_module = self.symbol_module(receiver);
        let method = self.resolution.modules[receiver_module.0]
            .methods
            .get(&MethodKey {
                receiver,
                name: member.text.clone(),
            })
            .copied()?;
        if !self.resolution.symbols[method.0].is_static {
            return None;
        }
        let signature = self.signatures.get(&method).cloned()?;
        self.arguments(module, &signature, arguments, span, context, false);
        self.call_targets.insert(span, method);
        Some(signature.result)
    }

    /// Resolves a bound instance-method call on a generic type parameter:
    /// `value.method()` where `value: T` and `T: Iface`. The call is checked
    /// against the bound's requirement and recorded for monomorphization, which
    /// binds it to the concrete type's method of the same name. No runtime
    /// dispatch is introduced.
    fn resolve_bound_method(
        &mut self,
        module: &Module,
        receiver: TypeId,
        member: &vut_ast::Name,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> TypeId {
        let Type::Param(parameter) = self.types[receiver.0] else {
            return self.intern(Type::Error);
        };
        let bounds = self
            .generic_param_bounds
            .get(&parameter)
            .cloned()
            .unwrap_or_default();
        let mut candidates: Vec<InterfaceMethod> = Vec::new();
        for interface in &bounds {
            if let Some(requirement) = self
                .interface_shapes
                .get(interface)
                .and_then(|shape| shape.methods.get(&member.text))
                .filter(|requirement| !requirement.is_static)
            {
                candidates.push(requirement.clone());
            }
        }
        let Some(first) = candidates.first().cloned() else {
            let names: Vec<String> = bounds
                .iter()
                .map(|interface| self.resolution.symbols[interface.0].name.clone())
                .collect();
            let suffix = if names.is_empty() {
                String::new()
            } else {
                format!(" (bounds: {})", names.join(", "))
            };
            self.error(
                codes::E1018,
                "method is not available on the generic constraint",
                member.span,
                &format!(
                    "no bound of this type parameter declares `{}`{suffix}",
                    member.text
                ),
            );
            return self.intern(Type::Error);
        };
        if candidates
            .iter()
            .skip(1)
            .any(|other| other.parameters != first.parameters || other.result != first.result)
        {
            self.error(
                codes::E1019,
                "ambiguous generic bound method",
                member.span,
                &format!(
                    "multiple bounds declare `{}` with incompatible signatures",
                    member.text
                ),
            );
            return self.intern(Type::Error);
        }
        let signature = Signature {
            parameters: first
                .parameters
                .iter()
                .enumerate()
                .map(|(index, ty)| (format!("argument{index}"), *ty))
                .collect(),
            result: first.result,
        };
        self.arguments(module, &signature, arguments, span, context, false);
        self.bound_calls.insert(
            span,
            BoundCall {
                receiver,
                method: member.text.clone(),
                is_static: false,
            },
        );
        signature.result
    }

    /// Checks the arguments of a call to a variadic function: fixed parameters
    /// first, then zero or more arguments of the variadic element type, or a
    /// single trailing spread of a matching sequence.
    fn variadic_arguments(
        &mut self,
        module: &Module,
        signature: &Signature,
        element: TypeId,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) {
        let fixed = signature.parameters.len();
        let mut positional = 0usize;
        let mut seen_spread = false;
        for (index, argument) in arguments.iter().enumerate() {
            if seen_spread {
                self.error(
                    codes::E7013,
                    "argument after spread",
                    argument.span,
                    "a spread argument must be the last argument",
                );
            }
            if argument.spread {
                if index + 1 != arguments.len() {
                    self.error(
                        codes::E7013,
                        "argument after spread",
                        argument.span,
                        "a spread argument must be the last argument",
                    );
                }
                seen_spread = true;
                let actual = self.expr(module, &argument.value, context, None);
                if !self.is_variadic_spread(actual, element) {
                    self.error(
                        codes::E1003,
                        "spread argument type mismatch",
                        argument.value.span(),
                        "the spread sequence element type must match the variadic parameter",
                    );
                }
                continue;
            }
            let expected = if let Some(name) = &argument.name {
                let Some(index) = signature
                    .parameters
                    .iter()
                    .position(|(n, _)| n == &name.text)
                else {
                    self.error(
                        codes::E6003,
                        "invalid argument",
                        name.span,
                        "unknown argument name",
                    );
                    continue;
                };
                signature.parameters[index].1
            } else if positional < fixed {
                signature.parameters[positional].1
            } else {
                element
            };
            let actual = self.expr(module, &argument.value, context, Some(expected));
            self.compatible(
                actual,
                expected,
                argument.value.span(),
                codes::E1003,
                "argument type mismatch",
            );
            if argument.name.is_none() {
                positional += 1;
            }
        }
        if positional < fixed && arguments.iter().all(|argument| argument.name.is_none()) {
            self.error(
                codes::E6002,
                "invalid argument count",
                span,
                "not enough arguments for the fixed parameters",
            );
        }
    }

    /// A spread argument must be a `list(T)`, `array(T, N)`, or another variadic
    /// view with the same element type.
    fn is_variadic_spread(&self, actual: TypeId, element: TypeId) -> bool {
        match self.types[actual.0] {
            Type::List(inner) | Type::Variadic(inner) | Type::Array(inner, _) => inner == element,
            _ => false,
        }
    }

    pub(super) fn call_value(
        &mut self,
        module: &Module,
        callee_type: TypeId,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> TypeId {
        let callable = match self.types[callee_type.0].clone() {
            Type::Function(symbol) => self.signatures.get(&symbol).cloned().map(|signature| {
                (
                    signature
                        .parameters
                        .into_iter()
                        .map(|(_, ty)| ty)
                        .collect::<Vec<_>>(),
                    signature.result,
                )
            }),
            Type::Callable {
                parameters, result, ..
            }
            | Type::FunctionPointer {
                parameters, result, ..
            } => Some((parameters, result)),
            _ => None,
        };
        let Some((parameters, result)) = callable else {
            if matches!(self.types[callee_type.0], Type::Error) {
                self.error(
                    codes::E2005,
                    "unknown callable",
                    span,
                    "function, method, or constructor was not found",
                );
            } else {
                self.error(
                    codes::E1014,
                    "value is not callable",
                    span,
                    "only named functions, methods, and lambdas can be called",
                );
            }
            return self.intern(Type::Error);
        };
        let signature = Signature {
            parameters: parameters
                .into_iter()
                .enumerate()
                .map(|(index, ty)| (format!("argument{index}"), ty))
                .collect(),
            result,
        };
        self.arguments(module, &signature, arguments, span, context, false);
        result
    }

    /// Type-checks a `match` expression: patterns, bindings, guards, arm
    /// result unification, exhaustiveness, and arm reachability.
    pub(super) fn check_match(
        &mut self,
        module: &Module,
        value: &Expr,
        arms: &[vut_ast::MatchArm],
        span: Span,
        context: &mut Context,
        expected: Option<TypeId>,
    ) -> TypeId {
        let subject = self.expr(module, value, context, None);
        let mut result = None;
        let mut coverage = Coverage::default();
        let mut seen_heads = HashSet::new();
        let mut wildcard_seen = false;

        for arm in arms {
            let arm_subject = self.arm_pattern_subject(subject, &arm.pattern);
            context.scopes.push(HashMap::new());
            let mut bindings = HashMap::new();
            self.check_pattern(module, &arm.pattern, arm_subject, &mut bindings, context);
            if let Some(guard) = &arm.guard {
                let guard_ty = self.expr(module, guard, context, None);
                let bool_ty = self.intern(Type::Bool);
                self.compatible(
                    guard_ty,
                    bool_ty,
                    guard.span(),
                    codes::E7110,
                    "match guard must be `bool`",
                );
            }
            let arm_ty = self.expr(module, &arm.value, context, expected);
            context.scopes.pop();

            if arm.guard.is_none() {
                if wildcard_seen {
                    self.warn(
                        codes::W5004,
                        "unreachable match arm",
                        arm.pattern.span(),
                        "a previous arm already matches every value",
                    );
                } else if let Some(head) = pattern_head(
                    &|name| self.bare_variant(arm_subject, name).is_some(),
                    &arm.pattern,
                ) {
                    if !seen_heads.insert(head.clone()) {
                        self.warn(
                            codes::W5004,
                            "unreachable match arm",
                            arm.pattern.span(),
                            "an earlier arm already covers this pattern",
                        );
                    }
                    if head == Head::Wild {
                        wildcard_seen = true;
                    }
                }
                coverage.merge(&self.coverage_of(&arm.pattern, arm_subject));
            }

            if let Some(first) = result {
                self.compatible(
                    arm_ty,
                    first,
                    arm.span,
                    codes::E1010,
                    "incompatible match arm types",
                );
            } else {
                result = Some(arm_ty);
            }
        }

        self.check_exhaustive(subject, &coverage, span);
        result.unwrap_or_else(|| self.intern(Type::Error))
    }

    /// Checks a single pattern against the scrutinee type.
    #[expect(
        clippy::too_many_lines,
        reason = "pattern checking keeps every pattern kind together"
    )]
    fn check_pattern(
        &mut self,
        module: &Module,
        pattern: &MatchPattern,
        subject: TypeId,
        bindings: &mut HashMap<String, TypeId>,
        context: &mut Context,
    ) {
        match pattern {
            MatchPattern::Wildcard(_) | MatchPattern::Error(_) => {}
            MatchPattern::Binding(name) => {
                if let Some(index) = self.bare_variant(subject, &name.text) {
                    let has_fields = self
                        .enum_variants_for(subject)
                        .and_then(|variants| variants.get(index))
                        .is_some_and(|variant| !variant.fields.is_empty());
                    if has_fields {
                        self.error(
                            codes::E7106,
                            "variant payload mismatch",
                            name.span,
                            "this variant carries a payload; bind it with `name(...)`",
                        );
                    }
                    return;
                }
                if bindings.contains_key(&name.text) {
                    self.error(
                        codes::E7108,
                        "duplicate pattern binding",
                        name.span,
                        "this name is bound more than once in the same pattern",
                    );
                } else {
                    bindings.insert(name.text.clone(), subject);
                    context
                        .scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.text.clone(), subject);
                }
            }
            MatchPattern::Literal { value, span } => {
                self.check_literal_pattern(value, subject, *span);
            }
            MatchPattern::Range {
                start,
                end,
                inclusive: _,
                span,
            } => {
                if !self.is_integer(subject) {
                    self.error(
                        codes::E7111,
                        "invalid range pattern",
                        *span,
                        "range patterns require an integer scrutinee",
                    );
                }
                let _ = (start, end);
            }
            MatchPattern::Variant { name, fields, span } => {
                self.check_variant_pattern(module, name, fields, subject, *span, bindings, context);
            }
            MatchPattern::ResultOk { pattern, span } => {
                if let Type::Result(ok_ty, _) = self.types[subject.0] {
                    self.check_pattern(module, pattern, ok_ty, bindings, context);
                } else {
                    self.error(
                        codes::E7107,
                        "pattern type mismatch",
                        *span,
                        "`ok(...)` can only match a `result(T, E)` value",
                    );
                    let error_ty = self.intern(Type::Error);
                    self.check_pattern(module, pattern, error_ty, bindings, context);
                }
            }
            MatchPattern::ResultErr { pattern, span } => {
                if let Type::Result(_, err_ty) = self.types[subject.0] {
                    self.check_pattern(module, pattern, err_ty, bindings, context);
                } else {
                    self.error(
                        codes::E7107,
                        "pattern type mismatch",
                        *span,
                        "`err(...)` can only match a `result(T, E)` value",
                    );
                    let error_ty = self.intern(Type::Error);
                    self.check_pattern(module, pattern, error_ty, bindings, context);
                }
            }
            MatchPattern::Or { alternatives, span } => {
                let Some(first) = alternatives.first() else {
                    return;
                };
                let mut first_bindings = HashMap::new();
                self.check_pattern(module, first, subject, &mut first_bindings, context);
                for alternative in &alternatives[1..] {
                    let mut other = HashMap::new();
                    self.check_pattern(module, alternative, subject, &mut other, context);
                    if other.keys().collect::<HashSet<_>>()
                        != first_bindings.keys().collect::<HashSet<_>>()
                    {
                        self.error(
                            codes::E7109,
                            "or-pattern binding mismatch",
                            *span,
                            "every alternative must bind the same names",
                        );
                        continue;
                    }
                    for (name, ty) in &other {
                        if let Some(first_ty) = first_bindings.get(name) {
                            self.compatible(
                                *ty,
                                *first_ty,
                                alternative.span(),
                                codes::E7109,
                                "or-pattern binding types differ",
                            );
                        }
                    }
                }
                for (name, ty) in first_bindings {
                    context.scopes.last_mut().unwrap().insert(name.clone(), ty);
                    bindings.insert(name, ty);
                }
            }
            MatchPattern::List { items, span, .. } => {
                self.error(
                    codes::E7112,
                    "list patterns are not yet supported",
                    *span,
                    "match lists with a `_` fallback or an `if` guard for now",
                );
                for item in items {
                    self.check_pattern(module, item, subject, bindings, context);
                }
            }
            MatchPattern::Group { pattern, .. } => {
                self.check_pattern(module, pattern, subject, bindings, context);
            }
        }
    }

    fn check_literal_pattern(&mut self, value: &LiteralPattern, subject: TypeId, span: Span) {
        if matches!(value, LiteralPattern::Null) {
            if matches!(self.types[subject.0], Type::Optional(_)) {
                return;
            }
            self.error(
                codes::E7107,
                "pattern type mismatch",
                span,
                "`null` patterns require an optional scrutinee",
            );
            return;
        }
        let ok = match value {
            LiteralPattern::Integer(_) => self.is_integer(subject),
            LiteralPattern::Float(_) => self.is_numeric(subject),
            LiteralPattern::Bool(_) => matches!(self.types[subject.0], Type::Bool),
            LiteralPattern::Str(_) => matches!(self.types[subject.0], Type::Str),
            LiteralPattern::Null => false,
        };
        if !ok {
            let expected = match value {
                LiteralPattern::Integer(_) => "integer",
                LiteralPattern::Float(_) => "numeric",
                LiteralPattern::Bool(_) => "bool",
                LiteralPattern::Str(_) => "str",
                LiteralPattern::Null => "optional",
            };
            self.error(
                codes::E7107,
                "pattern type mismatch",
                span,
                &format!("literal pattern requires a {expected} scrutinee"),
            );
        }
    }

    /// Recognizes `name == null` / `name != null` where `name` is an optional
    /// place. Returns the name, the present (`T`) type, and whether the
    /// operator is `==` (`true`) or `!=` (`false`).
    pub(super) fn optional_condition_narrowing(
        &self,
        expr: &Expr,
        context: &Context,
    ) -> Option<(String, TypeId, bool)> {
        let Expr::Binary {
            left, op, right, ..
        } = expr
        else {
            return None;
        };
        let is_eq = match op {
            BinaryOp::Equal => true,
            BinaryOp::NotEqual => false,
            _ => return None,
        };
        let name_expr = match (&**left, &**right) {
            (Expr::Name(_), Expr::Null(_)) => &**left,
            (Expr::Null(_), Expr::Name(_)) => &**right,
            _ => return None,
        };
        let Expr::Name(name) = name_expr else {
            return None;
        };
        let ty = context.lookup(&name.text)?;
        let Type::Optional(inner) = self.types[ty.0] else {
            return None;
        };
        Some((name.text.clone(), inner, is_eq))
    }

    /// The type a match arm's pattern is checked against. A present-requiring
    /// pattern on an optional sees the inner type (`T?` -> `T`); `null` and
    /// wildcard patterns keep the optional so absence still matches them.
    fn arm_pattern_subject(&self, subject: TypeId, pattern: &MatchPattern) -> TypeId {
        match self.optional_value_inner(subject) {
            Some(inner) if pattern_requires_presence(pattern) => inner,
            _ => subject,
        }
    }

    fn optional_value_inner(&self, ty: TypeId) -> Option<TypeId> {
        match self.types[ty.0] {
            Type::Optional(inner) => Some(inner),
            _ => None,
        }
    }

    /// Checks a block with an optional narrowing binding in scope.
    pub(super) fn block_narrowed(
        &mut self,
        module: &Module,
        block: &Block,
        context: &mut Context,
        binding: Option<(bool, &String, &TypeId)>,
    ) -> TypeId {
        match binding {
            Some((true, name, inner)) => {
                context.scopes.push(HashMap::from([(name.clone(), *inner)]));
                let ty = self.block(module, block, context);
                context.scopes.pop();
                ty
            }
            _ => self.block(module, block, context),
        }
    }

    /// True when the block provably leaves the enclosing flow on every path.
    pub(super) fn block_terminates(block: &Block) -> bool {
        matches!(
            block.statements.last(),
            Some(Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_))
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "variant patterns carry their fields and binding scope"
    )]
    fn check_variant_pattern(
        &mut self,
        module: &Module,
        name: &vut_ast::Name,
        fields: &[vut_ast::FieldPattern],
        subject: TypeId,
        span: Span,
        bindings: &mut HashMap<String, TypeId>,
        context: &mut Context,
    ) {
        let Type::Enum(symbol) = self.types[subject.0] else {
            self.error(
                codes::E7107,
                "pattern type mismatch",
                span,
                "variant patterns require an enum scrutinee",
            );
            return;
        };
        let Some(variant_index) = self.variant_index(symbol, &name.text) else {
            self.error(
                codes::E7101,
                "unknown enum variant",
                name.span,
                &format!(
                    "`{}` has no variant `{}`",
                    self.enum_symbol_name(symbol),
                    name.text
                ),
            );
            return;
        };
        let info = self.enum_variants(symbol).unwrap()[variant_index].clone();

        if info.fields.is_empty() {
            if !fields.is_empty() {
                self.error(
                    codes::E7106,
                    "variant payload mismatch",
                    span,
                    &format!("variant `{}` carries no payload", name.text),
                );
            }
            return;
        }

        let all_positional = fields
            .iter()
            .all(|field| matches!(field, vut_ast::FieldPattern::Positional(_)));
        if all_positional {
            if info.fields.len() != 1 || fields.len() != 1 {
                self.error(
                    codes::E7106,
                    "variant payload mismatch",
                    span,
                    "positional shorthand is only allowed for single-field variants",
                );
            }
            if let Some(field) = fields.first() {
                self.check_pattern(
                    module,
                    field.pattern(),
                    info.fields[0].ty,
                    bindings,
                    context,
                );
            }
            return;
        }

        let mut seen = HashSet::new();
        for field in fields {
            let vut_ast::FieldPattern::Named {
                field: field_name,
                pattern,
            } = field
            else {
                continue;
            };
            if !seen.insert(field_name.text.clone()) {
                self.error(
                    codes::E7106,
                    "duplicate variant field pattern",
                    field_name.span,
                    "field appears more than once",
                );
                continue;
            }
            let Some(info_field) = info.fields.iter().find(|item| item.name == field_name.text)
            else {
                self.error(
                    codes::E7106,
                    "unknown variant field",
                    field_name.span,
                    &format!("variant `{}` has no field `{}`", name.text, field_name.text),
                );
                continue;
            };
            self.check_pattern(module, pattern, info_field.ty, bindings, context);
        }
        for info_field in &info.fields {
            if !seen.contains(&info_field.name) {
                self.error(
                    codes::E7106,
                    "missing variant field pattern",
                    span,
                    &format!(
                        "field `{}` is not bound; use `_` to ignore it",
                        info_field.name
                    ),
                );
            }
        }
    }

    fn coverage_of(&self, pattern: &MatchPattern, subject: TypeId) -> Coverage {
        match pattern {
            MatchPattern::Wildcard(_) => Coverage {
                all: true,
                ..Coverage::default()
            },
            MatchPattern::Binding(name) => {
                if let Some(index) = self.bare_variant(subject, &name.text) {
                    Coverage {
                        variants: HashSet::from([index]),
                        ..Coverage::default()
                    }
                } else {
                    Coverage {
                        all: true,
                        ..Coverage::default()
                    }
                }
            }
            MatchPattern::Group { pattern, .. } => self.coverage_of(pattern, subject),
            MatchPattern::Or { alternatives, .. } => {
                let mut coverage = Coverage::default();
                for alternative in alternatives {
                    coverage.merge(&self.coverage_of(alternative, subject));
                }
                coverage
            }
            MatchPattern::Variant { name, .. } => {
                let mut coverage = Coverage::default();
                if let Type::Enum(symbol) = self.types[subject.0]
                    && let Some(index) = self.variant_index(symbol, &name.text)
                {
                    coverage.variants.insert(index);
                }
                coverage
            }
            MatchPattern::ResultOk { .. } => Coverage {
                variants: HashSet::from([0usize]),
                ..Coverage::default()
            },
            MatchPattern::ResultErr { .. } => Coverage {
                variants: HashSet::from([1usize]),
                ..Coverage::default()
            },
            MatchPattern::Literal { value, .. } => match value {
                LiteralPattern::Bool(value) => Coverage {
                    bools: HashSet::from([*value]),
                    ..Coverage::default()
                },
                LiteralPattern::Null => Coverage {
                    nulls: true,
                    ..Coverage::default()
                },
                _ => Coverage {
                    partial: true,
                    ..Coverage::default()
                },
            },
            MatchPattern::List { items, rest, .. } => {
                let mut coverage = Coverage::default();
                if rest.is_some() {
                    coverage.list_rest_min = Some(items.len());
                } else {
                    coverage.list_exact.insert(items.len());
                }
                coverage
            }
            MatchPattern::Range { .. } => Coverage {
                partial: true,
                ..Coverage::default()
            },
            MatchPattern::Error(_) => Coverage::default(),
        }
    }

    fn check_exhaustive(&mut self, subject: TypeId, coverage: &Coverage, span: Span) {
        if coverage.all {
            return;
        }
        let missing: Vec<String> = match &self.types[subject.0] {
            Type::Enum(symbol) => self
                .enum_variants
                .get(symbol)
                .map(|variants| {
                    variants
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| !coverage.variants.contains(index))
                        .map(|(_, variant)| variant.name.clone())
                        .collect()
                })
                .unwrap_or_default(),
            Type::Result(_, _) => {
                let mut items = Vec::new();
                if !coverage.variants.contains(&0) {
                    items.push("ok".to_owned());
                }
                if !coverage.variants.contains(&1) {
                    items.push("err".to_owned());
                }
                items
            }
            Type::Bool => {
                let mut items = Vec::new();
                if !coverage.bools.contains(&true) {
                    items.push("true".to_owned());
                }
                if !coverage.bools.contains(&false) {
                    items.push("false".to_owned());
                }
                items
            }
            Type::Optional(_) => {
                if coverage.nulls {
                    Vec::new()
                } else {
                    vec!["null".to_owned()]
                }
            }
            Type::List(_) => {
                let exhaustive = coverage.list_rest_min.is_some_and(|min| {
                    (0..min).all(|length| coverage.list_exact.contains(&length))
                });
                if exhaustive {
                    Vec::new()
                } else {
                    vec!["_".to_owned()]
                }
            }
            Type::Array(_, length) => {
                if coverage.list_exact.contains(length) {
                    Vec::new()
                } else {
                    vec!["_".to_owned()]
                }
            }
            _ => vec!["_".to_owned()],
        };
        if !missing.is_empty() {
            self.error(
                codes::E5009,
                "non-exhaustive match",
                span,
                &format!("missing patterns: {}", missing.join(", ")),
            );
        }
    }

    fn enum_symbol_name(&self, symbol: SymbolId) -> String {
        self.symbol_name(symbol)
    }

    /// Type-checks `Enum.variant(field = value, ...)` and records the
    /// construction for MIR lowering.
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "variant construction carries its enum, variant, and call span"
    )]
    fn construct_variant(
        &mut self,
        module: &Module,
        enum_symbol: SymbolId,
        variant_index: usize,
        info: &VariantInfo,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> TypeId {
        let enum_ty = self.symbol_type(enum_symbol);
        if info.fields.is_empty() {
            if !arguments.is_empty() {
                self.error(
                    codes::E7106,
                    "variant payload mismatch",
                    span,
                    &format!("variant `{}` carries no payload", info.name),
                );
            }
            self.variant_constructions.insert(
                span,
                VariantConstruction {
                    enum_symbol,
                    variant_index,
                    fields: Vec::new(),
                },
            );
            return enum_ty;
        }

        let mut field_indices = Vec::new();
        let positional = arguments.iter().all(|argument| argument.name.is_none());
        if positional {
            if arguments.len() != info.fields.len() {
                self.error(
                    codes::E7106,
                    "variant payload mismatch",
                    span,
                    &format!(
                        "variant `{}` expects {} field(s), found {}",
                        info.name,
                        info.fields.len(),
                        arguments.len()
                    ),
                );
            }
            for (index, argument) in arguments.iter().enumerate() {
                let Some(field) = info.fields.get(index) else {
                    continue;
                };
                let actual = self.expr(module, &argument.value, context, Some(field.ty));
                self.compatible(
                    actual,
                    field.ty,
                    argument.value.span(),
                    codes::E1003,
                    "variant field type mismatch",
                );
                field_indices.push(index);
            }
        } else {
            let mut seen = HashSet::new();
            for argument in arguments {
                let Some(name) = &argument.name else {
                    self.error(
                        codes::E7106,
                        "variant payload mismatch",
                        argument.span,
                        "positional and named arguments cannot be mixed",
                    );
                    continue;
                };
                let Some(index) = info.fields.iter().position(|field| field.name == name.text)
                else {
                    self.error(
                        codes::E7106,
                        "unknown variant field",
                        name.span,
                        &format!("variant `{}` has no field `{}`", info.name, name.text),
                    );
                    continue;
                };
                if !seen.insert(index) {
                    self.error(
                        codes::E7106,
                        "duplicate variant field",
                        name.span,
                        "field supplied more than once",
                    );
                    continue;
                }
                let field = &info.fields[index];
                let actual = self.expr(module, &argument.value, context, Some(field.ty));
                self.compatible(
                    actual,
                    field.ty,
                    argument.value.span(),
                    codes::E1003,
                    "variant field type mismatch",
                );
                field_indices.push(index);
            }
            for (index, field) in info.fields.iter().enumerate() {
                if !seen.contains(&index) {
                    self.error(
                        codes::E7106,
                        "missing variant field",
                        span,
                        &format!("field `{}` is not provided", field.name),
                    );
                }
            }
        }

        self.variant_constructions.insert(
            span,
            VariantConstruction {
                enum_symbol,
                variant_index,
                fields: field_indices,
            },
        );
        enum_ty
    }
}

/// Coverage summary used for exhaustiveness and reachability.
#[derive(Default)]
struct Coverage {
    all: bool,
    variants: HashSet<usize>,
    bools: HashSet<bool>,
    nulls: bool,
    partial: bool,
    /// Exact list lengths covered by patterns without a rest element.
    list_exact: HashSet<usize>,
    /// Smallest prefix length covered by patterns with a rest element.
    list_rest_min: Option<usize>,
}
impl Coverage {
    fn merge(&mut self, other: &Self) {
        self.all |= other.all;
        self.variants.extend(other.variants.iter().copied());
        self.bools.extend(other.bools.iter().copied());
        self.nulls |= other.nulls;
        self.partial |= other.partial;
        self.list_exact.extend(other.list_exact.iter().copied());
        self.list_rest_min = match (self.list_rest_min, other.list_rest_min) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        };
    }
}

/// A normalized top-level pattern head used for simple reachability checks.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Head {
    Wild,
    Variant(String),
    Ok,
    Err,
    Bool(bool),
    Null,
    Other(String),
}
fn pattern_head(is_variant: &impl Fn(&str) -> bool, pattern: &MatchPattern) -> Option<Head> {
    match pattern {
        MatchPattern::Wildcard(_) => Some(Head::Wild),
        MatchPattern::Binding(name) => {
            if is_variant(&name.text) {
                Some(Head::Variant(name.text.clone()))
            } else {
                Some(Head::Wild)
            }
        }
        MatchPattern::Group { pattern, .. } => pattern_head(is_variant, pattern),
        MatchPattern::Variant { name, fields, .. } if fields.is_empty() => {
            Some(Head::Variant(name.text.clone()))
        }
        MatchPattern::ResultOk { .. } => Some(Head::Ok),
        MatchPattern::ResultErr { .. } => Some(Head::Err),
        MatchPattern::Literal {
            value: LiteralPattern::Bool(value),
            ..
        } => Some(Head::Bool(*value)),
        MatchPattern::Literal {
            value: LiteralPattern::Null,
            ..
        } => Some(Head::Null),
        MatchPattern::Literal {
            value: LiteralPattern::Integer(text),
            ..
        }
        | MatchPattern::Literal {
            value: LiteralPattern::Float(text),
            ..
        }
        | MatchPattern::Literal {
            value: LiteralPattern::Str(text),
            ..
        } => Some(Head::Other(text.clone())),
        _ => None,
    }
}

/// Returns true when a pattern only matches a present value, so a match on an
/// optional should narrow the subject to its inner type for that arm. `null`
/// and wildcard patterns also match absence, so they keep the optional.
fn pattern_requires_presence(pattern: &MatchPattern) -> bool {
    match pattern {
        MatchPattern::Wildcard(_)
        | MatchPattern::Error(_)
        | MatchPattern::Literal {
            value: LiteralPattern::Null,
            ..
        } => false,
        MatchPattern::Group { pattern, .. } => pattern_requires_presence(pattern),
        MatchPattern::Or { alternatives, .. } => {
            !alternatives.is_empty() && alternatives.iter().all(pattern_requires_presence)
        }
        _ => true,
    }
}

/// Returns the span of the effective awaitable expression, seeing through
/// parentheses so `await (call())` matches the recorded async call span.
fn await_operand_span(expr: &Expr) -> Span {
    match expr {
        Expr::Group { value, .. } => await_operand_span(value),
        _ => expr.span(),
    }
}
