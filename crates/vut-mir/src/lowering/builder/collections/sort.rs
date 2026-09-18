//! Compiler-lowered `list.sort_by(compare)`: an iterative, stable bottom-up
//! merge sort.
//!
//! Stability comes from taking the left run whenever `compare(left, right) <= 0`.
//! The sort works on two scratch buffers so no comparison ever reaches the
//! runtime: the comparator is an ordinary Vut function value invoked through
//! `CallIndirect`. Ownership follows the list ABI: `ListAt` produces an owned
//! element (it retains), `ListSet`/`ListPush` retain what they store, and each
//! owned temporary is released after use.
use vut_hir::TypeId;
use vut_source::Span;

use super::super::super::{BinaryOp, Instruction, LocalId, Terminator, ValueId};
use super::super::Builder;

/// The loop-carried locals and types of one `sort_by` expansion.
struct SortBy {
    element: TypeId,
    callback: ValueId,
    callback_ty: Option<TypeId>,
    int_ty: TypeId,
    n: LocalId,
    width: LocalId,
    /// 0 for an even number of completed passes, 1 for an odd number.
    parity: LocalId,
    a: LocalId,
    b: LocalId,
    i: LocalId,
    mid: LocalId,
    end: LocalId,
    left: LocalId,
    right: LocalId,
    out: LocalId,
    take: LocalId,
    /// 1 when the element just taken came from the left run, else 0.
    from_left: LocalId,
}

impl Builder<'_> {
    /// `items.sort_by(compare)` — sorts `items` in place, stably, in O(n log n).
    #[expect(
        clippy::too_many_lines,
        reason = "the merge-sort CFG is emitted in one place"
    )]
    pub(super) fn lower_list_sort_by(
        &mut self,
        receiver: ValueId,
        element: TypeId,
        callback: ValueId,
        callback_ty: Option<TypeId>,
        span: Span,
        owned: Vec<(ValueId, TypeId)>,
    ) -> Option<ValueId> {
        let list_ty = self.list_type(element)?;
        let int_ty = self.int_type()?;
        let mark = self.local_data.len();
        let state = SortBy {
            element,
            callback,
            callback_ty,
            int_ty,
            n: self.add_temp_local(int_ty, span),
            width: self.add_temp_local(int_ty, span),
            parity: self.add_temp_local(int_ty, span),
            a: self.add_temp_local(list_ty, span),
            b: self.add_temp_local(list_ty, span),
            i: self.add_temp_local(int_ty, span),
            mid: self.add_temp_local(int_ty, span),
            end: self.add_temp_local(int_ty, span),
            left: self.add_temp_local(int_ty, span),
            right: self.add_temp_local(int_ty, span),
            out: self.add_temp_local(int_ty, span),
            take: self.add_temp_local(int_ty, span),
            from_left: self.add_temp_local(int_ty, span),
        };
        let length = self.list_length(receiver)?;
        self.store(state.n, length);
        let capacity = self.read_int(state.n);
        let buffer_a = self.emit_list_new(list_ty, element, Some(capacity));
        self.store(state.a, buffer_a);
        let capacity = self.read_int(state.n);
        let buffer_b = self.emit_list_new(list_ty, element, Some(capacity));
        self.store(state.b, buffer_b);
        self.emit_seed_buffers(receiver, &state);
        let one = self.int_const(1);
        self.store(state.width, one);
        let zero = self.int_const(0);
        self.store(state.parity, zero);

        // Main loop: while width < n, merge runs of `width` alternating the
        // source and destination buffers so a pass costs one traversal.
        let loop_head = self.new_block();
        let choose = self.new_block();
        let pass_end = self.new_block();
        let after_loop = self.new_block();
        self.terminate(Terminator::Jump(loop_head));
        self.switch_to(loop_head);
        let more = {
            let width = self.read_int(state.width);
            let length = self.read_int(state.n);
            self.int_binary(BinaryOp::Less, width, length)
        };
        self.terminate(Terminator::Branch {
            condition: more,
            then_block: choose,
            else_block: after_loop,
        });
        self.switch_to(choose);
        let odd = {
            let parity = self.read_int(state.parity);
            let zero = self.int_const(0);
            self.int_binary(BinaryOp::NotEqual, parity, zero)
        };
        let odd_block = self.new_block();
        let even_block = self.new_block();
        self.terminate(Terminator::Branch {
            condition: odd,
            then_block: odd_block,
            else_block: even_block,
        });
        self.switch_to(even_block);
        self.emit_merge_pass(&state, state.a, state.b);
        self.terminate(Terminator::Jump(pass_end));
        self.switch_to(odd_block);
        self.emit_merge_pass(&state, state.b, state.a);
        self.terminate(Terminator::Jump(pass_end));
        self.switch_to(pass_end);
        {
            let parity = self.read_int(state.parity);
            let one = self.int_const(1);
            let flipped = self.int_binary(BinaryOp::Subtract, one, parity);
            self.store(state.parity, flipped);
        }
        {
            let width = self.read_int(state.width);
            let two = self.int_const(2);
            let doubled = self.int_binary(BinaryOp::Multiply, width, two);
            self.store(state.width, doubled);
        }
        self.terminate(Terminator::Jump(loop_head));

        // Copy the sorted buffer back into the receiver.
        self.switch_to(after_loop);
        let odd = {
            let parity = self.read_int(state.parity);
            let zero = self.int_const(0);
            self.int_binary(BinaryOp::NotEqual, parity, zero)
        };
        let from_b = self.new_block();
        let from_a = self.new_block();
        let end = self.new_block();
        self.terminate(Terminator::Branch {
            condition: odd,
            then_block: from_b,
            else_block: from_a,
        });
        self.switch_to(from_a);
        self.emit_copy_into(&state, state.a, receiver);
        self.terminate(Terminator::Jump(end));
        self.switch_to(from_b);
        self.emit_copy_into(&state, state.b, receiver);
        self.terminate(Terminator::Jump(end));
        self.switch_to(end);

        for (value, ty) in owned {
            self.emit(Instruction::Release { value, ty });
        }
        // Releases both scratch buffers and marks every temp local consumed.
        self.close_scope(mark);
        None
    }

    /// `a[i] = receiver[i]; b[i] = receiver[i]` for every element.
    fn emit_seed_buffers(&mut self, receiver: ValueId, state: &SortBy) {
        let head = self.new_block();
        let body = self.new_block();
        let done = self.new_block();
        let zero = self.int_const(0);
        self.store(state.i, zero);
        self.terminate(Terminator::Jump(head));
        self.switch_to(head);
        let inside = self.loop_condition(state, state.i);
        self.terminate(Terminator::Branch {
            condition: inside,
            then_block: body,
            else_block: done,
        });
        self.switch_to(body);
        let index = self.read_int(state.i);
        let element = self.list_at(receiver, index, state.element);
        let buffer_a = self.read_borrow(state.a);
        self.list_push(buffer_a, element);
        let buffer_b = self.read_borrow(state.b);
        self.list_push(buffer_b, element);
        self.release_managed(element, state.element);
        self.increment(state.i);
        self.terminate(Terminator::Jump(head));
        self.switch_to(done);
    }

    /// `dst[i] = src[i]` for every element.
    fn emit_copy_into(&mut self, state: &SortBy, src: LocalId, dst: ValueId) {
        let head = self.new_block();
        let body = self.new_block();
        let done = self.new_block();
        let zero = self.int_const(0);
        self.store(state.i, zero);
        self.terminate(Terminator::Jump(head));
        self.switch_to(head);
        let inside = self.loop_condition(state, state.i);
        self.terminate(Terminator::Branch {
            condition: inside,
            then_block: body,
            else_block: done,
        });
        self.switch_to(body);
        let index = self.read_int(state.i);
        let source = self.read_borrow(src);
        let element = self.list_at(source, index, state.element);
        self.list_set(dst, index, element);
        self.release_managed(element, state.element);
        self.increment(state.i);
        self.terminate(Terminator::Jump(head));
        self.switch_to(done);
    }

    /// One bottom-up merge pass over all runs of `width` elements, moving data
    /// from `src` into `dst` (both already have length `n`).
    fn emit_merge_pass(&mut self, state: &SortBy, src: LocalId, dst: LocalId) {
        let pass_head = self.new_block();
        let run = self.new_block();
        let pass_done = self.new_block();
        let zero = self.int_const(0);
        self.store(state.i, zero);
        self.terminate(Terminator::Jump(pass_head));
        self.switch_to(pass_head);
        let inside = self.loop_condition(state, state.i);
        self.terminate(Terminator::Branch {
            condition: inside,
            then_block: run,
            else_block: pass_done,
        });
        self.switch_to(run);
        // mid = min(i + width, n); end = min(i + 2*width, n)
        let start = self.read_int(state.i);
        let width = self.read_int(state.width);
        let length = self.read_int(state.n);
        let left_end = self.int_binary(BinaryOp::Add, start, width);
        let mid = self.int_min(left_end, length);
        self.store(state.mid, mid);
        let left_end = self.read_int(state.i);
        let width = self.read_int(state.width);
        let length = self.read_int(state.n);
        let double = self.int_binary(BinaryOp::Add, width, width);
        let run_end = self.int_binary(BinaryOp::Add, left_end, double);
        let end = self.int_min(run_end, length);
        self.store(state.end, end);
        let start = self.read_int(state.i);
        self.store(state.left, start);
        let mid = self.read_int(state.mid);
        self.store(state.right, mid);
        let start = self.read_int(state.i);
        self.store(state.out, start);
        self.emit_merge(state, src, dst);
        let end = self.read_int(state.end);
        self.store(state.i, end);
        self.terminate(Terminator::Jump(pass_head));
        self.switch_to(pass_done);
    }

    /// The two-pointer merge of `src[left..mid]` and `src[mid..end]` into
    /// `dst[out..]`, preserving stability.
    #[expect(
        clippy::too_many_lines,
        reason = "the merge CFG is emitted in one place"
    )]
    fn emit_merge(&mut self, state: &SortBy, src: LocalId, dst: LocalId) {
        let head = self.new_block();
        let body = self.new_block();
        let left_exhausted = self.new_block();
        let compare = self.new_block();
        let take_left = self.new_block();
        let take_right = self.new_block();
        let advance = self.new_block();
        let inc_left = self.new_block();
        let inc_right = self.new_block();
        let step = self.new_block();
        let done = self.new_block();

        self.terminate(Terminator::Jump(head));
        self.switch_to(head);
        let has_left = {
            let left = self.read_int(state.left);
            let mid = self.read_int(state.mid);
            self.int_binary(BinaryOp::Less, left, mid)
        };
        let has_right = {
            let right = self.read_int(state.right);
            let end = self.read_int(state.end);
            self.int_binary(BinaryOp::Less, right, end)
        };
        let pending = self.int_binary(BinaryOp::Or, has_left, has_right);
        self.terminate(Terminator::Branch {
            condition: pending,
            then_block: body,
            else_block: done,
        });

        self.switch_to(body);
        let right_done = {
            let right = self.read_int(state.right);
            let end = self.read_int(state.end);
            self.int_binary(BinaryOp::GreaterEqual, right, end)
        };
        self.terminate(Terminator::Branch {
            condition: right_done,
            then_block: take_left,
            else_block: left_exhausted,
        });

        self.switch_to(left_exhausted);
        let left_done = {
            let left = self.read_int(state.left);
            let mid = self.read_int(state.mid);
            self.int_binary(BinaryOp::GreaterEqual, left, mid)
        };
        self.terminate(Terminator::Branch {
            condition: left_done,
            then_block: take_right,
            else_block: compare,
        });

        self.switch_to(compare);
        // `ListAt` yields an owned element; the comparator consumes its
        // arguments, so no extra retain is required.
        let left_index = self.read_int(state.left);
        let right_index = self.read_int(state.right);
        let source = self.read_borrow(src);
        let left_element = self.list_at(source, left_index, state.element);
        let right_element = self.list_at(source, right_index, state.element);
        let ordering = self.value();
        self.emit(Instruction::CallIndirect {
            value: Some(ordering),
            result_type: Some(state.int_ty),
            callable_ty: state.callback_ty,
            callee: state.callback,
            arguments: vec![left_element, right_element],
        });
        let keep_left = {
            let zero = self.int_const(0);
            self.int_binary(BinaryOp::LessEqual, ordering, zero)
        };
        self.terminate(Terminator::Branch {
            condition: keep_left,
            then_block: take_left,
            else_block: take_right,
        });

        self.switch_to(take_left);
        let left = self.read_int(state.left);
        self.store(state.take, left);
        let from_left = self.int_const(1);
        self.store(state.from_left, from_left);
        self.terminate(Terminator::Jump(advance));
        self.switch_to(take_right);
        let right = self.read_int(state.right);
        self.store(state.take, right);
        let not_left = self.int_const(0);
        self.store(state.from_left, not_left);
        self.terminate(Terminator::Jump(advance));

        self.switch_to(advance);
        let take = self.read_int(state.take);
        let source = self.read_borrow(src);
        let element = self.list_at(source, take, state.element);
        let out = self.read_int(state.out);
        let destination = self.read_borrow(dst);
        self.list_set(destination, out, element);
        self.release_managed(element, state.element);
        // An explicit flag distinguishes the runs: `take` can numerically equal
        // the other run's cursor when one side is exhausted.
        let from_left = self.read_int(state.from_left);
        let zero = self.int_const(0);
        let came_from_left = self.int_binary(BinaryOp::NotEqual, from_left, zero);
        self.terminate(Terminator::Branch {
            condition: came_from_left,
            then_block: inc_left,
            else_block: inc_right,
        });

        self.switch_to(inc_left);
        self.increment(state.left);
        self.terminate(Terminator::Jump(step));
        self.switch_to(inc_right);
        self.increment(state.right);
        self.terminate(Terminator::Jump(step));

        self.switch_to(step);
        self.increment(state.out);
        self.terminate(Terminator::Jump(head));
        self.switch_to(done);
    }

    /// `i < n` as a boolean value.
    fn loop_condition(&mut self, state: &SortBy, index: LocalId) -> ValueId {
        let index = self.read_int(index);
        let length = self.read_int(state.n);
        self.int_binary(BinaryOp::Less, index, length)
    }

    /// `local = local + 1`.
    fn increment(&mut self, local: LocalId) {
        let current = self.read_int(local);
        let one = self.int_const(1);
        let next = self.int_binary(BinaryOp::Add, current, one);
        self.store(local, next);
    }
}
