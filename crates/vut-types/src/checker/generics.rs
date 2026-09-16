//! Generic declaration collection, type substitution, and instantiation records.
use super::context::Context;
#[allow(clippy::wildcard_imports)]
use super::*;

impl Analyzer<'_> {
    /// Ordered type-parameter symbols declared within `decl_span`.
    pub(super) fn type_parameters_of(&self, module: ModuleId, decl_span: Span) -> Vec<SymbolId> {
        let mut parameters: Vec<SymbolId> = self
            .resolution
            .symbols
            .iter()
            .filter(|symbol| {
                symbol.module == module
                    && matches!(symbol.kind, vut_resolver::SymbolKind::TypeParameter)
                    && decl_span.start() <= symbol.span.start()
                    && symbol.span.end() <= decl_span.end()
            })
            .map(|symbol| symbol.id)
            .collect();
        parameters.sort_by_key(|symbol| self.resolution.symbols[symbol.0].span.start());
        parameters
    }

    /// Records the ordered type parameters of every generic declaration.
    pub(super) fn collect_generic_params(&mut self) {
        for module in &self.resolution.modules {
            for item in &module.file.items {
                let (symbol, span, declared) = match item {
                    Item::Function(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        value.span,
                        &value.type_parameters,
                    ),
                    Item::ExternFunction(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        value.span,
                        &value.type_parameters,
                    ),
                    Item::Data(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        value.span,
                        &value.type_parameters,
                    ),
                    Item::Interface(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        value.span,
                        &value.type_parameters,
                    ),
                    Item::Enum(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        value.span,
                        &value.type_parameters,
                    ),
                    _ => continue,
                };
                let Some(symbol) = symbol else { continue };
                let parameters = self.type_parameters_of(module.id, span);
                if parameters.is_empty() {
                    continue;
                }
                for (parameter, declared) in parameters.iter().zip(declared.iter()) {
                    let bounds: Vec<SymbolId> = declared
                        .bounds
                        .iter()
                        .filter_map(|bound| self.reference_symbols.get(&bound.span()).copied())
                        .collect();
                    if !bounds.is_empty() {
                        self.generic_param_bounds.insert(*parameter, bounds);
                    }
                }
                self.generic_params.insert(symbol, parameters);
            }
        }
    }

    /// Substitutes type parameters (and composites containing them) using `map`.
    pub(super) fn substitute_type(
        &mut self,
        ty: TypeId,
        map: &HashMap<SymbolId, TypeId>,
    ) -> TypeId {
        let value = self.types[ty.0].clone();
        match value {
            Type::Param(symbol) => map.get(&symbol).copied().unwrap_or(ty),
            Type::Optional(inner) => {
                let inner = self.substitute_type(inner, map);
                self.intern(Type::Optional(inner))
            }
            Type::Result(ok, err) => {
                let ok = self.substitute_type(ok, map);
                let err = self.substitute_type(err, map);
                self.intern(Type::Result(ok, err))
            }
            Type::List(element) => {
                let element = self.substitute_type(element, map);
                self.intern(Type::List(element))
            }
            Type::Pointer(inner) => {
                let inner = self.substitute_type(inner, map);
                self.intern(Type::Pointer(inner))
            }
            Type::Array(element, length) => {
                let element = self.substitute_type(element, map);
                self.intern(Type::Array(element, length))
            }
            Type::Map(key, value) => {
                let key = self.substitute_type(key, map);
                let value = self.substitute_type(value, map);
                self.intern(Type::Map(key, value))
            }
            Type::Range(element) => {
                let element = self.substitute_type(element, map);
                self.intern(Type::Range(element))
            }
            Type::Applied(base, arguments) => {
                let arguments: Vec<TypeId> = arguments
                    .into_iter()
                    .map(|argument| self.substitute_type(argument, map))
                    .collect();
                if arguments
                    .iter()
                    .any(|argument| self.type_contains_param(*argument))
                {
                    self.intern(Type::Applied(base, arguments))
                } else {
                    let instance = self.instantiate_symbol(base, &arguments);
                    let kind = self.resolution.symbols[base.0].kind;
                    if kind == vut_resolver::SymbolKind::Enum {
                        self.intern(Type::Enum(instance))
                    } else {
                        self.intern(Type::Data(instance))
                    }
                }
            }
            Type::Callable {
                receiver,
                parameters,
                result,
            } => {
                let receiver = receiver.map(|receiver| self.substitute_type(receiver, map));
                let parameters = parameters
                    .into_iter()
                    .map(|parameter| self.substitute_type(parameter, map))
                    .collect();
                let result = self.substitute_type(result, map);
                self.intern(Type::Callable {
                    receiver,
                    parameters,
                    result,
                })
            }
            Type::FunctionPointer {
                abi,
                parameters,
                result,
            } => {
                let parameters = parameters
                    .into_iter()
                    .map(|parameter| self.substitute_type(parameter, map))
                    .collect();
                let result = self.substitute_type(result, map);
                self.intern(Type::FunctionPointer {
                    abi,
                    parameters,
                    result,
                })
            }
            other => self.intern(other),
        }
    }

    /// Replaces `Self` in a requirement type with the implementing type.
    pub(super) fn substitute_self(&mut self, ty: TypeId, concrete: TypeId) -> TypeId {
        let value = self.types[ty.0].clone();
        match value {
            Type::SelfType => concrete,
            Type::Optional(inner) => {
                let inner = self.substitute_self(inner, concrete);
                self.intern(Type::Optional(inner))
            }
            Type::Result(ok, err) => {
                let ok = self.substitute_self(ok, concrete);
                let err = self.substitute_self(err, concrete);
                self.intern(Type::Result(ok, err))
            }
            Type::List(element) => {
                let element = self.substitute_self(element, concrete);
                self.intern(Type::List(element))
            }
            Type::Pointer(inner) => {
                let inner = self.substitute_self(inner, concrete);
                self.intern(Type::Pointer(inner))
            }
            Type::Array(element, length) => {
                let element = self.substitute_self(element, concrete);
                self.intern(Type::Array(element, length))
            }
            Type::Map(key, value) => {
                let key = self.substitute_self(key, concrete);
                let value = self.substitute_self(value, concrete);
                self.intern(Type::Map(key, value))
            }
            Type::Range(element) => {
                let element = self.substitute_self(element, concrete);
                self.intern(Type::Range(element))
            }
            Type::Applied(base, arguments) => {
                let arguments = arguments
                    .into_iter()
                    .map(|argument| self.substitute_self(argument, concrete))
                    .collect();
                self.intern(Type::Applied(base, arguments))
            }
            Type::Callable {
                receiver,
                parameters,
                result,
            } => {
                let receiver = receiver.map(|receiver| self.substitute_self(receiver, concrete));
                let parameters = parameters
                    .into_iter()
                    .map(|parameter| self.substitute_self(parameter, concrete))
                    .collect();
                let result = self.substitute_self(result, concrete);
                self.intern(Type::Callable {
                    receiver,
                    parameters,
                    result,
                })
            }
            other => self.intern(other),
        }
    }

    /// Unifies a formal (possibly generic) type with an actual type, filling `map`.
    pub(super) fn unify(
        &mut self,
        formal: TypeId,
        actual: TypeId,
        map: &mut HashMap<SymbolId, TypeId>,
    ) -> bool {
        if matches!(self.types[actual.0], Type::Error) {
            return true;
        }
        match self.types[formal.0].clone() {
            Type::Param(symbol) => {
                if let Some(existing) = map.get(&symbol).copied() {
                    self.types[existing.0] == self.types[actual.0]
                } else {
                    map.insert(symbol, actual);
                    true
                }
            }
            Type::Optional(formal) => {
                if let Type::Optional(actual) = self.types[actual.0] {
                    self.unify(formal, actual, map)
                } else {
                    false
                }
            }
            Type::Result(fok, ferr) => {
                if let Type::Result(aok, aerr) = self.types[actual.0] {
                    self.unify(fok, aok, map) && self.unify(ferr, aerr, map)
                } else {
                    false
                }
            }
            Type::List(formal) => {
                if let Type::List(actual) = self.types[actual.0] {
                    self.unify(formal, actual, map)
                } else {
                    false
                }
            }
            Type::Pointer(formal) => {
                if let Type::Pointer(actual) = self.types[actual.0] {
                    self.unify(formal, actual, map)
                } else {
                    false
                }
            }
            Type::Array(formal, flen) => {
                if let Type::Array(actual, alen) = self.types[actual.0] {
                    flen == alen && self.unify(formal, actual, map)
                } else {
                    false
                }
            }
            Type::Map(fkey, fvalue) => {
                if let Type::Map(akey, avalue) = self.types[actual.0] {
                    self.unify(fkey, akey, map) && self.unify(fvalue, avalue, map)
                } else {
                    false
                }
            }
            Type::Applied(base, formal_arguments) => {
                let actual_applied = match &self.types[actual.0] {
                    Type::Applied(actual_base, actual_arguments) => {
                        Some((*actual_base, actual_arguments.clone()))
                    }
                    _ => None,
                };
                if let Some((actual_base, actual_arguments)) = actual_applied {
                    actual_base == base
                        && formal_arguments.len() == actual_arguments.len()
                        && formal_arguments
                            .iter()
                            .zip(actual_arguments.iter())
                            .all(|(formal, actual)| self.unify(*formal, *actual, map))
                } else {
                    false
                }
            }
            _ => self.types[formal.0] == self.types[actual.0],
        }
    }

    /// Type-checks a call to a generic function, infers its arguments, records
    /// the specialization, and returns the substituted result type.
    #[expect(
        clippy::too_many_arguments,
        reason = "generic call checking threads module, symbol, parameters, signature, arguments, expectation, span, and context"
    )]
    pub(super) fn check_generic_call(
        &mut self,
        module: &Module,
        symbol: SymbolId,
        parameters: &[SymbolId],
        signature: &Signature,
        arguments: &[vut_ast::Argument],
        expected: Option<TypeId>,
        span: Span,
        context: &mut Context,
    ) -> TypeId {
        let signature = signature.clone();
        let mut map: HashMap<SymbolId, TypeId> = HashMap::new();
        let mut used = HashSet::new();
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = if let Some(name) = &argument.name {
                signature
                    .parameters
                    .iter()
                    .position(|(candidate, _)| candidate == &name.text)
            } else {
                Some(index)
            };
            let Some(index) = parameter.filter(|index| *index < signature.parameters.len()) else {
                self.error(
                    codes::E6003,
                    "invalid argument",
                    argument.span,
                    "unknown argument name or position",
                );
                continue;
            };
            if !used.insert(index) {
                self.error(
                    codes::E6004,
                    "duplicate argument",
                    argument.span,
                    "parameter supplied more than once",
                );
                continue;
            }
            let formal = signature.parameters[index].1;
            let actual = self.expr(module, &argument.value, context, None);
            if !self.unify(formal, actual, &mut map) {
                let substituted = self.substitute_type(formal, &map);
                self.compatible(
                    actual,
                    substituted,
                    argument.span,
                    codes::E1003,
                    "argument type mismatch",
                );
            }
        }
        if used.len() != signature.parameters.len() {
            self.error(
                codes::E6002,
                "invalid argument count",
                span,
                "not all required parameters were supplied",
            );
        }
        // Use the expected type to infer still-unconstrained parameters (for
        // example `user: User = json.decode(source)?`).
        if let Some(expected) = expected {
            let result = self.substitute_type(signature.result, &map);
            self.unify(result, expected, &mut map);
        }
        let arguments_types: Vec<TypeId> = parameters
            .iter()
            .map(|parameter| {
                map.get(parameter).copied().unwrap_or_else(|| {
                    self.error(
                        codes::E1015,
                        "cannot infer type argument",
                        span,
                        "the type parameter is not constrained by the call",
                    );
                    self.intern(Type::Error)
                })
            })
            .collect();
        self.check_constraints(parameters, &arguments_types, span);
        self.generic_function_calls.push(GenericCall {
            call_span: span,
            template: symbol,
            arguments: arguments_types.clone(),
        });
        self.record_substitution(symbol, &arguments_types, &map);
        self.substitute_type(signature.result, &map)
    }

    /// Records the full `TypeId` substitution for one concrete specialization.
    pub(super) fn record_substitution(
        &mut self,
        template: SymbolId,
        arguments: &[TypeId],
        map: &HashMap<SymbolId, TypeId>,
    ) {
        if arguments
            .iter()
            .any(|argument| self.type_contains_param(*argument))
        {
            return;
        }
        let count = self.types.len();
        let mut substitution = HashMap::new();
        for index in 0..count {
            let ty = TypeId(index);
            let substituted = self.substitute_type(ty, map);
            substitution.insert(ty, substituted);
        }
        self.generic_substitutions
            .insert((template, arguments.to_vec()), substitution);
    }

    /// Returns true when `ty` mentions a generic type parameter.
    pub(super) fn type_contains_param(&self, ty: TypeId) -> bool {
        match &self.types[ty.0] {
            Type::Param(_) => true,
            Type::Optional(inner)
            | Type::List(inner)
            | Type::Pointer(inner)
            | Type::Range(inner) => self.type_contains_param(*inner),
            Type::Result(ok, err) => {
                self.type_contains_param(*ok) || self.type_contains_param(*err)
            }
            Type::Array(element, _) => self.type_contains_param(*element),
            Type::Map(key, value) => {
                self.type_contains_param(*key) || self.type_contains_param(*value)
            }
            Type::Applied(_, arguments) => arguments
                .iter()
                .any(|argument| self.type_contains_param(*argument)),
            Type::Callable {
                receiver,
                parameters,
                result,
            } => {
                receiver.is_some_and(|ty| self.type_contains_param(ty))
                    || parameters.iter().any(|ty| self.type_contains_param(*ty))
                    || self.type_contains_param(*result)
            }
            Type::FunctionPointer {
                parameters, result, ..
            } => {
                parameters.iter().any(|ty| self.type_contains_param(*ty))
                    || self.type_contains_param(*result)
            }
            _ => false,
        }
    }

    /// Verifies every interface bound on a type parameter for a concrete argument.
    pub(super) fn check_constraints(
        &mut self,
        parameters: &[SymbolId],
        arguments: &[TypeId],
        span: Span,
    ) {
        for (parameter, argument) in parameters.iter().zip(arguments.iter()) {
            let Some(bounds) = self.generic_param_bounds.get(parameter).cloned() else {
                continue;
            };
            for interface in bounds {
                let satisfaction = self.interface_satisfaction(*argument, interface);
                let detail = match satisfaction {
                    Satisfaction::Satisfied => continue,
                    Satisfaction::Missing { method } => format!("missing method `{method}`"),
                    Satisfaction::Mismatch { method } => {
                        format!("method `{method}` has an incompatible signature")
                    }
                    Satisfaction::Private { method } => {
                        format!("method `{method}` is private to its module")
                    }
                };
                let interface_name = self.resolution.symbols[interface.0].name.clone();
                self.error(
                    codes::E1017,
                    "type does not satisfy a generic constraint",
                    span,
                    &format!("the type argument does not satisfy `{interface_name}`: {detail}"),
                );
            }
        }
    }

    /// Module that declares `symbol`, including synthetic instance symbols.
    pub(super) fn symbol_module(&self, symbol: SymbolId) -> ModuleId {
        self.instance_symbols.get(&symbol).map_or_else(
            || self.resolution.symbols[symbol.0].module,
            |value| value.module,
        )
    }

    /// Kind of `symbol`, including synthetic data/enum instance symbols.
    pub(super) fn symbol_kind(&self, symbol: SymbolId) -> vut_resolver::SymbolKind {
        self.instance_symbols.get(&symbol).map_or_else(
            || self.resolution.symbols[symbol.0].kind,
            |value| value.kind,
        )
    }

    /// Name of `symbol`, including synthetic instance symbols.
    pub(super) fn symbol_name(&self, symbol: SymbolId) -> String {
        self.instance_symbols.get(&symbol).map_or_else(
            || self.resolution.symbols[symbol.0].name.clone(),
            |value| value.name.clone(),
        )
    }

    /// Returns the generic template that produced `instance`, if any.
    pub(super) fn instance_template(&self, instance: SymbolId) -> Option<SymbolId> {
        self.instances
            .iter()
            .find_map(|((template, _), id)| (*id == instance).then_some(*template))
    }

    /// Resolves a user generic application `Name(args)` to an instance type.
    pub(super) fn instantiate_special(
        &mut self,
        module: &Module,
        name: &str,
        span: Span,
        arguments: Vec<TypeId>,
    ) -> Option<TypeId> {
        let symbol = module
            .symbols
            .get(name)
            .or_else(|| module.imports.get(name))
            .copied()?;
        let kind = self.resolution.symbols[symbol.0].kind;
        if !matches!(
            kind,
            vut_resolver::SymbolKind::Data | vut_resolver::SymbolKind::Enum
        ) {
            return None;
        }
        let parameters = self
            .generic_params
            .get(&symbol)
            .cloned()
            .unwrap_or_default();
        if parameters.is_empty() {
            return None;
        }
        if parameters.len() != arguments.len() {
            self.error(
                codes::E1016,
                "wrong number of type arguments",
                span,
                "type argument count does not match the declaration",
            );
            return Some(self.intern(Type::Error));
        }
        if arguments
            .iter()
            .any(|argument| self.type_contains_param(*argument))
        {
            return Some(self.intern(Type::Applied(symbol, arguments)));
        }
        self.check_constraints(&parameters, &arguments, span);
        let instance = self.instantiate_symbol(symbol, &arguments);
        self.generic_type_applications.push(GenericApplication {
            span,
            template: symbol,
            arguments,
        });
        Some(if kind == vut_resolver::SymbolKind::Data {
            self.intern(Type::Data(instance))
        } else {
            self.intern(Type::Enum(instance))
        })
    }

    /// Creates (or reuses) the concrete instance symbol for a generic type.
    pub(super) fn instantiate_symbol(
        &mut self,
        template: SymbolId,
        arguments: &[TypeId],
    ) -> SymbolId {
        if let Some(existing) = self.instances.get(&(template, arguments.to_vec())) {
            return *existing;
        }
        let id = SymbolId(self.next_instance);
        self.next_instance += 1;
        let template_symbol = self.resolution.symbols[template.0].clone();
        let name = format!("{}$inst{}", template_symbol.name, id.0);
        self.instance_symbols.insert(
            id,
            vut_resolver::Symbol {
                id,
                module: template_symbol.module,
                name,
                kind: template_symbol.kind,
                span: template_symbol.span,
                public: false,
                target_module: None,
                receiver: None,
                is_static: false,
            },
        );
        self.instances.insert((template, arguments.to_vec()), id);
        let parameters = self
            .generic_params
            .get(&template)
            .cloned()
            .unwrap_or_default();
        let map: HashMap<SymbolId, TypeId> = parameters
            .into_iter()
            .zip(arguments.iter().copied())
            .collect();
        if let Some(fields) = self.fields.get(&template).cloned() {
            let substituted = fields
                .into_iter()
                .map(|(name, field)| {
                    let ty = self.substitute_type(field.ty, &map);
                    (name, Field { ty, ..field })
                })
                .collect();
            self.fields.insert(id, substituted);
        }
        if let Some(order) = self.data_field_order.get(&template).cloned() {
            self.data_field_order.insert(id, order);
        }
        if let Some(variants) = self.enum_variants.get(&template).cloned() {
            let substituted = variants
                .into_iter()
                .map(|variant| {
                    let fields = variant
                        .fields
                        .into_iter()
                        .map(|field| {
                            let ty = self.substitute_type(field.ty, &map);
                            VariantFieldInfo { ty, ..field }
                        })
                        .collect();
                    VariantInfo {
                        name: variant.name,
                        fields,
                    }
                })
                .collect();
            self.enum_variants.insert(id, substituted);
        }
        id
    }
}
