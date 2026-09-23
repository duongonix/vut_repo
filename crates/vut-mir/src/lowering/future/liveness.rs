//! Liveness of MIR locals across `await` suspension points.
//!
//! Only locals survive a suspension (values are recomputed). This computes, for
//! every suspension instruction, the set of locals live immediately after it:
//! exactly the values the frame must persist while the future is suspended.
use std::collections::HashSet;

use super::super::{Function, Instruction, Terminator};

/// Locals live across one suspension point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwaitLive {
    pub block: usize,
    pub index: usize,
    /// Resume state assigned to this suspension point. State `0` is the initial
    /// "not started" state, so the first await resumes at state `1`.
    pub state: u32,
    /// `LocalId` indices that must persist across this await.
    pub locals: Vec<usize>,
    /// Number of SSA values live across this await. A nonzero count means the
    /// suspension needs value spilling.
    pub crossing_values: usize,
}

fn is_suspension(instruction: &Instruction) -> bool {
    // `StartFuture` only creates the future; the actual suspension point is the
    // subsequent await (for both Vut futures and native futures).
    matches!(
        instruction,
        Instruction::AwaitFuture { .. } | Instruction::AwaitChannelRecv { .. }
    )
}

fn local_uses(instruction: &Instruction, uses: &mut HashSet<usize>) {
    match instruction {
        Instruction::Copy { local, .. }
        | Instruction::Move { local, .. }
        | Instruction::Borrow { local, .. }
        | Instruction::Drop(local) => {
            uses.insert(local.0);
        }
        _ => {}
    }
}

fn local_defs(instruction: &Instruction, defs: &mut HashSet<usize>) {
    if let Instruction::Store { local, .. } = instruction {
        defs.insert(local.0);
    }
}

fn successors(terminator: &Terminator, successors: &mut Vec<usize>) {
    match terminator {
        Terminator::Jump(target) => successors.push(target.0),
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => {
            successors.push(then_block.0);
            successors.push(else_block.0);
        }
        Terminator::Return(_) | Terminator::PollReturn(_) | Terminator::Unreachable => {}
    }
}

/// Computes the live-across-await locals for every suspension site, in program
/// order.
#[must_use]
pub fn analyze(function: &Function) -> Vec<AwaitLive> {
    let block_count = function.blocks.len();
    if block_count == 0 {
        return Vec::new();
    }
    let mut live_in: Vec<HashSet<usize>> = vec![HashSet::new(); block_count];
    let mut live_out: Vec<HashSet<usize>> = vec![HashSet::new(); block_count];
    let mut changed = true;
    while changed {
        changed = false;
        for index in (0..block_count).rev() {
            let block = &function.blocks[index];
            let mut out = HashSet::new();
            let mut succs = Vec::new();
            successors(&block.terminator, &mut succs);
            for succ in succs {
                out.extend(live_in[succ].iter().copied());
            }
            let mut live = out.clone();
            for instruction in block.instructions.iter().rev() {
                let mut defs = HashSet::new();
                local_defs(instruction, &mut defs);
                let mut uses = HashSet::new();
                local_uses(instruction, &mut uses);
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

    let mut awaits = Vec::new();
    for (block_index, block) in function.blocks.iter().enumerate() {
        let mut live = live_out[block_index].clone();
        let mut after: Vec<HashSet<usize>> = vec![HashSet::new(); block.instructions.len()];
        for (index, instruction) in block.instructions.iter().enumerate().rev() {
            after[index].clone_from(&live);
            let mut defs = HashSet::new();
            local_defs(instruction, &mut defs);
            let mut uses = HashSet::new();
            local_uses(instruction, &mut uses);
            for def in &defs {
                live.remove(def);
            }
            live.extend(uses);
        }
        for (index, instruction) in block.instructions.iter().enumerate() {
            if is_suspension(instruction) {
                let mut locals: Vec<usize> = after[index].iter().copied().collect();
                locals.sort_unstable();
                awaits.push(AwaitLive {
                    block: block_index,
                    index,
                    state: 0,
                    locals,
                    crossing_values: 0,
                });
            }
        }
    }
    // Assign resume states in program order; state 0 is "not started".
    for (state, site) in awaits.iter_mut().enumerate() {
        site.state = u32::try_from(state).unwrap_or(u32::MAX).saturating_add(1);
    }
    awaits
}
