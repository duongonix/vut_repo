//! Control-flow simplification: remove blocks unreachable from the entry.
use super::OptimizationReport;
use crate::{BlockId, Function, Program, Terminator};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.unreachable_blocks += remove_unreachable(function);
    }
    report
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

fn remove_unreachable(function: &mut Function) -> usize {
    let count = function.blocks.len();
    if count == 0 {
        return 0;
    }
    let mut reachable = vec![false; count];
    let entry = function.entry.0;
    if entry < count {
        reachable[entry] = true;
    }
    let mut stack = vec![entry];
    while let Some(node) = stack.pop() {
        for successor in successors(&function.blocks[node].terminator) {
            if successor < count && !reachable[successor] {
                reachable[successor] = true;
                stack.push(successor);
            }
        }
    }
    let removed = reachable.iter().filter(|value| !**value).count();
    if removed == 0 {
        return 0;
    }

    let mut remap = vec![0_usize; count];
    let mut next = 0_usize;
    for (index, reachable) in reachable.iter().enumerate() {
        if *reachable {
            remap[index] = next;
            next += 1;
        }
    }

    let mut blocks = std::mem::take(&mut function.blocks);
    let mut kept = Vec::with_capacity(next);
    for (index, block) in blocks.drain(..).enumerate() {
        if !reachable[index] {
            continue;
        }
        let mut block = block;
        block.terminator = remap_terminator(block.terminator, &remap);
        kept.push(block);
    }
    function.blocks = kept;
    function.entry = BlockId(remap[entry]);
    removed
}

fn remap_terminator(terminator: Terminator, remap: &[usize]) -> Terminator {
    match terminator {
        Terminator::Jump(target) => Terminator::Jump(BlockId(remap[target.0])),
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => Terminator::Branch {
            condition,
            then_block: BlockId(remap[then_block.0]),
            else_block: BlockId(remap[else_block.0]),
        },
        other => other,
    }
}
