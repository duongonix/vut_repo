//! Devirtualization analysis (M2.6.11).
//!
//! Identifies interface calls that could become direct calls: those whose
//! receiver is a freshly constructed interface with a statically known vtable,
//! and interfaces with a single concrete implementor in the whole program.
//!
//! The analysis is delivered as infrastructure. Rewriting `InterfaceCall` to a
//! direct `Call` additionally needs the box's data pointer as the receiver,
//! which the MIR has no operation to produce today; that rewrite is deferred
//! until an interface-data MIR operation exists.

use std::collections::{HashMap, HashSet};

use crate::{Instruction, Program, SymbolId, ValueId};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DevirtSummary {
    /// Interface calls whose receiver is a freshly constructed interface with a
    /// known vtable.
    pub known_receiver_calls: usize,
    /// Interfaces implemented by exactly one concrete type.
    pub single_implementor_interfaces: usize,
    /// Interface calls on a single-implementor interface.
    pub single_implementor_calls: usize,
}

#[must_use]
pub fn analyze(program: &Program) -> DevirtSummary {
    let mut vtables: HashSet<(usize, Option<SymbolId>)> = HashSet::new();
    let mut implementors: HashMap<Option<SymbolId>, HashSet<usize>> = HashMap::new();
    for vtable in &program.interface_vtables {
        vtables.insert((vtable.concrete_ty.0, vtable.interface));
        implementors
            .entry(vtable.interface)
            .or_default()
            .insert(vtable.concrete_ty.0);
    }
    let single: HashSet<Option<SymbolId>> = implementors
        .iter()
        .filter(|(_, set)| set.len() == 1)
        .map(|(interface, _)| *interface)
        .collect();

    let mut summary = DevirtSummary {
        single_implementor_interfaces: single.len(),
        ..DevirtSummary::default()
    };
    for function in &program.functions {
        let mut constructed: HashMap<ValueId, (usize, Option<SymbolId>)> = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                match instruction {
                    Instruction::ConstructInterface {
                        value,
                        concrete_ty,
                        interface,
                        ..
                    } => {
                        constructed.insert(*value, (concrete_ty.0, *interface));
                    }
                    Instruction::InterfaceCall { callee, .. } => {
                        let Some(key) = constructed.get(callee) else {
                            continue;
                        };
                        if vtables.contains(key) {
                            summary.known_receiver_calls += 1;
                        }
                        if single.contains(&key.1) {
                            summary.single_implementor_calls += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_program_has_no_devirtualization() {
        let program = Program {
            functions: Vec::new(),
            external_functions: Vec::new(),
            layouts: crate::LayoutTable {
                types: Vec::new(),
                pointer_size: 8,
                fields: std::collections::HashMap::new(),
                arrays: std::collections::HashMap::new(),
                lists: std::collections::HashMap::new(),
                maps: std::collections::HashMap::new(),
                channels: std::collections::HashMap::new(),
                results: std::collections::HashMap::new(),
                enums: std::collections::HashMap::new(),
                optionals: std::collections::HashMap::new(),
                callables: std::collections::HashMap::new(),
                aggregates: std::collections::HashSet::new(),
                unsigned: std::collections::HashSet::new(),
            },
            interface_vtables: Vec::new(),
            diagnostics: vut_diagnostics::DiagnosticSink::new(),
            frames: std::collections::HashMap::new(),
            awaits: std::collections::HashMap::new(),
            closure_layouts: std::collections::HashMap::new(),
        };
        assert_eq!(analyze(&program), DevirtSummary::default());
    }
}
