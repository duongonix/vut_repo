//! Module, function, and statement checking.
use super::constants::is_constant_name;
use super::context::Context;
#[allow(clippy::wildcard_imports)]
use super::*;

impl Analyzer<'_> {
    pub(super) fn module(&mut self, module: &Module) {
        let mut root = HashMap::new();
        for (name, symbol) in &module.symbols {
            root.insert(name.clone(), self.symbol_type(*symbol));
        }
        // Module-level bindings are declarations. The resolver pre-declares their
        // names with an unknown type, so infer the real type first (so functions
        // and later items see it) and skip the declaration in the item pass.
        self.module_constants.clear();
        for item in &module.file.items {
            if let Item::Statement(Stmt::Binding {
                target: Expr::Name(name),
                annotation,
                value,
                ..
            }) = item
            {
                self.module_constants
                    .insert(name.text.clone(), value.clone());
                self.module_binding(module, name, annotation.as_ref(), value, &mut root);
            }
        }
        for item in &module.file.items {
            match item {
                Item::Function(value) => self.function(
                    module,
                    module.symbols.get(&value.name.text).copied(),
                    None,
                    value.is_async,
                    &value.parameters,
                    value.return_type.as_ref(),
                    &value.body,
                    root.clone(),
                ),
                Item::Method(value) => {
                    let method = module
                        .methods
                        .iter()
                        .find(|(_, id)| self.resolution.symbols[id.0].span == value.name.span);
                    let symbol = method.map(|(_, id)| *id);
                    let receiver = if value.is_static {
                        None
                    } else {
                        method.map(|(key, _)| self.symbol_type(key.receiver))
                    };
                    self.function(
                        module,
                        symbol,
                        receiver,
                        value.is_async,
                        &value.parameters,
                        value.return_type.as_ref(),
                        &value.body,
                        root.clone(),
                    );
                }
                // Module-level name bindings were declared in the pass above.
                Item::Statement(Stmt::Binding {
                    target: Expr::Name(_),
                    ..
                }) => {}
                Item::Statement(stmt) => {
                    let mut context = Context::new(root.clone());
                    self.statement(module, stmt, &mut context, None);
                }
                Item::Data(value) => {
                    let mut context = Context::new(root.clone());
                    if let Some(symbol) = module.symbols.get(&value.name.text)
                        && let Some(fields) = self.fields.get(symbol).cloned()
                    {
                        for field in &value.fields {
                            if let Some(default) = &field.default
                                && let Some(expected) =
                                    fields.get(&field.name.text).map(|item| item.ty)
                            {
                                let actual =
                                    self.expr(module, default, &mut context, Some(expected));
                                self.compatible(
                                    actual,
                                    expected,
                                    default.span(),
                                    codes::E1003,
                                    "field default type mismatch",
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        self.module_constants.clear();
    }
    /// Checks a module-level `name = value` declaration and records its type so
    /// later items resolve the name to the real type instead of the resolver's
    /// unknown placeholder.
    fn module_binding(
        &mut self,
        module: &Module,
        name: &vut_ast::Name,
        annotation: Option<&TypeExpr>,
        value: &Expr,
        root: &mut HashMap<String, TypeId>,
    ) {
        let expected = annotation.map(|ty| self.resolve_type(module, ty));
        let mut context = Context::new(root.clone());
        if let Some(scope) = context.scopes.last_mut() {
            scope.remove(&name.text);
        }
        let actual = self.expr(module, value, &mut context, expected);
        if let Some(expected) = expected {
            self.compatible(
                actual,
                expected,
                value.span(),
                codes::E1003,
                "assignment type mismatch",
            );
        }
        let ty = expected.unwrap_or(actual);
        self.expression_types.insert(name.span, ty);
        root.insert(name.text.clone(), ty);
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "function checking requires its complete declaration context"
    )]
    pub(super) fn function(
        &mut self,
        module: &Module,
        symbol: Option<SymbolId>,
        receiver: Option<TypeId>,
        is_async: bool,
        parameters: &[vut_ast::Parameter],
        return_type: Option<&TypeExpr>,
        body: &Block,
        root: HashMap<String, TypeId>,
    ) {
        // Defaults are constant expressions evaluated at the call site, so they
        // are checked against the module scope only: a default cannot reference
        // other parameters or locals.
        let mut defaults_context = Context::new(root.clone());
        for parameter in parameters {
            let Some(default) = &parameter.default else {
                continue;
            };
            let expected = self.resolve_type(module, &parameter.ty);
            let actual = self.expr(module, default, &mut defaults_context, Some(expected));
            self.compatible(
                actual,
                expected,
                default.span(),
                codes::E1003,
                "parameter default type mismatch",
            );
        }
        let mut context = Context::new(root);
        context.function = true;
        context.in_async = is_async;
        context.current_function = symbol;
        if let Some(receiver) = receiver {
            context
                .scopes
                .last_mut()
                .unwrap()
                .insert("self".into(), receiver);
        }
        for parameter in parameters {
            let ty = self.resolve_type(module, &parameter.ty);
            let binding = if parameter.variadic {
                self.intern(Type::Variadic(ty))
            } else {
                ty
            };
            context
                .scopes
                .last_mut()
                .unwrap()
                .insert(parameter.name.text.clone(), binding);
        }
        let expected = match return_type {
            Some(ty) => self.resolve_type(module, ty),
            None => self.intern(Type::Void),
        };
        context.return_type = Some(expected);
        let actual = self.function_block(module, body, &mut context, expected);
        self.compatible(
            actual,
            expected,
            body.span,
            codes::E6005,
            "function return type mismatch",
        );
    }
    /// Splits an expected function type into its receiver, parameters, and
    /// result. Ordinary function pointers and unconstrained contexts have no
    /// receiver.
    pub(super) fn expected_callable(
        &mut self,
        expected: Option<TypeId>,
    ) -> (Option<TypeId>, Option<Vec<TypeId>>, Option<TypeId>) {
        let Some(expected) = expected else {
            return (None, None, None);
        };
        match self.types[expected.0].clone() {
            Type::Callable {
                receiver,
                parameters,
                result,
            } => (receiver, Some(parameters), Some(result)),
            Type::FunctionPointer {
                parameters, result, ..
            } => (None, Some(parameters), Some(result)),
            _ => (None, None, None),
        }
    }

    /// Binds a lambda's parameters into the current scope, resolving explicit
    /// annotations and falling back to the expected parameter types.
    fn bind_lambda_parameters(
        &mut self,
        module: &Module,
        parameters: &[LambdaParameter],
        expected_parameters: Option<&[TypeId]>,
        context: &mut Context,
    ) -> Vec<(String, TypeId)> {
        let mut parameter_types = Vec::new();
        for (index, parameter) in parameters.iter().enumerate() {
            let ty = if let Some(annotation) = &parameter.ty {
                self.resolve_type(module, annotation)
            } else if let Some(expected) = expected_parameters
                .and_then(|items| items.get(index))
                .copied()
            {
                expected
            } else {
                self.error(
                    codes::E1012,
                    "cannot infer lambda parameter type",
                    parameter.span,
                    "annotate the parameter or pass the lambda where a function type is expected",
                );
                self.intern(Type::Error)
            };
            parameter_types.push((parameter.name.text.clone(), ty));
            context
                .scopes
                .last_mut()
                .unwrap()
                .insert(parameter.name.text.clone(), ty);
        }
        parameter_types
    }
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "lambda checking threads module, body, async flag, span, context, and expectation"
    )]
    pub(super) fn lambda(
        &mut self,
        module: &Module,
        parameters: &[LambdaParameter],
        receiver: &LambdaReceiver,
        body: &LambdaBody,
        is_async: bool,
        span: Span,
        expected: Option<TypeId>,
        context: &mut Context,
    ) -> TypeId {
        let (expected_receiver, expected_parameters, expected_result) =
            self.expected_callable(expected);
        let receiver_type = if matches!(receiver, LambdaReceiver::Inferred)
            && expected_receiver.is_none()
        {
            self.error(
                codes::E1020,
                "trailing body requires a receiver function parameter",
                span,
                "the callee's last parameter must be a receiver function such as `fn(Scope)() -> void`",
            );
            None
        } else {
            expected_receiver
        };
        context.scopes.push(HashMap::new());
        if let Some(receiver_type) = receiver_type {
            context
                .scopes
                .last_mut()
                .unwrap()
                .insert("self".into(), receiver_type);
        }
        let parameter_types = self.bind_lambda_parameters(
            module,
            parameters,
            expected_parameters.as_deref(),
            context,
        );
        if let Some(expected_parameters) = &expected_parameters
            && expected_parameters.len() != parameters.len()
        {
            self.error(
                codes::E1014,
                "lambda parameter count mismatch",
                span,
                "the lambda does not match the expected function type",
            );
        }
        let saved_function = context.function;
        let saved_return = context.return_type;
        let saved_async = context.in_async;
        let saved_current = context.current_function;
        context.function = true;
        context.in_async = is_async;
        context.current_function = self.resolution.lambda_symbols.get(&span).copied();
        let result = match body {
            LambdaBody::Expression(value) => self.expr(module, value, context, expected_result),
            LambdaBody::Block(block) => {
                if let Some(expected_return) = expected_result {
                    context.return_type = Some(expected_return);
                    self.function_block(module, block, context, expected_return)
                } else {
                    // No contextual result: infer the lambda's result from its
                    // `return` statements / trailing expression.
                    context.return_type = None;
                    self.block(module, block, context)
                }
            }
        };
        context.function = saved_function;
        context.return_type = saved_return;
        context.in_async = saved_async;
        context.current_function = saved_current;
        // Record capture types for the closure, resolved from the enclosing
        // scopes. The lambda's own scope is on top and must be excluded.
        if let Some(lambda) = self
            .resolution
            .lambdas
            .iter()
            .find(|lambda| lambda.span == span)
            .cloned()
        {
            let enclosing = context.scopes.len().saturating_sub(1);
            let mut captures = Vec::new();
            for capture in &lambda.captures {
                let ty = context.scopes[..enclosing]
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(&capture.name).copied())
                    .unwrap_or_else(|| self.intern(Type::Error));
                captures.push((capture.name.clone(), ty));
            }
            if !captures.is_empty() {
                self.closure_captures.insert(span, captures);
            }
        }
        context.scopes.pop();
        let Some(symbol) = self.resolution.lambda_symbols.get(&span).copied() else {
            return self.intern(Type::Error);
        };
        self.signatures.insert(
            symbol,
            Signature {
                parameters: parameter_types,
                result,
            },
        );
        if let Some(receiver_type) = receiver_type {
            self.receiver_functions.insert(symbol, receiver_type);
        }
        if is_async {
            self.async_symbols.insert(symbol);
        }
        self.intern(Type::Function(symbol))
    }
    pub(super) fn block(
        &mut self,
        module: &Module,
        block: &Block,
        context: &mut Context,
    ) -> TypeId {
        context.scopes.push(HashMap::new());
        let mut result = self.intern(Type::Void);
        for statement in &block.statements {
            result = self.statement(module, statement, context, Some(result));
        }
        context.scopes.pop();
        result
    }
    pub(super) fn function_block(
        &mut self,
        module: &Module,
        block: &Block,
        context: &mut Context,
        expected: TypeId,
    ) -> TypeId {
        self.block_expect(module, block, context, Some(expected))
    }

    /// Checks a block, forwarding an optional contextual result type to its
    /// final statement so result constructors bind to the surrounding type.
    pub(super) fn block_expect(
        &mut self,
        module: &Module,
        block: &Block,
        context: &mut Context,
        contextual: Option<TypeId>,
    ) -> TypeId {
        context.scopes.push(HashMap::new());
        let last = block.statements.len().saturating_sub(1);
        let mut result = self.intern(Type::Void);
        for (index, statement) in block.statements.iter().enumerate() {
            let statement_context = if index == last { contextual } else { None };
            result = self.statement(module, statement, context, statement_context);
        }
        context.scopes.pop();
        result
    }
    #[expect(
        clippy::too_many_lines,
        reason = "keeps the exhaustive statement visitor together"
    )]
    pub(super) fn statement(
        &mut self,
        module: &Module,
        statement: &Stmt,
        context: &mut Context,
        contextual: Option<TypeId>,
    ) -> TypeId {
        match statement {
            Stmt::Binding {
                target,
                annotation,
                value,
                span,
            } => {
                let expected = annotation.as_ref().map(|ty| self.resolve_type(module, ty));
                let actual = self.expr(module, value, context, expected);
                if let Some(expected) = expected {
                    self.compatible(
                        actual,
                        expected,
                        value.span(),
                        codes::E1003,
                        "assignment type mismatch",
                    );
                }
                if let Expr::Name(name) = target {
                    // Record the variable's declared type at its binding span so
                    // MIR lowering stores it with the annotated (optional) type.
                    self.expression_types
                        .insert(name.span, expected.unwrap_or(actual));
                    if let Some(old) = context.lookup(&name.text) {
                        if is_constant_name(&name.text) {
                            self.error(
                                codes::E1104,
                                "constant reassignment",
                                name.span,
                                "ALL-CAPS bindings are constants and cannot be reassigned",
                            );
                        }
                        self.compatible(actual, old, *span, codes::E1003, "variable type is fixed");
                    } else {
                        context
                            .scopes
                            .last_mut()
                            .unwrap()
                            .insert(name.text.clone(), expected.unwrap_or(actual));
                    }
                } else if let Expr::Member { object, member, .. } = target {
                    let owner = self.expr(module, object, context, None);
                    self.field(module.id, owner, member, Some(actual));
                } else if let Expr::Subscript { .. } = target {
                    let element = self.expr(module, target, context, Some(actual));
                    self.compatible(
                        actual,
                        element,
                        value.span(),
                        codes::E1003,
                        "indexed assignment type mismatch",
                    );
                }
                self.intern(Type::Void)
            }
            Stmt::Expression(expr) => self.expr(module, expr, context, contextual),
            Stmt::If(value) => {
                self.condition(module, &value.condition, context, codes::E5007);
                let narrowing = self.optional_condition_narrowing(&value.condition, context);
                let then_binding = narrowing
                    .as_ref()
                    .map(|(name, inner, is_eq)| (!*is_eq, name, inner));
                self.block_narrowed(module, &value.body, context, then_binding);
                let mut last_narrowing = narrowing.clone();
                for (condition, body) in &value.elifs {
                    self.condition(module, condition, context, codes::E5007);
                    let elif_narrowing = self.optional_condition_narrowing(condition, context);
                    let binding = elif_narrowing
                        .as_ref()
                        .map(|(name, inner, is_eq)| (!*is_eq, name, inner));
                    self.block_narrowed(module, body, context, binding);
                    last_narrowing = elif_narrowing;
                }
                if let Some(body) = &value.otherwise {
                    let binding = last_narrowing
                        .as_ref()
                        .map(|(name, inner, is_eq)| (*is_eq, name, inner));
                    self.block_narrowed(module, body, context, binding);
                }
                // Guard clause: `if x == null: continue/return/break` narrows the
                // present type for the code after the statement.
                if let Some((name, inner, is_eq)) = &narrowing
                    && *is_eq
                    && Self::block_terminates(&value.body)
                    && let Some(scope) = context.scopes.last_mut()
                {
                    scope.insert(name.clone(), *inner);
                }
                self.intern(Type::Void)
            }
            Stmt::For(value) => {
                context.loop_depth += 1;
                match &value.kind {
                    ForKind::Infinite => {}
                    ForKind::Conditional(expr) => {
                        self.condition(module, expr, context, codes::E5003);
                    }
                    ForKind::Iterable {
                        value: binding,
                        index,
                        iterable,
                    } => {
                        let ty = self.expr(module, iterable, context, None);
                        let element = match self.types[ty.0] {
                            Type::List(id)
                            | Type::Array(id, _)
                            | Type::Range(id)
                            | Type::Variadic(id) => id,
                            _ => {
                                self.error(
                                    codes::E5004,
                                    "non-iterable value",
                                    iterable.span(),
                                    "`for` requires a list or range",
                                );
                                self.intern(Type::Error)
                            }
                        };
                        context.scopes.push(HashMap::new());
                        context
                            .scopes
                            .last_mut()
                            .unwrap()
                            .insert(binding.text.clone(), element);
                        if let Some(index) = index {
                            let int = self.intern(Type::Int);
                            context
                                .scopes
                                .last_mut()
                                .unwrap()
                                .insert(index.text.clone(), int);
                        }
                        self.block(module, &value.body, context);
                        context.scopes.pop();
                        context.loop_depth -= 1;
                        return self.intern(Type::Void);
                    }
                }
                self.block(module, &value.body, context);
                context.loop_depth -= 1;
                self.intern(Type::Void)
            }
            Stmt::Break(span) => {
                if context.loop_depth == 0 {
                    self.error(
                        codes::E5005,
                        "`break` outside loop",
                        *span,
                        "no enclosing loop",
                    );
                }
                self.intern(Type::Void)
            }
            Stmt::Continue(span) => {
                if context.loop_depth == 0 {
                    self.error(
                        codes::E5006,
                        "`continue` outside loop",
                        *span,
                        "no enclosing loop",
                    );
                }
                self.intern(Type::Void)
            }
            Stmt::Return { value, span } => {
                if !context.function {
                    self.error(
                        codes::E6007,
                        "`return` outside function",
                        *span,
                        "no enclosing function or method",
                    );
                }
                let actual = if let Some(value) = value {
                    context.return_depth += 1;
                    let actual = self.expr(module, value, context, context.return_type);
                    context.return_depth -= 1;
                    actual
                } else {
                    self.intern(Type::Void)
                };
                if let Some(expected) = context.return_type {
                    self.compatible(
                        actual,
                        expected,
                        *span,
                        codes::E6005,
                        "return type mismatch",
                    );
                }
                actual
            }
            Stmt::Unsafe(body) => {
                context.unsafe_depth += 1;
                let result = self.block_expect(module, body, context, contextual);
                context.unsafe_depth -= 1;
                result
            }
            Stmt::Error(_) => self.intern(Type::Error),
        }
    }
}
