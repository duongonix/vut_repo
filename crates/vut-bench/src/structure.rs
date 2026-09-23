//! Deterministic MIR structure metrics.
//!
//! These are compiler-visible, noise-free counters (unlike wall time), so they
//! can be committed to the baseline and used as PR regression gates.

use crate::model::StructureMetrics;
use vut_mir::{BuiltinFunction, Instruction, Program};

#[must_use]
pub fn collect(program: &Program) -> StructureMetrics {
    let mut metrics = StructureMetrics {
        functions: program.functions.len(),
        frame_count: program.frames.len(),
        frame_bytes: program.frames.values().map(|frame| frame.size).sum(),
        ..StructureMetrics::default()
    };
    for function in &program.functions {
        metrics.blocks += function.blocks.len();
        for block in &function.blocks {
            for instruction in &block.instructions {
                metrics.instructions += 1;
                count_instruction(&mut metrics, instruction);
            }
        }
    }
    metrics
}

fn count_instruction(metrics: &mut StructureMetrics, instruction: &Instruction) {
    match instruction {
        Instruction::Allocate { .. } => metrics.allocations += 1,
        Instruction::Retain { .. } => metrics.retains += 1,
        Instruction::Release { .. } => metrics.releases += 1,
        Instruction::MakeUnique { .. } => metrics.make_unique += 1,
        Instruction::Drop(_) => metrics.drops += 1,
        Instruction::RuntimeCall { function, .. } => {
            metrics.runtime_calls += 1;
            match function {
                BuiltinFunction::ListAt => metrics.list_at += 1,
                BuiltinFunction::ListAtUnchecked => metrics.list_at_unchecked += 1,
                BuiltinFunction::ArrayAt => metrics.array_at += 1,
                BuiltinFunction::ArrayAtUnchecked => metrics.array_at_unchecked += 1,
                _ => {}
            }
        }
        Instruction::VariadicAt { in_bounds, .. } => {
            metrics.variadic_at += 1;
            if *in_bounds {
                metrics.variadic_at_in_bounds += 1;
            }
        }
        Instruction::InterfaceCall { .. } => metrics.interface_calls += 1,
        Instruction::CallIndirect { .. } => metrics.indirect_calls += 1,
        Instruction::Call { .. } => metrics.direct_calls += 1,
        Instruction::Spawn { .. } => metrics.spawn += 1,
        Instruction::MakeClosure { .. } => metrics.make_closure += 1,
        _ => {}
    }
}
