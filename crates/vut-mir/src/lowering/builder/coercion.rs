//! Typed value coercion and builtin argument storage types.
use super::{Builder, Instruction, Type, TypeId, ValueId};

impl Builder<'_> {
    /// Wraps `value` into an optional when `expected` is `T?` and `actual` is
    /// `T`. Managed handles wrap by identity; scalars are boxed.
    pub(super) fn coerce_optional(
        &mut self,
        value: ValueId,
        actual: Option<TypeId>,
        expected: Option<TypeId>,
    ) -> ValueId {
        let (Some(actual), Some(expected)) = (actual, expected) else {
            return value;
        };
        let Type::Optional(inner) = self.semantics.types[expected.0] else {
            return value;
        };
        // A `null` literal has the placeholder `Null` type: managed optionals
        // represent absence as the zero handle, but tagged optionals need a
        // zeroed aggregate block, so re-emit the constant with the optional type.
        if matches!(self.semantics.types[actual.0], Type::Null) {
            if !matches!(
                self.semantics.types[inner.0],
                Type::Str
                    | Type::Bytes
                    | Type::List(_)
                    | Type::Map(_, _)
                    | Type::Vutcon(_)
                    | Type::Future(_)
                    | Type::Resource(_)
                    | Type::Interface(_)
                    | Type::Dyn
            ) {
                let absent = self.value();
                self.emit(Instruction::ConstNull {
                    value: absent,
                    ty: Some(expected),
                });
                return absent;
            }
            return value;
        }
        if actual != inner {
            return value;
        }
        let wrapped = self.value();
        self.emit(Instruction::OptionalWrap {
            value: wrapped,
            operand: value,
            ty: expected,
            inner,
        });
        wrapped
    }
    /// Applies the full value coercion toward an expected type: interface/`dyn`
    /// boxing (`T` -> `interface`/`dyn`) followed by optional wrapping
    /// (`T` -> `T?`). Used wherever a value is stored into a typed destination.
    pub(in crate::lowering) fn coerce_value(
        &mut self,
        value: ValueId,
        actual: Option<TypeId>,
        expected: Option<TypeId>,
    ) -> ValueId {
        let (Some(actual), Some(expected)) = (actual, expected) else {
            return value;
        };
        let boxed = self.coerce_interface(value, actual, expected);
        self.coerce_optional(boxed, Some(actual), Some(expected))
    }
    /// Whether storing a value into `expected` requires boxing/wrapping rather
    /// than a plain bit copy.
    pub(super) fn value_needs_coercion(&self, expected: TypeId) -> bool {
        matches!(
            self.semantics.types[expected.0],
            Type::Dyn | Type::Interface(_) | Type::Optional(_)
        )
    }
    /// Expected concrete type for each non-receiver argument of a builtin call,
    /// for arguments that store the receiver's element/value type.
    pub(super) fn builtin_argument_types(
        &self,
        function: crate::BuiltinFunction,
        receiver_ty: Option<TypeId>,
    ) -> Vec<Option<TypeId>> {
        use crate::BuiltinFunction;
        let Some(receiver) = receiver_ty.and_then(|ty| self.semantics.types.get(ty.0).cloned())
        else {
            return Vec::new();
        };
        match receiver {
            Type::List(element) => match function {
                BuiltinFunction::ListPush | BuiltinFunction::ListContains => {
                    vec![Some(element)]
                }
                BuiltinFunction::ListInsert | BuiltinFunction::ListSet => {
                    vec![None, Some(element)]
                }
                _ => Vec::new(),
            },
            Type::Map(_, value) => match function {
                BuiltinFunction::MapSet | BuiltinFunction::MapGetOr => {
                    vec![None, Some(value)]
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }
}
