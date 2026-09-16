//! Interface/`dyn` value construction lowering.
use vut_hir::TypeId;
use vut_types::Type;

use super::super::{Instruction, ValueId};
use super::Builder;

impl Builder<'_> {
    /// Boxes `value` when a concrete `data`/`enum` is used where an `interface`
    /// or `dyn` value is expected. Non-conversions are returned unchanged.
    pub(super) fn coerce_interface(
        &mut self,
        value: ValueId,
        actual: TypeId,
        expected: TypeId,
    ) -> ValueId {
        let interface = match self.semantics.types[expected.0] {
            Type::Interface(symbol) => Some(symbol),
            Type::Dyn => None,
            _ => return value,
        };
        let (Type::Data(concrete) | Type::Enum(concrete)) = self.semantics.types[actual.0] else {
            return value;
        };
        if let Some(symbol) = interface
            && !self
                .semantics
                .interface_satisfaction
                .contains_key(&(actual, symbol))
        {
            return value;
        }
        let result = self.value();
        self.emit(Instruction::ConstructInterface {
            value: result,
            ty: expected,
            concrete_ty: actual,
            concrete,
            interface,
            source: value,
        });
        result
    }
}
