//! Reference-count operation optimization.
//!
//! A `Retain` followed by a `Release` of the same value with no intervening
//! reference-count observer is a net-zero pair and can be removed. The only
//! observer is `MakeUnique` (which reads the count to decide whether to detach),
//! so a `MakeUnique` of the same value between the two invalidates the pairing.
//!
//! Pairs are matched within a block, and across an unconditional `Jump` edge
//! (where the path is unique). At a control-flow join the pending set is
//! dropped, so a pair is never cancelled across a branch.
use std::collections::{HashMap, HashSet};

use super::OptimizationReport;
use crate::analyze::Cfg;
use crate::{BlockId, Function, Instruction, Program, Terminator, ValueId};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        let (retains, releases) = cancel_pairs(function);
        report.retains_elided += retains;
        report.releases_elided += releases;
    }
    report
}

type Location = (BlockId, usize);

fn cancel_pairs(function: &mut Function) -> (usize, usize) {
    let cfg = Cfg::build(function);
    let order = cfg.reverse_postorder(function.entry);
    let mut pending_by_block: Vec<HashMap<ValueId, Vec<Location>>> =
        vec![HashMap::new(); cfg.block_count()];
    let mut remove: Vec<Vec<usize>> = vec![Vec::new(); cfg.block_count()];
    let mut retains = 0;
    let mut releases = 0;

    for &block in &order {
        let mut pending = match jump_predecessor(&cfg, function, block) {
            Some(predecessor) => pending_by_block[predecessor.0].clone(),
            None => HashMap::new(),
        };
        for (index, instruction) in function.blocks[block.0].instructions.iter().enumerate() {
            match instruction {
                Instruction::Retain { value, .. } => {
                    pending.entry(*value).or_default().push((block, index));
                }
                Instruction::Release { value, .. } => {
                    if let Some(stack) = pending.get_mut(value)
                        && let Some((retain_block, retain_index)) = stack.pop()
                    {
                        remove[retain_block.0].push(retain_index);
                        remove[block.0].push(index);
                        retains += 1;
                        releases += 1;
                    }
                }
                Instruction::MakeUnique { operand, .. } => {
                    pending.remove(operand);
                }
                _ => {}
            }
        }
        pending_by_block[block.0] = pending;
    }

    for (block_index, indices) in remove.iter_mut().enumerate() {
        if indices.is_empty() {
            continue;
        }
        let set: HashSet<usize> = indices.iter().copied().collect();
        let mut index = 0;
        function.blocks[block_index].instructions.retain(|_| {
            let keep = !set.contains(&index);
            index += 1;
            keep
        });
    }
    (retains, releases)
}

/// The unique predecessor when it reaches `block` through an unconditional
/// `Jump` (so there is exactly one execution path into `block`).
fn jump_predecessor(cfg: &Cfg, function: &Function, block: BlockId) -> Option<BlockId> {
    let predecessors = cfg.predecessors(block);
    if predecessors.len() != 1 {
        return None;
    }
    let predecessor = predecessors[0];
    match function.blocks[predecessor.0].terminator {
        Terminator::Jump(target) if target == block => Some(predecessor),
        _ => None,
    }
}
