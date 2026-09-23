//! Native object compilation lifecycle.
use super::futures::{define_entry_shim, define_future_thunks};
use super::instruction::lower_instruction;
use super::managed::manage_value;
use super::signatures::{
    c_call_conv, coerce_integer, copy_aggregate, external_signature_for, machine_type,
    signature_for,
};
use super::{CodegenError, CraneliftBackend};

use cranelift_codegen::{
    Context,
    ir::{AbiParam, Function, InstBuilder, MemFlagsData, Signature, UserFuncName, types},
    isa,
    settings::{self, Configurable as _},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;
use target_lexicon::Triple;
use vut_hir::TypeId;
use vut_mir::{Program, Terminator};

#[expect(
    clippy::too_many_lines,
    reason = "Cranelift module lifecycle remains visible in one backend operation"
)]
pub(super) fn compile(
    backend: &CraneliftBackend,
    program: &Program,
    entry: Option<(vut_mir::SymbolId, bool)>,
) -> Result<Vec<u8>, CodegenError> {
    let triple: Triple = backend
        .target
        .triple
        .parse()
        .map_err(|error| CodegenError::InvalidTarget(format!("invalid target: {error}")))?;
    let mut flag_builder = settings::builder();
    // Emit position-independent code. Required by platforms that forbid text
    // relocations (macOS arm64) and harmless on Windows/Linux, where it keeps
    // the object compatible with PIE-based linkers.
    flag_builder
        .set("is_pic", "true")
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    if backend.level.is_optimizing() {
        flag_builder
            .set("opt_level", "speed")
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
    }
    // The Cranelift verifier is on by default (development/tests/CI). Production
    // builds may disable it with `VUT_CL_VERIFIER=0` to reduce compile time; the
    // policy is independent of the optimization level.
    let verifier = std::env::var("VUT_CL_VERIFIER").map_or(true, |value| value != "0");
    flag_builder
        .set("enable_verifier", if verifier { "true" } else { "false" })
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    let flags = settings::Flags::new(flag_builder);
    let mut isa_builder =
        isa::lookup(triple).map_err(|error| CodegenError::InvalidTarget(error.to_string()))?;
    super::features::configure(
        &mut isa_builder,
        &backend.target.triple,
        backend.target.cpu.as_deref(),
        &backend.target.features,
    )?;
    let isa = isa_builder
        .finish(flags)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, "vut", default_libcall_names())
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    let mut module = ObjectModule::new(builder);
    let mut declarations = HashMap::new();
    let mut next_data = 0_usize;
    for external in &program.external_functions {
        let signature = external_signature_for(&module, external, &program.layouts);
        let id = module
            .declare_function(&external.link_name, Linkage::Import, &signature)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        declarations.insert(external.symbol, (id, signature));
    }
    for mir in &program.functions {
        let signature = signature_for(&module, mir, &program.layouts);
        let id = module
            .declare_function(
                &format!("vut_fn_{}", mir.symbol.0),
                Linkage::Export,
                &signature,
            )
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        declarations.insert(mir.symbol, (id, signature));
    }
    // Declare one resume/drop thunk per async function that owns a frame. The
    // resume thunk reconstructs the body arguments from the frame, runs the
    // body, and writes the result into the polled output slot; the drop thunk
    // releases the frame exactly once.
    let mut future_thunks: HashMap<vut_mir::SymbolId, (FuncId, FuncId)> = HashMap::new();
    for mir in &program.functions {
        if !mir.is_async || !program.frames.contains_key(&mir.symbol) {
            continue;
        }
        let mut resume_signature = Signature::new(c_call_conv(&module));
        resume_signature.params.push(AbiParam::new(types::I64));
        resume_signature.params.push(AbiParam::new(types::I64));
        resume_signature.returns.push(AbiParam::new(types::I32));
        let resume = if mir.is_poll {
            // A state-machine body is its own `(frame, out) -> i32` poll entry.
            declarations[&mir.symbol].0
        } else {
            module
                .declare_function(
                    &format!("vut_future_resume_{}", mir.symbol.0),
                    Linkage::Local,
                    &resume_signature,
                )
                .map_err(|error| CodegenError::Backend(error.to_string()))?
        };
        let mut drop_signature = Signature::new(c_call_conv(&module));
        drop_signature.params.push(AbiParam::new(types::I64));
        let drop = module
            .declare_function(
                &format!("vut_future_drop_{}", mir.symbol.0),
                Linkage::Local,
                &drop_signature,
            )
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        future_thunks.insert(mir.symbol, (resume, drop));
    }
    // A sync callable spawned by `vut(...)` gets a task poll thunk that runs the
    // body on its first poll, plus a no-op drop (it owns no frame). These
    // converge with async task thunks on the same `(op, out) -> i32` contract.
    for mir in &program.functions {
        for block in &mir.blocks {
            for instruction in &block.instructions {
                let vut_mir::Instruction::Spawn { callable, .. } = instruction else {
                    continue;
                };
                if program.frames.contains_key(callable) || future_thunks.contains_key(callable) {
                    continue;
                }
                let mut poll_signature = Signature::new(c_call_conv(&module));
                poll_signature.params.push(AbiParam::new(types::I64));
                poll_signature.params.push(AbiParam::new(types::I64));
                poll_signature.returns.push(AbiParam::new(types::I32));
                let poll = module
                    .declare_function(
                        &format!("vut_task_sync_{}", callable.0),
                        Linkage::Local,
                        &poll_signature,
                    )
                    .map_err(|error| CodegenError::Backend(error.to_string()))?;
                let mut drop_signature = Signature::new(c_call_conv(&module));
                drop_signature.params.push(AbiParam::new(types::I64));
                let drop = module
                    .declare_function(
                        &format!("vut_task_sync_drop_{}", callable.0),
                        Linkage::Local,
                        &drop_signature,
                    )
                    .map_err(|error| CodegenError::Backend(error.to_string()))?;
                future_thunks.insert(*callable, (poll, drop));
            }
        }
    }
    // Generate one retain/release callback per managed type so the runtime can
    // manage collection elements of any element type, including aggregates.
    let mut callback_signature = Signature::new(c_call_conv(&module));
    callback_signature.params.push(AbiParam::new(types::I64));
    let mut needed_types = std::collections::HashSet::new();
    for mir in &program.functions {
        for block in &mir.blocks {
            for instruction in &block.instructions {
                match instruction {
                    vut_mir::Instruction::TypeRetain { ty, .. }
                    | vut_mir::Instruction::TypeRelease { ty, .. } => {
                        needed_types.insert(ty.0);
                    }
                    _ => {}
                }
            }
        }
    }
    let mut type_functions: HashMap<(usize, bool), FuncId> = HashMap::new();
    for index in needed_types {
        let info = program.layouts.types[index];
        if !info.needs_drop {
            continue;
        }
        // A move-only type (resource/future/vutcon) has no retain callback; the
        // runtime transfer operations never duplicate it.
        let variants: &[bool] = if info.ownership.is_linear() {
            &[false]
        } else {
            &[true, false]
        };
        for &retain in variants {
            let name = format!(
                "vut_type_{}_{}",
                if retain { "retain" } else { "release" },
                index
            );
            let id = module
                .declare_function(&name, Linkage::Local, &callback_signature)
                .map_err(|error| CodegenError::Backend(error.to_string()))?;
            type_functions.insert((index, retain), id);
        }
    }
    // One drop thunk per capturing closure that owns managed captures. The
    // runtime calls it when the closure's last reference is released.
    let mut closure_drops: HashMap<vut_mir::SymbolId, FuncId> = HashMap::new();
    for (symbol, layout) in &program.closure_layouts {
        if !layout
            .captures
            .iter()
            .any(|capture| program.layouts.types[capture.ty.0].needs_drop)
        {
            continue;
        }
        let name = format!("vut_closure_drop_{}", symbol.0);
        let id = module
            .declare_function(&name, Linkage::Local, &callback_signature)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        closure_drops.insert(*symbol, id);
    }
    // One drop thunk per boxed concrete type; each interface/dyn vtable stores
    // the thunk in slot zero ahead of the concrete method addresses.
    let mut interface_drops: HashMap<usize, FuncId> = HashMap::new();
    for vtable in &program.interface_vtables {
        if interface_drops.contains_key(&vtable.concrete_ty.0) {
            continue;
        }
        let name = format!("vut_interface_drop_{}", vtable.concrete_ty.0);
        let id = module
            .declare_function(&name, Linkage::Local, &callback_signature)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        interface_drops.insert(vtable.concrete_ty.0, id);
    }
    let mut vtables: HashMap<(usize, Option<vut_mir::SymbolId>), DataId> = HashMap::new();
    for (index, vtable) in program.interface_vtables.iter().enumerate() {
        let name = format!("vut_interface_vtable_{index}");
        let data = module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        vtables.insert((vtable.concrete_ty.0, vtable.interface), data);
    }
    for mir in &program.functions {
        let (function_id, signature) = declarations[&mir.symbol].clone();
        let symbol_index = u32::try_from(mir.symbol.0)
            .map_err(|_| CodegenError::Backend("too many symbols for backend namespace".into()))?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(0, symbol_index),
            signature,
        ));
        let frontend_config = module.target_config();
        let mut builder_context = FunctionBuilderContext::new();
        let mut function = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let blocks: Vec<_> = mir.blocks.iter().map(|_| function.create_block()).collect();
        function.append_block_params_for_function_params(blocks[mir.entry.0]);
        let variables: Vec<_> = mir
            .locals
            .iter()
            .map(|local| {
                function.declare_var(
                    local
                        .ty
                        .map_or(types::I64, |ty| machine_type(&program.layouts, ty)),
                )
            })
            .collect();
        let local_types: Vec<_> = mir.locals.iter().map(|local| local.ty).collect();
        let storages: Vec<vut_mir::LocalStorage> =
            mir.locals.iter().map(|local| local.storage).collect();
        let frame_local = mir.frame_param.map(vut_mir::LocalId);
        let mut values = HashMap::new();
        let mut value_types = HashMap::new();
        let mut spill_types = HashMap::new();
        function.switch_to_block(blocks[mir.entry.0]);
        let sret = mir
            .return_type
            .filter(|ty| !mir.is_poll && program.layouts.is_aggregate(*ty));
        let sret_variable = sret.map(|_| function.declare_var(types::I64));
        let sret_offset = usize::from(sret_variable.is_some());
        let parameters = function.block_params(blocks[mir.entry.0]).to_vec();
        if let Some(frame_param) = mir.frame_param {
            // Async body: parameters are `[sret?][frame]` (plus `out` for a poll
            // body).
            if let (Some(variable), Some(value)) = (sret_variable, parameters.first().copied()) {
                function.def_var(variable, value);
            }
            let frame = parameters
                .get(sret_offset)
                .copied()
                .unwrap_or_else(|| function.ins().iconst(types::I64, 0));
            function.def_var(variables[frame_param], frame);
            if let Some(out_param) = mir.out_param {
                let out = parameters
                    .get(sret_offset.saturating_add(1))
                    .copied()
                    .unwrap_or_else(|| function.ins().iconst(types::I64, 0));
                function.def_var(variables[out_param], out);
            }
        } else {
            for (index, value) in parameters.into_iter().enumerate() {
                if let Some(variable) = sret_variable
                    && index == 0
                {
                    function.def_var(variable, value);
                } else {
                    function.def_var(variables[index - sret_offset], value);
                }
            }
        }
        for (index, mir_block) in mir.blocks.iter().enumerate() {
            if index != mir.entry.0 {
                function.switch_to_block(blocks[index]);
            }
            for instruction in &mir_block.instructions {
                lower_instruction(
                    &mut function,
                    &mut module,
                    &declarations,
                    instruction,
                    &variables,
                    &local_types,
                    &storages,
                    frame_local,
                    &mut values,
                    &mut value_types,
                    &mut spill_types,
                    &program.layouts,
                    &type_functions,
                    &vtables,
                    &program.frames,
                    &future_thunks,
                    &program.closure_layouts,
                    &closure_drops,
                    &mut next_data,
                )?;
            }
            match mir_block.terminator {
                Terminator::Return(Some(value)) if mir.is_poll => {
                    let out =
                        function.use_var(variables[mir.out_param.expect("poll body has out")]);
                    let ty = mir.return_type.expect("poll body has a result type");
                    if program.layouts.is_aggregate(ty) {
                        copy_aggregate(&mut function, &program.layouts, ty, values[&value], out)?;
                    } else {
                        function
                            .ins()
                            .store(MemFlagsData::trusted(), values[&value], out, 0);
                    }
                    let ready = function.ins().iconst(types::I32, 1);
                    function.ins().return_(&[ready]);
                }
                Terminator::Return(Some(value)) => {
                    let mut returned = values[&value];
                    if let (Some(ty), Some(variable)) = (sret, sret_variable) {
                        let destination = function.use_var(variable);
                        copy_aggregate(&mut function, &program.layouts, ty, returned, destination)?;
                        returned = destination;
                    } else if let Some(ty) = mir.return_type {
                        returned = coerce_integer(
                            &mut function,
                            returned,
                            machine_type(&program.layouts, ty),
                        );
                    }
                    function.ins().return_(&[returned]);
                }
                Terminator::Return(None) => {
                    if mir.is_poll {
                        let ready = function.ins().iconst(types::I32, 1);
                        function.ins().return_(&[ready]);
                    } else if let Some(variable) = sret_variable {
                        let destination = function.use_var(variable);
                        function.ins().return_(&[destination]);
                    } else {
                        function.ins().return_(&[]);
                    }
                }
                Terminator::PollReturn(status) => {
                    let status = function.ins().iconst(types::I32, i64::from(status));
                    function.ins().return_(&[status]);
                }
                Terminator::Jump(target) => {
                    function.ins().jump(blocks[target.0], &[]);
                }
                Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    function.ins().brif(
                        values[&condition],
                        blocks[then_block.0],
                        &[],
                        blocks[else_block.0],
                        &[],
                    );
                }
                Terminator::Unreachable => {
                    function
                        .ins()
                        .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
                }
            }
        }
        function.seal_all_blocks();
        function.finalize(frontend_config);
        module
            .define_function(function_id, &mut context)
            .map_err(|error| {
                eprintln!("VUT-CODEGEN-DEBUG: {error:?}");
                CodegenError::Backend(error.to_string())
            })?;
        module.clear_context(&mut context);
    }
    define_future_thunks(&mut module, program, &declarations, &future_thunks)?;
    for ((index, retain), function_id) in &type_functions {
        let ty = vut_hir::TypeId(*index);
        let symbol_index = u32::try_from(*index)
            .map_err(|_| CodegenError::Backend("too many types for backend namespace".into()))?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(2, symbol_index.saturating_mul(2) + u32::from(*retain)),
            callback_signature.clone(),
        ));
        let mut builder_context = FunctionBuilderContext::new();
        let mut function = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let block = function.create_block();
        function.append_block_params_for_function_params(block);
        function.switch_to_block(block);
        let pointer = function.block_params(block)[0];
        // The callback receives the address of one element slot. Aggregate
        // elements are stored inline, so that address is the aggregate value;
        // other managed values store their handle directly at the slot.
        let value = if program.layouts.is_aggregate(ty) {
            pointer
        } else {
            function.ins().load(
                machine_type(&program.layouts, ty),
                cranelift_codegen::ir::MemFlagsData::trusted(),
                pointer,
                0,
            )
        };
        manage_value(
            &mut function,
            &mut module,
            &program.layouts,
            ty,
            value,
            *retain,
        )?;
        function.ins().return_(&[]);
        function.seal_all_blocks();
        function.finalize(module.target_config());
        module
            .define_function(*function_id, &mut context)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        module.clear_context(&mut context);
    }
    for (symbol, layout) in &program.closure_layouts {
        let Some(function_id) = closure_drops.get(symbol) else {
            continue;
        };
        let symbol_index = u32::try_from(symbol.0)
            .map_err(|_| CodegenError::Backend("too many symbols for backend namespace".into()))?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(4, symbol_index),
            callback_signature.clone(),
        ));
        let mut builder_context = FunctionBuilderContext::new();
        let mut function = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let block = function.create_block();
        function.append_block_params_for_function_params(block);
        function.switch_to_block(block);
        let base = function.block_params(block)[0];
        for capture in &layout.captures {
            if !program.layouts.types[capture.ty.0].needs_drop {
                continue;
            }
            let offset = i32::try_from(32_usize.saturating_add(capture.offset)).map_err(|_| {
                CodegenError::Backend("closure capture offset exceeds backend limit".into())
            })?;
            let value = function
                .ins()
                .load(types::I64, MemFlagsData::trusted(), base, offset);
            manage_value(
                &mut function,
                &mut module,
                &program.layouts,
                capture.ty,
                value,
                false,
            )?;
        }
        function.ins().return_(&[]);
        function.seal_all_blocks();
        function.finalize(module.target_config());
        module
            .define_function(*function_id, &mut context)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        module.clear_context(&mut context);
    }
    for (index, function_id) in &interface_drops {
        let ty = TypeId(*index);
        let symbol_index = u32::try_from(*index)
            .map_err(|_| CodegenError::Backend("too many types for backend namespace".into()))?;
        let mut context = Context::for_function(Function::with_name_signature(
            UserFuncName::user(3, symbol_index),
            callback_signature.clone(),
        ));
        let mut builder_context = FunctionBuilderContext::new();
        let mut function = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let block = function.create_block();
        function.append_block_params_for_function_params(block);
        function.switch_to_block(block);
        let pointer = function.block_params(block)[0];
        let value = if program.layouts.is_aggregate(ty) {
            pointer
        } else {
            function.ins().load(
                machine_type(&program.layouts, ty),
                MemFlagsData::trusted(),
                pointer,
                0,
            )
        };
        manage_value(
            &mut function,
            &mut module,
            &program.layouts,
            ty,
            value,
            false,
        )?;
        function.ins().return_(&[]);
        function.seal_all_blocks();
        function.finalize(module.target_config());
        module
            .define_function(*function_id, &mut context)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
        module.clear_context(&mut context);
    }
    for vtable in &program.interface_vtables {
        let data_id = vtables[&(vtable.concrete_ty.0, vtable.interface)];
        let drop_id = interface_drops[&vtable.concrete_ty.0];
        let mut description = DataDescription::new();
        let pointer_bytes = module.target_config().pointer_type().bytes() as usize;
        let size = pointer_bytes.saturating_mul(vtable.methods.len() + 1);
        description.set_align(u64::try_from(pointer_bytes).unwrap_or(8));
        description.define(vec![0_u8; size].into_boxed_slice());
        let drop_ref = module.declare_func_in_data(drop_id, &mut description);
        description.write_function_addr(0, drop_ref);
        for (slot, method) in vtable.methods.iter().enumerate() {
            let (func_id, _) = declarations.get(method).ok_or_else(|| {
                CodegenError::Backend(format!(
                    "invalid typed MIR: interface method {} has no declaration",
                    method.0
                ))
            })?;
            let func_ref = module.declare_func_in_data(*func_id, &mut description);
            let offset = u32::try_from(slot.saturating_add(1).saturating_mul(8))
                .map_err(|_| CodegenError::Backend("vtable offset exceeds target".into()))?;
            description.write_function_addr(offset, func_ref);
        }
        module
            .define_data(data_id, &description)
            .map_err(|error| CodegenError::Backend(error.to_string()))?;
    }
    if let Some((entry, memory_check)) = entry {
        define_entry_shim(
            &mut module,
            &declarations,
            program,
            entry,
            &future_thunks,
            memory_check,
        )?;
    }
    let product = module.finish();
    product
        .emit()
        .map_err(|error| CodegenError::Backend(error.to_string()))
}
