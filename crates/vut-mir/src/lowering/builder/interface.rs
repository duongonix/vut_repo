//! Interface/`dyn` value construction lowering.
use vut_hir::TypeId;
use vut_resolver::SymbolId;
use vut_types::Type;

use super::super::{Instruction, ValueId};
use super::Builder;

impl Builder<'_> {
    /// Boxes `value` when a concrete value is used where an `interface` or
    /// `dyn` value is expected. Non-conversions are returned unchanged.
    ///
    /// Interface targets accept structurally-satisfying `data`/`enum` values;
    /// `dyn` accepts any value with a native representation. `null` becomes the
    /// zero (absent) box pointer.
    pub(super) fn coerce_interface(
        &mut self,
        value: ValueId,
        actual: TypeId,
        expected: TypeId,
    ) -> ValueId {
        if actual == expected {
            return value;
        }
        let interface = match self.semantics.types[expected.0] {
            Type::Interface(symbol) => Some(symbol),
            Type::Dyn => None,
            _ => return value,
        };
        if matches!(self.semantics.types[actual.0], Type::Null) {
            let absent = self.value();
            self.emit(Instruction::ConstNull {
                value: absent,
                ty: Some(expected),
            });
            return absent;
        }
        if !self.interface_conversion_allowed(actual, interface) {
            return value;
        }
        let result = self.value();
        self.emit(Instruction::ConstructInterface {
            value: result,
            ty: expected,
            concrete_ty: actual,
            interface,
            source: value,
        });
        result
    }

    fn interface_conversion_allowed(&self, actual: TypeId, interface: Option<SymbolId>) -> bool {
        if let Some(symbol) = interface {
            // Structural interface target: only concrete data/enum declarations
            // that were proven to satisfy the interface.
            matches!(
                self.semantics.types[actual.0],
                Type::Data(_) | Type::Enum(_)
            ) && self
                .semantics
                .interface_satisfaction
                .contains_key(&(actual, symbol))
        } else {
            // `dyn` target: any value with a runtime representation.
            !matches!(
                self.semantics.types[actual.0],
                Type::Void
                    | Type::Error
                    | Type::Param(_)
                    | Type::Applied(_, _)
                    | Type::SelfType
                    | Type::Variadic(_)
            )
        }
    }
}
