//! MIR lowering orchestration: HIR functions to MIR functions.
use std::collections::{HashMap, HashSet};

use vut_hir::{HirDeclaration, HirProgram, TypeId};
use vut_memory::LastUse;
use vut_resolver::SymbolId;
use vut_source::Span;
use vut_types::{SemanticResult, Type};

use super::builder::Builder;
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
    let mut program = lower_program(hir, semantics, pointer_size, &call_sites);
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
) -> Program {
    let layouts = LayoutTable::from_semantics(semantics, pointer_size);
    let references: HashMap<_, _> = hir
        .resolved_references
        .iter()
        .map(|item| (item.span, item.symbol))
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
                    call_instances,
                    loop_targets: Vec::new(),
                    terminated: HashSet::new(),
                    return_type,
                    tail_type: None,
                    narrowed: Vec::new(),
                    conditional_depth: 0,
                    receiver_local: function.receiver.map(|_| LocalId(0)),
                    pattern_borrowed: false,
                    variadic_param: None,
                    diagnostics: vut_diagnostics::DiagnosticSink::new(),
                };
                let parameter_offset = usize::from(function.receiver.is_some());
                if let Some(receiver) = function.receiver {
                    builder.add_local("self".into(), None, function.span);
                    builder.initialized.insert(LocalId(0));
                    builder.local_data[0].ty = semantics.types.iter().position(|ty| matches!(ty, Type::Data(symbol) | Type::Enum(symbol) if *symbol == receiver)).map(TypeId);
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
                let returned = builder.lower_statements(&body.statements);
                if !builder.is_terminated() {
                    let returned = return_type
                        .filter(|ty| !matches!(semantics.types[ty.0], Type::Void))
                        .and(returned);
                    // Coerce an implicit tail value into an optional result
                    // (`T` -> `T?`); already-optional values pass through.
                    let returned = returned.map(|value| {
                        builder.coerce_optional(value, builder.tail_type, return_type)
                    });
                    builder.cleanup_except(returned);
                    builder.terminate(Terminator::Return(returned));
                }
                functions.push(Function {
                    symbol: function.symbol,
                    receiver: function.receiver,
                    parameter_count: function.parameters.len()
                        + usize::from(function.receiver.is_some())
                        + usize::from(variadic.is_some()),
                    locals: builder.local_data,
                    blocks: builder.blocks,
                    entry: BlockId(0),
                    return_type,
                    is_async: function.is_async,
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
    };
    program.interface_vtables = super::interface::build_vtables(&program.functions, semantics);
    program
}
