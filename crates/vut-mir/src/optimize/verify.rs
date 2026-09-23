//! MIR verifier.
//!
//! Runs before and after every optimization batch. It checks the invariants an
//! optimization could plausibly break: block ids in range, every operand is
//! defined somewhere in its function (so a deleted definition cannot leave a
//! dangling use), and local indices in range. It is deliberately lenient about
//! dominance so valid MIR is never rejected.
use std::collections::HashSet;
use std::fmt;

use super::effects;
use crate::analyze::{Cfg, DefUse, DominatorTree};
use crate::{BlockId, Function, Instruction, LocalId, Program, Terminator, ValueId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationError {
    pub function: usize,
    pub message: String,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "function {}: {}", self.function, self.message)
    }
}

/// Verifies every function in `program`.
///
/// # Errors
/// Returns the list of violations found.
pub fn verify(program: &Program) -> Result<(), Vec<VerificationError>> {
    let mut errors = Vec::new();
    for (index, function) in program.functions.iter().enumerate() {
        verify_function(function, index, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn push(errors: &mut Vec<VerificationError>, index: usize, message: String) {
    errors.push(VerificationError {
        function: index,
        message,
    });
}

fn check_block(errors: &mut Vec<VerificationError>, index: usize, id: BlockId, count: usize) {
    if id.0 >= count {
        push(errors, index, format!("block id {} out of range", id.0));
    }
}

fn verify_function(function: &Function, index: usize, errors: &mut Vec<VerificationError>) {
    let block_count = function.blocks.len();
    let local_count = function.locals.len();
    let mut defined: HashSet<ValueId> = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            let mut ids = Vec::new();
            effects::defined_values(instruction, &mut ids);
            defined.extend(ids);
        }
    }

    if block_count == 0 {
        push(errors, index, "function has no blocks".to_owned());
        return;
    }
    if function.entry.0 >= block_count {
        push(
            errors,
            index,
            format!("entry block {} out of range", function.entry.0),
        );
    }

    for block in &function.blocks {
        match &block.terminator {
            Terminator::Jump(target) => check_block(errors, index, *target, block_count),
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                check_block(errors, index, *then_block, block_count);
                check_block(errors, index, *else_block, block_count);
                if !defined.contains(condition) {
                    push(
                        errors,
                        index,
                        format!("branch condition value {} is not defined", condition.0),
                    );
                }
            }
            Terminator::Return(Some(value)) => {
                if !defined.contains(value) {
                    push(
                        errors,
                        index,
                        format!("return value {} is not defined", value.0),
                    );
                }
            }
            Terminator::Return(None) | Terminator::PollReturn(_) | Terminator::Unreachable => {}
        }

        for instruction in &block.instructions {
            let mut operands = Vec::new();
            effects::operands(instruction, &mut operands);
            for operand in operands {
                if !defined.contains(&operand) {
                    push(
                        errors,
                        index,
                        format!("operand value {} is not defined", operand.0),
                    );
                }
            }
            check_locals(instruction, local_count, index, errors);
        }
    }

    verify_dominance(function, index, errors);
    verify_moves(function, index, errors);
}

/// Every operand must be defined in a block that dominates its use (or earlier
/// in the same block). Values are single-assignment, so this is a real
/// invariant; it catches a pass that moves a definition past its use.
fn verify_dominance(function: &Function, index: usize, errors: &mut Vec<VerificationError>) {
    let cfg = Cfg::build(function);
    let dominators = DominatorTree::build(&cfg, function.entry);
    let def_use = DefUse::build(function);
    let reachable = cfg.reachable(function.entry);

    for (block_index, block) in function.blocks.iter().enumerate() {
        if !reachable.get(block_index).copied().unwrap_or(false) {
            continue;
        }
        let use_block = BlockId(block_index);
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            let mut operands = Vec::new();
            effects::operands(instruction, &mut operands);
            for value in operands {
                check_dominance(
                    &def_use,
                    &dominators,
                    value,
                    use_block,
                    instruction_index,
                    index,
                    errors,
                );
            }
        }
        let mut terminator = Vec::new();
        effects::terminator_uses(&block.terminator, &mut terminator);
        for value in terminator {
            check_dominance(
                &def_use,
                &dominators,
                value,
                use_block,
                usize::MAX,
                index,
                errors,
            );
        }
    }
}

fn check_dominance(
    def_use: &DefUse,
    dominators: &DominatorTree,
    value: ValueId,
    use_block: BlockId,
    use_index: usize,
    index: usize,
    errors: &mut Vec<VerificationError>,
) {
    let Some((def_block, def_index)) = def_use.definition(value) else {
        // Undefined operands are reported by the definition check.
        return;
    };
    if def_block == use_block {
        if def_index >= use_index {
            push(
                errors,
                index,
                format!("value {} is used before its definition", value.0),
            );
        }
    } else if !dominators.dominates(def_block, use_block) {
        push(
            errors,
            index,
            format!(
                "value {} defined in block {} does not dominate its use in block {}",
                value.0, def_block.0, use_block.0
            ),
        );
    }
}

/// Ownership-state check: a local must not be read, moved, or dropped after it
/// has been moved out. This models the ownership *states* (`moved`) rather than
/// counting retain/release operations.
fn verify_moves(function: &Function, index: usize, errors: &mut Vec<VerificationError>) {
    let cfg = Cfg::build(function);
    let reachable = cfg.reachable(function.entry);
    let count = cfg.block_count();
    if count == 0 {
        return;
    }
    let mut live_in: Vec<HashSet<LocalId>> = vec![HashSet::new(); count];
    let mut live_out: Vec<HashSet<LocalId>> = vec![HashSet::new(); count];

    // Fixpoint: a local is "moved" on entry to a block only if it is moved on
    // every reachable predecessor path (intersection).
    let mut changed = true;
    while changed {
        changed = false;
        for block in 0..count {
            if !reachable.get(block).copied().unwrap_or(false) {
                continue;
            }
            let entry_moved = if block == function.entry.0 {
                HashSet::new()
            } else {
                intersect_predecessors(&cfg, &reachable, &live_out, BlockId(block))
            };
            let block_out = transfer_moves(&function.blocks[block], entry_moved.clone());
            if entry_moved != live_in[block] || block_out != live_out[block] {
                live_in[block] = entry_moved;
                live_out[block] = block_out;
                changed = true;
            }
        }
    }

    for (block, block_data) in function.blocks.iter().enumerate() {
        if !reachable.get(block).copied().unwrap_or(false) {
            continue;
        }
        let mut moved = live_in[block].clone();
        for instruction in &block_data.instructions {
            match instruction {
                Instruction::Store { local, .. } => {
                    moved.remove(local);
                }
                Instruction::Move { local, .. } => {
                    if moved.contains(local) {
                        push(
                            errors,
                            index,
                            format!("local {} is moved after being moved", local.0),
                        );
                    }
                    moved.insert(*local);
                }
                Instruction::Drop(local) => {
                    if moved.contains(local) {
                        push(
                            errors,
                            index,
                            format!("local {} is dropped after being moved", local.0),
                        );
                    }
                    moved.insert(*local);
                }
                Instruction::Copy { local, .. } | Instruction::Borrow { local, .. } => {
                    if moved.contains(local) {
                        push(
                            errors,
                            index,
                            format!("local {} is used after being moved", local.0),
                        );
                    }
                }
                Instruction::FieldStore { base, .. } | Instruction::BorrowField { base, .. }
                    if moved.contains(base) =>
                {
                    push(
                        errors,
                        index,
                        format!("local {} is used after being moved", base.0),
                    );
                }
                _ => {}
            }
        }
    }
}

fn intersect_predecessors(
    cfg: &Cfg,
    reachable: &[bool],
    live_out: &[HashSet<LocalId>],
    block: BlockId,
) -> HashSet<LocalId> {
    let mut predecessors = cfg
        .predecessors(block)
        .iter()
        .filter(|predecessor| reachable.get(predecessor.0).copied().unwrap_or(false));
    let Some(first) = predecessors.next() else {
        return HashSet::new();
    };
    let mut moved = live_out[first.0].clone();
    for predecessor in predecessors {
        moved.retain(|local| live_out[predecessor.0].contains(local));
    }
    moved
}

fn transfer_moves(block: &crate::BasicBlock, mut moved: HashSet<LocalId>) -> HashSet<LocalId> {
    for instruction in &block.instructions {
        match instruction {
            Instruction::Store { local, .. } => {
                moved.remove(local);
            }
            Instruction::Move { local, .. } | Instruction::Drop(local) => {
                moved.insert(*local);
            }
            _ => {}
        }
    }
    moved
}

fn check_locals(
    instruction: &Instruction,
    local_count: usize,
    index: usize,
    errors: &mut Vec<VerificationError>,
) {
    let check = |local: LocalId, errors: &mut Vec<VerificationError>| {
        if local.0 >= local_count {
            push(errors, index, format!("local {} out of range", local.0));
        }
    };
    match instruction {
        Instruction::Copy { local, .. }
        | Instruction::Borrow { local, .. }
        | Instruction::Move { local, .. }
        | Instruction::Store { local, .. }
        | Instruction::Drop(local)
        | Instruction::FrameState { frame: local, .. }
        | Instruction::SetFrameState { frame: local, .. }
        | Instruction::SetFrameChild { frame: local, .. }
        | Instruction::FieldStore { base: local, .. }
        | Instruction::BorrowField { base: local, .. } => check(*local, errors),
        Instruction::PollFuture { frame, .. } | Instruction::PollChannelRecv { frame, .. } => {
            check(*frame, errors);
        }
        _ => {}
    }
}
