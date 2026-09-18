//! Inline MIR lowering for compiler-known higher-order `list` builtins.
//!
//! `map`, `filter`, `any`, `all`, `fold`, `find_index`, and `sort_by` are
//! *builtins*: the compiler knows their signatures, but they are not native
//! runtime functions. Each is expanded into ordinary loops that invoke a normal
//! Vut function value through `CallIndirect`, so no callback ever crosses the
//! runtime ABI. `sort_by` (an iterative stable merge sort) lives in `sort`.
use vut_ast::{Argument, Expr};
use vut_hir::TypeId;
use vut_source::Span;
use vut_types::{BuiltinFunction, Type};

use super::super::{BlockId, Instruction, Terminator, ValueId};
use super::Builder;

mod emit;
mod sort;

/// Whether `function` is a higher-order `list` builtin lowered inline in MIR.
pub(super) fn is_higher_order(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::ListMap
            | BuiltinFunction::ListFilter
            | BuiltinFunction::ListAny
            | BuiltinFunction::ListAll
            | BuiltinFunction::ListFold
            | BuiltinFunction::ListFindIndexBy
            | BuiltinFunction::ListSortBy
    )
}

/// The blocks and per-iteration values of a `list` iteration loop.
struct ListLoop {
    head: BlockId,
    done: BlockId,
    /// The borrowed element produced by `IteratorNext`.
    element: ValueId,
    /// The 0-based index produced by `IteratorNext`.
    index: ValueId,
}

impl Builder<'_> {
    /// Lowers a higher-order `list` builtin at a call site.
    pub(super) fn lower_higher_order_builtin(
        &mut self,
        function: BuiltinFunction,
        callee: &Expr,
        arguments: &[Argument],
        span: Span,
    ) -> Option<ValueId> {
        let Expr::Member { object, .. } = callee else {
            return None;
        };
        let receiver_ty = self
            .semantics
            .expression_types
            .get(&object.span())
            .copied()?;
        let Type::List(element) = self.semantics.types[receiver_ty.0] else {
            return None;
        };
        // A named receiver is borrowed; a temporary collection is owned here
        // and released when the loop exits.
        let mut operands = Vec::new();
        let mut owned = Vec::new();
        self.lower_builtin_value(object, &mut operands, &mut owned)?;
        let receiver = *operands.first()?;
        // `fold` takes `(initial, combine)` and `sort_by` takes `(compare)`;
        // every other builtin takes the callback first.
        let callback_argument = usize::from(function == BuiltinFunction::ListFold);
        let callback = self.expr(&arguments[callback_argument].value)?;
        let callback_ty = self
            .semantics
            .expression_types
            .get(&arguments[callback_argument].value.span())
            .copied();
        match function {
            BuiltinFunction::ListMap => {
                self.lower_list_map(receiver, element, callback, callback_ty, span, owned)
            }
            BuiltinFunction::ListFilter => {
                self.lower_list_filter(receiver, element, callback, callback_ty, span, owned)
            }
            BuiltinFunction::ListAny => {
                self.lower_list_any(receiver, element, callback, callback_ty, span, owned)
            }
            BuiltinFunction::ListAll => {
                self.lower_list_all(receiver, element, callback, callback_ty, span, owned)
            }
            BuiltinFunction::ListFold => self.lower_list_fold(
                receiver,
                element,
                callback,
                callback_ty,
                arguments,
                span,
                owned,
            ),
            BuiltinFunction::ListSortBy => {
                self.lower_list_sort_by(receiver, element, callback, callback_ty, span, owned)
            }
            _ => self.lower_list_find_index(receiver, element, callback, callback_ty, span, owned),
        }
    }

    /// `items.map(transform)` — builds `list(R)` preallocated to the input
    /// length, pushing `transform(item)` for every element.
    fn lower_list_map(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let result_ty = self.semantics.expression_types.get(&span).copied()?;
        let Type::List(result_element) = self.semantics.types[result_ty.0] else {
            return None;
        };
        let length = self.list_length(receiver)?;
        let result = self.emit_list_new(result_ty, result_element, Some(length));
        let loop_ = self.begin_list_loop(receiver, element);
        if self.layouts.types[element.0].needs_drop {
            self.emit(Instruction::Retain {
                value: loop_.element,
                ty: element,
            });
        }
        let mapped = self.value();
        self.emit(Instruction::CallIndirect {
            value: Some(mapped),
            result_type: Some(result_element),
            callable_ty: callback_ty,
            callee: callback,
            arguments: vec![loop_.element],
        });
        self.list_push(result, mapped);
        if self.layouts.types[result_element.0].needs_drop {
            self.emit(Instruction::Release {
                value: mapped,
                ty: result_element,
            });
        }
        self.terminate(Terminator::Jump(loop_.head));
        self.finish_loop(&loop_, owned);
        Some(result)
    }

    /// `items.filter(keep)` — builds `list(T)` with the elements the predicate
    /// accepts. The pushed element is borrowed and retained by `ListPush`.
    fn lower_list_filter(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let result_ty = self.semantics.expression_types.get(&span).copied()?;
        let bool_ty = self.bool_type()?;
        let length = self.list_length(receiver)?;
        let result = self.emit_list_new(result_ty, element, Some(length));
        let loop_ = self.begin_list_loop(receiver, element);
        if self.layouts.types[element.0].needs_drop {
            self.emit(Instruction::Retain {
                value: loop_.element,
                ty: element,
            });
        }
        let keep = self.value();
        self.emit(Instruction::CallIndirect {
            value: Some(keep),
            result_type: Some(bool_ty),
            callable_ty: callback_ty,
            callee: callback,
            arguments: vec![loop_.element],
        });
        let push = self.new_block();
        self.terminate(Terminator::Branch {
            condition: keep,
            then_block: push,
            else_block: loop_.head,
        });
        self.switch_to(push);
        self.list_push(result, loop_.element);
        self.terminate(Terminator::Jump(loop_.head));
        self.finish_loop(&loop_, owned);
        Some(result)
    }

    /// `items.any(predicate)` — short-circuits to `true` on the first match.
    fn lower_list_any(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let bool_ty = self.bool_type()?;
        let local = self.add_temp_local(bool_ty, span);
        self.store_bool(local, false);
        let loop_ = self.begin_list_loop(receiver, element);
        let condition = self.call_predicate(&loop_, element, callback, callback_ty, bool_ty);
        let hit = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block: hit,
            else_block: loop_.head,
        });
        self.switch_to(hit);
        self.store_bool(local, true);
        self.terminate(Terminator::Jump(loop_.done));
        self.finish_loop(&loop_, owned);
        Some(self.read_int(local))
    }

    /// `items.all(predicate)` — short-circuits to `false` on the first miss.
    fn lower_list_all(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let bool_ty = self.bool_type()?;
        let local = self.add_temp_local(bool_ty, span);
        self.store_bool(local, true);
        let loop_ = self.begin_list_loop(receiver, element);
        let condition = self.call_predicate(&loop_, element, callback, callback_ty, bool_ty);
        let miss = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block: loop_.head,
            else_block: miss,
        });
        self.switch_to(miss);
        self.store_bool(local, false);
        self.terminate(Terminator::Jump(loop_.done));
        self.finish_loop(&loop_, owned);
        Some(self.read_int(local))
    }

    /// `items.fold(initial, combine)` — threads an accumulator through the
    /// collection with no intermediate collection. The accumulator lives in a
    /// local because it is carried across loop iterations.
    #[expect(
        clippy::too_many_arguments,
        reason = "the lowering threads the receiver, callback, and argument list together"
    )]
    fn lower_list_fold(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        arguments: &[Argument],
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let accumulator_ty = self.semantics.expression_types.get(&span).copied()?;
        let initial = self.expr(&arguments[0].value)?;
        let local = self.add_temp_local(accumulator_ty, span);
        self.emit(Instruction::Store {
            local,
            value: initial,
        });
        self.initialized.insert(local);
        let loop_ = self.begin_list_loop(receiver, element);
        // The accumulator is moved out of the local into the call, which
        // consumes it; the returned value is stored back as the new owner.
        let accumulator = self.value();
        self.emit(Instruction::Move {
            value: accumulator,
            local,
        });
        self.moved.insert(local);
        if self.layouts.types[element.0].needs_drop {
            self.emit(Instruction::Retain {
                value: loop_.element,
                ty: element,
            });
        }
        let next = self.value();
        self.emit(Instruction::CallIndirect {
            value: Some(next),
            result_type: Some(accumulator_ty),
            callable_ty: callback_ty,
            callee: callback,
            arguments: vec![accumulator, loop_.element],
        });
        self.emit(Instruction::Store { local, value: next });
        self.moved.remove(&local);
        self.terminate(Terminator::Jump(loop_.head));
        self.finish_loop(&loop_, owned);
        let out = self.value();
        self.emit(Instruction::Move { value: out, local });
        self.moved.insert(local);
        Some(out)
    }

    /// `items.find_index(predicate)` — first matching index, or `-1`.
    fn lower_list_find_index(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let int_ty = self.int_type()?;
        let bool_ty = self.bool_type()?;
        let local = self.add_temp_local(int_ty, span);
        let missing = self.value();
        self.emit(Instruction::ConstInt {
            value: missing,
            literal: -1,
        });
        self.emit(Instruction::Store {
            local,
            value: missing,
        });
        let loop_ = self.begin_list_loop(receiver, element);
        let condition = self.call_predicate(&loop_, element, callback, callback_ty, bool_ty);
        let hit = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block: hit,
            else_block: loop_.head,
        });
        self.switch_to(hit);
        self.emit(Instruction::Store {
            local,
            value: loop_.index,
        });
        self.terminate(Terminator::Jump(loop_.done));
        self.finish_loop(&loop_, owned);
        Some(self.read_int(local))
    }

    /// Emits the loop header, the `IteratorNext` call, and switches to the body
    /// block. The caller terminates every body path with a jump back to `head`
    /// or to `done`.
    fn begin_list_loop(&mut self, receiver: ValueId, element: TypeId) -> ListLoop {
        let head = self.new_block();
        let body = self.new_block();
        let done = self.new_block();
        let iterator = self.value();
        self.emit(Instruction::IteratorInit {
            iterator,
            iterable: receiver,
            length: None,
            length_value: None,
            stride: Some(self.layouts.element_storage_size(element)),
            slot: None,
        });
        self.terminate(Terminator::Jump(head));
        self.switch_to(head);
        let has_value = self.value();
        let element_value = self.value();
        let index = self.value();
        self.emit(Instruction::IteratorNext {
            has_value,
            value: element_value,
            index,
            iterator,
            element_type: element,
        });
        self.terminate(Terminator::Branch {
            condition: has_value,
            then_block: body,
            else_block: done,
        });
        self.switch_to(body);
        ListLoop {
            head,
            done,
            element: element_value,
            index,
        }
    }

    /// Switches to the loop exit and releases any collection the expression
    /// owned (a temporary receiver).
    fn finish_loop(&mut self, loop_: &ListLoop, owned: Vec<(ValueId, TypeId)>) {
        self.switch_to(loop_.done);
        for (value, ty) in owned {
            self.emit(Instruction::Release { value, ty });
        }
    }

    /// Invokes the predicate with the borrowed element. Managed elements are
    /// retained first because the callee consumes its arguments.
    fn call_predicate(
        &mut self,
        loop_: &ListLoop,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        bool_ty: TypeId,
    ) -> ValueId {
        if self.layouts.types[element.0].needs_drop {
            self.emit(Instruction::Retain {
                value: loop_.element,
                ty: element,
            });
        }
        let condition = self.value();
        self.emit(Instruction::CallIndirect {
            value: Some(condition),
            result_type: Some(bool_ty),
            callable_ty: callback_ty,
            callee: callback,
            arguments: vec![loop_.element],
        });
        condition
    }

    fn store_bool(&mut self, local: super::LocalId, literal: bool) {
        let value = self.value();
        self.emit(Instruction::ConstBool { value, literal });
        self.emit(Instruction::Store { local, value });
    }
}
