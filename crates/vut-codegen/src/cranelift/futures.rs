//! Native codegen for async future frames, resume/drop thunks, and the entry
//! shim that allocates and drives the root future.
use std::collections::HashMap;

use cranelift_codegen::{
    Context,
    ir::{
        AbiParam, Function, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind,
        UserFuncName, condcodes::IntCC, types,
    },
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use vut_mir::Program;

use super::CodegenError;
use super::signatures::{
    c_call_conv, coerce_integer, copy_aggregate, machine_type, runtime_function,
};
fn require_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn require_i32(value: usize) -> Result<i32, CodegenError> {
    i32::try_from(value).map_err(|_| CodegenError::Backend("offset exceeds backend limit".into()))
}

/// Defines the resume/drop thunks for every function spawned as a task or used
/// as an async body. Sync callables get a first-poll thunk plus a no-op drop;
/// async bodies get a resume/poll entry and a frame drop thunk.
pub(super) fn define_future_thunks(
    module: &mut ObjectModule,
    program: &Program,
    declarations: &HashMap<vut_mir::SymbolId, (FuncId, Signature)>,
    future_thunks: &HashMap<vut_mir::SymbolId, (FuncId, FuncId)>,
) -> Result<(), CodegenError> {
    for mir in &program.functions {
        let Some((resume_id, drop_id)) = future_thunks.get(&mir.symbol).copied() else {
            continue;
        };
        let symbol_index = u32::try_from(mir.symbol.0)
            .map_err(|_| CodegenError::Backend("too many symbols for backend namespace".into()))?;
        if !mir.is_async {
            define_sync_task_thunk(module, mir, &program.layouts, declarations, resume_id)?;
            define_noop_drop(module, symbol_index, drop_id)?;
            continue;
        }
        let Some(layout) = program.frames.get(&mir.symbol) else {
            continue;
        };
        // A poll body is its own `(frame, out) -> i32` resume entry, so it needs
        // no resume thunk.
        if !mir.is_poll {
            define_resume_thunk(
                module,
                mir,
                layout,
                &program.layouts,
                declarations,
                symbol_index,
                resume_id,
            )?;
        }
        define_drop_thunk(module, mir, layout, symbol_index, drop_id)?;
    }
    Ok(())
}

/// Defines the task poll thunk for a sync callable: run its body once, write the
/// result into the polled output slot, and report `Ready`.
fn define_sync_task_thunk(
    module: &mut ObjectModule,
    mir: &vut_mir::Function,
    layouts: &vut_mir::LayoutTable,
    declarations: &HashMap<vut_mir::SymbolId, (FuncId, Signature)>,
    thunk_id: FuncId,
) -> Result<(), CodegenError> {
    let mut signature = Signature::new(c_call_conv(module));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I32));
    let mut context = Context::for_function(Function::with_name_signature(
        UserFuncName::user(5, u32::try_from(mir.symbol.0).unwrap_or(0)),
        signature,
    ));
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let block = builder.create_block();
    builder.append_block_params_for_function_params(block);
    builder.switch_to_block(block);
    builder.seal_block(block);
    let out = builder.block_params(block)[1];
    let (body_id, _) = declarations[&mir.symbol];
    let body_ref = module.declare_func_in_func(body_id, builder.func);
    let aggregate = mir.return_type.filter(|ty| layouts.is_aggregate(*ty));
    let call = if aggregate.is_some() {
        // Aggregate returns take a caller-provided destination (sret); the task
        // buffer is exactly that destination.
        builder.ins().call(body_ref, &[out])
    } else {
        builder.ins().call(body_ref, &[])
    };
    if aggregate.is_none()
        && let Some(ty) = mir.return_type
        && layouts.types[ty.0].repr != vut_mir::ValueRepr::Void
    {
        let result = builder.inst_results(call)[0];
        builder.ins().store(MemFlagsData::trusted(), result, out, 0);
    }
    let ready = builder.ins().iconst(types::I32, 1);
    builder.ins().return_(&[ready]);
    builder.finalize(module.target_config());
    module
        .define_function(thunk_id, &mut context)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

/// Defines the no-op drop thunk for a sync task (it owns no frame).
fn define_noop_drop(
    module: &mut ObjectModule,
    symbol_index: u32,
    drop_id: FuncId,
) -> Result<(), CodegenError> {
    let mut signature = Signature::new(c_call_conv(module));
    signature.params.push(AbiParam::new(types::I64));
    let mut context = Context::for_function(Function::with_name_signature(
        UserFuncName::user(6, symbol_index),
        signature,
    ));
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let block = builder.create_block();
    builder.append_block_params_for_function_params(block);
    builder.switch_to_block(block);
    builder.seal_block(block);
    builder.ins().return_(&[]);
    builder.finalize(module.target_config());
    module
        .define_function(drop_id, &mut context)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

/// Defines the resume thunk for a blocking (non-state-machine) async body:
/// reconstruct the body arguments from the frame, run the body, and write the
/// logical result into the polled output slot.
fn define_resume_thunk(
    module: &mut ObjectModule,
    mir: &vut_mir::Function,
    layout: &vut_mir::FrameLayout,
    layouts: &vut_mir::LayoutTable,
    declarations: &HashMap<vut_mir::SymbolId, (FuncId, Signature)>,
    symbol_index: u32,
    resume_id: FuncId,
) -> Result<(), CodegenError> {
    let mut resume_signature = Signature::new(c_call_conv(module));
    resume_signature.params.push(AbiParam::new(types::I64));
    resume_signature.params.push(AbiParam::new(types::I64));
    resume_signature.returns.push(AbiParam::new(types::I32));
    let mut context = Context::for_function(Function::with_name_signature(
        UserFuncName::user(4, symbol_index.saturating_mul(2)),
        resume_signature,
    ));
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let block = builder.create_block();
    builder.append_block_params_for_function_params(block);
    builder.switch_to_block(block);
    let params = builder.block_params(block).to_vec();
    let frame = params[0];
    let out = params[1];
    let aggregate_return = mir.return_type.filter(|ty| layouts.is_aggregate(*ty));
    let mut call_args = Vec::new();
    let destination = if let (Some(_), Some(slot)) = (aggregate_return, layout.completion) {
        let destination = builder.ins().iadd_imm_u(frame, require_i64(slot.offset));
        call_args.push(destination);
        Some(destination)
    } else {
        None
    };
    call_args.push(frame);
    let (body_id, _) = declarations[&mir.symbol];
    let body_ref = module.declare_func_in_func(body_id, builder.func);
    let call = builder.ins().call(body_ref, &call_args);
    if let Some(ty) = aggregate_return {
        let destination = destination.expect("aggregate return has a destination");
        copy_aggregate(&mut builder, layouts, ty, destination, out)?;
    } else if let Some(ty) = mir.return_type
        && layouts.types[ty.0].repr != vut_mir::ValueRepr::Void
    {
        let result = builder.inst_results(call)[0];
        builder.ins().store(MemFlagsData::trusted(), result, out, 0);
    }
    let ready = builder.ins().iconst(types::I32, 1);
    builder.ins().return_(&[ready]);
    builder.seal_all_blocks();
    builder.finalize(module.target_config());
    module
        .define_function(resume_id, &mut context)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

/// Defines the drop thunk: release the in-flight child (if any) and then the
/// frame exactly once.
fn define_drop_thunk(
    module: &mut ObjectModule,
    mir: &vut_mir::Function,
    layout: &vut_mir::FrameLayout,
    symbol_index: u32,
    drop_id: FuncId,
) -> Result<(), CodegenError> {
    let mut drop_signature = Signature::new(c_call_conv(module));
    drop_signature.params.push(AbiParam::new(types::I64));
    let mut context = Context::for_function(Function::with_name_signature(
        UserFuncName::user(4, symbol_index.saturating_mul(2).saturating_add(1)),
        drop_signature,
    ));
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let block = builder.create_block();
    builder.append_block_params_for_function_params(block);
    builder.switch_to_block(block);
    let frame = builder.block_params(block)[0];
    if mir.is_poll {
        let child = builder.ins().load(
            types::I64,
            MemFlagsData::trusted(),
            frame,
            require_i32(vut_mir::FRAME_CHILD_OFFSET)?,
        );
        let zero = builder.ins().iconst(types::I64, 0);
        let has_child = builder.ins().icmp(IntCC::NotEqual, child, zero);
        let drop_block = builder.create_block();
        let done_block = builder.create_block();
        builder
            .ins()
            .brif(has_child, drop_block, &[], done_block, &[]);
        builder.switch_to_block(drop_block);
        let drop_child =
            runtime_function(module, vut_runtime::abi::ASYNC_DROP, &[types::I64], &[])?;
        let drop_child_ref = module.declare_func_in_func(drop_child, builder.func);
        builder.ins().call(drop_child_ref, &[child]);
        builder.ins().jump(done_block, &[]);
        builder.switch_to_block(done_block);
    }
    let free = runtime_function(
        module,
        vut_runtime::abi::FRAME_FREE,
        &[types::I64, types::I64, types::I64],
        &[],
    )?;
    let free_ref = module.declare_func_in_func(free, builder.func);
    let size = builder.ins().iconst(types::I64, require_i64(layout.size));
    let align = builder.ins().iconst(types::I64, require_i64(layout.align));
    builder.ins().call(free_ref, &[frame, size, align]);
    builder.ins().return_(&[]);
    builder.seal_all_blocks();
    builder.finalize(module.target_config());
    module
        .define_function(drop_id, &mut context)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

/// Wraps a state-machine `async main` in a task, drives the executor to
/// completion, copies the result out, and returns the process exit code.
fn entry_poll_body(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    mir: Option<&vut_mir::Function>,
    layouts: &vut_mir::LayoutTable,
    target: FuncId,
    layout: Option<&vut_mir::FrameLayout>,
    drop_thunk: Option<FuncId>,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    let layout = layout.ok_or_else(|| CodegenError::Backend("poll entry has no frame".into()))?;
    let drop_thunk =
        drop_thunk.ok_or_else(|| CodegenError::Backend("poll entry has no drop thunk".into()))?;
    let allocate = runtime_function(
        module,
        vut_runtime::abi::FRAME_ALLOC,
        &[types::I64, types::I64],
        &[types::I64],
    )?;
    let allocate = module.declare_func_in_func(allocate, builder.func);
    let size = builder.ins().iconst(types::I64, require_i64(layout.size));
    let align = builder.ins().iconst(types::I64, require_i64(layout.align));
    let allocation = builder.ins().call(allocate, &[size, align]);
    let frame = builder.inst_results(allocation)[0];
    let slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 4));
    let out = builder.ins().stack_addr(types::I64, slot, 0);
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().store(MemFlagsData::trusted(), zero, out, 0);
    let pointer = module.target_config().pointer_type();
    let body_ref = module.declare_func_in_func(target, builder.func);
    let body_address = builder.ins().func_addr(pointer, body_ref);
    let drop_ref = module.declare_func_in_func(drop_thunk, builder.func);
    let drop_address = builder.ins().func_addr(pointer, drop_ref);
    // The root is an ordinary eager-scheduled Vutcon task.
    let result_size = mir
        .and_then(|function| function.return_type)
        .map_or(0, |ty| layouts.types[ty.0].size);
    let result_align = mir
        .and_then(|function| function.return_type)
        .map_or(1, |ty| layouts.types[ty.0].alignment.max(1));
    let spawn = runtime_function(
        module,
        vut_runtime::abi::VUTCON_SPAWN,
        &[types::I64, types::I64, types::I64, types::I64, types::I64],
        &[types::I64],
    )?;
    let spawn_ref = module.declare_func_in_func(spawn, builder.func);
    let size = builder.ins().iconst(types::I64, require_i64(result_size));
    let align = builder.ins().iconst(types::I64, require_i64(result_align));
    let spawned = builder
        .ins()
        .call(spawn_ref, &[frame, body_address, drop_address, size, align]);
    let handle = builder.inst_results(spawned)[0];
    let run = runtime_function(
        module,
        vut_runtime::abi::EXECUTOR_RUN,
        &[types::I64],
        &[types::I32],
    )?;
    let run_ref = module.declare_func_in_func(run, builder.func);
    builder.ins().call(run_ref, &[handle]);
    let copy = runtime_function(
        module,
        vut_runtime::abi::TASK_RESULT,
        &[types::I64, types::I64],
        &[],
    )?;
    let copy_ref = module.declare_func_in_func(copy, builder.func);
    builder.ins().call(copy_ref, &[handle, out]);
    let drop_fn = runtime_function(module, vut_runtime::abi::ASYNC_DROP, &[types::I64], &[])?;
    let drop_fn_ref = module.declare_func_in_func(drop_fn, builder.func);
    builder.ins().call(drop_fn_ref, &[handle]);
    let drain = runtime_function(module, vut_runtime::abi::EXECUTOR_DRAIN, &[], &[])?;
    let drain_ref = module.declare_func_in_func(drain, builder.func);
    builder.ins().call(drain_ref, &[]);
    if let Some(ty) = mir.and_then(|function| function.return_type)
        && layouts.types[ty.0].repr != vut_mir::ValueRepr::Void
    {
        let loaded = builder
            .ins()
            .load(machine_type(layouts, ty), MemFlagsData::trusted(), out, 0);
        Ok(coerce_integer(builder, loaded, types::I64))
    } else {
        Ok(builder.ins().iconst(types::I64, 0))
    }
}

/// Calls a plain (`frame -> T`) `async main` body, allocating and releasing its
/// frame, and returns the process exit code.
fn entry_plain_body(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    target: FuncId,
    target_signature: &Signature,
    layout: Option<&vut_mir::FrameLayout>,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    let mut call_args = Vec::new();
    if let Some(layout) = layout {
        let allocate = runtime_function(
            module,
            vut_runtime::abi::FRAME_ALLOC,
            &[types::I64, types::I64],
            &[types::I64],
        )?;
        let allocate = module.declare_func_in_func(allocate, builder.func);
        let size = builder.ins().iconst(types::I64, require_i64(layout.size));
        let align = builder.ins().iconst(types::I64, require_i64(layout.align));
        let allocation = builder.ins().call(allocate, &[size, align]);
        call_args.push(builder.inst_results(allocation)[0]);
    }
    let reference = module.declare_func_in_func(target, builder.func);
    let call = builder.ins().call(reference, &call_args);
    if let Some(layout) = layout {
        let free = runtime_function(
            module,
            vut_runtime::abi::FRAME_FREE,
            &[types::I64, types::I64, types::I64],
            &[],
        )?;
        let free = module.declare_func_in_func(free, builder.func);
        let size = builder.ins().iconst(types::I64, require_i64(layout.size));
        let align = builder.ins().iconst(types::I64, require_i64(layout.align));
        builder.ins().call(free, &[call_args[0], size, align]);
    }
    if target_signature.returns.is_empty() {
        Ok(builder.ins().iconst(types::I64, 0))
    } else {
        Ok(builder.inst_results(call)[0])
    }
}

pub(super) fn define_entry_shim(
    module: &mut ObjectModule,
    declarations: &HashMap<vut_mir::SymbolId, (FuncId, Signature)>,
    program: &Program,
    entry: vut_mir::SymbolId,
    future_thunks: &HashMap<vut_mir::SymbolId, (FuncId, FuncId)>,
    memory_check: bool,
) -> Result<(), CodegenError> {
    let (target, target_signature) = declarations
        .get(&entry)
        .ok_or_else(|| CodegenError::Backend("entry function is absent from MIR".into()))?;
    let mut signature = Signature::new(c_call_conv(module));
    signature.returns.push(AbiParam::new(types::I64));
    let id = module
        .declare_function("vut_entry", Linkage::Export, &signature)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    let mut context = Context::for_function(Function::with_name_signature(
        UserFuncName::user(1, 0),
        signature,
    ));
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let block = builder.create_block();
    builder.switch_to_block(block);
    builder.seal_block(block);
    let mir = program
        .functions
        .iter()
        .find(|function| function.symbol == entry);
    let frame_layout = program.frames.get(&entry);
    let is_poll = mir.is_some_and(|function| function.is_poll);
    let exit = if is_poll {
        let drop_thunk = future_thunks.get(&entry).map(|(_, drop)| *drop);
        entry_poll_body(
            module,
            &mut builder,
            mir,
            &program.layouts,
            *target,
            frame_layout,
            drop_thunk,
        )?
    } else {
        entry_plain_body(
            module,
            &mut builder,
            *target,
            target_signature,
            frame_layout,
        )?
    };
    let exit = if memory_check {
        let counter = runtime_function(
            module,
            vut_runtime::abi::LIVE_ALLOCATION_COUNT,
            &[],
            &[types::I64],
        )?;
        let counter = module.declare_func_in_func(counter, builder.func);
        let live = builder.ins().call(counter, &[]);
        let live = builder.inst_results(live)[0];
        let clean = builder.ins().icmp_imm_u(IntCC::Equal, live, 0);
        let leak_exit = builder.ins().iconst(types::I64, 254);
        builder.ins().select(clean, exit, leak_exit)
    } else {
        exit
    };
    builder.ins().return_(&[exit]);
    builder.finalize(module.target_config());
    module
        .define_function(id, &mut context)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    Ok(())
}
