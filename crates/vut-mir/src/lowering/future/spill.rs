//! Spills SSA values that live across a suspension into the future frame.
//!
//! A value defined before an `await` and used after it cannot survive the
//! `return PENDING` as a register: on resume the defining block is skipped. This
//! pass stores such values into a frame region at their definition and reloads
//! them before every use, so the state-machine pass sees no value crossing a
//! suspension.
//!
//! The spill/reload pair is representation-neutral: codegen stores the value
//! with its own machine type and reloads it with that same type, so ownership
//! instructions already attached to the value keep operating on the same
//! pointer.
use std::collections::HashMap;

use super::super::{BasicBlock, Function, Instruction, Terminator, ValueId};
use super::values;

/// Rewrites `function` so every value in `spills` (value -> frame slot) is
/// stored at its definition and reloaded before each use.
pub(super) fn spill_values(function: &mut Function, spills: &HashMap<ValueId, usize>) {
    if spills.is_empty() {
        return;
    }
    let mut next = values::next_value_id(function);
    let mut fresh = || {
        let id = ValueId(next);
        next += 1;
        id
    };
    let blocks = std::mem::take(&mut function.blocks);
    let mut rewritten = Vec::with_capacity(blocks.len());
    for block in blocks {
        let mut instructions = Vec::with_capacity(block.instructions.len());
        for mut instruction in block.instructions {
            let mut reloads = Vec::new();
            values::for_each_operand_mut(&mut instruction, &mut |value: &mut ValueId| {
                if let Some(&slot) = spills.get(value) {
                    let reload = fresh();
                    reloads.push(Instruction::ReloadValue {
                        value: reload,
                        slot,
                    });
                    *value = reload;
                }
            });
            instructions.extend(reloads);
            instructions.push(instruction);
            let mut defined = Vec::new();
            values::defined(instructions.last().expect("just pushed"), &mut defined);
            for value in defined {
                if let Some(&slot) = spills.get(&value) {
                    instructions.push(Instruction::SpillValue { slot, value });
                }
            }
        }
        let mut terminator = block.terminator;
        let mut reloads = Vec::new();
        if let Terminator::Branch { condition, .. } = &mut terminator
            && let Some(&slot) = spills.get(condition)
        {
            let reload = fresh();
            reloads.push(Instruction::ReloadValue {
                value: reload,
                slot,
            });
            *condition = reload;
        }
        if let Terminator::Return(Some(value)) = &mut terminator
            && let Some(&slot) = spills.get(value)
        {
            let reload = fresh();
            reloads.push(Instruction::ReloadValue {
                value: reload,
                slot,
            });
            *value = reload;
        }
        instructions.extend(reloads);
        rewritten.push(BasicBlock {
            instructions,
            terminator,
        });
    }
    function.blocks = rewritten;
}
