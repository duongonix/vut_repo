//! Value escape analysis.
//!
//! Classifies each SSA value as escaping or non-escaping. A value escapes when
//! it is passed to a call, captured by a closure, spawned as a Vutcon, stored
//! into an aggregate or raw memory, boxed into an interface, or returned. The
//! result is conservative: a value reported as non-escaping is guaranteed not to
//! leave the function, so passes may act on it safely.
use std::collections::HashSet;

use crate::optimize::effects;
use crate::{Function, Instruction, Terminator, ValueId};

#[derive(Clone, Debug, Default)]
pub struct EscapeSummary {
    escaping: HashSet<ValueId>,
}

impl EscapeSummary {
    /// Returns `true` when `value` may escape the function.
    #[must_use]
    pub fn value_escapes(&self, value: ValueId) -> bool {
        self.escaping.contains(&value)
    }

    /// Returns `true` when `value` provably does not escape.
    #[must_use]
    pub fn is_local(&self, value: ValueId) -> bool {
        !self.escaping.contains(&value)
    }
}

/// Computes the escape summary for `function`.
#[must_use]
pub fn analyze(function: &Function) -> EscapeSummary {
    let mut escaping: HashSet<ValueId> = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            #[expect(
                clippy::match_same_arms,
                reason = "each escaping position reads a differently named operand"
            )]
            match instruction {
                Instruction::Call { arguments, .. }
                | Instruction::StartFuture { arguments, .. }
                | Instruction::RuntimeCall { arguments, .. }
                | Instruction::InterfaceCall { arguments, .. }
                | Instruction::CallIndirect { arguments, .. } => {
                    escaping.extend(arguments.iter().copied());
                }
                Instruction::MakeClosure { captures, .. } => {
                    escaping.extend(captures.iter().copied());
                }
                Instruction::Spawn { start, .. } => {
                    escaping.insert(*start);
                }
                Instruction::StoreRaw { value, .. } => {
                    escaping.insert(*value);
                }
                Instruction::FieldStore { value, .. } => {
                    escaping.insert(*value);
                }
                Instruction::ConstructInterface { source, .. } => {
                    escaping.insert(*source);
                }
                _ => {}
            }
        }
        if let Terminator::Return(Some(value)) = &block.terminator {
            escaping.insert(*value);
        }
    }

    // A value produced from an escaping value also escapes.
    loop {
        let mut changed = false;
        for block in &function.blocks {
            for instruction in &block.instructions {
                let mut results = Vec::new();
                effects::defined_values(instruction, &mut results);
                if results.iter().any(|value| escaping.contains(value)) {
                    let mut operands = Vec::new();
                    effects::operands(instruction, &mut operands);
                    for operand in operands {
                        changed |= escaping.insert(operand);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    EscapeSummary { escaping }
}
