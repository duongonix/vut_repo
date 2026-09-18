//! Low-level MIR emission primitives shared by the collection builtins.
//!
//! These compose the existing instruction set; they are not new semantics. The
//! higher-order lowerings (`mod` and `sort`) build their loops from them.
use vut_hir::TypeId;
use vut_types::{BuiltinFunction, Type};

use super::super::super::{BinaryOp, Instruction, LocalId, ValueId};
use super::super::Builder;

impl Builder<'_> {
    /// The interned `int` type, if any.
    pub(super) fn int_type(&self) -> Option<TypeId> {
        self.semantics
            .types
            .iter()
            .position(|ty| matches!(ty, Type::Int))
            .map(TypeId)
    }

    /// The interned `bool` type, if any.
    pub(super) fn bool_type(&self) -> Option<TypeId> {
        self.semantics
            .types
            .iter()
            .position(|ty| matches!(ty, Type::Bool))
            .map(TypeId)
    }

    /// The interned `list(element)` type, if any.
    pub(super) fn list_type(&self, element: TypeId) -> Option<TypeId> {
        self.semantics
            .types
            .iter()
            .position(|ty| matches!(ty, Type::List(inner) if *inner == element))
            .map(TypeId)
    }

    pub(super) fn int_const(&mut self, literal: i64) -> ValueId {
        let value = self.value();
        self.emit(Instruction::ConstInt { value, literal });
        value
    }

    /// Reads an integer (or boolean) local as an owned value.
    pub(super) fn read_int(&mut self, local: LocalId) -> ValueId {
        let value = self.value();
        self.emit(Instruction::Copy { value, local });
        value
    }

    /// Borrows a collection local without transferring ownership.
    pub(super) fn read_borrow(&mut self, local: LocalId) -> ValueId {
        let value = self.value();
        self.emit(Instruction::Borrow { value, local });
        value
    }

    pub(super) fn store(&mut self, local: LocalId, value: ValueId) {
        self.emit(Instruction::Store { local, value });
    }

    pub(super) fn int_binary(&mut self, op: BinaryOp, left: ValueId, right: ValueId) -> ValueId {
        let value = self.value();
        self.emit(Instruction::Binary {
            value,
            op,
            left,
            right,
        });
        value
    }

    /// `min(left, right)` for two integer values.
    pub(super) fn int_min(&mut self, left: ValueId, right: ValueId) -> ValueId {
        let value = self.value();
        self.emit(Instruction::RuntimeCall {
            value: Some(value),
            result_type: Some(self.int_type().expect("int is interned")),
            function: BuiltinFunction::IntMin,
            arguments: vec![left, right],
        });
        value
    }

    /// `receiver.len()` as an `int` value.
    pub(super) fn list_length(&mut self, receiver: ValueId) -> Option<ValueId> {
        let int_ty = self.int_type()?;
        let length = self.value();
        self.emit(Instruction::RuntimeCall {
            value: Some(length),
            result_type: Some(int_ty),
            function: BuiltinFunction::ListLen,
            arguments: vec![receiver],
        });
        Some(length)
    }

    /// Emits a `list(T)` allocation with the element layout and ownership
    /// callbacks plus an optional preallocated capacity.
    pub(super) fn emit_list_new(
        &mut self,
        list_ty: TypeId,
        element: TypeId,
        capacity: Option<ValueId>,
    ) -> ValueId {
        let size = self.value();
        self.emit(Instruction::ConstInt {
            value: size,
            literal: i64::try_from(self.layouts.element_storage_size(element)).unwrap_or(i64::MAX),
        });
        let align = self.value();
        self.emit(Instruction::ConstInt {
            value: align,
            literal: i64::try_from(self.layouts.element_storage_align(element)).unwrap_or(i64::MAX),
        });
        let retain = self.value();
        self.emit(Instruction::TypeRetain {
            value: retain,
            ty: element,
        });
        let release = self.value();
        self.emit(Instruction::TypeRelease {
            value: release,
            ty: element,
        });
        let capacity = capacity.unwrap_or_else(|| {
            let value = self.value();
            self.emit(Instruction::ConstInt { value, literal: 0 });
            value
        });
        let value = self.value();
        self.emit(Instruction::RuntimeCall {
            value: Some(value),
            result_type: Some(list_ty),
            function: BuiltinFunction::ListNew,
            arguments: vec![size, align, retain, release, capacity],
        });
        value
    }

    /// Copies the element at `index` out of `list`. The result is an owned
    /// reference (the runtime retains it).
    pub(super) fn list_at(&mut self, list: ValueId, index: ValueId, element: TypeId) -> ValueId {
        let value = self.value();
        self.emit(Instruction::RuntimeCall {
            value: Some(value),
            result_type: Some(element),
            function: BuiltinFunction::ListAt,
            arguments: vec![list, index],
        });
        value
    }

    /// Stores `element` at `index` in `list`.
    pub(super) fn list_set(&mut self, list: ValueId, index: ValueId, element: ValueId) {
        self.emit(Instruction::RuntimeCall {
            value: None,
            result_type: None,
            function: BuiltinFunction::ListSet,
            arguments: vec![list, index, element],
        });
    }

    /// Appends `element` to `list`, retaining it.
    pub(super) fn list_push(&mut self, list: ValueId, element: ValueId) {
        self.emit(Instruction::RuntimeCall {
            value: None,
            result_type: None,
            function: BuiltinFunction::ListPush,
            arguments: vec![list, element],
        });
    }

    /// Releases an owned temporary when its type manages resources.
    pub(super) fn release_managed(&mut self, value: ValueId, ty: TypeId) {
        if self.layouts.types[ty.0].needs_drop {
            self.emit(Instruction::Release { value, ty });
        }
    }
}
