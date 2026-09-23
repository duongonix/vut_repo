//! Shared MIR effect model used by optimization passes.
//!
//! The effect model is the safety contract of the optimizer: an instruction
//! classified as *observable* must never be removed, even when its result is
//! unused. Getting this wrong silently changes program behavior (for example
//! deleting a `Spawn` or an `AwaitFuture`).
use std::collections::HashMap;

use crate::lowering::future::values;
use crate::{BinaryOp, BuiltinFunction, Function, Instruction, Terminator, ValueId};

/// Returns `true` when `instruction` has an observable effect: a side effect, a
/// trap, a scheduling action, or an ownership operation. Observable
/// instructions are always kept.
#[must_use]
pub fn is_observable(instruction: &Instruction) -> bool {
    match instruction {
        Instruction::Store { .. }
        | Instruction::Drop(_)
        | Instruction::SetFrameState { .. }
        | Instruction::SetFrameChild { .. }
        | Instruction::FieldStore { .. }
        | Instruction::SpillValue { .. }
        | Instruction::PollFuture { .. }
        | Instruction::AwaitChannelRecv { .. }
        | Instruction::PollChannelRecv { .. }
        | Instruction::Allocate { .. }
        | Instruction::StoreRaw { .. }
        | Instruction::Retain { .. }
        | Instruction::MakeUnique { .. }
        | Instruction::Release { .. }
        | Instruction::IteratorInit { .. }
        | Instruction::IteratorNext { .. }
        | Instruction::Spawn { .. }
        | Instruction::AwaitFuture { .. }
        | Instruction::Call { .. }
        | Instruction::StartFuture { .. }
        | Instruction::MakeClosure { .. }
        | Instruction::CallIndirect { .. }
        | Instruction::ConstructInterface { .. }
        | Instruction::InterfaceCall { .. }
        | Instruction::VariadicAt { .. } => true,
        // Scalar numeric builtins are pure: they neither trap, allocate, nor
        // have side effects, so they are removable when unused and eligible for
        // inlining. Trapping conversions (`FloatToInt`, `NumericCast`) and
        // allocating builtins remain observable.
        Instruction::RuntimeCall { function, .. } => !is_pure_scalar_builtin(*function),
        // Division and remainder can trap on a zero divisor, so they are
        // observable even when their result is unused.
        Instruction::Binary { op, .. } => matches!(op, BinaryOp::Divide | BinaryOp::Modulo),
        // Everything else (including `OptionalFromValue`, which builds an
        // optional from an existing payload address and presence flag with no
        // allocation) is pure and removable when its result is unused.
        _ => false,
    }
}

/// Returns `true` when `function` is a scalar numeric builtin with no side
/// effects, no allocation, and no trap.
///
/// `FloatToInt` and `NumericCast` are excluded because an out-of-range
/// conversion traps; allocation-producing conversions are excluded too.
#[must_use]
pub fn is_pure_scalar_builtin(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::FloatAbs
            | BuiltinFunction::FloatFloor
            | BuiltinFunction::FloatCeil
            | BuiltinFunction::FloatRound
            | BuiltinFunction::FloatTrunc
            | BuiltinFunction::FloatSqrt
            | BuiltinFunction::FloatPow
            | BuiltinFunction::FloatMin
            | BuiltinFunction::FloatMax
            | BuiltinFunction::FloatClamp
            | BuiltinFunction::FloatIsNan
            | BuiltinFunction::FloatIsFinite
            | BuiltinFunction::FloatFma
            | BuiltinFunction::FloatCopysign
            | BuiltinFunction::IntAbs
            | BuiltinFunction::IntPow
            | BuiltinFunction::IntMin
            | BuiltinFunction::IntMax
            | BuiltinFunction::IntClamp
    )
}

/// Collects the result slots an instruction defines.
pub fn defined_values(instruction: &Instruction, out: &mut Vec<ValueId>) {
    values::defined(instruction, out);
}

/// Collects the operands an instruction reads.
pub fn operands(instruction: &Instruction, out: &mut Vec<ValueId>) {
    values::used(instruction, out);
}

/// Collects the values a terminator reads.
pub fn terminator_uses(terminator: &Terminator, out: &mut Vec<ValueId>) {
    match terminator {
        Terminator::Branch { condition, .. } => out.push(*condition),
        Terminator::Return(Some(value)) => out.push(*value),
        Terminator::Jump(_)
        | Terminator::Return(None)
        | Terminator::PollReturn(_)
        | Terminator::Unreachable => {}
    }
}

/// Applies `apply` to every operand of `instruction` in place.
pub fn for_each_operand_mut(instruction: &mut Instruction, apply: &mut impl FnMut(&mut ValueId)) {
    values::for_each_operand_mut(instruction, apply);
}

/// Replaces every use of `from` with `to` across `function`, in instructions and
/// terminators. Returns `true` when at least one use changed.
pub fn rewrite_uses(function: &mut Function, from: ValueId, to: ValueId) -> bool {
    let mut changed = false;
    for block in &mut function.blocks {
        for instruction in &mut block.instructions {
            values::for_each_operand_mut(instruction, &mut |value| {
                if *value == from {
                    *value = to;
                    changed = true;
                }
            });
        }
        match &mut block.terminator {
            Terminator::Branch { condition, .. } if *condition == from => {
                *condition = to;
                changed = true;
            }
            Terminator::Return(Some(value)) if *value == from => {
                *value = to;
                changed = true;
            }
            _ => {}
        }
    }
    changed
}

/// Counts how many times each value is read across `function`.
#[must_use]
pub fn use_counts(function: &Function) -> HashMap<ValueId, usize> {
    let mut counts: HashMap<ValueId, usize> = HashMap::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            let mut ops = Vec::new();
            operands(instruction, &mut ops);
            for op in ops {
                *counts.entry(op).or_default() += 1;
            }
        }
        let mut terminator = Vec::new();
        terminator_uses(&block.terminator, &mut terminator);
        for value in terminator {
            *counts.entry(value).or_default() += 1;
        }
    }
    counts
}
