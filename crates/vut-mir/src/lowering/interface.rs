//! Interface vtable construction from recorded conversions.
use std::collections::HashSet;

use vut_hir::TypeId;
use vut_resolver::SymbolId;
use vut_types::{SemanticResult, Type};

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
    let mut conversions: HashSet<(TypeId, Option<SymbolId>)> = HashSet::new();
    for function in functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Instruction::ConstructInterface {
                    concrete_ty,
                    interface,
                    ..
                } = instruction
                {
                    conversions.insert((*concrete_ty, *interface));
                }
            }
        }
    }
    let mut entries: Vec<(TypeId, Option<SymbolId>)> = conversions.into_iter().collect();
    entries.sort_by_key(|(concrete_ty, interface)| {
        (
            concrete_ty.0,
            interface.map_or(usize::MAX, |symbol| symbol.0),
        )
    });
    let mut vtables = Vec::new();
    for (concrete_ty, interface) in entries {
        let concrete = match semantics.types[concrete_ty.0] {
            Type::Data(symbol) | Type::Enum(symbol) => Some(symbol),
            _ => None,
        };
        let methods = match (interface, concrete) {
            (Some(symbol), Some(concrete)) => method_names(semantics, symbol)
                .into_iter()
                .filter_map(|name| semantics.method_symbols.get(&(concrete, name)).copied())
                .collect(),
            _ => Vec::new(),
        };
        vtables.push(InterfaceVtable {
            concrete,
            interface,
            concrete_ty,
            methods,
        });
    }
    vtables
}
