//! Interprocedural call graph.
//!
//! Records direct call edges and the set of address-taken functions (whose
//! address escapes as a function/closure value). Used by inlining and, later, by
//! devirtualization and whole-program reachability.

use std::collections::{HashMap, HashSet};

use crate::{Instruction, Program, SymbolId};

#[derive(Clone, Debug, Default)]
pub struct CallGraph {
    edges: HashMap<SymbolId, HashSet<SymbolId>>,
    address_taken: HashSet<SymbolId>,
    has_indirect_calls: bool,
}

impl CallGraph {
    #[must_use]
    pub fn build(program: &Program) -> Self {
        let mut graph = Self::default();
        for function in &program.functions {
            for block in &function.blocks {
                for instruction in &block.instructions {
                    match instruction {
                        Instruction::Call { target, .. }
                        | Instruction::StartFuture { target, .. } => {
                            graph
                                .edges
                                .entry(function.symbol)
                                .or_default()
                                .insert(*target);
                        }
                        Instruction::MakeFunction { symbol, .. }
                        | Instruction::MakeClosure { symbol, .. } => {
                            graph.address_taken.insert(*symbol);
                        }
                        Instruction::Spawn { callable, .. } => {
                            graph.address_taken.insert(*callable);
                        }
                        Instruction::CallIndirect { .. } | Instruction::InterfaceCall { .. } => {
                            graph.has_indirect_calls = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        graph
    }

    #[must_use]
    pub fn callees(&self, symbol: SymbolId) -> &HashSet<SymbolId> {
        static EMPTY: std::sync::OnceLock<HashSet<SymbolId>> = std::sync::OnceLock::new();
        self.edges
            .get(&symbol)
            .unwrap_or_else(|| EMPTY.get_or_init(HashSet::new))
    }

    #[must_use]
    pub fn is_address_taken(&self, symbol: SymbolId) -> bool {
        self.address_taken.contains(&symbol)
    }

    #[must_use]
    pub fn has_indirect_calls(&self) -> bool {
        self.has_indirect_calls
    }

    /// Every function reachable from `roots` through direct call edges.
    #[must_use]
    pub fn reachable_from(&self, roots: &[SymbolId]) -> HashSet<SymbolId> {
        let mut visited = HashSet::new();
        let mut stack: Vec<SymbolId> = roots.to_vec();
        while let Some(symbol) = stack.pop() {
            if !visited.insert(symbol) {
                continue;
            }
            for callee in self.callees(symbol) {
                stack.push(*callee);
            }
        }
        visited
    }
}
