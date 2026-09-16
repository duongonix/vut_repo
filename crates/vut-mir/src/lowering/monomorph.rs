//! Monomorphization of generic function specializations.
//!
//! The type checker records every concrete `(template, arguments)` call. For
//! each specialization this module lowers a dedicated copy of the template
//! function body against a type-substituted view of the semantics, producing
//! concrete MIR with specialized layouts.
use std::collections::HashMap;

use vut_hir::{HirDeclaration, HirModule, HirProgram, TypeId};
use vut_resolver::SymbolId;
use vut_types::{
    DataFieldInfo, FunctionSignatureInfo, SemanticResult, VariantFieldInfo, VariantInfo,
};

use super::{Function, LayoutTable, Program};

/// Concrete instance symbols and the call-site retargeting table.
pub(super) struct Instances {
    pub functions: Vec<Function>,
    pub call_sites: HashMap<vut_source::Span, SymbolId>,
    pub diagnostics: vut_diagnostics::DiagnosticSink,
}

/// Builds the substituted semantics and MIR for every requested specialization.
pub(super) fn lower_instances(
    hir: &HirProgram,
    semantics: &SemanticResult,
    pointer_size: usize,
    base_layouts: &LayoutTable,
) -> Instances {
    let mut next_symbol = next_instance_symbol(hir).max(semantics.next_instance_symbol);
    let mut symbols: HashMap<(SymbolId, Vec<TypeId>), SymbolId> = HashMap::new();
    let mut call_sites = HashMap::new();
    let mut functions = Vec::new();
    let mut diagnostics = vut_diagnostics::DiagnosticSink::new();

    for call in &semantics.generic_function_calls {
        let key = (call.template, call.arguments.clone());
        let instance = *symbols.entry(key).or_insert_with(|| {
            let symbol = SymbolId(next_symbol);
            next_symbol += 1;
            symbol
        });
        call_sites.insert(call.call_span, instance);
    }

    let mut ordered: Vec<((SymbolId, Vec<TypeId>), SymbolId)> = symbols
        .iter()
        .map(|(key, value)| (key.clone(), *value))
        .collect();
    ordered.sort_by_key(|(_, instance)| instance.0);

    for ((template, arguments), instance) in ordered {
        let Some(substitution) = semantics
            .generic_substitutions
            .get(&(template, arguments.clone()))
        else {
            continue;
        };
        let Some(function) = find_function(hir, template) else {
            continue;
        };
        let instance_semantics = substitute_semantics(semantics, substitution);
        let mut instance_semantics = instance_semantics;
        if let Some(signature) = semantics.function_signatures.get(&template) {
            let map = |ty: TypeId| substitution.get(&ty).copied().unwrap_or(ty);
            instance_semantics.function_signatures.insert(
                instance,
                FunctionSignatureInfo {
                    receiver: signature.receiver.map(map),
                    parameters: signature.parameters.iter().copied().map(map).collect(),
                    variadic: signature.variadic.map(map),
                    result: map(signature.result),
                },
            );
        }
        let mut instance_function = function.clone();
        instance_function.symbol = instance;
        instance_function.type_parameters = Vec::new();
        let instance_hir = HirProgram {
            modules: vec![single_function_module(hir, instance_function)],
            resolved_references: hir.resolved_references.clone(),
            lambda_symbols: hir.lambda_symbols.clone(),
        };
        let program = super::lower::lower_program(
            &instance_hir,
            &instance_semantics,
            pointer_size,
            &call_sites,
        );
        let mut lowered = program.functions;
        functions.append(&mut lowered);
        diagnostics.extend(program.diagnostics);
    }

    // Reuse the base layouts: substituted concrete `TypeId`s live in the same
    // type table, so the main layout table already describes them.
    let _ = base_layouts;
    Instances {
        functions,
        call_sites,
        diagnostics,
    }
}

fn next_instance_symbol(hir: &HirProgram) -> usize {
    let mut next = 0;
    for module in &hir.modules {
        for declaration in &module.declarations {
            if let HirDeclaration::Function(function) = declaration {
                next = next.max(function.symbol.0 + 1);
            }
        }
    }
    for reference in &hir.resolved_references {
        next = next.max(reference.symbol.0 + 1);
    }
    next
}

/// Constructor templates whose applied form became a concrete instance symbol.
fn concrete_instances(
    base: &SemanticResult,
    substitution: &HashMap<TypeId, TypeId>,
) -> HashMap<SymbolId, SymbolId> {
    let mut instances = HashMap::new();
    for (old, new) in substitution {
        if let vut_types::Type::Applied(template, _) = base.types[old.0]
            && let vut_types::Type::Data(instance) | vut_types::Type::Enum(instance) =
                base.types[new.0]
        {
            instances.insert(template, instance);
        }
    }
    instances
}

/// Binds deferred bound method calls to concrete methods for a specialization.
///
/// A bound method call on a type parameter resolves, for this concrete
/// specialization, to the implementing type's method of the same name; the
/// result is an ordinary concrete call target with no runtime dispatch. The
/// checker validates that the concrete argument satisfies the bound and reports
/// `E1017` otherwise, so an unresolvable call is not reachable on a program
/// that passes checking and is skipped here rather than duplicated.
fn resolve_bound_calls(
    base: &SemanticResult,
    substitution: &HashMap<TypeId, TypeId>,
    call_targets: &mut HashMap<vut_source::Span, SymbolId>,
) {
    for (span, bound) in &base.bound_calls {
        let receiver = substitution
            .get(&bound.receiver)
            .copied()
            .unwrap_or(bound.receiver);
        if let Some(symbol) = resolve_concrete_method(base, receiver, &bound.method) {
            call_targets.insert(*span, symbol);
        }
    }
}

/// Concrete implementing method for a resolved bound call receiver.
///
/// Only directly-named data/enum types are handled in this revision; generic
/// instances and type constructors are out of scope (deferred to G3).
fn resolve_concrete_method(base: &SemanticResult, ty: TypeId, name: &str) -> Option<SymbolId> {
    match &base.types[ty.0] {
        vut_types::Type::Data(symbol) | vut_types::Type::Enum(symbol) => base
            .method_symbols
            .get(&(*symbol, name.to_owned()))
            .copied(),
        _ => None,
    }
}

fn find_function(hir: &HirProgram, symbol: SymbolId) -> Option<&vut_hir::HirFunction> {
    for module in &hir.modules {
        for declaration in &module.declarations {
            if let HirDeclaration::Function(function) = declaration
                && function.symbol == symbol
            {
                return Some(function);
            }
        }
    }
    None
}

fn single_function_module(hir: &HirProgram, function: vut_hir::HirFunction) -> HirModule {
    let source = function.span.source();
    let module = hir
        .modules
        .iter()
        .find(|module| module.source == source)
        .or_else(|| hir.modules.first());
    HirModule {
        id: module.map_or(vut_resolver::ModuleId(0), |module| module.id),
        source,
        imports: Vec::new(),
        declarations: vec![HirDeclaration::Function(function)],
    }
}

/// Clones the semantics with every `TypeId` remapped through `substitution`.
pub(super) fn substitute_semantics(
    base: &SemanticResult,
    substitution: &HashMap<TypeId, TypeId>,
) -> SemanticResult {
    let map = |ty: TypeId| substitution.get(&ty).copied().unwrap_or(ty);
    let template_instances = concrete_instances(base, substitution);
    let mut call_targets: HashMap<vut_source::Span, SymbolId> = base
        .call_targets
        .iter()
        .map(|(span, symbol)| {
            (
                *span,
                template_instances.get(symbol).copied().unwrap_or(*symbol),
            )
        })
        .collect();
    resolve_bound_calls(base, substitution, &mut call_targets);
    SemanticResult {
        types: base.types.clone(),
        expression_types: base
            .expression_types
            .iter()
            .map(|(span, ty)| (*span, map(*ty)))
            .collect(),
        diagnostics: vut_diagnostics::DiagnosticSink::new(),
        interface_shapes: base.interface_shapes.clone(),
        interface_satisfaction: base.interface_satisfaction.clone(),
        data_fields: base
            .data_fields
            .iter()
            .map(|(symbol, fields)| {
                (
                    *symbol,
                    fields
                        .iter()
                        .map(|field| DataFieldInfo {
                            ty: map(field.ty),
                            ..field.clone()
                        })
                        .collect(),
                )
            })
            .collect(),
        data_field_defaults: base.data_field_defaults.clone(),
        opaque_data: base.opaque_data.clone(),
        function_signatures: base
            .function_signatures
            .iter()
            .map(|(symbol, signature)| {
                (
                    *symbol,
                    FunctionSignatureInfo {
                        receiver: signature.receiver.map(map),
                        parameters: signature.parameters.iter().copied().map(map).collect(),
                        variadic: signature.variadic.map(map),
                        result: map(signature.result),
                    },
                )
            })
            .collect(),
        builtin_functions: base.builtin_functions.clone(),
        builtin_calls: base.builtin_calls.clone(),
        call_targets,
        receiver_calls: base.receiver_calls.clone(),
        implicit_receiver_calls: base.implicit_receiver_calls.clone(),
        bound_calls: HashMap::new(),
        async_symbols: base.async_symbols.clone(),
        async_externs: base.async_externs.clone(),
        extern_symbols: base.extern_symbols.clone(),
        enum_variants: base
            .enum_variants
            .iter()
            .map(|(symbol, variants)| {
                (
                    *symbol,
                    variants
                        .iter()
                        .map(|variant| VariantInfo {
                            name: variant.name.clone(),
                            fields: variant
                                .fields
                                .iter()
                                .map(|field| VariantFieldInfo {
                                    name: field.name.clone(),
                                    ty: map(field.ty),
                                })
                                .collect(),
                        })
                        .collect(),
                )
            })
            .collect(),
        variant_constructions: base.variant_constructions.clone(),
        method_symbols: base.method_symbols.clone(),
        attributes: base.attributes.clone(),
        static_methods: base.static_methods.clone(),
        generic_params: base.generic_params.clone(),
        generic_param_bounds: base.generic_param_bounds.clone(),
        generic_function_calls: base.generic_function_calls.clone(),
        generic_type_applications: base.generic_type_applications.clone(),
        generic_substitutions: base.generic_substitutions.clone(),
        next_instance_symbol: base.next_instance_symbol,
    }
}

impl Program {
    /// Merges specialization functions and call retargeting into a program.
    pub(super) fn merge_instances(&mut self, instances: Instances) {
        self.functions.extend(instances.functions);
        self.diagnostics.extend(instances.diagnostics);
    }
}
