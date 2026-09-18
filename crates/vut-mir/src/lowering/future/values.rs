//! Value liveness across `await` suspension points.
//!
//! Locals are tracked separately (`liveness.rs`). This module tracks SSA
//! `ValueId`s: a value defined before a suspension and used after it cannot
//! survive a `return PENDING` unless it is persisted. The state-machine pass
//! uses this to decide whether a suspension point is safe, or must fall back to
//! driving the child to completion inline.
use std::collections::{HashMap, HashSet};

use super::super::{Function, Instruction, Terminator, ValueId};

/// Values defined by an instruction (its result slots).
#[expect(
    clippy::match_same_arms,
    reason = "exhaustive MIR instruction scanner; arms differ only in value names"
)]
pub(super) fn defined(instruction: &Instruction, out: &mut Vec<ValueId>) {
    match instruction {
        Instruction::ConstNull { value }
        | Instruction::ConstInt { value, .. }
        | Instruction::ConstFloat { value, .. }
        | Instruction::ConstBool { value, .. }
        | Instruction::ConstString { value, .. }
        | Instruction::FormatValue { value, .. }
        | Instruction::ConcatString { value, .. }
        | Instruction::Copy { value, .. }
        | Instruction::Borrow { value, .. }
        | Instruction::Move { value, .. }
        | Instruction::Allocate { value, .. }
        | Instruction::Binary { value, .. }
        | Instruction::Unary { value, .. }
        | Instruction::ConstructEnum { value, .. }
        | Instruction::EnumPayload { value, .. }
        | Instruction::TypeRetain { value, .. }
        | Instruction::TypeRelease { value, .. }
        | Instruction::ConstructResult { value, .. }
        | Instruction::ResultPayload { value, .. }
        | Instruction::Spawn { value, .. }
        | Instruction::FrameState { value, .. }
        | Instruction::BorrowField { value, .. }
        | Instruction::ResourceDeref { value, .. }
        | Instruction::AwaitFuture { value, .. }
        | Instruction::MakeFunction { value, .. }
        | Instruction::Field { value, .. }
        | Instruction::CopyAggregate { value, .. }
        | Instruction::Construct { value, .. }
        | Instruction::ConstructArray { value, .. }
        | Instruction::ConstructVariadicBuffer { value, .. }
        | Instruction::VariadicAt { value, .. }
        | Instruction::ConstructInterface { value, .. } => out.push(*value),
        Instruction::IteratorInit { iterator, .. } => out.push(*iterator),
        Instruction::IteratorNext {
            has_value,
            value,
            index,
            ..
        } => {
            out.push(*has_value);
            out.push(*value);
            out.push(*index);
        }
        Instruction::EnumTag { result, .. } | Instruction::ResultState { result, .. } => {
            out.push(*result);
        }
        Instruction::Call { value, .. }
        | Instruction::StartFuture { value, .. }
        | Instruction::CallIndirect { value, .. }
        | Instruction::RuntimeCall { value, .. }
        | Instruction::InterfaceCall { value, .. } => out.extend(value.iter().copied()),
        Instruction::PollFuture { value, ready, .. } => {
            out.extend(value.iter().copied());
            out.push(*ready);
        }
        Instruction::ReloadValue { value, .. } => out.push(*value),
        Instruction::Store { .. }
        | Instruction::Drop(_)
        | Instruction::Retain { .. }
        | Instruction::Release { .. }
        | Instruction::SetFrameState { .. }
        | Instruction::SetFrameChild { .. }
        | Instruction::FieldStore { .. }
        | Instruction::SpillValue { .. } => {}
    }
}

/// Values read by an instruction.
#[expect(
    clippy::match_same_arms,
    reason = "exhaustive MIR instruction scanner; arms differ only in value names"
)]
fn used(instruction: &Instruction, out: &mut Vec<ValueId>) {
    match instruction {
        Instruction::FormatValue { operand, .. } | Instruction::Unary { operand, .. } => {
            out.push(*operand);
        }
        Instruction::ConcatString { left, right, .. } => {
            out.push(*left);
            out.push(*right);
        }
        Instruction::Store { value, .. } | Instruction::Retain { value, .. } => out.push(*value),
        Instruction::SetFrameChild { value, .. } => out.push(*value),
        Instruction::FieldStore { value, .. } => out.push(*value),
        Instruction::ResourceDeref { handle, .. } => out.push(*handle),
        Instruction::SpillValue { value, .. } => out.push(*value),
        Instruction::Release { value, .. } => out.push(*value),
        Instruction::Binary { left, right, .. } => {
            out.push(*left);
            out.push(*right);
        }
        Instruction::IteratorInit {
            iterable,
            length_value,
            ..
        } => {
            out.push(*iterable);
            out.extend(length_value.iter().copied());
        }
        Instruction::IteratorNext { iterator, .. } => out.push(*iterator),
        Instruction::ConstructEnum { payload, .. } => out.extend(payload.iter().copied()),
        Instruction::EnumTag { value, .. } => out.push(*value),
        Instruction::EnumPayload { source, .. } => out.push(*source),
        Instruction::ConstructResult { payload, .. } => out.push(*payload),
        Instruction::ResultState { value, .. } => out.push(*value),
        Instruction::ResultPayload { source, .. } => out.push(*source),
        Instruction::Spawn { start, .. } => out.push(*start),
        Instruction::AwaitFuture { handle, .. } => {
            out.push(*handle);
        }
        Instruction::Call { arguments, .. }
        | Instruction::StartFuture { arguments, .. }
        | Instruction::RuntimeCall { arguments, .. }
        | Instruction::InterfaceCall { arguments, .. } => out.extend(arguments.iter().copied()),
        Instruction::CallIndirect {
            callee, arguments, ..
        } => {
            out.push(*callee);
            out.extend(arguments.iter().copied());
        }
        Instruction::Field { base, .. } => out.push(*base),
        Instruction::CopyAggregate { source, .. } => out.push(*source),
        Instruction::Construct { fields, .. } => {
            out.extend(fields.iter().map(|(_, value)| *value));
        }
        Instruction::ConstructArray { elements, .. }
        | Instruction::ConstructVariadicBuffer { elements, .. } => {
            out.extend(elements.iter().copied());
        }
        Instruction::VariadicAt {
            data, len, index, ..
        } => {
            out.push(*data);
            out.push(*len);
            out.push(*index);
        }
        Instruction::ConstructInterface { source, .. } => out.push(*source),
        Instruction::ConstNull { .. }
        | Instruction::ConstInt { .. }
        | Instruction::ConstFloat { .. }
        | Instruction::ConstBool { .. }
        | Instruction::ConstString { .. }
        | Instruction::Copy { .. }
        | Instruction::Borrow { .. }
        | Instruction::Move { .. }
        | Instruction::Drop(_)
        | Instruction::Allocate { .. }
        | Instruction::TypeRetain { .. }
        | Instruction::TypeRelease { .. }
        | Instruction::FrameState { .. }
        | Instruction::BorrowField { .. }
        | Instruction::SetFrameState { .. }
        | Instruction::PollFuture { .. }
        | Instruction::ReloadValue { .. }
        | Instruction::MakeFunction { .. } => {}
    }
}

/// Applies `visit` to every value an instruction reads (its operands), leaving
/// its result slots untouched. Mirrors [`used`].
#[expect(
    clippy::match_same_arms,
    reason = "exhaustive MIR instruction scanner; arms differ only in value names"
)]
pub(super) fn for_each_operand_mut(
    instruction: &mut Instruction,
    visit: &mut impl FnMut(&mut ValueId),
) {
    match instruction {
        Instruction::FormatValue { operand, .. } | Instruction::Unary { operand, .. } => {
            visit(operand);
        }
        Instruction::ConcatString { left, right, .. } => {
            visit(left);
            visit(right);
        }
        Instruction::Store { value, .. }
        | Instruction::Retain { value, .. }
        | Instruction::Release { value, .. } => visit(value),
        Instruction::SetFrameChild { value, .. } => visit(value),
        Instruction::FieldStore { value, .. } => visit(value),
        Instruction::ResourceDeref { handle, .. } => visit(handle),
        Instruction::SpillValue { value, .. } => visit(value),
        Instruction::Binary { left, right, .. } => {
            visit(left);
            visit(right);
        }
        Instruction::IteratorInit {
            iterable,
            length_value,
            ..
        } => {
            visit(iterable);
            for value in length_value.iter_mut() {
                visit(value);
            }
        }
        Instruction::IteratorNext { iterator, .. } => visit(iterator),
        Instruction::ConstructEnum { payload, .. } => {
            for value in payload.iter_mut() {
                visit(value);
            }
        }
        Instruction::EnumTag { value, .. } => visit(value),
        Instruction::EnumPayload { source, .. } => visit(source),
        Instruction::ConstructResult { payload, .. } => visit(payload),
        Instruction::ResultState { value, .. } => visit(value),
        Instruction::ResultPayload { source, .. } => visit(source),
        Instruction::Spawn { start, .. } => visit(start),
        Instruction::AwaitFuture { handle, .. } => {
            visit(handle);
        }
        Instruction::Call { arguments, .. }
        | Instruction::StartFuture { arguments, .. }
        | Instruction::RuntimeCall { arguments, .. }
        | Instruction::InterfaceCall { arguments, .. } => {
            for argument in arguments.iter_mut() {
                visit(argument);
            }
        }
        Instruction::CallIndirect {
            callee, arguments, ..
        } => {
            visit(callee);
            for argument in arguments.iter_mut() {
                visit(argument);
            }
        }
        Instruction::Field { base, .. } => visit(base),
        Instruction::CopyAggregate { source, .. } => visit(source),
        Instruction::Construct { fields, .. } => {
            for (_, value) in fields.iter_mut() {
                visit(value);
            }
        }
        Instruction::ConstructArray { elements, .. }
        | Instruction::ConstructVariadicBuffer { elements, .. } => {
            for value in elements.iter_mut() {
                visit(value);
            }
        }
        Instruction::VariadicAt {
            data, len, index, ..
        } => {
            visit(data);
            visit(len);
            visit(index);
        }
        Instruction::ConstructInterface { source, .. } => visit(source),
        Instruction::ConstNull { .. }
        | Instruction::ConstInt { .. }
        | Instruction::ConstFloat { .. }
        | Instruction::ConstBool { .. }
        | Instruction::ConstString { .. }
        | Instruction::Copy { .. }
        | Instruction::Borrow { .. }
        | Instruction::Move { .. }
        | Instruction::Drop(_)
        | Instruction::Allocate { .. }
        | Instruction::TypeRetain { .. }
        | Instruction::TypeRelease { .. }
        | Instruction::FrameState { .. }
        | Instruction::BorrowField { .. }
        | Instruction::SetFrameState { .. }
        | Instruction::PollFuture { .. }
        | Instruction::ReloadValue { .. }
        | Instruction::MakeFunction { .. } => {}
    }
}

fn terminator_uses(terminator: &Terminator, out: &mut Vec<ValueId>) {
    match terminator {
        Terminator::Branch { condition, .. } => out.push(*condition),
        Terminator::Return(Some(value)) => out.push(*value),
        Terminator::Jump(_)
        | Terminator::Return(None)
        | Terminator::PollReturn(_)
        | Terminator::Unreachable => {}
    }
}

fn successors(terminator: &Terminator) -> Vec<usize> {
    match terminator {
        Terminator::Jump(target) => vec![target.0],
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![then_block.0, else_block.0],
        Terminator::Return(_) | Terminator::PollReturn(_) | Terminator::Unreachable => Vec::new(),
    }
}

/// The next unused `ValueId` index for `function`.
#[must_use]
pub fn next_value_id(function: &Function) -> usize {
    let mut max = 0;
    let mut bump = |id: ValueId| max = max.max(id.0.saturating_add(1));
    for block in &function.blocks {
        for instruction in &block.instructions {
            let mut defined_ids = Vec::new();
            defined(instruction, &mut defined_ids);
            for id in defined_ids {
                bump(id);
            }
            let mut read = Vec::new();
            used(instruction, &mut read);
            for id in read {
                bump(id);
            }
        }
        let mut read = Vec::new();
        terminator_uses(&block.terminator, &mut read);
        for id in read {
            bump(id);
        }
    }
    max
}

/// For every suspension site, the values live immediately after it.
#[must_use]
pub fn live_across(function: &Function) -> HashMap<(usize, usize), Vec<ValueId>> {
    let block_count = function.blocks.len();
    let mut live_in: Vec<HashSet<ValueId>> = vec![HashSet::new(); block_count];
    let mut live_out: Vec<HashSet<ValueId>> = vec![HashSet::new(); block_count];
    let mut changed = true;
    while changed {
        changed = false;
        for index in (0..block_count).rev() {
            let block = &function.blocks[index];
            let mut out: HashSet<ValueId> = HashSet::new();
            for succ in successors(&block.terminator) {
                out.extend(live_in[succ].iter().copied());
            }
            let mut terminator_values = Vec::new();
            terminator_uses(&block.terminator, &mut terminator_values);
            out.extend(terminator_values);
            let mut live = out.clone();
            for instruction in block.instructions.iter().rev() {
                let mut defs = Vec::new();
                defined(instruction, &mut defs);
                let mut uses = Vec::new();
                used(instruction, &mut uses);
                for def in &defs {
                    live.remove(def);
                }
                live.extend(uses);
            }
            if live != live_in[index] || out != live_out[index] {
                live_in[index] = live;
                live_out[index] = out;
                changed = true;
            }
        }
    }

    let mut result = HashMap::new();
    for (block_index, block) in function.blocks.iter().enumerate() {
        let mut live = live_out[block_index].clone();
        let mut terminator_values = Vec::new();
        terminator_uses(&block.terminator, &mut terminator_values);
        live.extend(terminator_values);
        let mut after: Vec<HashSet<ValueId>> = vec![HashSet::new(); block.instructions.len()];
        for (index, instruction) in block.instructions.iter().enumerate().rev() {
            after[index].clone_from(&live);
            let mut defs = Vec::new();
            defined(instruction, &mut defs);
            let mut uses = Vec::new();
            used(instruction, &mut uses);
            for def in &defs {
                live.remove(def);
            }
            live.extend(uses);
        }
        for (index, instruction) in block.instructions.iter().enumerate() {
            if matches!(instruction, Instruction::AwaitFuture { .. }) {
                // The await's own result is produced on resume, so it is not a
                // value that needs to survive the suspension.
                let mut own = Vec::new();
                defined(instruction, &mut own);
                let own: HashSet<ValueId> = own.into_iter().collect();
                let mut values: Vec<ValueId> = after[index]
                    .iter()
                    .copied()
                    .filter(|value| !own.contains(value))
                    .collect();
                values.sort_by_key(|value| value.0);
                result.insert((block_index, index), values);
            }
        }
    }
    result
}
