//! Analyzer construction, type resolution, and declaration collection.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<'a> Analyzer<'a> {
    #[must_use]
    pub fn new(resolution: &'a Resolution) -> Self {
        let mut value = Self {
            resolution,
            types: Vec::new(),
            type_ids: HashMap::new(),
            expression_types: HashMap::new(),
            diagnostics: DiagnosticSink::new(),
            signatures: HashMap::new(),
            fields: HashMap::new(),
            aliases: HashMap::new(),
            interface_shapes: HashMap::new(),
            satisfaction: HashMap::new(),
            call_targets: HashMap::new(),
            builtin_calls: HashMap::new(),
            bound_calls: HashMap::new(),
            variadic_functions: HashMap::new(),
            receiver_functions: HashMap::new(),
            receiver_calls: HashSet::new(),
            implicit_receiver_calls: HashSet::new(),
            extern_symbols: HashSet::new(),
            async_symbols: HashSet::new(),
            async_externs: HashSet::new(),
            async_calls: HashSet::new(),
            enum_variants: HashMap::new(),
            variant_constructions: HashMap::new(),
            method_symbols: HashMap::new(),
            opaque_data: HashSet::new(),
            attributes: AttributeSemantics::default(),
            builtin_utf8_error: resolution.builtin_utf8_error,
            reference_symbols: resolution
                .references
                .iter()
                .map(|reference| (reference.span, reference.symbol))
                .collect(),
            self_type_allowed: false,
            generic_params: HashMap::new(),
            generic_param_bounds: HashMap::new(),
            generic_function_calls: Vec::new(),
            generic_type_applications: Vec::new(),
            generic_substitutions: HashMap::new(),
            instance_symbols: HashMap::new(),
            instances: HashMap::new(),
            data_field_order: HashMap::new(),
            next_instance: resolution.symbols.len(),
        };
        for ty in [
            Type::Error,
            Type::Void,
            Type::Bool,
            Type::Int,
            Type::Float,
            Type::Str,
            Type::Bytes,
            Type::Dyn,
            Type::Null,
        ] {
            value.intern(ty);
        }
        value
    }
    #[must_use]
    pub fn analyze(mut self) -> SemanticResult {
        let (attributes, attribute_diagnostics) = attributes::validate(self.resolution);
        self.diagnostics.extend(attribute_diagnostics);
        self.attributes = attributes;
        self.collect_generic_params();
        self.collect_types();
        self.collect_signatures();
        let mut builtin_functions = HashMap::new();
        for symbol in &self.resolution.symbols {
            let builtin = match symbol.name.as_str() {
                "print" => Some(BuiltinFunction::Print),
                "out" => Some(BuiltinFunction::Out),
                "input" => Some(BuiltinFunction::Input),
                _ => None,
            };
            if let Some(builtin) = builtin {
                let string = self.intern(Type::Str);
                let result = if builtin == BuiltinFunction::Input {
                    string
                } else {
                    self.intern(Type::Void)
                };
                self.signatures
                    .entry(symbol.id)
                    .or_insert_with(|| Signature {
                        parameters: vec![("value".into(), string)],
                        result,
                    });
                builtin_functions.insert(symbol.id, builtin);
            }
        }
        self.collect_interfaces();
        for module in &self.resolution.modules {
            self.module(module);
        }
        self.diagnostics.sort_deterministically();
        let data_fields = self.data_fields();
        let data_field_defaults = self.data_field_defaults();
        self.validate_repr_fields(&self.attributes.clone(), &data_fields);
        let function_signatures = self
            .signatures
            .iter()
            .map(|(symbol, signature)| {
                (
                    *symbol,
                    FunctionSignatureInfo {
                        receiver: self.receiver_functions.get(symbol).copied(),
                        parameters: signature.parameters.iter().map(|(_, ty)| *ty).collect(),
                        variadic: self.variadic_functions.get(symbol).copied(),
                        result: signature.result,
                    },
                )
            })
            .collect();
        let resolution = self.resolution;
        for module in &resolution.modules {
            for (key, symbol) in &module.methods {
                self.method_symbols
                    .insert((key.receiver, key.name.clone()), *symbol);
            }
        }
        SemanticResult {
            types: self.types,
            expression_types: self.expression_types,
            diagnostics: self.diagnostics,
            interface_shapes: self.interface_shapes,
            interface_satisfaction: self.satisfaction,
            data_fields,
            data_field_defaults,
            opaque_data: self.opaque_data,
            function_signatures,
            builtin_functions,
            builtin_calls: self.builtin_calls,
            call_targets: self.call_targets,
            receiver_calls: self.receiver_calls,
            implicit_receiver_calls: self.implicit_receiver_calls,
            bound_calls: self.bound_calls,
            async_symbols: self.async_symbols,
            async_externs: self.async_externs,
            extern_symbols: self.extern_symbols,
            enum_variants: self.enum_variants,
            variant_constructions: self.variant_constructions,
            method_symbols: self.method_symbols,
            attributes: self.attributes,
            static_methods: self
                .resolution
                .symbols
                .iter()
                .filter(|symbol| symbol.is_static)
                .map(|symbol| symbol.id)
                .collect(),
            generic_params: self.generic_params,
            generic_param_bounds: self.generic_param_bounds,
            generic_function_calls: self.generic_function_calls,
            generic_type_applications: self.generic_type_applications,
            generic_substitutions: self.generic_substitutions,
            next_instance_symbol: self.next_instance,
        }
    }

    pub(super) fn data_fields(&self) -> HashMap<SymbolId, Vec<DataFieldInfo>> {
        let mut data_fields = HashMap::new();
        for module in &self.resolution.modules {
            for item in &module.file.items {
                let Item::Data(value) = item else { continue };
                let Some(symbol) = module.symbols.get(&value.name.text).copied() else {
                    continue;
                };
                let Some(fields) = self.fields.get(&symbol) else {
                    continue;
                };
                data_fields.insert(
                    symbol,
                    value
                        .fields
                        .iter()
                        .filter_map(|field| {
                            fields.get(&field.name.text).map(|info| DataFieldInfo {
                                name: field.name.text.clone(),
                                ty: info.ty,
                                required: info.required,
                                public: info.public,
                            })
                        })
                        .collect(),
                );
            }
        }
        if let Some(fields) = self.fields.get(&self.builtin_utf8_error) {
            let mut builtin = Vec::new();
            for name in ["valid_up_to", "error_len"] {
                if let Some(field) = fields.get(name) {
                    builtin.push(DataFieldInfo {
                        name: name.to_owned(),
                        ty: field.ty,
                        required: field.required,
                        public: field.public,
                    });
                }
            }
            data_fields.insert(self.builtin_utf8_error, builtin);
        }
        for (symbol, order) in &self.data_field_order {
            if !self.instance_symbols.contains_key(symbol)
                || self.symbol_kind(*symbol) != vut_resolver::SymbolKind::Data
            {
                continue;
            }
            let Some(fields) = self.fields.get(symbol) else {
                continue;
            };
            let instance_fields: Vec<DataFieldInfo> = order
                .iter()
                .filter_map(|name| {
                    fields.get(name).map(|field| DataFieldInfo {
                        name: name.clone(),
                        ty: field.ty,
                        required: field.required,
                        public: field.public,
                    })
                })
                .collect();
            data_fields.insert(*symbol, instance_fields);
        }
        data_fields
    }

    /// Default field expressions per data type, aligned with [`Self::data_fields`]
    /// order. Generic specializations inherit their template's defaults by field
    /// name.
    pub(super) fn data_field_defaults(&self) -> HashMap<SymbolId, Vec<Option<vut_ast::Expr>>> {
        let mut defaults: HashMap<SymbolId, Vec<Option<vut_ast::Expr>>> = HashMap::new();
        for module in &self.resolution.modules {
            for item in &module.file.items {
                let Item::Data(value) = item else { continue };
                let Some(symbol) = module.symbols.get(&value.name.text).copied() else {
                    continue;
                };
                defaults.insert(
                    symbol,
                    value
                        .fields
                        .iter()
                        .map(|field| field.default.clone())
                        .collect(),
                );
            }
        }
        let mut template_of: HashMap<SymbolId, SymbolId> = HashMap::new();
        for ((template, _arguments), instance) in &self.instances {
            template_of.insert(*instance, *template);
        }
        for (instance, template) in template_of {
            let (Some(order), Some(template_order)) = (
                self.data_field_order.get(&instance),
                self.data_field_order.get(&template),
            ) else {
                continue;
            };
            let Some(template_defaults) = defaults.get(&template).cloned() else {
                continue;
            };
            let by_name: HashMap<&str, &Option<vut_ast::Expr>> = template_order
                .iter()
                .zip(template_defaults.iter())
                .map(|(name, value)| (name.as_str(), value))
                .collect();
            let entry: Vec<Option<vut_ast::Expr>> = order
                .iter()
                .map(|name| {
                    by_name
                        .get(name.as_str())
                        .and_then(|value| (*value).clone())
                })
                .collect();
            defaults.insert(instance, entry);
        }
        defaults
    }

    pub(super) fn validate_repr_fields(
        &mut self,
        attributes: &AttributeSemantics,
        data_fields: &HashMap<SymbolId, Vec<DataFieldInfo>>,
    ) {
        for (symbol, repr) in &attributes.data_repr {
            let Some(fields) = data_fields.get(symbol) else {
                continue;
            };
            for field in fields {
                if !self.ffi_safe(field.ty, true) {
                    self.error(
                        codes::E8004,
                        "field is not FFI-safe",
                        self.resolution.symbols[symbol.0].span,
                        &format!(
                            "`repr({})` data contains field `{}` without a defined C ABI",
                            match repr {
                                Repr::C => "C",
                                Repr::Transparent => "transparent",
                            },
                            field.name
                        ),
                    );
                }
            }
        }
    }
    pub(super) fn intern(&mut self, ty: Type) -> TypeId {
        if let Some(id) = self.type_ids.get(&ty) {
            return *id;
        }
        let id = TypeId(self.types.len());
        self.types.push(ty.clone());
        self.type_ids.insert(ty, id);
        id
    }
    pub(super) fn builtin(&mut self, name: &str) -> Option<TypeId> {
        Some(match name {
            "void" => self.intern(Type::Void),
            "bool" => self.intern(Type::Bool),
            "int" => self.intern(Type::Int),
            "float" => self.intern(Type::Float),
            "str" => self.intern(Type::Str),
            "bytes" => self.intern(Type::Bytes),
            "Utf8Error" => self.intern(Type::Data(self.builtin_utf8_error)),
            "dyn" => self.intern(Type::Dyn),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize"
            | "f32" | "f64" => self.intern(Type::Numeric(name.into())),
            _ => return None,
        })
    }
    #[expect(
        clippy::too_many_lines,
        reason = "type resolution keeps every builtin and applied form together"
    )]
    pub(super) fn resolve_type(&mut self, module: &Module, syntax: &TypeExpr) -> TypeId {
        match syntax {
            TypeExpr::Named { path, span } => {
                if path.len() == 1 {
                    if path[0].text == "Self" {
                        if self.self_type_allowed {
                            return self.intern(Type::SelfType);
                        }
                        self.error(
                            codes::E1001,
                            "`Self` is only valid inside an interface requirement",
                            *span,
                            "use a concrete type here",
                        );
                        return self.intern(Type::Error);
                    }
                    if let Some(symbol) =
                        self.reference_symbols
                            .get(&path[0].span)
                            .copied()
                            .filter(|symbol| {
                                matches!(
                                    self.resolution.symbols[symbol.0].kind,
                                    vut_resolver::SymbolKind::TypeParameter
                                )
                            })
                    {
                        return self.intern(Type::Param(symbol));
                    }
                    if let Some(id) = self.builtin(&path[0].text) {
                        return id;
                    }
                    if let Some(symbol) = module
                        .symbols
                        .get(&path[0].text)
                        .or_else(|| module.imports.get(&path[0].text))
                        .copied()
                    {
                        if let Some(alias) = self.aliases.get(&symbol) {
                            return *alias;
                        }
                        return self.symbol_type(symbol);
                    }
                }
                // A qualified type path (`module.Type`) resolves through the
                // reference the resolver registered for its final segment.
                if let Some(symbol) = path
                    .last()
                    .and_then(|segment| self.reference_symbols.get(&segment.span).copied())
                    .filter(|symbol| {
                        !matches!(
                            self.resolution.symbols[symbol.0].kind,
                            vut_resolver::SymbolKind::Module
                        )
                    })
                {
                    if let Some(alias) = self.aliases.get(&symbol) {
                        return *alias;
                    }
                    return self.symbol_type(symbol);
                }
                self.error(
                    codes::E1001,
                    "unknown type",
                    *span,
                    "type is not visible in this module",
                );
                self.intern(Type::Error)
            }
            TypeExpr::Applied {
                name,
                arguments,
                span,
            } => match (name.text.as_str(), arguments.as_slice()) {
                ("list", [inner]) => {
                    let inner = self.resolve_type(module, inner);
                    self.intern(Type::List(inner))
                }
                ("ptr", [inner]) => {
                    let inner = self.resolve_type(module, inner);
                    self.intern(Type::Pointer(inner))
                }
                ("map", [key, value]) => {
                    let key = self.resolve_type(module, key);
                    let value = self.resolve_type(module, value);
                    self.intern(Type::Map(key, value))
                }
                ("result", [ok, err]) => {
                    let ok = self.resolve_type(module, ok);
                    let err = self.resolve_type(module, err);
                    self.intern(Type::Result(ok, err))
                }
                ("vutcon", [inner]) => {
                    let inner = self.resolve_type(module, inner);
                    self.intern(Type::Vutcon(inner))
                }
                ("future", [inner]) => {
                    let inner = self.resolve_type(module, inner);
                    self.intern(Type::Future(inner))
                }
                ("resource", [inner]) => {
                    let inner = self.resolve_type(module, inner);
                    let valid = matches!(
                        self.types[inner.0],
                        Type::Void | Type::Numeric(_) | Type::Pointer(_)
                    ) || matches!(
                        self.types[inner.0],
                        Type::Data(symbol)
                            if self.opaque_data.contains(&symbol)
                                || self.attributes.data_repr.contains_key(&symbol)
                    );
                    if !valid {
                        self.error(
                            codes::E8004,
                            "invalid resource pointee",
                            *span,
                            "`resource(T)` requires an `opaque data`, `@repr(C)` data, scalar, pointer, or `void` pointee",
                        );
                    }
                    self.intern(Type::Resource(inner))
                }
                _ => {
                    let arguments = arguments
                        .iter()
                        .map(|argument| self.resolve_type(module, argument))
                        .collect();
                    if let Some(instance) =
                        self.instantiate_special(module, &name.text, *span, arguments)
                    {
                        return instance;
                    }
                    self.error(
                        codes::E1001,
                        "unknown parameterized type",
                        *span,
                        "expected `list(T)`, `map(K, V)`, `ptr(T)`, `result(T, E)`, or a generic declaration",
                    );
                    self.intern(Type::Error)
                }
            },
            TypeExpr::Optional { inner, .. } => {
                let inner = self.resolve_type(module, inner);
                self.intern(Type::Optional(inner))
            }
            TypeExpr::Function {
                receiver,
                parameters,
                return_type,
                ..
            } => {
                let receiver = receiver
                    .as_ref()
                    .map(|receiver| self.resolve_type(module, receiver));
                let parameters = parameters
                    .iter()
                    .map(|parameter| self.resolve_type(module, parameter))
                    .collect();
                let result = match return_type {
                    Some(ty) => self.resolve_type(module, ty),
                    None => self.intern(Type::Void),
                };
                self.intern(Type::Callable {
                    receiver,
                    parameters,
                    result,
                })
            }
            TypeExpr::ExternFunction {
                abi,
                abi_span,
                parameters,
                return_type,
                ..
            } => {
                if abi != "C" {
                    self.error(
                        codes::E8005,
                        "unsupported extern ABI",
                        *abi_span,
                        "FFI v1 only supports `extern \"C\" fn` function pointer types",
                    );
                }
                let parameters = parameters
                    .iter()
                    .map(|parameter| self.resolve_type(module, parameter))
                    .collect();
                let result = match return_type {
                    Some(ty) => self.resolve_type(module, ty),
                    None => self.intern(Type::Void),
                };
                self.intern(Type::FunctionPointer {
                    abi: abi.clone(),
                    parameters,
                    result,
                })
            }
            TypeExpr::Array {
                element,
                length,
                length_span,
                ..
            } => {
                if *length == 0 {
                    self.error(
                        codes::E1003,
                        "zero-length array is not supported",
                        *length_span,
                        "array length must be greater than zero",
                    );
                }
                let element = self.resolve_type(module, element);
                self.intern(Type::Array(element, *length))
            }
            TypeExpr::Error(_) => self.intern(Type::Error),
        }
    }
    pub(super) fn symbol_type(&mut self, symbol: SymbolId) -> TypeId {
        use vut_resolver::SymbolKind;
        if let Some(instance) = self.instance_symbols.get(&symbol) {
            return match instance.kind {
                SymbolKind::Data => self.intern(Type::Data(symbol)),
                SymbolKind::Enum => self.intern(Type::Enum(symbol)),
                _ => self.intern(Type::Error),
            };
        }
        match self.resolution.symbols[symbol.0].kind {
            SymbolKind::Data => self.intern(Type::Data(symbol)),
            SymbolKind::Enum => self.intern(Type::Enum(symbol)),
            SymbolKind::Interface => self.intern(Type::Interface(symbol)),
            SymbolKind::TypeParameter => self.intern(Type::Param(symbol)),
            SymbolKind::Function | SymbolKind::Method | SymbolKind::AnonymousFunction => {
                self.intern(Type::Function(symbol))
            }
            SymbolKind::TypeAlias => self
                .aliases
                .get(&symbol)
                .copied()
                .unwrap_or_else(|| self.intern(Type::Error)),
            _ => self.intern(Type::Error),
        }
    }
    pub(super) fn collect_types(&mut self) {
        for module in &self.resolution.modules {
            for item in &module.file.items {
                if let Item::TypeAlias(value) = item
                    && let Some(symbol) = module.symbols.get(&value.name.text).copied()
                {
                    let ty = self.resolve_type(module, &value.ty);
                    self.aliases.insert(symbol, ty);
                }
            }
            for item in &module.file.items {
                if let Item::Data(value) = item
                    && let Some(symbol) = module.symbols.get(&value.name.text).copied()
                {
                    if value.opaque {
                        self.opaque_data.insert(symbol);
                        continue;
                    }
                    self.data_field_order.insert(
                        symbol,
                        value
                            .fields
                            .iter()
                            .map(|field| field.name.text.clone())
                            .collect(),
                    );
                    let mut fields = HashMap::new();
                    for field in &value.fields {
                        if fields.contains_key(&field.name.text) {
                            self.error(
                                codes::E2002,
                                "duplicate field",
                                field.name.span,
                                "field declared more than once",
                            );
                            continue;
                        }
                        fields.insert(
                            field.name.text.clone(),
                            Field {
                                ty: self.resolve_type(module, &field.ty),
                                required: field.default.is_none(),
                                public: !field.name.text.starts_with('_'),
                            },
                        );
                    }
                    self.fields.insert(symbol, fields);
                }
            }
            for item in &module.file.items {
                if let Item::Enum(value) = item
                    && let Some(symbol) = module.symbols.get(&value.name.text).copied()
                {
                    let mut variants: Vec<VariantInfo> = Vec::new();
                    for variant in &value.variants {
                        if variants
                            .iter()
                            .any(|existing| existing.name == variant.name.text)
                        {
                            self.error(
                                codes::E7102,
                                "duplicate enum variant",
                                variant.name.span,
                                "variant declared more than once",
                            );
                            continue;
                        }
                        let mut fields = Vec::new();
                        for field in &variant.fields {
                            if fields
                                .iter()
                                .any(|existing: &VariantFieldInfo| existing.name == field.name.text)
                            {
                                self.error(
                                    codes::E2002,
                                    "duplicate variant field",
                                    field.name.span,
                                    "field declared more than once",
                                );
                                continue;
                            }
                            fields.push(VariantFieldInfo {
                                name: field.name.text.clone(),
                                ty: self.resolve_type(module, &field.ty),
                            });
                        }
                        variants.push(VariantInfo {
                            name: variant.name.text.clone(),
                            fields,
                        });
                    }
                    self.enum_variants.insert(symbol, variants);
                }
            }
        }
        self.detect_recursive_enums();
        self.collect_builtin_utf8_error();
    }
    /// Rejects enums whose variants contain themselves by value, which would
    /// have infinite size. Recursive structures must use an indirect container
    /// such as `list(T)`.
    pub(super) fn detect_recursive_enums(&mut self) {
        let symbols: Vec<SymbolId> = self.enum_variants.keys().copied().collect();
        let mut adjacency: HashMap<SymbolId, Vec<SymbolId>> = HashMap::new();
        for symbol in &symbols {
            let mut edges = Vec::new();
            if let Some(variants) = self.enum_variants.get(symbol) {
                for variant in variants {
                    for field in &variant.fields {
                        if let Type::Enum(target) = self.types[field.ty.0] {
                            edges.push(target);
                        }
                    }
                }
            }
            adjacency.insert(*symbol, edges);
        }
        let mut state: HashMap<SymbolId, u8> = HashMap::new();
        let mut stack = Vec::new();
        let mut in_cycle = Vec::new();
        let graph = adjacency.clone();
        for symbol in &symbols {
            if state.get(symbol).copied().unwrap_or(0) == 0 {
                visit_enums(*symbol, &graph, &mut state, &mut stack, &mut in_cycle);
            }
        }
        for symbol in in_cycle {
            let span = self.resolution.symbols[symbol.0].span;
            self.error(
                codes::E7105,
                "recursive enum payload",
                span,
                "an enum cannot contain itself by value; use an indirect container such as `list(T)`",
            );
        }
    }
    pub(super) fn collect_builtin_utf8_error(&mut self) {
        let int = self.intern(Type::Int);
        let mut fields = HashMap::new();
        fields.insert(
            "valid_up_to".to_owned(),
            Field {
                ty: int,
                required: true,
                public: true,
            },
        );
        fields.insert(
            "error_len".to_owned(),
            Field {
                ty: int,
                required: true,
                public: true,
            },
        );
        self.fields.insert(self.builtin_utf8_error, fields);
    }
    pub(super) fn collect_signatures(&mut self) {
        for module in &self.resolution.modules {
            for item in &module.file.items {
                let (symbol, parameters, result, is_extern, is_async, abi) = match item {
                    Item::Function(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        &value.parameters,
                        value.return_type.as_ref(),
                        false,
                        value.is_async,
                        None,
                    ),
                    Item::ExternFunction(value) => (
                        module.symbols.get(&value.name.text).copied(),
                        &value.parameters,
                        value.return_type.as_ref(),
                        true,
                        value.is_async,
                        Some((value.abi.as_str(), value.abi_span)),
                    ),
                    Item::Method(value) => {
                        let symbol = module
                            .methods
                            .iter()
                            .find(|(_, id)| self.resolution.symbols[id.0].span == value.name.span)
                            .map(|(_, id)| *id);
                        (
                            symbol,
                            &value.parameters,
                            value.return_type.as_ref(),
                            false,
                            value.is_async,
                            None,
                        )
                    }
                    _ => continue,
                };
                let Some(symbol) = symbol else { continue };
                if is_extern {
                    self.extern_symbols.insert(symbol);
                }
                if is_async {
                    if is_extern {
                        self.async_externs.insert(symbol);
                    } else {
                        self.async_symbols.insert(symbol);
                    }
                }
                if let Some((abi, abi_span)) = abi
                    && abi != "C"
                {
                    self.error(
                        codes::E8005,
                        "unsupported FFI ABI",
                        abi_span,
                        "FFI v1 only supports the `C` ABI",
                    );
                }
                let (parameters, variadic) = self.resolve_parameters(module, parameters, is_extern);
                if is_async && variadic.is_some() {
                    self.error(
                        codes::E7013,
                        "invalid variadic parameter",
                        self.resolution.symbols[symbol.0].span,
                        "async functions cannot be variadic",
                    );
                }
                let result = self.resolve_result(module, symbol, result, is_extern);
                self.signatures
                    .insert(symbol, Signature { parameters, result });
                if let Some(element) = variadic {
                    self.variadic_functions.insert(symbol, element);
                }
            }
        }
    }

    /// Resolves declared parameter types, enforcing variadic placement rules and
    /// returning the trailing variadic element type when present.
    fn resolve_parameters(
        &mut self,
        module: &Module,
        parameters: &[vut_ast::Parameter],
        is_extern: bool,
    ) -> (Vec<(String, TypeId)>, Option<TypeId>) {
        let mut resolved = Vec::new();
        let mut variadic = None;
        let last_index = parameters.len().saturating_sub(1);
        for (index, parameter) in parameters.iter().enumerate() {
            let ty = self.resolve_type(module, &parameter.ty);
            if is_extern && !self.ffi_boundary_safe(ty, false) {
                self.ffi_error(
                    "parameter is not FFI-safe",
                    parameter.name.span,
                    "extern signatures may only use C ABI-safe scalar, pointer, callback, or repr(C) data types",
                    ty,
                );
            }
            if parameter.variadic {
                if index != last_index {
                    self.error(
                        codes::E7013,
                        "invalid variadic parameter",
                        parameter.span,
                        "a variadic parameter must be the last parameter",
                    );
                } else if is_extern {
                    self.error(
                        codes::E7013,
                        "invalid variadic parameter",
                        parameter.span,
                        "extern functions cannot be variadic",
                    );
                } else {
                    variadic = Some(ty);
                }
            } else {
                resolved.push((parameter.name.text.clone(), ty));
            }
        }
        (resolved, variadic)
    }

    /// Resolves a declared return type, checking FFI safety for extern functions.
    fn resolve_result(
        &mut self,
        module: &Module,
        symbol: SymbolId,
        result: Option<&TypeExpr>,
        is_extern: bool,
    ) -> TypeId {
        let Some(ty) = result else {
            return self.intern(Type::Void);
        };
        let ty = self.resolve_type(module, ty);
        if is_extern && !self.ffi_boundary_safe(ty, true) {
            self.ffi_error(
                "return type is not FFI-safe",
                self.resolution.symbols[symbol.0].span,
                "extern signatures may only return C ABI-safe scalar, pointer, callback, repr(C) data, or void types",
                ty,
            );
        }
        ty
    }
    pub(super) fn collect_interfaces(&mut self) {
        let mut declarations = HashMap::new();
        for module in &self.resolution.modules {
            for item in &module.file.items {
                if let Item::Interface(value) = item
                    && let Some(symbol) = module.symbols.get(&value.name.text).copied()
                {
                    declarations.insert(symbol, (module.id, value.clone()));
                }
            }
        }
        let mut visiting = Vec::new();
        let mut complete = HashSet::new();
        let symbols: Vec<_> = declarations.keys().copied().collect();
        for symbol in symbols {
            self.build_interface_shape(symbol, &declarations, &mut visiting, &mut complete);
        }
    }
    pub(super) fn build_interface_shape(
        &mut self,
        symbol: SymbolId,
        declarations: &HashMap<SymbolId, (ModuleId, vut_ast::Interface)>,
        visiting: &mut Vec<SymbolId>,
        complete: &mut HashSet<SymbolId>,
    ) {
        if complete.contains(&symbol) {
            return;
        }
        if visiting.contains(&symbol) {
            let span = self.resolution.symbols[symbol.0].span;
            self.error(
                codes::E4011,
                "interface composition cycle",
                span,
                "interface inheritance forms a cycle",
            );
            return;
        }
        let Some((module_id, declaration)) = declarations.get(&symbol).cloned() else {
            return;
        };
        visiting.push(symbol);
        let module = &self.resolution.modules[module_id.0];
        let mut shape = InterfaceShape::default();
        for parent in &declaration.parents {
            let Some(parent_symbol) = module.symbols.get(&parent.text).copied() else {
                continue;
            };
            self.build_interface_shape(parent_symbol, declarations, visiting, complete);
            if let Some(parent_shape) = self.interface_shapes.get(&parent_symbol).cloned() {
                for (name, requirement) in parent_shape.methods {
                    vut_interface::merge_requirement(
                        &mut self.diagnostics,
                        &mut shape,
                        name,
                        requirement,
                        declaration.span,
                    );
                }
            }
        }
        for method in &declaration.methods {
            let parameters = method
                .parameters
                .iter()
                .map(|p| self.resolve_type(module, &p.ty))
                .collect();
            self.self_type_allowed = true;
            let result = if let Some(ty) = &method.return_type {
                self.resolve_type(module, ty)
            } else {
                self.intern(Type::Void)
            };
            self.self_type_allowed = false;
            vut_interface::merge_requirement(
                &mut self.diagnostics,
                &mut shape,
                method.name.text.clone(),
                InterfaceMethod {
                    parameters,
                    result,
                    is_static: method.is_static,
                    span: method.span,
                },
                declaration.span,
            );
        }
        visiting.pop();
        complete.insert(symbol);
        self.interface_shapes.insert(symbol, shape);
    }
}

/// Depth-first cycle detection over the enum value-reference graph.
fn visit_enums(
    node: SymbolId,
    graph: &HashMap<SymbolId, Vec<SymbolId>>,
    state: &mut HashMap<SymbolId, u8>,
    stack: &mut Vec<SymbolId>,
    in_cycle: &mut Vec<SymbolId>,
) {
    state.insert(node, 1);
    stack.push(node);
    if let Some(edges) = graph.get(&node) {
        for &next in edges {
            match state.get(&next).copied().unwrap_or(0) {
                0 => visit_enums(next, graph, state, stack, in_cycle),
                1 => {
                    if let Some(position) = stack.iter().position(|&item| item == next) {
                        for &member in &stack[position..] {
                            if !in_cycle.contains(&member) {
                                in_cycle.push(member);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    stack.pop();
    state.insert(node, 2);
}
