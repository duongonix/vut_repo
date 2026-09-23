//! Common subexpression elimination for pure length reads.
//!
//! Cranelift's egraph already performs GVN/CSE for scalar arithmetic, but Vut
//! runtime calls are opaque to it. The `*Len` builtins are pure reads, so a
//! repeated read of the same receiver in a block can reuse the first result.
//! The table is cleared by any observable instruction (stores, calls, RC ops),
//! which conservatively covers every mutation of the receiver.
use std::collections::{HashMap, HashSet};

use super::{OptimizationReport, effects};
use crate::{BuiltinFunction, Function, Instruction, Program, ValueId};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum PureLen {
    StringBytes,
    StringChars,
    Bytes,
    List,
    Map,
    Array,
    Variadic,
}

fn pure_len(builtin: BuiltinFunction) -> Option<PureLen> {
    match builtin {
        BuiltinFunction::StringByteLen => Some(PureLen::StringBytes),
        BuiltinFunction::StringCharLen => Some(PureLen::StringChars),
        BuiltinFunction::BytesLen => Some(PureLen::Bytes),
        BuiltinFunction::ListLen => Some(PureLen::List),
        BuiltinFunction::MapLen => Some(PureLen::Map),
        BuiltinFunction::ArrayLen => Some(PureLen::Array),
        BuiltinFunction::VariadicLen => Some(PureLen::Variadic),
        _ => None,
    }
}

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.cse_eliminated += eliminate(function);
    }
    report
}

fn eliminate(function: &mut Function) -> usize {
    let mut redundant: Vec<(ValueId, ValueId)> = Vec::new();
    for block in &function.blocks {
        let mut table: HashMap<(PureLen, Vec<ValueId>), ValueId> = HashMap::new();
        for instruction in &block.instructions {
            if let Instruction::RuntimeCall {
                value: Some(value),
                function: builtin,
                arguments,
                ..
            } = instruction
                && let Some(pure) = pure_len(*builtin)
            {
                let key = (pure, arguments.clone());
                if let Some(existing) = table.get(&key).copied() {
                    redundant.push((*value, existing));
                } else {
                    table.insert(key, *value);
                }
                continue;
            }
            if effects::is_observable(instruction) {
                table.clear();
            }
        }
    }
    if redundant.is_empty() {
        return 0;
    }

    for (duplicate, canonical) in &redundant {
        effects::rewrite_uses(function, *duplicate, *canonical);
    }
    let drop: HashSet<ValueId> = redundant.iter().map(|(duplicate, _)| *duplicate).collect();
    for block in &mut function.blocks {
        block.instructions.retain(|instruction| {
            let mut defined = Vec::new();
            effects::defined_values(instruction, &mut defined);
            !defined.iter().any(|value| drop.contains(value))
        });
    }
    redundant.len()
}
