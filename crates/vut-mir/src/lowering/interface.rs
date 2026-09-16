//! Interface vtable construction from recorded conversions.
use std::collections::HashMap;

use vut_hir::TypeId;
use vut_resolver::SymbolId;
use vut_types::SemanticResult;

use super::{Function, Instruction, InterfaceVtable};

/// Canonical (sorted) method-name order shared by MIR and codegen vtables.
pub(super) fn method_names(semantics: &SemanticResult, interface: SymbolId) -> Vec<String> {
    let mut names: Vec<String> = semantics
        .interface_shapes
        .get(&interface)
        .map(|shape| shape.methods.keys().cloned().collect())
        .unwrap_or_default();
    names.sort();
    names
}

/// Builds the vtables required by all interface/`dyn` conversions present in
/// `functions`, resolving each interface method to its concrete symbol.
pub(super) fn build_vtables(
    functions: &[Function],
    semantics: &SemanticResult,
) -> Vec<InterfaceVtable> {
    let mut conversions: HashMap<(SymbolId, Option<SymbolId>), TypeId> = HashMap::new();
    for function in functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Instruction::ConstructInterface {
                    concrete_ty,
                    concrete,
                    interface,
                    ..
                } = instruction
                {
                    conversions.insert((*concrete, *interface), *concrete_ty);
                }
            }
        }
    }
    let mut entries: Vec<((SymbolId, Option<SymbolId>), TypeId)> =
        conversions.into_iter().collect();
    entries.sort_by_key(|((concrete, interface), _)| {
        (concrete.0, interface.map_or(usize::MAX, |symbol| symbol.0))
    });
    let mut vtables = Vec::new();
    for ((concrete, interface), concrete_ty) in entries {
        let methods = interface.map_or_else(Vec::new, |symbol| {
            method_names(semantics, symbol)
                .into_iter()
                .filter_map(|name| semantics.method_symbols.get(&(concrete, name)).copied())
                .collect()
        });
        vtables.push(InterfaceVtable {
            concrete,
            interface,
            concrete_ty,
            methods,
        });
    }
    vtables
}
