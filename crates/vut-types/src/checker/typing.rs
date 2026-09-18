//! Argument, field, compatibility, and FFI-safe typing rules.
use super::context::Context;
#[allow(clippy::wildcard_imports)]
use super::*;

impl Analyzer<'_> {
    pub(super) fn validate_map_key(&mut self, key: TypeId, span: Span) {
        if self.supports_hash(key) {
            return;
        }
        self.error(
            codes::E1009,
            "invalid map key type",
            span,
            "map keys must support stable equality and hashing",
        );
    }

    pub(super) fn supports_hash(&self, ty: TypeId) -> bool {
        matches!(
            self.types[ty.0],
            Type::Bool | Type::Int | Type::Float | Type::Numeric(_) | Type::Str
        )
    }

    pub(super) fn arguments(
        &mut self,
        module: &Module,
        signature: &Signature,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
        borrow_resources: bool,
    ) {
        let mut used = HashSet::new();
        for (index, arg) in arguments.iter().enumerate() {
            if arg.spread {
                self.error(
                    codes::E7013,
                    "unexpected spread argument",
                    arg.span,
                    "spread arguments require a variadic parameter",
                );
                self.expr(module, &arg.value, context, None);
                continue;
            }
            let parameter = if let Some(name) = &arg.name {
                signature
                    .parameters
                    .iter()
                    .position(|(n, _)| n == &name.text)
            } else {
                Some(index)
            };
            let Some(index) = parameter.filter(|i| *i < signature.parameters.len()) else {
                self.error(
                    codes::E6003,
                    "invalid argument",
                    arg.span,
                    "unknown argument name or position",
                );
                continue;
            };
            if !used.insert(index) {
                self.error(
                    codes::E6004,
                    "duplicate argument",
                    arg.span,
                    "parameter supplied more than once",
                );
                continue;
            }
            let formal = signature.parameters[index].1;
            let actual = self.expr(module, &arg.value, context, Some(formal));
            // A native (`extern`) parameter of pointer type borrows a
            // `resource(T)` argument at the ABI boundary: the owner is not
            // moved, retained, or released. This is an ABI borrow, not a
            // general language coercion.
            if borrow_resources && arg.name.is_none() && self.resource_abi_borrow(formal, actual) {
                continue;
            }
            self.compatible(
                actual,
                formal,
                arg.span,
                codes::E1003,
                "argument type mismatch",
            );
        }
        if used.len() != signature.parameters.len() {
            self.error(
                codes::E6002,
                "invalid argument count",
                span,
                "not all required parameters were supplied",
            );
        }
    }

    /// Whether `actual` (`resource(T)`) may be borrowed by `formal` (`ptr(T)`)
    /// at an `extern` boundary. `ptr(void)` accepts any resource.
    fn resource_abi_borrow(&self, formal: TypeId, actual: TypeId) -> bool {
        let Type::Pointer(pointer) = self.types[formal.0] else {
            return false;
        };
        let Type::Resource(resource) = self.types[actual.0] else {
            return false;
        };
        matches!(self.types[pointer.0], Type::Void) || pointer == resource
    }
    pub(super) fn construct(
        &mut self,
        module: &Module,
        symbol: SymbolId,
        fields: &HashMap<String, Field>,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> TypeId {
        let mut used = HashSet::new();
        for arg in arguments {
            let Some(name) = &arg.name else {
                self.error(
                    codes::E6003,
                    "data fields must be named",
                    arg.span,
                    "use `field = value`",
                );
                continue;
            };
            let Some(field) = fields.get(&name.text) else {
                self.error(
                    codes::E2004,
                    "unknown field",
                    name.span,
                    "constructor has no such field",
                );
                continue;
            };
            if !used.insert(name.text.clone()) {
                self.error(
                    codes::E6004,
                    "duplicate field",
                    name.span,
                    "field initialized more than once",
                );
                continue;
            }
            let actual = self.expr(module, &arg.value, context, Some(field.ty));
            self.compatible(
                actual,
                field.ty,
                arg.span,
                codes::E1003,
                "field type mismatch",
            );
        }
        for (name, field) in fields {
            if field.required && !used.contains(name) {
                self.error(
                    codes::E6002,
                    "missing required field",
                    span,
                    &format!("field `{name}` is required"),
                );
            }
        }
        self.intern(Type::Data(symbol))
    }
    pub(super) fn field(
        &mut self,
        module: ModuleId,
        owner: TypeId,
        name: &vut_ast::Name,
        assigned: Option<TypeId>,
    ) -> TypeId {
        let (Type::Data(symbol) | Type::Enum(symbol)) = self.types[owner.0] else {
            self.error(
                codes::E2004,
                "unknown field",
                name.span,
                "value has no fields",
            );
            return self.intern(Type::Error);
        };
        if let Some(fields) = self.fields.get(&symbol)
            && let Some(field) = fields.get(&name.text).cloned()
        {
            if !field.public && self.symbol_module(symbol) != module {
                self.error(
                    codes::E2006,
                    "private field",
                    name.span,
                    "field is private to its module",
                );
            }
            if let Some(actual) = assigned {
                self.compatible(
                    actual,
                    field.ty,
                    name.span,
                    codes::E1003,
                    "field assignment type mismatch",
                );
            }
            return field.ty;
        }
        if let Type::Enum(id) = self.types[owner.0]
            && self
                .enum_variants
                .get(&id)
                .is_some_and(|v| v.iter().any(|variant| variant.name == name.text))
        {
            return owner;
        }
        self.error(
            codes::E2004,
            "unknown field",
            name.span,
            "type has no such field or variant",
        );
        self.intern(Type::Error)
    }
    pub(super) fn enum_variants(&self, symbol: SymbolId) -> Option<&Vec<VariantInfo>> {
        self.enum_variants.get(&symbol)
    }
    pub(super) fn variant_index(&self, symbol: SymbolId, name: &str) -> Option<usize> {
        self.enum_variants
            .get(&symbol)
            .and_then(|variants| variants.iter().position(|variant| variant.name == name))
    }
    /// The variants of an enum-typed subject, if it is an enum.
    pub(super) fn enum_variants_for(&self, subject: TypeId) -> Option<&Vec<VariantInfo>> {
        match self.types[subject.0] {
            Type::Enum(symbol) => self.enum_variants.get(&symbol),
            _ => None,
        }
    }
    /// Resolves a bare identifier pattern to an enum variant index when the
    /// subject is an enum that has a variant with that name.
    pub(super) fn bare_variant(&self, subject: TypeId, name: &str) -> Option<usize> {
        self.enum_variants_for(subject)
            .and_then(|variants| variants.iter().position(|variant| variant.name == name))
    }
    pub(super) fn condition(
        &mut self,
        module: &Module,
        expr: &Expr,
        context: &mut Context,
        code: vut_diagnostics::DiagnosticCode,
    ) {
        let actual = self.expr(module, expr, context, None);
        let expected = self.intern(Type::Bool);
        self.compatible(
            actual,
            expected,
            expr.span(),
            code,
            "condition must be bool",
        );
    }
    pub(super) fn is_numeric(&self, id: TypeId) -> bool {
        matches!(self.types[id.0], Type::Int | Type::Float | Type::Numeric(_))
    }
    pub(super) fn integer_fits(&self, text: &str, id: TypeId) -> bool {
        let Ok(value) = text.replace('_', "").parse::<u128>() else {
            return false;
        };
        match &self.types[id.0] {
            Type::Numeric(name) => match name.as_str() {
                "i8" => value <= i8::MAX as u128,
                "i16" => value <= i16::MAX as u128,
                "i32" => value <= i32::MAX as u128,
                "i64" => value <= i64::MAX as u128,
                "u8" => value <= u128::from(u8::MAX),
                "u16" => value <= u128::from(u16::MAX),
                "u32" => value <= u128::from(u32::MAX),
                "u64" => value <= u128::from(u64::MAX),
                _ => true,
            },
            _ => true,
        }
    }
    pub(super) fn is_integer(&self, id: TypeId) -> bool {
        match &self.types[id.0] {
            Type::Int => true,
            Type::Numeric(name) => !name.starts_with('f'),
            _ => false,
        }
    }
    pub(super) fn is_float(&self, id: TypeId) -> bool {
        match &self.types[id.0] {
            Type::Float => true,
            Type::Numeric(name) => name.starts_with('f'),
            _ => false,
        }
    }
    pub(super) fn ffi_error(&mut self, title: &str, span: Span, message: &str, ty: TypeId) {
        let mut diagnostic = Diagnostic::error(codes::E8004, title, span, message);
        if let Some(help) = self.ffi_safety_help(ty) {
            diagnostic = diagnostic.with_help(help);
        }
        self.diagnostics.push(diagnostic);
    }
    pub(super) fn ffi_safety_help(&self, ty: TypeId) -> Option<String> {
        if let Type::Data(symbol) = self.types[ty.0] {
            let name = &self.resolution.symbols[symbol.0].name;
            if self.attributes.data_repr.contains_key(&symbol) {
                return Some(format!(
                    "pass `ptr({name})`; FFI v1 passes repr(C) data by pointer only"
                ));
            }
            if !self.opaque_data.contains(&symbol) {
                return Some(format!(
                    "add `@repr(C)` to `data {name}` if its fields are FFI-safe"
                ));
            }
        }
        None
    }
    /// FFI boundary safety: like [`Self::ffi_safe`] but rejects by-value
    /// `data` values. FFI v1 passes aggregate data through `ptr(T)` only.
    pub(super) fn ffi_boundary_safe(&self, id: TypeId, allow_void: bool) -> bool {
        match &self.types[id.0] {
            // Aggregates stay pointer-only in FFI v1.
            Type::Data(_) => false,
            // Official-runtime handle ABI: `str`/`bytes` cross the boundary as a
            // single runtime-owned handle pointer. Parameters are borrowed for
            // the duration of the call; returned handles transfer ownership to
            // Vut. Third-party C code must not use this representation. An owned
            // native resource handle crosses the same way and keeps a single
            // owner, dropped deterministically by Vut.
            Type::Str | Type::Bytes | Type::Resource(_) => true,
            Type::List(inner) => self.ffi_boundary_safe(*inner, false),
            _ => self.ffi_safe(id, allow_void),
        }
    }
    pub(super) fn ffi_safe(&self, id: TypeId, allow_void: bool) -> bool {
        match &self.types[id.0] {
            Type::Void => allow_void,
            Type::Numeric(_) | Type::Resource(_) => true,
            Type::Pointer(inner) => match self.types[inner.0] {
                Type::Void | Type::Numeric(_) | Type::Pointer(_) | Type::FunctionPointer { .. } => {
                    true
                }
                Type::Data(symbol) => {
                    self.opaque_data.contains(&symbol)
                        || self.attributes.data_repr.contains_key(&symbol)
                }
                _ => self.ffi_safe(*inner, false),
            },
            Type::Data(symbol) => {
                !self.opaque_data.contains(symbol) && self.attributes.data_repr.contains_key(symbol)
            }
            Type::FunctionPointer {
                abi,
                parameters,
                result,
            } => {
                abi == "C"
                    && parameters
                        .iter()
                        .all(|parameter| self.ffi_boundary_safe(*parameter, false))
                    && self.ffi_boundary_safe(*result, true)
            }
            _ => false,
        }
    }
    pub(super) fn is_compatible(&mut self, actual: TypeId, expected: TypeId) -> bool {
        if actual == expected
            || matches!(self.types[expected.0], Type::Dyn)
            || matches!(self.types[actual.0], Type::Error)
        {
            return true;
        }
        match (&self.types[actual.0], &self.types[expected.0]) {
            (Type::Null, Type::Optional(_)) => true,
            (actual_ty, Type::Optional(inner)) => actual_ty == &self.types[inner.0],
            (_, Type::Interface(interface)) => {
                let interface = *interface;
                self.interface_satisfaction(actual, interface) == Satisfaction::Satisfied
            }
            (Type::List(actual), Type::List(expected)) => {
                let (actual, expected) = (*actual, *expected);
                self.is_compatible(actual, expected)
            }
            (Type::Result(actual_ok, actual_err), Type::Result(expected_ok, expected_err)) => {
                let (actual_ok, actual_err, expected_ok, expected_err) =
                    (*actual_ok, *actual_err, *expected_ok, *expected_err);
                self.is_compatible(actual_ok, expected_ok)
                    && self.is_compatible(actual_err, expected_err)
            }
            (
                Type::Function(symbol),
                Type::FunctionPointer {
                    abi,
                    parameters,
                    result,
                },
            ) => {
                let symbol = *symbol;
                let abi = abi.clone();
                let parameters = parameters.clone();
                let result = *result;
                abi == "C"
                    && self
                        .signatures
                        .get(&symbol)
                        .cloned()
                        .is_some_and(|signature| {
                            signature.parameters.len() == parameters.len()
                                && signature
                                    .parameters
                                    .iter()
                                    .map(|(_, ty)| *ty)
                                    .zip(parameters.iter().copied())
                                    .all(|(actual, expected)| self.is_compatible(actual, expected))
                                && self.is_compatible(signature.result, result)
                        })
            }
            (Type::Function(_), Type::Callable { .. })
            | (Type::Callable { .. }, Type::Callable { .. }) => {
                self.callable_compatible(actual, expected)
            }
            _ => false,
        }
    }

    /// Returns the receiver, parameter types, and result of a callable type, or
    /// `None` when the type is not callable.
    pub(super) fn callable_shape(
        &mut self,
        ty: TypeId,
    ) -> Option<(Option<TypeId>, Vec<TypeId>, TypeId)> {
        match self.types[ty.0].clone() {
            Type::Function(symbol) => {
                let receiver = self.receiver_functions.get(&symbol).copied();
                self.signatures.get(&symbol).cloned().map(|signature| {
                    (
                        receiver,
                        signature.parameters.into_iter().map(|(_, ty)| ty).collect(),
                        signature.result,
                    )
                })
            }
            Type::Callable {
                receiver,
                parameters,
                result,
            } => Some((receiver, parameters, result)),
            _ => None,
        }
    }

    /// Compares two callable types, including their receiver, parameter, and
    /// result compatibility.
    fn callable_compatible(&mut self, actual: TypeId, expected: TypeId) -> bool {
        let Some((actual_receiver, actual_parameters, actual_result)) = self.callable_shape(actual)
        else {
            return false;
        };
        let Some((expected_receiver, expected_parameters, expected_result)) =
            self.callable_shape(expected)
        else {
            return false;
        };
        self.callable_receivers_match(actual_receiver, expected_receiver)
            && actual_parameters.len() == expected_parameters.len()
            && actual_parameters
                .iter()
                .copied()
                .zip(expected_parameters.iter().copied())
                .all(|(actual, expected)| self.is_compatible(actual, expected))
            && self.is_compatible(actual_result, expected_result)
    }

    /// A receiver function is compatible only with a function type that carries
    /// the same receiver. Receiver and ordinary parameter positions differ.
    fn callable_receivers_match(
        &mut self,
        actual: Option<TypeId>,
        expected: Option<TypeId>,
    ) -> bool {
        match (actual, expected) {
            (None, None) => true,
            (Some(actual), Some(expected)) => self.is_compatible(actual, expected),
            _ => false,
        }
    }
    pub(super) fn interface_satisfaction(
        &mut self,
        actual: TypeId,
        interface: SymbolId,
    ) -> Satisfaction {
        if let Some(result) = self.satisfaction.get(&(actual, interface)) {
            return result.clone();
        }
        let Some(required) = self.interface_shapes.get(&interface).cloned() else {
            return Satisfaction::Missing {
                method: "<invalid interface>".into(),
            };
        };
        let result = if let Type::Interface(source) = self.types[actual.0] {
            let exposed = self
                .interface_shapes
                .get(&source)
                .cloned()
                .unwrap_or_default();
            let mut outcome = Satisfaction::Satisfied;
            for (name, wanted) in &required.methods {
                match exposed.methods.get(name) {
                    None => {
                        outcome = Satisfaction::Missing {
                            method: name.clone(),
                        };
                        break;
                    }
                    Some(found)
                        if found.parameters != wanted.parameters
                            || found.result != wanted.result
                            || found.is_static != wanted.is_static =>
                    {
                        outcome = Satisfaction::Mismatch {
                            method: name.clone(),
                        };
                        break;
                    }
                    Some(_) => {}
                }
            }
            outcome
        } else if let Type::Data(receiver) | Type::Enum(receiver) = self.types[actual.0] {
            self.nominal_satisfaction(actual, receiver, &required)
        } else {
            required
                .methods
                .keys()
                .next()
                .map_or(Satisfaction::Satisfied, |method| Satisfaction::Missing {
                    method: method.clone(),
                })
        };
        self.satisfaction
            .insert((actual, interface), result.clone());
        result
    }

    /// Checks a concrete data/enum type against an interface requirement,
    /// substituting the requirement's `Self` with the concrete type.
    fn nominal_satisfaction(
        &mut self,
        actual: TypeId,
        receiver: SymbolId,
        required: &InterfaceShape,
    ) -> Satisfaction {
        let receiver_module = self.symbol_module(receiver);
        for (name, wanted) in &required.methods {
            let method = self.resolution.modules[receiver_module.0]
                .methods
                .get(&MethodKey {
                    receiver,
                    name: name.clone(),
                })
                .copied();
            let Some(method) = method else {
                return Satisfaction::Missing {
                    method: name.clone(),
                };
            };
            if !self.resolution.symbols[method.0].public {
                return Satisfaction::Private {
                    method: name.clone(),
                };
            }
            if self.resolution.symbols[method.0].is_static != wanted.is_static {
                return Satisfaction::Mismatch {
                    method: name.clone(),
                };
            }
            let wanted_parameters: Vec<TypeId> = wanted
                .parameters
                .iter()
                .map(|ty| self.substitute_self(*ty, actual))
                .collect();
            let wanted_result = self.substitute_self(wanted.result, actual);
            match self.signatures.get(&method) {
                Some(found)
                    if found
                        .parameters
                        .iter()
                        .map(|(_, ty)| *ty)
                        .eq(wanted_parameters)
                        && found.result == wanted_result => {}
                _ => {
                    return Satisfaction::Mismatch {
                        method: name.clone(),
                    };
                }
            }
        }
        Satisfaction::Satisfied
    }

    pub(super) fn compatible(
        &mut self,
        actual: TypeId,
        expected: TypeId,
        span: Span,
        code: vut_diagnostics::DiagnosticCode,
        title: &str,
    ) {
        if !self.is_compatible(actual, expected) {
            if let Type::Interface(interface) = self.types[expected.0]
                && let Some(failure) = self.satisfaction.get(&(actual, interface)).cloned()
            {
                let (code, message) = match failure {
                    Satisfaction::Missing { method } => (
                        codes::E4102,
                        format!("missing public method `{method}` required by interface"),
                    ),
                    Satisfaction::Mismatch { method } => (
                        codes::E4103,
                        format!("method `{method}` has an incompatible signature"),
                    ),
                    Satisfaction::Private { method } => (
                        codes::E4104,
                        format!("method `{method}` is private and cannot satisfy an interface"),
                    ),
                    Satisfaction::Satisfied => unreachable!(),
                };
                self.diagnostics.push(
                    Diagnostic::error(code, "interface is not satisfied", span, message)
                        .with_related(
                            self.resolution.symbols[interface.0].span,
                            "required interface declared here",
                        ),
                );
                return;
            }
            let found = format!("{:?}", self.types[actual.0]);
            let wanted = format!("{:?}", self.types[expected.0]);
            self.diagnostics.push(
                Diagnostic::error(code, title, span, "incompatible static types")
                    .with_expected_found(wanted, found),
            );
        }
    }
    pub(super) fn error(
        &mut self,
        code: vut_diagnostics::DiagnosticCode,
        title: &str,
        span: Span,
        message: &str,
    ) {
        self.diagnostics
            .push(Diagnostic::error(code, title, span, message));
    }
    pub(super) fn warn(
        &mut self,
        code: vut_diagnostics::DiagnosticCode,
        title: &str,
        span: Span,
        message: &str,
    ) {
        self.diagnostics
            .push(Diagnostic::warning(code, title, span, message));
    }
}
