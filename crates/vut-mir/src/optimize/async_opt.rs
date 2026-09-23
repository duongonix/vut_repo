//! Async state-machine optimization.
//!
//! - Redundant frame reloads: two reloads of the same slot with no intervening
//!   spill yield the same value, so the second is forwarded to the first.
//! - Dead frame-state stores: a `SetFrameState` overwritten by another store to
//!   the same frame before the state is read is removed.
//! - Dead scalar spills: a `SpillValue` whose slot is never reloaded and whose
//!   value is a scalar constant is removed (no ownership is involved).
//!
//! Frame compaction / dead-field elimination across the whole frame is deferred
//! to the lowering layer.
use std::collections::{HashMap, HashSet};

use super::OptimizationReport;
use super::effects;
use crate::{Instruction, Program, ValueId};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.async_reloads_elided += dedup_reloads(function);
        report.async_frame_states_elided += dedup_frame_states(function);
        report.async_spills_elided += remove_dead_scalar_spills(function);
    }
    report
}

fn dedup_reloads(function: &mut crate::Function) -> usize {
    let mut replacements: Vec<(ValueId, ValueId)> = Vec::new();
    for block in &function.blocks {
        let mut known: HashMap<usize, ValueId> = HashMap::new();
        for instruction in &block.instructions {
            match instruction {
                Instruction::ReloadValue { value, slot } => {
                    if let Some(existing) = known.get(slot).copied() {
                        replacements.push((*value, existing));
                    } else {
                        known.insert(*slot, *value);
                    }
                }
                Instruction::SpillValue { slot, .. } => {
                    known.remove(slot);
                }
                _ => {}
            }
        }
    }
    if replacements.is_empty() {
        return 0;
    }
    for (from, to) in &replacements {
        effects::rewrite_uses(function, *from, *to);
    }
    replacements.len()
}

fn dedup_frame_states(function: &mut crate::Function) -> usize {
    let mut removed = 0;
    for block in &mut function.blocks {
        let mut last: HashMap<usize, usize> = HashMap::new();
        let mut dead: HashSet<usize> = HashSet::new();
        for (index, instruction) in block.instructions.iter().enumerate() {
            match instruction {
                Instruction::SetFrameState { frame, .. } => {
                    if let Some(previous) = last.insert(frame.0, index) {
                        dead.insert(previous);
                    }
                }
                Instruction::FrameState { frame, .. } => {
                    // Reading the state makes the previous store live.
                    last.remove(&frame.0);
                }
                _ => {}
            }
        }
        if dead.is_empty() {
            continue;
        }
        let mut index = 0;
        block.instructions.retain(|_| {
            let keep = !dead.contains(&index);
            index += 1;
            keep
        });
        removed += dead.len();
    }
    removed
}

fn remove_dead_scalar_spills(function: &mut crate::Function) -> usize {
    let mut reloaded: HashSet<usize> = HashSet::new();
    let mut scalar_constants: HashSet<ValueId> = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::ReloadValue { slot, .. } => {
                    reloaded.insert(*slot);
                }
                Instruction::ConstInt { value, .. }
                | Instruction::ConstBool { value, .. }
                | Instruction::ConstFloat { value, .. } => {
                    scalar_constants.insert(*value);
                }
                _ => {}
            }
        }
    }
    if reloaded.is_empty() && scalar_constants.is_empty() {
        return 0;
    }
    let mut removed = 0;
    for block in &mut function.blocks {
        block.instructions.retain(|instruction| {
            if let Instruction::SpillValue { slot, value } = instruction
                && !reloaded.contains(slot)
                && scalar_constants.contains(value)
            {
                removed += 1;
                return false;
            }
            true
        });
    }
    removed
}
