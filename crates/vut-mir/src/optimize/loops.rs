//! Loop optimization: loop-invariant code motion.
//!
//! Hoists pure, loop-invariant, non-trapping instructions from a loop body into
//! its preheader. Only scalar-producing operations are hoisted (`Binary` except
//! division/remainder, `Unary`, and the pure `*Len` reads), so hoisting cannot
//! change ownership or retention. A `*Len` read is only hoisted when the loop
//! contains no instruction that could mutate any collection.
use std::collections::HashSet;

use super::{OptimizationReport, effects};
use crate::analyze::{Cfg, DefUse, DominatorTree, LoopInfo};
use crate::{BlockId, BuiltinFunction, Function, Instruction, Program};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.instructions_hoisted += hoist(function);
    }
    report
}

fn hoist(function: &mut Function) -> usize {
    let cfg = Cfg::build(function);
    let dominators = DominatorTree::build(&cfg, function.entry);
    let loops = LoopInfo::analyze(&cfg, &dominators);
    if loops.is_empty() {
        return 0;
    }
    let def_use = DefUse::build(function);

    let mut hoisted = 0;
    for header in loops.headers() {
        let Some(body) = loops.body(header).cloned() else {
            continue;
        };
        let Some(preheader) = preheader_of(&cfg, &body, header) else {
            continue;
        };
        let loop_mutates = loop_mutates(function, &body);

        let mut candidates: Vec<(usize, usize)> = Vec::new();
        for &block in &body {
            for (index, instruction) in function.blocks[block.0].instructions.iter().enumerate() {
                if !is_hoistable(instruction, loop_mutates) {
                    continue;
                }
                let mut operands = Vec::new();
                effects::operands(instruction, &mut operands);
                if !operands.iter().all(|operand| {
                    def_use.definition(*operand).is_some_and(|(def_block, _)| {
                        !body.contains(&def_block) && dominators.dominates(def_block, preheader)
                    })
                }) {
                    continue;
                }
                let mut results = Vec::new();
                effects::defined_values(instruction, &mut results);
                if !results.iter().all(|result| {
                    def_use
                        .uses(*result)
                        .iter()
                        .all(|(use_block, _)| body.contains(use_block))
                }) {
                    continue;
                }
                candidates.push((block.0, index));
            }
        }
        if candidates.is_empty() {
            continue;
        }
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        for (block, index) in candidates {
            let instruction = function.blocks[block].instructions.remove(index);
            function.blocks[preheader.0].instructions.push(instruction);
            hoisted += 1;
        }
    }
    hoisted
}

/// The unique predecessor of the loop header that is outside the loop.
fn preheader_of(cfg: &Cfg, body: &HashSet<BlockId>, header: BlockId) -> Option<BlockId> {
    let mut outside = cfg
        .predecessors(header)
        .iter()
        .copied()
        .filter(|predecessor| !body.contains(predecessor));
    let first = outside.next()?;
    if outside.next().is_some() {
        return None;
    }
    Some(first)
}

fn is_hoistable(instruction: &Instruction, loop_mutates: bool) -> bool {
    match instruction {
        Instruction::Binary { op, .. } => {
            !matches!(op, crate::BinaryOp::Divide | crate::BinaryOp::Modulo)
        }
        Instruction::Unary { .. } => true,
        Instruction::RuntimeCall { function, .. } if !loop_mutates => {
            matches!(
                function,
                BuiltinFunction::StringByteLen
                    | BuiltinFunction::StringCharLen
                    | BuiltinFunction::BytesLen
                    | BuiltinFunction::ListLen
                    | BuiltinFunction::MapLen
                    | BuiltinFunction::ArrayLen
                    | BuiltinFunction::VariadicLen
            )
        }
        _ => false,
    }
}

/// `true` when the loop contains an instruction that could mutate a collection.
fn loop_mutates(function: &Function, body: &HashSet<BlockId>) -> bool {
    body.iter().any(|block| {
        function.blocks[block.0]
            .instructions
            .iter()
            .any(|instruction| {
                matches!(
                    instruction,
                    Instruction::Store { .. }
                        | Instruction::FieldStore { .. }
                        | Instruction::StoreRaw { .. }
                        | Instruction::MakeUnique { .. }
                        | Instruction::Call { .. }
                        | Instruction::StartFuture { .. }
                        | Instruction::CallIndirect { .. }
                        | Instruction::InterfaceCall { .. }
                        | Instruction::RuntimeCall { .. }
                        | Instruction::ConstructInterface { .. }
                        | Instruction::Spawn { .. }
                )
            })
    })
}
