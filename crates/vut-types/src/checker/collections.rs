//! Compiler-known higher-order `list(T)` builtins.
//!
//! These operations are *builtins* (the compiler knows their signatures) but
//! they are not native runtime functions: the MIR lowering expands them into
//! ordinary loops that invoke a normal Vut function value. No callback ever
//! crosses the runtime ABI.
use super::context::Context;
#[allow(clippy::wildcard_imports)]
use super::*;

/// A type-checked callback argument of a collection builtin.
struct Callback {
    result: TypeId,
}

impl Analyzer<'_> {
    /// Types a higher-order `list(T)` builtin and records it for MIR lowering.
    /// Returns `None` when `name` is not one of these builtins, so ordinary
    /// builtin/method resolution can continue.
    #[expect(
        clippy::too_many_lines,
        reason = "collection builtin typing is intentionally centralized"
    )]
    pub(super) fn try_list_higher_order(
        &mut self,
        module: &Module,
        element: TypeId,
        name: &str,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        match name {
            "fold" => {
                if arguments.len() != 2
                    || arguments[0].name.is_some()
                    || arguments[1].name.is_some()
                {
                    self.error(
                        codes::E1003,
                        "invalid fold call",
                        span,
                        "`fold` takes an initial accumulator and a combiner callback",
                    );
                    return Some(self.intern(Type::Error));
                }
                let accumulator = self.expr(module, &arguments[0].value, context, None);
                let callback = self.collection_callback(
                    module,
                    &arguments[1],
                    &[accumulator, element],
                    Some(accumulator),
                    name,
                    context,
                );
                let Some(callback) = callback else {
                    return Some(self.intern(Type::Error));
                };
                self.compatible(
                    callback.result,
                    accumulator,
                    arguments[1].value.span(),
                    codes::E1014,
                    "the combiner must return the accumulator type",
                );
                self.builtin_calls.insert(span, BuiltinFunction::ListFold);
                Some(accumulator)
            }
            "sort_by" => {
                if arguments.len() != 1 || arguments[0].name.is_some() {
                    self.error(
                        codes::E1003,
                        "invalid sort_by call",
                        span,
                        "`sort_by` takes a single comparison callback",
                    );
                    return Some(self.intern(Type::Error));
                }
                let int_ty = self.intern(Type::Int);
                let callback = self.collection_callback(
                    module,
                    &arguments[0],
                    &[element, element],
                    Some(int_ty),
                    name,
                    context,
                );
                let Some(callback) = callback else {
                    return Some(self.intern(Type::Error));
                };
                self.compatible(
                    callback.result,
                    int_ty,
                    arguments[0].value.span(),
                    codes::E1014,
                    "the comparison callback must return `int`",
                );
                self.builtin_calls.insert(span, BuiltinFunction::ListSortBy);
                Some(self.intern(Type::Void))
            }
            "map" | "filter" | "any" | "all" | "find_index" => {
                let builtin = match name {
                    "map" => BuiltinFunction::ListMap,
                    "filter" => BuiltinFunction::ListFilter,
                    "any" => BuiltinFunction::ListAny,
                    "all" => BuiltinFunction::ListAll,
                    _ => BuiltinFunction::ListFindIndexBy,
                };
                if arguments.len() != 1 || arguments[0].name.is_some() {
                    self.error(
                        codes::E1003,
                        "invalid collection callback call",
                        span,
                        &format!("`{name}` takes a single positional callback argument"),
                    );
                    return Some(self.intern(Type::Error));
                }
                let expects_bool = builtin != BuiltinFunction::ListMap;
                // For `map` the callback result is inferred; the placeholder is
                // only the contextual result and never becomes the element type.
                let expected_result = expects_bool.then(|| self.intern(Type::Bool));
                let callback = self.collection_callback(
                    module,
                    &arguments[0],
                    &[element],
                    expected_result,
                    name,
                    context,
                );
                let Some(callback) = callback else {
                    return Some(self.intern(Type::Error));
                };
                if expects_bool {
                    let bool_ty = self.intern(Type::Bool);
                    self.compatible(
                        callback.result,
                        bool_ty,
                        arguments[0].value.span(),
                        codes::E1014,
                        "the predicate must return `bool`",
                    );
                }
                let result = match builtin {
                    BuiltinFunction::ListMap => self.intern(Type::List(callback.result)),
                    BuiltinFunction::ListFilter => self.intern(Type::List(element)),
                    BuiltinFunction::ListAny | BuiltinFunction::ListAll => self.intern(Type::Bool),
                    _ => self.intern(Type::Int),
                };
                self.builtin_calls.insert(span, builtin);
                Some(result)
            }
            _ => None,
        }
    }

    /// Type-checks a callback argument against the expected parameter types,
    /// using the expected callable for lambda parameter inference.
    fn collection_callback(
        &mut self,
        module: &Module,
        argument: &vut_ast::Argument,
        expected_parameters: &[TypeId],
        expected_result: Option<TypeId>,
        name: &str,
        context: &mut Context,
    ) -> Option<Callback> {
        let result_placeholder = expected_result.unwrap_or_else(|| self.intern(Type::Error));
        let expected = self.intern(Type::Callable {
            receiver: None,
            parameters: expected_parameters.to_owned(),
            result: result_placeholder,
        });
        let actual = self.expr(module, &argument.value, context, Some(expected));
        let Some((receiver, parameters, result)) = self.callable_shape(actual) else {
            self.error(
                codes::E1014,
                "invalid collection callback",
                argument.value.span(),
                &format!("`{name}` requires a function value such as a named function or a lambda"),
            );
            return None;
        };
        if receiver.is_some() {
            self.error(
                codes::E1014,
                "invalid collection callback",
                argument.value.span(),
                "receiver functions cannot be passed as collection callbacks",
            );
            return None;
        }
        if parameters.len() != expected_parameters.len() {
            self.error(
                codes::E1014,
                "callback parameter count mismatch",
                argument.value.span(),
                &format!("`{name}` expects a callback matching the collection signature"),
            );
            return None;
        }
        for (actual, expected) in parameters.iter().zip(expected_parameters.iter()) {
            self.compatible(
                *actual,
                *expected,
                argument.value.span(),
                codes::E1014,
                "callback parameter type mismatch",
            );
        }
        Some(Callback { result })
    }
}
