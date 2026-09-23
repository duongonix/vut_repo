//! MIR lowering orchestration: HIR functions to MIR functions.
use std::collections::{HashMap, HashSet};

use vut_hir::{HirDeclaration, HirProgram, TypeId};
use vut_memory::LastUse;
use vut_resolver::SymbolId;
use vut_source::Span;
use vut_types::{SemanticResult, Type};

use super::builder::Builder;
use super::ir::{ClosureCapture, ClosureLayout};
use super::monomorph::lower_instances;
use super::uses::count_uses;
use super::{
    BasicBlock, BlockId, ExternalFunction, Function, LayoutTable, LocalId, Program, Terminator,
};

#[must_use]
pub fn lower(hir: &HirProgram, semantics: &SemanticResult, pointer_size: usize) -> Program {
    let base_layouts = LayoutTable::from_semantics(semantics, pointer_size);
    let instances = lower_instances(hir, semantics, pointer_size, &base_layouts);
    let call_sites = instances.call_sites.clone();
    let mut program = lower_program(
        hir,
        semantics,
        pointer_size,
        &call_sites,
        &instances.templates,
    );
    program.merge_instances(instances);
    program.interface_vtables = super::interface::build_vtables(&program.functions, semantics);
    super::future::lower(&mut program);
    program
}

#[expect(
    clippy::too_many_lines,
    reason = "function lowering keeps parameter setup and body lowering together"
)]
pub(crate) fn lower_program(
    hir: &HirProgram,
    semantics: &SemanticResult,
    pointer_size: usize,
    call_instances: &HashMap<Span, SymbolId>,
    instance_templates: &HashMap<SymbolId, SymbolId>,
) -> Program {
    let layouts = LayoutTable::from_semantics(semantics, pointer_size);
    // Precompute heap environment layouts for every capturing closure body so
    // both the body lowering and the creation site share one layout.
    let mut closure_layouts: HashMap<SymbolId, ClosureLayout> = HashMap::new();
    for module in &hir.modules {
        for declaration in &module.declarations {
            let HirDeclaration::Function(function) = declaration else {
                continue;
            };
            if !function.is_closure || !function.type_parameters.is_empty() {
                continue;
            }
            if let Some(captures) = semantics.closure_captures.get(&function.span)
                && !captures.is_empty()
            {
                closure_layouts.insert(function.symbol, closure_layout(captures, &layouts));
            }
        }
    }
    let references: HashMap<_, _> = hir
        .resolved_references
        .iter()
        .map(|item| (item.span, item.symbol))
        .collect();
    let parameter_names: HashMap<SymbolId, Vec<String>> = hir
        .modules
        .iter()
        .flat_map(|module| &module.declarations)
        .filter_map(|declaration| match declaration {
            HirDeclaration::Function(function) => Some((
                function.symbol,
                function
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .collect(),
            )),
            _ => None,
        })
        .collect();
    let mut functions = Vec::new();
    let mut external_functions = Vec::new();
    let mut diagnostics = vut_diagnostics::DiagnosticSink::new();
    for module in &hir.modules {
        for declaration in &module.declarations {
            if let HirDeclaration::Function(function) = declaration {
                // Generic templates are not concrete code; monomorphization
                // lowers each instantiation instead.
                if !function.type_parameters.is_empty() {
                    continue;
                }
                let signature = semantics.function_signatures.get(&function.symbol);
                let return_type = signature.map(|signature| {
                    // An async extern returns a native future handle at the ABI
                    // level; its logical result is produced when awaited.
                    if semantics.async_externs.contains(&function.symbol) {
                        semantics
                            .types
                            .iter()
                            .position(|ty| matches!(ty, Type::Future(inner) if *inner == signature.result))
                            .map_or(signature.result, TypeId)
                    } else {
                        signature.result
                    }
                });
                let Some(body) = &function.body else {
                    external_functions.push(ExternalFunction {
                        symbol: function.symbol,
                        link_name: semantics
                            .attributes
                            .link_names
                            .get(&function.symbol)
                            .cloned()
                            .or_else(|| function.external_name.clone())
                            .unwrap_or_else(|| format!("vut_fn_{}", function.symbol.0)),
                        parameters: signature
                            .map(|signature| signature.parameters.clone())
                            .unwrap_or_default(),
                        return_type,
                    });
                    continue;
                };
                let closure_layout = closure_layouts.get(&function.symbol).cloned();
                let mut builder = Builder {
                    semantics,
                    layouts: &layouts,
                    locals: HashMap::new(),
                    local_data: Vec::new(),
                    blocks: vec![BasicBlock {
                        instructions: Vec::new(),
                        terminator: Terminator::Unreachable,
                    }],
                    current: BlockId(0),
                    next_value: 0,
                    remaining_uses: LastUse::from_counts(count_uses(body)),
                    moved: HashSet::new(),
                    initialized: HashSet::new(),
                    references: &references,
                    lambda_symbols: &hir.lambda_symbols,
                    field_defaults: &semantics.data_field_defaults,
                    parameter_defaults: &semantics.parameter_defaults,
                    parameter_names: &parameter_names,
                    instance_templates,
                    call_instances,
                    loop_targets: Vec::new(),
                    terminated: HashSet::new(),
                    return_type,
                    tail_type: None,
                    narrowed: Vec::new(),
                    conditional_depth: 0,
                    receiver_local: function
                        .receiver
                        .map(|_| LocalId(usize::from(closure_layout.is_some()))),
                    pattern_borrowed: false,
                    borrowed_locals: HashSet::new(),
                    variadic_param: None,
                    pending_writeback: Vec::new(),
                    diagnostics: vut_diagnostics::DiagnosticSink::new(),
                    constant_depth: 0,
                };
                let env_offset = usize::from(closure_layout.is_some());
                if closure_layout.is_some() {
                    let int_type = semantics
                        .types
                        .iter()
                        .position(|ty| matches!(ty, Type::Int))
                        .map(TypeId);
                    let env_local = builder.add_local("$env".into(), None, function.span);
                    builder.initialized.insert(env_local);
                    builder.local_data[env_local.0].ty = int_type;
                }
                let parameter_offset = env_offset + usize::from(function.receiver.is_some());
                if let Some(receiver) = function.receiver {
                    let receiver_local = builder.add_local("self".into(), None, function.span);
                    builder.initialized.insert(receiver_local);
                    builder.local_data[receiver_local.0].ty = semantics.types.iter().position(|ty| matches!(ty, Type::Data(symbol) | Type::Enum(symbol) if *symbol == receiver)).map(TypeId);
                }
                let mut variadic_param = None;
                let variadic = signature.and_then(|signature| signature.variadic);
                let fixed = function
                    .parameters
                    .len()
                    .saturating_sub(usize::from(variadic.is_some()));
                for (index, parameter) in function.parameters.iter().take(fixed).enumerate() {
                    builder.add_local(parameter.name.clone(), None, parameter.span);
                    builder
                        .initialized
                        .insert(LocalId(index + parameter_offset));
                    builder.local_data[index + parameter_offset].ty =
                        signature.and_then(|signature| signature.parameters.get(index).copied());
                }
                if let Some(element) = variadic {
                    let int_type = semantics
                        .types
                        .iter()
                        .position(|ty| matches!(ty, Type::Int))
                        .map(TypeId);
                    let data = builder.add_local("$variadic_data".into(), None, function.span);
                    builder.initialized.insert(data);
                    builder.local_data[data.0].ty = int_type;
                    let len = builder.add_local("$variadic_len".into(), None, function.span);
                    builder.initialized.insert(len);
                    builder.local_data[len.0].ty = int_type;
                    if let Some(parameter) = function.parameters.get(fixed) {
                        variadic_param = Some(super::builder::VariadicParam {
                            name: parameter.name.clone(),
                            element,
                            data,
                            len,
                        });
                    }
                }
                builder.variadic_param = variadic_param;
                if let Some(layout) = &closure_layout {
                    let names = semantics
                        .closure_captures
                        .get(&function.span)
                        .cloned()
                        .unwrap_or_default();
                    builder.setup_closure_captures(layout, &names, function.span);
                }
                let returned = builder.lower_statements(&body.statements);
                if !builder.is_terminated() {
                    let returned = return_type
                        .filter(|ty| !matches!(semantics.types[ty.0], Type::Void))
                        .and(returned);
                    // Coerce an implicit tail value into the result type
                    // (`T` -> `T?`, `T` -> `interface`/`dyn`).
                    let returned = returned
                        .map(|value| builder.coerce_value(value, builder.tail_type, return_type));
                    builder.cleanup_except(returned);
                    builder.terminate(Terminator::Return(returned));
                }
                functions.push(Function {
                    symbol: function.symbol,
                    receiver: function.receiver,
                    parameter_count: function.parameters.len()
                        + env_offset
                        + usize::from(function.receiver.is_some())
                        + usize::from(variadic.is_some()),
                    locals: builder.local_data,
                    blocks: builder.blocks,
                    entry: BlockId(0),
                    return_type,
                    is_async: function.is_async
                        || semantics.async_symbols.contains(&function.symbol),
                    frame_param: None,
                    is_poll: false,
                    out_param: None,
                });
                diagnostics.extend(std::mem::take(&mut builder.diagnostics));
            }
        }
    }
    let mut program = Program {
        functions,
        external_functions,
        layouts,
        interface_vtables: Vec::new(),
        diagnostics,
        frames: std::collections::HashMap::new(),
        awaits: std::collections::HashMap::new(),
        closure_layouts,
    };
    program.interface_vtables = super::interface::build_vtables(&program.functions, semantics);
    program
}

/// Computes aligned byte offsets for a closure environment from its capture
/// types.
fn closure_layout(captures: &[(String, TypeId)], layouts: &LayoutTable) -> ClosureLayout {
    let mut offset = 0_usize;
    let mut alignment = 1_usize;
    let mut fields = Vec::with_capacity(captures.len());
    for (_, ty) in captures {
        let info = &layouts.types[ty.0];
        let align = info.alignment.max(1);
        alignment = alignment.max(align);
        offset = offset.div_ceil(align) * align;
        fields.push(ClosureCapture { ty: *ty, offset });
        offset += info.size.max(1);
    }
    ClosureLayout {
        size: offset,
        alignment,
        captures: fields,
    }
}
