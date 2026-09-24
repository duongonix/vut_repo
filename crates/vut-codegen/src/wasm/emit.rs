//! MIR → wasm function body lowering (scalar + minimal managed subset).
//!
//! Control flow is lowered with a switch-dispatch loop, which maps arbitrary
//! reducible CFGs onto wasm's structured control flow.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::match_same_arms,
    clippy::trivially_copy_pass_by_ref
)]
use std::collections::HashMap;

use vut_ast::{BinaryOp, UnaryOp};
use vut_mir::{
    BasicBlock, Function, Instruction, LayoutTable, LocalId, NumericKind, OwnershipKind, SymbolId,
    ValueId,
};
use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, ValType};

use super::layout::valtype;
use super::memarg;
use crate::CodegenError;

use super::Runtime;

/// Scratch address in linear memory for a poll result (scalars, pointers).
const POLL_SCRATCH: i32 = 32;

/// Shared lowering context for a program.
pub(super) struct LowerCtx<'a> {
    pub layouts: &'a LayoutTable,
    pub symbols: &'a HashMap<SymbolId, u32>,
    pub strings: &'a HashMap<String, u32>,
    pub callee_params: &'a HashMap<SymbolId, Vec<ValType>>,
    pub callable_types: &'a HashMap<vut_hir::TypeId, u32>,
    pub rt: &'a Runtime,
    pub frames: &'a HashMap<SymbolId, vut_mir::FrameLayout>,
    pub poll_thunks: &'a HashMap<SymbolId, u32>,
    pub async_rt: &'a super::asyncrt::AsyncRt,
}

/// Maps MIR local ids to wasm local indices.
///
/// Ordinary functions place declared parameters first, so the mapping is the
/// identity. A frame-based async body instead receives a frame pointer (and,
/// for a poll entry, an output pointer) as its wasm parameters while every
/// declared parameter becomes frame-resident; those locals are remapped and the
/// remaining locals are shifted past the wasm parameters.
pub(super) struct LocalMap {
    map: Vec<u32>,
    /// The wasm local index where value temporaries begin.
    pub base: u32,
}

impl LocalMap {
    fn new(function: &Function) -> Self {
        let param_count = if function.frame_param.is_some() {
            usize::from(function.out_param.is_some()) + 1
        } else {
            function.parameter_count
        };
        let mut map = vec![0_u32; function.locals.len()];
        let mut next = param_count as u32;
        for (index, slot) in map.iter_mut().enumerate() {
            if Some(index) == function.frame_param {
                *slot = 0;
            } else if Some(index) == function.out_param {
                *slot = 1;
            } else if index < function.parameter_count {
                *slot = index as u32;
            } else {
                *slot = next;
                next += 1;
            }
        }
        Self { map, base: next }
    }

    fn get(&self, local: LocalId) -> u32 {
        self.map[local.0]
    }
}

#[expect(clippy::too_many_lines, reason = "linear lowering of one MIR function")]
pub(super) fn function_body(
    ctx: &LowerCtx<'_>,
    function: &Function,
) -> Result<WasmFunction, CodegenError> {
    let layouts = ctx.layouts;
    let parameter_count = function.parameter_count;
    let map = LocalMap::new(function);
    let mut value_locals: HashMap<ValueId, u32> = HashMap::new();
    let mut declared_types: Vec<ValType> = Vec::new();
    let mut value_types: HashMap<ValueId, ValType> = HashMap::new();
    let mut borrow_of: HashMap<ValueId, LocalId> = HashMap::new();
    let mut value_mir_types: HashMap<ValueId, vut_hir::TypeId> = HashMap::new();
    // Types of values spilled into the frame, keyed by frame slot, so a reload
    // declares its local with the same wasm type the spill stored.
    let mut slot_types: HashMap<usize, ValType> = HashMap::new();
    {
        let mut seen: HashMap<ValueId, ValType> = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Some(value) = defined_value(instruction) {
                    let ty = infer_type(layouts, function, instruction, &seen, &value_mir_types);
                    seen.insert(value, ty);
                }
                if let Instruction::SpillValue { slot, value } = instruction
                    && let Some(ty) = seen.get(value)
                {
                    slot_types.insert(*slot, *ty);
                }
            }
        }
    }
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::Borrow { value, local } = instruction {
                borrow_of.insert(*value, *local);
            }
            if let Instruction::ReloadValue { slot, value } = instruction {
                let index = map.base + declared_types.len() as u32;
                let ty = slot_types.get(slot).copied().unwrap_or(ValType::I32);
                value_locals.insert(*value, index);
                declared_types.push(ty);
                value_types.insert(*value, ty);
                if let Some(mir) = value_result_type(function, instruction) {
                    value_mir_types.insert(*value, mir);
                }
                continue;
            }
            if let Some(value) = defined_value(instruction) {
                let index = map.base + declared_types.len() as u32;
                let ty = infer_type(
                    layouts,
                    function,
                    instruction,
                    &value_types,
                    &value_mir_types,
                );
                value_locals.insert(value, index);
                declared_types.push(ty);
                value_types.insert(value, ty);
                if let Some(mir) = value_result_type(function, instruction) {
                    value_mir_types.insert(value, mir);
                }
            }
            let mut declare = |value: ValueId, ty: ValType| {
                if value_locals.contains_key(&value) {
                    return;
                }
                let index = map.base + declared_types.len() as u32;
                value_locals.insert(value, index);
                declared_types.push(ty);
                value_types.insert(value, ty);
            };
            match instruction {
                Instruction::IteratorNext {
                    has_value: has,
                    index,
                    ..
                } => {
                    declare(*has, ValType::I32);
                    declare(*index, ValType::I64);
                }
                Instruction::PollFuture {
                    ready,
                    value,
                    result_type,
                    ..
                } => {
                    declare(*ready, ValType::I32);
                    if let Some(value) = value {
                        declare(*value, valtype(layouts, *result_type));
                    }
                }
                Instruction::PollChannelRecv {
                    value,
                    present,
                    ready,
                    ..
                } => {
                    declare(*value, ValType::I32);
                    declare(*present, ValType::I32);
                    declare(*ready, ValType::I32);
                }
                _ => {}
            }
        }
    }
    let pc = map.base + declared_types.len() as u32;

    let mut declared: Vec<(u32, ValType)> = Vec::new();
    for (index, local) in function.locals.iter().enumerate() {
        if index < parameter_count
            || Some(index) == function.frame_param
            || Some(index) == function.out_param
        {
            continue;
        }
        declared.push((1, local.ty.map_or(ValType::I32, |ty| valtype(layouts, ty))));
    }
    for ty in &declared_types {
        declared.push((1, *ty));
    }
    declared.push((1, ValType::I32)); // pc
    declared.push((1, ValType::I32)); // scratch
    let scratch = pc + 1;

    let mut code = WasmFunction::new(declared);
    let blocks = &function.blocks;
    let count = blocks.len() as u32;
    let entry = function.entry.0 as u32;

    code.instruction(&Ins::I32Const(entry as i32));
    code.instruction(&Ins::LocalSet(pc));
    code.instruction(&Ins::Loop(BlockType::Empty));
    for _ in 0..count {
        code.instruction(&Ins::Block(BlockType::Empty));
    }
    let targets: Vec<u32> = (0..count).collect();
    code.instruction(&Ins::LocalGet(pc));
    code.instruction(&Ins::BrTable(targets.into(), 0));

    for (k, block) in blocks.iter().enumerate() {
        code.instruction(&Ins::End); // close block `count-1-k`
        let loop_depth = count - 1 - k as u32;
        lower_block(
            ctx,
            function,
            &map,
            block,
            loop_depth,
            pc,
            scratch,
            &value_locals,
            &value_types,
            &borrow_of,
            &value_mir_types,
            &mut code,
        )?;
    }
    code.instruction(&Ins::End); // close loop
    code.instruction(&Ins::Unreachable);
    code.instruction(&Ins::End); // close function
    Ok(code)
}

#[allow(clippy::too_many_arguments)]
fn lower_block(
    ctx: &LowerCtx<'_>,
    function: &Function,
    map: &LocalMap,
    block: &BasicBlock,
    loop_depth: u32,
    pc: u32,
    scratch: u32,
    value_locals: &HashMap<ValueId, u32>,
    value_types: &HashMap<ValueId, ValType>,
    borrow_of: &HashMap<ValueId, LocalId>,
    value_mir_types: &HashMap<ValueId, vut_hir::TypeId>,
    code: &mut WasmFunction,
) -> Result<(), CodegenError> {
    for instruction in &block.instructions {
        lower_instruction(
            ctx,
            function,
            map,
            instruction,
            value_locals,
            value_types,
            borrow_of,
            value_mir_types,
            scratch,
            code,
        )?;
    }
    match &block.terminator {
        vut_mir::Terminator::Jump(target) => {
            code.instruction(&Ins::I32Const(target.0 as i32));
            code.instruction(&Ins::LocalSet(pc));
            code.instruction(&Ins::Br(loop_depth));
        }
        vut_mir::Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            code.instruction(&Ins::LocalGet(value_locals[condition]));
            code.instruction(&Ins::If(BlockType::Empty));
            code.instruction(&Ins::I32Const(then_block.0 as i32));
            code.instruction(&Ins::LocalSet(pc));
            code.instruction(&Ins::Else);
            code.instruction(&Ins::I32Const(else_block.0 as i32));
            code.instruction(&Ins::LocalSet(pc));
            code.instruction(&Ins::End);
            code.instruction(&Ins::Br(loop_depth));
        }
        vut_mir::Terminator::Return(value) => {
            if function.is_poll {
                let out = LocalId(function.out_param.expect("poll body has an out pointer"));
                if let Some(value) = value {
                    code.instruction(&Ins::LocalGet(map.get(out)));
                    code.instruction(&Ins::LocalGet(value_locals[value]));
                    let ty = value_types.get(value).copied().unwrap_or(ValType::I32);
                    mem_store(code, ty, 0);
                }
                code.instruction(&Ins::I32Const(1));
                code.instruction(&Ins::Return);
            } else {
                if let Some(value) = value {
                    code.instruction(&Ins::LocalGet(value_locals[value]));
                }
                code.instruction(&Ins::Return);
            }
        }
        vut_mir::Terminator::Unreachable => {
            code.instruction(&Ins::Unreachable);
        }
        vut_mir::Terminator::PollReturn(status) => {
            code.instruction(&Ins::I32Const(*status));
            code.instruction(&Ins::Return);
        }
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "the instruction lowering table is intentionally centralized"
)]
#[allow(clippy::too_many_arguments)]
fn lower_instruction(
    ctx: &LowerCtx<'_>,
    function: &Function,
    map: &LocalMap,
    instruction: &Instruction,
    value_locals: &HashMap<ValueId, u32>,
    value_types: &HashMap<ValueId, ValType>,
    borrow_of: &HashMap<ValueId, LocalId>,
    value_mir_types: &HashMap<ValueId, vut_hir::TypeId>,
    scratch: u32,
    code: &mut WasmFunction,
) -> Result<(), CodegenError> {
    let layouts = ctx.layouts;
    let set = |code: &mut WasmFunction, value: ValueId| {
        if let Some(index) = value_locals.get(&value) {
            code.instruction(&Ins::LocalSet(*index));
        }
    };
    let get = |code: &mut WasmFunction, value: ValueId| {
        code.instruction(&Ins::LocalGet(value_locals[&value]));
    };
    match instruction {
        Instruction::ConstInt { value, literal } => {
            code.instruction(&Ins::I64Const(*literal));
            set(code, *value);
        }
        Instruction::ConstBool { value, literal } => {
            code.instruction(&Ins::I32Const(i32::from(*literal)));
            set(code, *value);
        }
        Instruction::ConstFloat { value, literal, ty } => {
            if ty.is_some_and(|ty| layouts.types[ty.0].size == 4) {
                code.instruction(&Ins::F32Const(wasm_encoder::Ieee32::from(*literal as f32)));
            } else {
                code.instruction(&Ins::F64Const(wasm_encoder::Ieee64::from(*literal)));
            }
            set(code, *value);
        }
        Instruction::ConstString { value, literal } => {
            let address = *ctx.strings.get(literal).ok_or_else(|| {
                CodegenError::Backend(format!(
                    "wasm backend: unregistered string literal `{literal}`"
                ))
            })?;
            code.instruction(&Ins::I32Const(address as i32));
            code.instruction(&Ins::Call(ctx.rt.str_from_static));
            set(code, *value);
        }
        Instruction::Copy { value, local } | Instruction::Move { value, local } => {
            code.instruction(&Ins::LocalGet(map.get(*local)));
            set(code, *value);
        }
        Instruction::Borrow { value, local } => {
            // A borrow is an address. By-reference values (aggregates and managed
            // handles) already hold the address of their storage; other values
            // are copied into a fresh heap slot.
            let ty = function.locals.get(local.0).and_then(|local| local.ty);
            let by_ref =
                ty.is_some_and(|ty| layouts.is_aggregate(ty) || is_managed_handle(layouts, ty));
            if by_ref {
                code.instruction(&Ins::LocalGet(map.get(*local)));
            } else {
                let size = ty.map_or(8, |ty| layouts.types[ty.0].size);
                code.instruction(&Ins::I32Const(size as i32));
                code.instruction(&Ins::Call(ctx.rt.alloc));
                code.instruction(&Ins::LocalTee(scratch));
                code.instruction(&Ins::LocalGet(map.get(*local)));
                if let Some(ty) = ty {
                    code.instruction(&store_instruction(layouts, ty, 0));
                } else {
                    code.instruction(&Ins::I64Store(memarg(0)));
                }
                code.instruction(&Ins::LocalGet(scratch));
            }
            set(code, *value);
        }
        Instruction::Store { local, value } => {
            get(code, *value);
            let to = function
                .locals
                .get(local.0)
                .and_then(|local| local.ty)
                .map_or(ValType::I32, |ty| valtype(layouts, ty));
            coerce(code, value_types.get(value).copied(), to);
            code.instruction(&Ins::LocalSet(map.get(*local)));
        }
        Instruction::Binary {
            operand_type,
            value,
            op,
            left,
            right,
        } => {
            let operand = operand_type
                .map(|ty| valtype(layouts, ty))
                .unwrap_or(ValType::I64);
            let unsigned = operand_type.is_some_and(|ty| layouts.unsigned.contains(&ty));
            get(code, *left);
            coerce(code, value_types.get(left).copied(), operand);
            get(code, *right);
            coerce(code, value_types.get(right).copied(), operand);
            let (instruction, _) = binary_instruction(*op, operand, unsigned)?;
            code.instruction(&instruction);
            set(code, *value);
        }
        Instruction::Unary { value, op, operand } => {
            match op {
                UnaryOp::Negate => {
                    match value_types.get(operand).copied().unwrap_or(ValType::I64) {
                        ValType::F64 => {
                            get(code, *operand);
                            code.instruction(&Ins::F64Neg);
                        }
                        ValType::F32 => {
                            get(code, *operand);
                            code.instruction(&Ins::F32Neg);
                        }
                        _ => {
                            code.instruction(&Ins::I64Const(0));
                            get(code, *operand);
                            code.instruction(&Ins::I64Sub);
                        }
                    }
                }
                UnaryOp::Not => {
                    get(code, *operand);
                    code.instruction(&Ins::I32Eqz);
                }
                UnaryOp::Positive => get(code, *operand),
            }
            set(code, *value);
        }
        Instruction::Call {
            value,
            result_type,
            target,
            arguments,
        } => {
            let params = ctx.callee_params.get(target);
            for (position, argument) in arguments.iter().enumerate() {
                get(code, *argument);
                if let Some(param) = params.and_then(|params| params.get(position)) {
                    coerce(code, value_types.get(argument).copied(), *param);
                }
            }
            let index = ctx.symbols.get(target).ok_or_else(|| {
                CodegenError::Backend(format!("wasm: unknown function {target:?}"))
            })?;
            code.instruction(&Ins::Call(*index));
            if let (Some(value), Some(result_type)) = (value, result_type) {
                if matches!(layouts.types[result_type.0].repr, vut_mir::ValueRepr::Void) {
                    code.instruction(&Ins::Drop);
                } else {
                    set(code, *value);
                }
            }
        }
        Instruction::AwaitFuture {
            value,
            handle,
            result_type,
        } => {
            get(code, *handle);
            code.instruction(&Ins::I32Const(POLL_SCRATCH));
            code.instruction(&Ins::Call(ctx.async_rt.future_await));
            code.instruction(&Ins::Drop);
            code.instruction(&Ins::I32Const(POLL_SCRATCH));
            let ty = valtype(layouts, *result_type);
            mem_load(code, ty, 0);
            set(code, *value);
        }
        Instruction::Spawn {
            value, callable, ..
        } => {
            if let Some(index) = ctx.symbols.get(callable) {
                code.instruction(&Ins::Call(*index));
                code.instruction(&Ins::Drop);
            }
            set(code, *value);
        }
        Instruction::FrameState { value, frame } => {
            code.instruction(&Ins::LocalGet(map.get(*frame)));
            code.instruction(&Ins::I32Load(memarg(vut_mir::FRAME_STATE_OFFSET as i64)));
            code.instruction(&Ins::I64ExtendI32U);
            set(code, *value);
        }
        Instruction::SetFrameState { frame, state } => {
            code.instruction(&Ins::LocalGet(map.get(*frame)));
            code.instruction(&Ins::I32Const(*state as i32));
            code.instruction(&Ins::I32Store(memarg(vut_mir::FRAME_STATE_OFFSET as i64)));
        }
        Instruction::SetFrameChild { frame, value } => {
            code.instruction(&Ins::LocalGet(map.get(*frame)));
            get(code, *value);
            code.instruction(&Ins::I32Store(memarg(vut_mir::FRAME_CHILD_OFFSET as i64)));
        }
        Instruction::SpillValue { slot, value } => {
            let frame = LocalId(function.frame_param.expect("spill body has a frame"));
            code.instruction(&Ins::LocalGet(map.get(frame)));
            get(code, *value);
            let ty = value_types.get(value).copied().unwrap_or(ValType::I64);
            mem_store(code, ty, *slot as i32);
        }
        Instruction::ReloadValue { value, slot } => {
            let frame = LocalId(function.frame_param.expect("reload body has a frame"));
            code.instruction(&Ins::LocalGet(map.get(frame)));
            let ty = value_types.get(value).copied().unwrap_or(ValType::I64);
            mem_load(code, ty, *slot as i32);
            set(code, *value);
        }
        Instruction::PollFuture {
            value,
            ready,
            frame,
            result_type,
        } => {
            code.instruction(&Ins::LocalGet(map.get(*frame)));
            code.instruction(&Ins::I32Const(vut_mir::FRAME_CHILD_OFFSET as i32));
            code.instruction(&Ins::I32Add);
            code.instruction(&Ins::I32Const(POLL_SCRATCH));
            code.instruction(&Ins::Call(ctx.async_rt.await_child));
            code.instruction(&Ins::LocalSet(value_locals[ready]));
            if let Some(value) = value {
                code.instruction(&Ins::I32Const(POLL_SCRATCH));
                let ty = valtype(layouts, *result_type);
                mem_load(code, ty, 0);
                set(code, *value);
            }
        }
        Instruction::PollChannelRecv { .. } => {
            return Err(unsupported(function, "channel recv on wasm"));
        }
        Instruction::StartFuture {
            value,
            result_type,
            target,
            arguments,
        } => {
            let layout = ctx.frames.get(target).ok_or_else(|| {
                CodegenError::Backend(format!("wasm: async fn {} has no frame layout", target.0))
            })?;
            code.instruction(&Ins::I32Const(layout.size as i32));
            code.instruction(&Ins::Call(ctx.async_rt.frame_alloc));
            code.instruction(&Ins::LocalSet(scratch));
            for (argument, slot) in arguments.iter().zip(layout.parameters.iter()) {
                code.instruction(&Ins::LocalGet(scratch));
                get(code, *argument);
                let ty = value_types.get(argument).copied().unwrap_or(ValType::I32);
                mem_store(code, ty, slot.offset as i32);
            }
            let result_size = layout.completion.map_or(0, |slot| {
                if layouts.is_aggregate(slot.ty) || is_managed_handle(layouts, slot.ty) {
                    4
                } else {
                    i32::try_from(layouts.types[slot.ty.0].size).unwrap_or(0)
                }
            });
            let poll_fn = ctx.poll_thunks.get(target).copied().ok_or_else(|| {
                CodegenError::Backend(format!("wasm: async fn {} has no poll entry", target.0))
            })?;
            code.instruction(&Ins::I32Const(poll_fn as i32));
            code.instruction(&Ins::I32Const(0));
            code.instruction(&Ins::LocalGet(scratch));
            code.instruction(&Ins::I32Const(result_size));
            code.instruction(&Ins::Call(ctx.async_rt.handle_new));
            let _ = result_type;
            if let Some(value) = value {
                set(code, *value);
            } else {
                code.instruction(&Ins::Drop);
            }
        }
        Instruction::MakeFunction { value, symbol } => {
            let index = ctx.symbols.get(symbol).copied().unwrap_or(0);
            code.instruction(&Ins::I32Const(index as i32));
            set(code, *value);
        }
        Instruction::Allocate { value, ty } => {
            code.instruction(&Ins::I32Const(layouts.types[ty.0].size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            set(code, *value);
        }
        Instruction::LoadRaw {
            value,
            pointer,
            offset,
            ty,
        } => {
            get(code, *pointer);
            code.instruction(&load_instruction(layouts, *ty, *offset));
            set(code, *value);
        }
        Instruction::StoreRaw {
            pointer,
            offset,
            value,
            ty,
        } => {
            get(code, *pointer);
            get(code, *value);
            coerce(code, value_types.get(value).copied(), valtype(layouts, *ty));
            code.instruction(&store_instruction(layouts, *ty, *offset));
        }
        Instruction::ConcatString { value, left, right } => {
            get(code, *left);
            get(code, *right);
            code.instruction(&Ins::Call(ctx.rt.concat));
            set(code, *value);
        }
        Instruction::FormatValue {
            value,
            operand,
            ty,
            unsigned,
        } => {
            if matches!(layouts.types[ty.0].repr, vut_mir::ValueRepr::Float) {
                get(code, *operand);
                if value_types.get(operand).copied() == Some(ValType::F32) {
                    code.instruction(&Ins::F64PromoteF32);
                }
                code.instruction(&Ins::Call(ctx.rt.fmt_f64));
            } else {
                get(code, *operand);
                if value_types.get(operand).copied().unwrap_or(ValType::I64) == ValType::I32 {
                    if *unsigned {
                        code.instruction(&Ins::I64ExtendI32U);
                    } else {
                        code.instruction(&Ins::I64ExtendI32S);
                    }
                }
                code.instruction(&Ins::Call(ctx.rt.format));
            }
            set(code, *value);
        }
        Instruction::RuntimeCall {
            value,
            result_type,
            function,
            arguments,
        } => {
            lower_runtime_call(
                ctx,
                map,
                value_locals,
                value_types,
                borrow_of,
                value_mir_types,
                scratch,
                *value,
                *result_type,
                function,
                arguments,
                code,
            )?;
        }
        Instruction::Retain { value, ty } => {
            if let Some(target) = retain_call(ctx.rt, layouts.types[ty.0].ownership) {
                get(code, *value);
                code.instruction(&Ins::If(BlockType::Empty));
                get(code, *value);
                code.instruction(&Ins::Call(target));
                code.instruction(&Ins::End);
            }
        }
        Instruction::Release { value, ty } => {
            if let Some(target) = release_call(ctx.rt, layouts.types[ty.0].ownership) {
                get(code, *value);
                code.instruction(&Ins::If(BlockType::Empty));
                get(code, *value);
                code.instruction(&Ins::Call(target));
                code.instruction(&Ins::End);
            }
        }
        Instruction::Drop(local) => {
            let ty = function.locals.get(local.0).and_then(|local| local.ty);
            if let Some(target) =
                ty.and_then(|ty| release_call(ctx.rt, layouts.types[ty.0].ownership))
            {
                code.instruction(&Ins::LocalGet(map.get(*local)));
                code.instruction(&Ins::If(BlockType::Empty));
                code.instruction(&Ins::LocalGet(map.get(*local)));
                code.instruction(&Ins::Call(target));
                code.instruction(&Ins::End);
            }
        }
        // Type retain/release callbacks are null (no element retain/release).
        Instruction::TypeRetain { value, .. } | Instruction::TypeRelease { value, .. } => {
            code.instruction(&Ins::I32Const(0));
            set(code, *value);
        }
        Instruction::MakeUnique { value, operand, ty } => {
            let target = match layouts.types[ty.0].ownership {
                OwnershipKind::RcList => Some(ctx.rt.list_make_unique),
                OwnershipKind::RcBytes => Some(ctx.rt.bytes_make_unique),
                _ => None,
            };
            if let Some(target) = target {
                get(code, *operand);
                code.instruction(&Ins::If(BlockType::Result(ValType::I32)));
                get(code, *operand);
                code.instruction(&Ins::Call(target));
                code.instruction(&Ins::Else);
                code.instruction(&Ins::I32Const(0));
                code.instruction(&Ins::End);
            } else {
                get(code, *operand);
            }
            set(code, *value);
        }
        Instruction::ResultPayload { value, source, ok } => {
            let result = value_mir_types
                .get(source)
                .and_then(|ty| layouts.results.get(ty).cloned());
            if let Some(result) = result {
                let (offset, payload_ty) = if *ok {
                    (result.ok_offset, result.ok)
                } else {
                    (result.err_offset, result.err)
                };
                get(code, *source);
                code.instruction(&Ins::I32Const(offset as i32));
                code.instruction(&Ins::I32Add);
                if !layouts.is_aggregate(payload_ty) && layouts.types[payload_ty.0].size != 0 {
                    code.instruction(&load_instruction(layouts, payload_ty, 0));
                }
            } else {
                return Err(CodegenError::Backend(
                    "wasm backend: result payload without a known result type".into(),
                ));
            }
            set(code, *value);
        }
        Instruction::ResultState { result, value, ok } => {
            get(code, *value);
            if layouts.pointer_size == 4 {
                code.instruction(&Ins::I32Load(memarg(0)));
                code.instruction(&Ins::I32Const(i32::from(!*ok)));
                code.instruction(&Ins::I32Eq);
            } else {
                code.instruction(&Ins::I64Load(memarg(0)));
                code.instruction(&Ins::I64Const(i64::from(!*ok)));
                code.instruction(&Ins::I64Eq);
            }
            set(code, *result);
        }
        Instruction::ConstructResult {
            value,
            ty,
            ok,
            payload,
        } => {
            let result = layouts.results.get(ty).cloned();
            let size = layouts.types[ty.0].size.max(1);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalTee(scratch));
            if let Some(result) = result {
                let (offset, payload_ty) = if *ok {
                    (result.ok_offset, result.ok)
                } else {
                    (result.err_offset, result.err)
                };
                code.instruction(&Ins::LocalGet(scratch));
                if ctx.layouts.pointer_size == 4 {
                    code.instruction(&Ins::I32Const(i32::from(!*ok)));
                    code.instruction(&Ins::I32Store(memarg(result.tag_offset as i64)));
                } else {
                    code.instruction(&Ins::I64Const(i64::from(!*ok)));
                    code.instruction(&Ins::I64Store(memarg(result.tag_offset as i64)));
                }
                if layouts.is_aggregate(payload_ty) {
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(offset as i32));
                    code.instruction(&Ins::I32Add);
                    get(code, *payload);
                    code.instruction(&Ins::I32Const(layouts.types[payload_ty.0].size as i32));
                    code.instruction(&Ins::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                } else if layouts.types[payload_ty.0].size != 0 {
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(offset as i32));
                    code.instruction(&Ins::I32Add);
                    get(code, *payload);
                    coerce(
                        code,
                        value_types.get(payload).copied(),
                        valtype(layouts, payload_ty),
                    );
                    code.instruction(&store_instruction(layouts, payload_ty, 0));
                }
            }
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *value);
        }
        Instruction::ConstNull { value, ty } => {
            let tagged = ty
                .and_then(|ty| layouts.optionals.get(&ty).copied())
                .filter(|inner| !is_managed_handle(layouts, *inner));
            if let (Some(opt_ty), Some(_)) = (ty, tagged) {
                let size = layouts.types[opt_ty.0].size.max(1);
                code.instruction(&Ins::I32Const(size as i32));
                code.instruction(&Ins::Call(ctx.rt.alloc));
                code.instruction(&Ins::LocalTee(scratch));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I64Const(0));
                code.instruction(&Ins::I64Store(memarg(0)));
            } else {
                code.instruction(&Ins::I32Const(0));
            }
            set(code, *value);
        }
        Instruction::OptionalWrap {
            value,
            operand,
            ty: opt_ty,
            inner,
        } => {
            if is_managed_handle(layouts, *inner) {
                get(code, *operand);
            } else {
                let size = layouts.types[opt_ty.0].size.max(1);
                let payload_offset = optional_payload_offset(layouts, *inner);
                code.instruction(&Ins::I32Const(size as i32));
                code.instruction(&Ins::Call(ctx.rt.alloc));
                code.instruction(&Ins::LocalTee(scratch));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I64Const(1));
                code.instruction(&Ins::I64Store(memarg(0)));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(payload_offset as i32));
                code.instruction(&Ins::I32Add);
                if layouts.is_aggregate(*inner) {
                    get(code, *operand);
                    code.instruction(&Ins::I32Const(layouts.types[inner.0].size as i32));
                    code.instruction(&Ins::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                } else {
                    get(code, *operand);
                    code.instruction(&store_instruction(layouts, *inner, 0));
                }
                code.instruction(&Ins::LocalGet(scratch));
            }
            set(code, *value);
        }
        Instruction::OptionalUnwrap {
            value,
            operand,
            inner,
        } => {
            if is_managed_handle(layouts, *inner) {
                get(code, *operand);
            } else {
                let payload_offset = optional_payload_offset(layouts, *inner);
                get(code, *operand);
                code.instruction(&Ins::I32Const(payload_offset as i32));
                code.instruction(&Ins::I32Add);
                if !layouts.is_aggregate(*inner) && layouts.types[inner.0].size != 0 {
                    code.instruction(&load_instruction(layouts, *inner, 0));
                }
            }
            set(code, *value);
        }
        Instruction::OptionalIsPresent {
            value,
            operand,
            inner,
        } => {
            if is_managed_handle(layouts, *inner) {
                get(code, *operand);
                code.instruction(&Ins::I32Const(0));
                code.instruction(&Ins::I32Ne);
            } else {
                get(code, *operand);
                code.instruction(&Ins::I64Load(memarg(0)));
                code.instruction(&Ins::I64Const(0));
                code.instruction(&Ins::I64Ne);
            }
            set(code, *value);
        }
        Instruction::OptionalFromValue {
            value,
            present,
            payload,
            ty: opt_ty,
            inner,
        } => {
            if is_managed_handle(layouts, *inner) {
                get(code, *present);
                code.instruction(&Ins::If(BlockType::Result(ValType::I32)));
                get(code, *payload);
                code.instruction(&Ins::I32Load(memarg(0)));
                code.instruction(&Ins::Else);
                code.instruction(&Ins::I32Const(0));
                code.instruction(&Ins::End);
            } else {
                let size = layouts.types[opt_ty.0].size.max(1);
                let payload_offset = optional_payload_offset(layouts, *inner);
                code.instruction(&Ins::I32Const(size as i32));
                code.instruction(&Ins::Call(ctx.rt.alloc));
                code.instruction(&Ins::LocalTee(scratch));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I64Const(0));
                code.instruction(&Ins::I64Store(memarg(0)));
                if layouts.is_aggregate(*inner) {
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(payload_offset as i32));
                    code.instruction(&Ins::I32Add);
                    get(code, *payload);
                    code.instruction(&Ins::I32Const(layouts.types[inner.0].size as i32));
                    code.instruction(&Ins::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                } else if layouts.types[inner.0].size != 0 {
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(payload_offset as i32));
                    code.instruction(&Ins::I32Add);
                    get(code, *payload);
                    code.instruction(&load_instruction(layouts, *inner, 0));
                    code.instruction(&store_instruction(layouts, *inner, 0));
                }
                code.instruction(&Ins::LocalGet(scratch));
                get(code, *present);
                code.instruction(&Ins::I64ExtendI32U);
                code.instruction(&Ins::I64Store(memarg(0)));
                code.instruction(&Ins::LocalGet(scratch));
            }
            set(code, *value);
        }
        Instruction::ConstructArray {
            value,
            ty,
            elements,
        } => {
            let size = layouts.types[ty.0].size.max(1);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalTee(scratch));
            if let Some((elem, _len)) = layouts.arrays.get(ty).copied() {
                let stride = layouts.types[elem.0].size;
                for (index, element) in elements.iter().enumerate() {
                    let offset = index.saturating_mul(stride);
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(offset as i32));
                    code.instruction(&Ins::I32Add);
                    get(code, *element);
                    if layouts.is_aggregate(elem) {
                        code.instruction(&Ins::I32Const(stride as i32));
                        code.instruction(&Ins::MemoryCopy {
                            src_mem: 0,
                            dst_mem: 0,
                        });
                    } else {
                        coerce(
                            code,
                            value_types.get(element).copied(),
                            valtype(layouts, elem),
                        );
                        code.instruction(&store_instruction(layouts, elem, 0));
                    }
                }
            }
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *value);
        }
        Instruction::Construct { value, ty, fields } => {
            let size = layouts.types[ty.0].size.max(1);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalTee(scratch));
            let layout = layouts.fields.get(ty).cloned().unwrap_or_default();
            for (name, field_value) in fields {
                let Some(field) = layout.iter().find(|field| &field.name == name) else {
                    continue;
                };
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(field.offset as i32));
                code.instruction(&Ins::I32Add);
                get(code, *field_value);
                if layouts.is_aggregate(field.ty) {
                    code.instruction(&Ins::I32Const(layouts.types[field.ty.0].size as i32));
                    code.instruction(&Ins::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                } else {
                    coerce(
                        code,
                        value_types.get(field_value).copied(),
                        valtype(layouts, field.ty),
                    );
                    code.instruction(&store_instruction(layouts, field.ty, 0));
                }
            }
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *value);
        }
        Instruction::Field { value, base, name } => {
            let base_ty = value_mir_types.get(base).copied();
            let field = base_ty
                .and_then(|ty| layouts.fields.get(&ty).cloned())
                .and_then(|fields| fields.into_iter().find(|field| &field.name == name));
            if let Some(field) = field {
                get(code, *base);
                code.instruction(&Ins::I32Const(field.offset as i32));
                code.instruction(&Ins::I32Add);
                if !layouts.is_aggregate(field.ty) && layouts.types[field.ty.0].size != 0 {
                    code.instruction(&load_instruction(layouts, field.ty, 0));
                }
            } else {
                return Err(CodegenError::Backend(format!(
                    "wasm backend: unknown field `{name}`"
                )));
            }
            set(code, *value);
        }
        Instruction::FieldStore {
            base,
            name,
            value: field_value,
            ..
        } => {
            let base_ty = function.locals.get(base.0).and_then(|local| local.ty);
            let field = base_ty
                .and_then(|ty| layouts.fields.get(&ty).cloned())
                .and_then(|fields| fields.into_iter().find(|field| &field.name == name));
            if let Some(field) = field {
                code.instruction(&Ins::LocalGet(map.get(*base)));
                code.instruction(&Ins::I32Const(field.offset as i32));
                code.instruction(&Ins::I32Add);
                get(code, *field_value);
                if layouts.is_aggregate(field.ty) {
                    code.instruction(&Ins::I32Const(layouts.types[field.ty.0].size as i32));
                    code.instruction(&Ins::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                } else {
                    coerce(
                        code,
                        value_types.get(field_value).copied(),
                        valtype(layouts, field.ty),
                    );
                    code.instruction(&store_instruction(layouts, field.ty, 0));
                }
            } else {
                return Err(CodegenError::Backend(format!(
                    "wasm backend: unknown field `{name}`"
                )));
            }
        }
        Instruction::BorrowField { value, base, name } => {
            let base_ty = function.locals.get(base.0).and_then(|local| local.ty);
            let field = base_ty
                .and_then(|ty| layouts.fields.get(&ty).cloned())
                .and_then(|fields| fields.into_iter().find(|field| &field.name == name));
            if let Some(field) = field {
                code.instruction(&Ins::LocalGet(map.get(*base)));
                code.instruction(&Ins::I32Const(field.offset as i32));
                code.instruction(&Ins::I32Add);
            } else {
                return Err(CodegenError::Backend(format!(
                    "wasm backend: unknown field `{name}`"
                )));
            }
            set(code, *value);
        }
        Instruction::CopyAggregate { value, source, ty } => {
            let size = layouts.types[ty.0].size.max(1);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalTee(scratch));
            code.instruction(&Ins::LocalGet(scratch));
            get(code, *source);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::MemoryCopy {
                src_mem: 0,
                dst_mem: 0,
            });
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *value);
        }
        Instruction::EnumTag { result, value, ty } => {
            let tag = layouts.enums.get(ty).map_or(0, |layout| layout.tag_offset);
            get(code, *value);
            code.instruction(&match layouts.enums.get(ty).map(|e| e.tag_size) {
                Some(1) => Ins::I32Load8U(memarg(tag as i64)),
                Some(2) => Ins::I32Load16U(memarg(tag as i64)),
                Some(4) => Ins::I32Load(memarg(tag as i64)),
                _ => Ins::I64Load(memarg(tag as i64)),
            });
            if layouts.enums.get(ty).map_or(8, |e| e.tag_size) < 8 {
                code.instruction(&Ins::I64ExtendI32U);
            }
            set(code, *result);
        }
        Instruction::ConstructEnum {
            value,
            ty,
            variant_index,
            payload,
        } => {
            let size = layouts.types[ty.0].size.max(1);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalTee(scratch));
            if let Some(layout) = layouts.enums.get(ty).cloned() {
                code.instruction(&Ins::LocalGet(scratch));
                match layout.tag_size {
                    1 => {
                        code.instruction(&Ins::I32Const(*variant_index as i32));
                        code.instruction(&Ins::I32Store8(memarg(layout.tag_offset as i64)));
                    }
                    2 => {
                        code.instruction(&Ins::I32Const(*variant_index as i32));
                        code.instruction(&Ins::I32Store16(memarg(layout.tag_offset as i64)));
                    }
                    4 => {
                        code.instruction(&Ins::I32Const(*variant_index as i32));
                        code.instruction(&Ins::I32Store(memarg(layout.tag_offset as i64)));
                    }
                    _ => {
                        code.instruction(&Ins::I64Const(*variant_index as i64));
                        code.instruction(&Ins::I64Store(memarg(layout.tag_offset as i64)));
                    }
                }
                if let Some(variant) = layout.variants.get(*variant_index) {
                    for (field, field_value) in variant.fields.iter().zip(payload) {
                        let offset = field.offset;
                        code.instruction(&Ins::LocalGet(scratch));
                        code.instruction(&Ins::I32Const(offset as i32));
                        code.instruction(&Ins::I32Add);
                        get(code, *field_value);
                        if layouts.is_aggregate(field.ty) {
                            code.instruction(&Ins::I32Const(layouts.types[field.ty.0].size as i32));
                            code.instruction(&Ins::MemoryCopy {
                                src_mem: 0,
                                dst_mem: 0,
                            });
                        } else {
                            coerce(
                                code,
                                value_types.get(field_value).copied(),
                                valtype(layouts, field.ty),
                            );
                            code.instruction(&store_instruction(layouts, field.ty, 0));
                        }
                    }
                }
            }
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *value);
        }
        Instruction::EnumPayload {
            value,
            source,
            ty,
            variant_index,
            field_index,
        } => {
            let layout = layouts.enums.get(ty).cloned();
            let field = layout
                .as_ref()
                .and_then(|layout| layout.variants.get(*variant_index).cloned())
                .and_then(|variant| variant.fields.get(*field_index).cloned());
            if let Some(field) = field {
                let offset = field.offset;
                get(code, *source);
                code.instruction(&Ins::I32Const(offset as i32));
                code.instruction(&Ins::I32Add);
                if !layouts.is_aggregate(field.ty) && layouts.types[field.ty.0].size != 0 {
                    code.instruction(&load_instruction(layouts, field.ty, 0));
                }
            } else {
                return Err(CodegenError::Backend(
                    "wasm backend: unknown enum payload".into(),
                ));
            }
            set(code, *value);
        }
        Instruction::IteratorInit {
            iterator,
            iterable,
            length,
            stride,
            ..
        } => {
            let stride = stride.unwrap_or(8) as i32;
            let iterable_ty = value_mir_types.get(iterable).copied();
            let is_list = iterable_ty.is_some_and(|ty| layouts.lists.contains_key(&ty));
            let array = iterable_ty.and_then(|ty| layouts.arrays.get(&ty).copied());
            code.instruction(&Ins::I32Const(24));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalSet(scratch));
            // data
            code.instruction(&Ins::LocalGet(scratch));
            get(code, *iterable);
            if is_list {
                code.instruction(&Ins::I32Load(memarg(12)));
            }
            code.instruction(&Ins::I32Store(memarg(0)));
            // current = 0
            code.instruction(&Ins::LocalGet(scratch));
            code.instruction(&Ins::I32Const(0));
            code.instruction(&Ins::I32Store(memarg(4)));
            // length
            code.instruction(&Ins::LocalGet(scratch));
            if let Some(length) = length {
                code.instruction(&Ins::I32Const(*length as i32));
            } else if is_list {
                get(code, *iterable);
                code.instruction(&Ins::I32Load(memarg(0)));
            } else if let Some((_, len)) = array {
                code.instruction(&Ins::I32Const(len as i32));
            } else {
                code.instruction(&Ins::I32Const(0));
            }
            code.instruction(&Ins::I32Store(memarg(8)));
            // stride
            code.instruction(&Ins::LocalGet(scratch));
            code.instruction(&Ins::I32Const(stride));
            code.instruction(&Ins::I32Store(memarg(12)));
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *iterator);
        }
        Instruction::IteratorNext {
            has_value,
            value: element_value,
            index,
            iterator,
            element_type,
        } => {
            get(code, *iterator);
            code.instruction(&Ins::I32Load(memarg(4)));
            code.instruction(&Ins::LocalSet(scratch));
            code.instruction(&Ins::LocalGet(scratch));
            get(code, *iterator);
            code.instruction(&Ins::I32Load(memarg(8)));
            code.instruction(&Ins::I32LtU);
            code.instruction(&Ins::If(BlockType::Empty));
            code.instruction(&Ins::I32Const(1));
            set(code, *has_value);
            get(code, *iterator);
            code.instruction(&Ins::I32Load(memarg(4)));
            code.instruction(&Ins::I64ExtendI32U);
            set(code, *index);
            get(code, *iterator);
            code.instruction(&Ins::I32Load(memarg(0)));
            code.instruction(&Ins::LocalGet(scratch));
            get(code, *iterator);
            code.instruction(&Ins::I32Load(memarg(12)));
            code.instruction(&Ins::I32Mul);
            code.instruction(&Ins::I32Add);
            if !is_managed_handle(layouts, *element_type) && !layouts.is_aggregate(*element_type) {
                code.instruction(&load_instruction(layouts, *element_type, 0));
            }
            set(code, *element_value);
            get(code, *iterator);
            get(code, *iterator);
            code.instruction(&Ins::I32Load(memarg(4)));
            code.instruction(&Ins::I32Const(1));
            code.instruction(&Ins::I32Add);
            code.instruction(&Ins::I32Store(memarg(4)));
            code.instruction(&Ins::Else);
            code.instruction(&Ins::I32Const(0));
            set(code, *has_value);
            code.instruction(&Ins::I64Const(0));
            set(code, *index);
            default_value(code, valtype(layouts, *element_type));
            set(code, *element_value);
            code.instruction(&Ins::End);
        }
        Instruction::CallIndirect {
            value,
            callable_ty,
            callee,
            arguments,
            ..
        } => {
            let type_index = callable_ty
                .and_then(|ty| ctx.callable_types.get(&ty).copied())
                .ok_or_else(|| {
                    CodegenError::Backend("wasm backend: unknown callable type".into())
                })?;
            let params = callable_ty
                .and_then(|ty| ctx.layouts.callables.get(&ty))
                .map(|(params, _)| params.clone());
            for (position, argument) in arguments.iter().enumerate() {
                get(code, *argument);
                if let Some(param) = params.as_ref().and_then(|params| params.get(position)) {
                    coerce(
                        code,
                        value_types.get(argument).copied(),
                        valtype(layouts, *param),
                    );
                }
            }
            get(code, *callee);
            code.instruction(&Ins::CallIndirect {
                type_index,
                table_index: 0,
            });
            if let Some(value) = value {
                set(code, *value);
            }
        }
        Instruction::ConstructVariadicBuffer {
            value,
            element,
            elements,
        } => {
            let stride = layouts.types[element.0].size.max(1);
            let size = stride.saturating_mul(elements.len()).max(1);
            code.instruction(&Ins::I32Const(size as i32));
            code.instruction(&Ins::Call(ctx.rt.alloc));
            code.instruction(&Ins::LocalSet(scratch));
            for (index, element_value) in elements.iter().enumerate() {
                let offset = index.saturating_mul(stride);
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(offset as i32));
                code.instruction(&Ins::I32Add);
                get(code, *element_value);
                if layouts.is_aggregate(*element) {
                    code.instruction(&Ins::I32Const(stride as i32));
                    code.instruction(&Ins::MemoryCopy {
                        src_mem: 0,
                        dst_mem: 0,
                    });
                } else {
                    coerce(
                        code,
                        value_types.get(element_value).copied(),
                        valtype(layouts, *element),
                    );
                    code.instruction(&store_instruction(layouts, *element, 0));
                }
            }
            code.instruction(&Ins::LocalGet(scratch));
            set(code, *value);
        }
        Instruction::VariadicAt {
            value,
            data,
            index,
            element,
            ..
        } => {
            let stride = layouts.types[element.0].size.max(1);
            get(code, *data);
            get(code, *index);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::I32Const(stride as i32));
            code.instruction(&Ins::I32Mul);
            code.instruction(&Ins::I32Add);
            if !layouts.is_aggregate(*element) && layouts.types[element.0].size != 0 {
                code.instruction(&load_instruction(layouts, *element, 0));
            }
            set(code, *value);
        }
        other => {
            return Err(CodegenError::Backend(format!(
                "wasm backend: unsupported instruction {other:?}"
            )));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn lower_runtime_call(
    ctx: &LowerCtx<'_>,
    map: &LocalMap,
    value_locals: &HashMap<ValueId, u32>,
    value_types: &HashMap<ValueId, ValType>,
    borrow_of: &HashMap<ValueId, LocalId>,
    value_mir_types: &HashMap<ValueId, vut_hir::TypeId>,
    scratch: u32,
    value: Option<ValueId>,
    result_type: Option<vut_hir::TypeId>,
    function: &vut_mir::BuiltinFunction,
    arguments: &[ValueId],
    code: &mut WasmFunction,
) -> Result<(), CodegenError> {
    use vut_mir::BuiltinFunction as B;
    let get = |code: &mut WasmFunction, value: ValueId| {
        if let Some(local) = borrow_of.get(&value) {
            code.instruction(&Ins::LocalGet(map.get(*local)));
        } else {
            code.instruction(&Ins::LocalGet(value_locals[&value]));
        }
    };
    let set = |code: &mut WasmFunction, value: ValueId| {
        if let Some(index) = value_locals.get(&value) {
            code.instruction(&Ins::LocalSet(*index));
        }
    };
    let vty = |value: &ValueId| value_types.get(value).copied().unwrap_or(ValType::I64);
    match function {
        B::Print | B::Out => {
            let target = if matches!(function, B::Print) {
                ctx.rt.print
            } else {
                ctx.rt.out
            };
            get(code, arguments[0]);
            code.instruction(&Ins::Call(target));
        }
        B::ListNew => {
            let kind = result_type
                .and_then(|ty| ctx.layouts.lists.get(&ty).copied())
                .map_or(super::rc::KIND_NONE, |elem| element_kind(ctx.layouts, elem));
            code.instruction(&Ins::I32Const(kind));
            if let Some(capacity) = arguments.get(4) {
                get(code, *capacity);
                code.instruction(&Ins::I32WrapI64);
            } else {
                code.instruction(&Ins::I32Const(0));
            }
            code.instruction(&Ins::Call(ctx.rt.list_new));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ListLen | B::ListIsEmpty => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.list_len));
            if matches!(function, B::ListIsEmpty) {
                code.instruction(&Ins::I32Eqz);
            }
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::ListPush => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.list_push));
        }
        B::ListAt | B::ListAtUnchecked => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64); // index is `int`
            code.instruction(&Ins::Call(ctx.rt.list_at));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::ListSet => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            push_as_i64(
                code,
                map,
                arguments[2],
                vty(&arguments[2]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.list_set));
        }
        B::ListPop => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.list_pop));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::ListClear => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.list_clear));
        }
        B::BytesNew => {
            code.instruction(&Ins::I32Const(0));
            code.instruction(&Ins::Call(ctx.rt.bytes_new));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesLen | B::StringByteLen | B::StringCharLen | B::StringIsEmpty => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bytes_len));
            if matches!(function, B::StringIsEmpty) {
                code.instruction(&Ins::I32Eqz);
            }
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::BytesAt => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.bytes_at));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::BytesSet => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            get(code, arguments[2]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.bytes_set));
        }
        B::StringToBytes | B::BytesClone => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bytes_from_str));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesToStr => {
            // `bytes.to_str()` yields `result[str, Utf8Error]`. The wasm runtime
            // does not validate UTF-8, so the ok variant is always produced.
            if let Some(result) = result_type
                .and_then(|ty| ctx.layouts.results.get(&ty).cloned())
                .filter(|result| ctx.layouts.types[result.ok.0].size != 0)
            {
                let ty = result_type.expect("result type");
                let size = ctx.layouts.types[ty.0].size.max(1);
                code.instruction(&Ins::I32Const(size as i32));
                code.instruction(&Ins::Call(ctx.rt.alloc));
                code.instruction(&Ins::LocalSet(scratch));
                code.instruction(&Ins::LocalGet(scratch));
                if ctx.layouts.pointer_size == 4 {
                    code.instruction(&Ins::I32Const(0));
                    code.instruction(&Ins::I32Store(memarg(result.tag_offset as i64)));
                } else {
                    code.instruction(&Ins::I64Const(0));
                    code.instruction(&Ins::I64Store(memarg(result.tag_offset as i64)));
                }
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(result.ok_offset as i32));
                code.instruction(&Ins::I32Add);
                get(code, arguments[0]);
                code.instruction(&Ins::Call(ctx.rt.bytes_to_str));
                code.instruction(&Ins::I32Store(memarg(0)));
                code.instruction(&Ins::LocalGet(scratch));
            }
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::MapNew => {
            code.instruction(&Ins::Call(ctx.rt.map_new));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::MapLen => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.map_len));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::MapSet => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            push_as_i64(
                code,
                map,
                arguments[2],
                vty(&arguments[2]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.map_set));
        }
        B::MapGet | B::MapGetOr => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.map_get));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::MapContainsKey => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.map_contains));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::MapRemove => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.map_remove));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::MapClear => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.map_clear));
        }
        B::FloatToInt => {
            let arg = arguments[0];
            let source = borrow_of.get(&arg).map_or_else(
                || value_locals.get(&arg).copied().unwrap_or(0),
                |local| map.get(*local),
            );
            code.instruction(&Ins::LocalGet(source));
            if vty(&arg) == ValType::F32 {
                code.instruction(&Ins::F64PromoteF32);
            }
            code.instruction(&Ins::I64TruncSatF64S);
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::IntToFloat => {
            let arg = arguments[0];
            let source = borrow_of.get(&arg).map_or_else(
                || value_locals.get(&arg).copied().unwrap_or(0),
                |local| map.get(*local),
            );
            code.instruction(&Ins::LocalGet(source));
            if vty(&arg) == ValType::I32 {
                code.instruction(&Ins::F64ConvertI32S);
            } else {
                code.instruction(&Ins::F64ConvertI64S);
            }
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::NumericCast { from, to } => {
            let arg = arguments[0];
            let source = borrow_of.get(&arg).map_or_else(
                || value_locals.get(&arg).copied().unwrap_or(0),
                |local| map.get(*local),
            );
            let dest_ty = value.map_or(ValType::I64, |v| vty(&v));
            let dest = value.and_then(|v| value_locals.get(&v).copied());
            emit_numeric_cast(
                code,
                *from,
                *to,
                u32::try_from(ctx.layouts.pointer_size).unwrap_or(8) * 8,
                source,
                vty(&arg),
                dest_ty,
                dest,
            );
        }
        B::StringEquals => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_eq));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringCompare => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_cmp));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringContains => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_contains));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringStartsWith => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_starts));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringEndsWith => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_ends));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringFind => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_find));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringRfind => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_rfind));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringSubstring => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            get(code, arguments[2]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.str_substring));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringRepeat => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.str_repeat));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringStripPrefix => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_strip_prefix));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringStripSuffix => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.str_strip_suffix));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringTrim => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_trim_start));
            code.instruction(&Ins::Call(ctx.rt.str_trim_end));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringTrimStart => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_trim_start));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringTrimEnd => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_trim_end));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringToLower => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_lower));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringToUpper => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_upper));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringToI64 => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_to_i64));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringToF64 => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.str_to_f64));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringCharAt => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.so_char_at));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringChars => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.so_chars));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringPadLeft => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            get(code, arguments[2]);
            code.instruction(&Ins::Call(ctx.rt.so_pad_left));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringPadRight => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            get(code, arguments[2]);
            code.instruction(&Ins::Call(ctx.rt.so_pad_right));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringSplit => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.so_split));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringSplitWhitespace => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.so_split_ws));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringLines => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.so_lines));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::StringReplace => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            get(code, arguments[2]);
            code.instruction(&Ins::Call(ctx.rt.so_replace));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesIsEmpty => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bytes_len));
            code.instruction(&Ins::I32Eqz);
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesCapacity => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_capacity));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::BytesReserve => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.bt_reserve));
        }
        B::BytesFirst => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_first));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::BytesLast => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_last));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::BytesSlice => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            get(code, arguments[2]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.bt_slice));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesByteAt => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.bytes_at));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::BytesPush => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            if vty(&arguments[1]) == ValType::I64 {
                code.instruction(&Ins::I32WrapI64);
            }
            code.instruction(&Ins::Call(ctx.rt.bt_push));
        }
        B::BytesExtend => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.bt_extend));
        }
        B::BytesTruncate => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.bt_truncate));
        }
        B::BytesResize => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            get(code, arguments[2]);
            if vty(&arguments[2]) == ValType::I64 {
                code.instruction(&Ins::I32WrapI64);
            }
            code.instruction(&Ins::Call(ctx.rt.bt_resize));
        }
        B::BytesFind => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.bt_find));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesStartsWith => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.bt_starts));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesEndsWith => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.bt_ends));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesCompare => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.bt_cmp));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesClear => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_clear));
        }
        B::BytesToHex => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_to_hex));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesToList => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_to_list));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesFromList => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.bt_from_list));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesReadInt {
            width,
            big_endian,
            signed,
        } => {
            let flags = i32::from(*big_endian) | (i32::from(*signed) << 1);
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::I32Const(i32::from(*width)));
            code.instruction(&Ins::I32Const(flags));
            code.instruction(&Ins::Call(ctx.rt.bt_read_int));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BytesWriteInt { width, big_endian } => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::I32Const(i32::from(*width)));
            push_as_i64(
                code,
                map,
                arguments[2],
                vty(&arguments[2]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::I32Const(i32::from(*big_endian)));
            code.instruction(&Ins::Call(ctx.rt.bt_write_int));
        }
        B::ListCapacity => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.ls_capacity));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::ListReserve => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.ls_reserve));
        }
        B::ListTruncate => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.ls_truncate));
        }
        B::ListSwap => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            get(code, arguments[2]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.ls_swap));
        }
        B::ListReverse => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.ls_reverse));
        }
        B::ListShrinkToFit => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.ls_shrink));
        }
        B::ListFirst => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.ls_first));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::ListLast => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.ls_last));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::ListInsert => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            push_as_i64(
                code,
                map,
                arguments[2],
                vty(&arguments[2]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.ls_insert));
        }
        B::ListRemove => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.ls_remove));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::ListExtend => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.ls_extend));
        }
        B::ListSlice => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            get(code, arguments[2]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.ls_slice));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ListContains => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.ls_contains));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ListFindIndex => {
            get(code, arguments[0]);
            push_as_i64(
                code,
                map,
                arguments[1],
                vty(&arguments[1]),
                value_locals,
                borrow_of,
            );
            code.instruction(&Ins::Call(ctx.rt.ls_find));
            if let Some(value) = value {
                set_from_i64(code, value, vty(&value), value_locals);
            }
        }
        B::ListJoin => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::Call(ctx.rt.ls_join));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::NumericToStr => {
            let arg = arguments[0];
            get(code, arg);
            if matches!(vty(&arg), ValType::F32) {
                code.instruction(&Ins::F64PromoteF32);
            }
            if matches!(vty(&arg), ValType::F32 | ValType::F64) {
                code.instruction(&Ins::Call(ctx.rt.fmt_f64));
            } else {
                code.instruction(&Ins::Call(ctx.rt.format));
            }
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::BoolToStr => {
            let truthy = *ctx.strings.get("true").ok_or_else(|| {
                CodegenError::Backend("wasm backend: missing `true` literal".into())
            })?;
            let falsy = *ctx.strings.get("false").ok_or_else(|| {
                CodegenError::Backend("wasm backend: missing `false` literal".into())
            })?;
            get(code, arguments[0]);
            code.instruction(&Ins::If(BlockType::Result(ValType::I32)));
            code.instruction(&Ins::I32Const(truthy as i32));
            code.instruction(&Ins::Call(ctx.rt.str_from_static));
            code.instruction(&Ins::Else);
            code.instruction(&Ins::I32Const(falsy as i32));
            code.instruction(&Ins::Call(ctx.rt.str_from_static));
            code.instruction(&Ins::End);
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ArrayLen => {
            let len = array_info(ctx.layouts, value_mir_types, arguments[0]).map_or(0, |(_, l)| l);
            code.instruction(&Ins::I64Const(len as i64));
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ArrayAt | B::ArrayAtUnchecked => {
            if let Some((elem, _)) = array_info(ctx.layouts, value_mir_types, arguments[0]) {
                let stride = ctx.layouts.types[elem.0].size;
                get(code, arguments[0]);
                get(code, arguments[1]);
                code.instruction(&Ins::I32WrapI64);
                code.instruction(&Ins::I32Const(stride as i32));
                code.instruction(&Ins::I32Mul);
                code.instruction(&Ins::I32Add);
                if !ctx.layouts.is_aggregate(elem) && ctx.layouts.types[elem.0].size != 0 {
                    code.instruction(&load_instruction(ctx.layouts, elem, 0));
                }
            } else {
                return Err(CodegenError::Backend("wasm backend: array element".into()));
            }
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ArraySet => {
            if let Some((elem, _)) = array_info(ctx.layouts, value_mir_types, arguments[0]) {
                let stride = ctx.layouts.types[elem.0].size;
                get(code, arguments[0]);
                get(code, arguments[1]);
                code.instruction(&Ins::I32WrapI64);
                code.instruction(&Ins::I32Const(stride as i32));
                code.instruction(&Ins::I32Mul);
                code.instruction(&Ins::I32Add);
                get(code, arguments[2]);
                coerce(
                    code,
                    value_types.get(&arguments[2]).copied(),
                    valtype(ctx.layouts, elem),
                );
                code.instruction(&store_instruction(ctx.layouts, elem, 0));
            }
        }
        B::ArrayFirst | B::ArrayLast => {
            if let Some((elem, len)) = array_info(ctx.layouts, value_mir_types, arguments[0]) {
                let stride = ctx.layouts.types[elem.0].size;
                let index = if matches!(function, B::ArrayLast) {
                    len.saturating_sub(1)
                } else {
                    0
                };
                get(code, arguments[0]);
                code.instruction(&Ins::I32Const(index.saturating_mul(stride) as i32));
                code.instruction(&Ins::I32Add);
                if !ctx.layouts.is_aggregate(elem) && ctx.layouts.types[elem.0].size != 0 {
                    code.instruction(&load_instruction(ctx.layouts, elem, 0));
                }
            }
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::ArrayFill => {
            if let Some((elem, len)) = array_info(ctx.layouts, value_mir_types, arguments[0]) {
                let stride = ctx.layouts.types[elem.0].size;
                code.instruction(&Ins::I32Const(0));
                code.instruction(&Ins::LocalSet(scratch));
                code.instruction(&Ins::Block(BlockType::Empty));
                code.instruction(&Ins::Loop(BlockType::Empty));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(len as i32));
                code.instruction(&Ins::I32GeU);
                code.instruction(&Ins::BrIf(1));
                get(code, arguments[0]);
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(stride as i32));
                code.instruction(&Ins::I32Mul);
                code.instruction(&Ins::I32Add);
                get(code, arguments[1]);
                coerce(
                    code,
                    value_types.get(&arguments[1]).copied(),
                    valtype(ctx.layouts, elem),
                );
                code.instruction(&store_instruction(ctx.layouts, elem, 0));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(1));
                code.instruction(&Ins::I32Add);
                code.instruction(&Ins::LocalSet(scratch));
                code.instruction(&Ins::Br(0));
                code.instruction(&Ins::End);
                code.instruction(&Ins::End);
            }
        }
        B::ArrayToList => {
            if let (Some((elem, len)), Some(value_id)) = (
                array_info(ctx.layouts, value_mir_types, arguments[0]),
                value,
            ) {
                {
                    let stride = ctx.layouts.types[elem.0].size;
                    let kind = element_kind(ctx.layouts, elem);
                    code.instruction(&Ins::I32Const(kind));
                    code.instruction(&Ins::I32Const(len as i32));
                    code.instruction(&Ins::Call(ctx.rt.list_new));
                    set(code, value_id);
                    code.instruction(&Ins::I32Const(0));
                    code.instruction(&Ins::LocalSet(scratch));
                    code.instruction(&Ins::Block(BlockType::Empty));
                    code.instruction(&Ins::Loop(BlockType::Empty));
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(len as i32));
                    code.instruction(&Ins::I32GeU);
                    code.instruction(&Ins::BrIf(1));
                    get(code, value_id);
                    get(code, arguments[0]);
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(stride as i32));
                    code.instruction(&Ins::I32Mul);
                    code.instruction(&Ins::I32Add);
                    if !ctx.layouts.is_aggregate(elem) && stride != 0 {
                        code.instruction(&load_instruction(ctx.layouts, elem, 0));
                        widen_to_slot(code, valtype(ctx.layouts, elem));
                    }
                    code.instruction(&Ins::Call(ctx.rt.list_push));
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(1));
                    code.instruction(&Ins::I32Add);
                    code.instruction(&Ins::LocalSet(scratch));
                    code.instruction(&Ins::Br(0));
                    code.instruction(&Ins::End);
                    code.instruction(&Ins::End);
                }
            }
        }
        B::ArrayContains => {
            if let Some((elem, len)) = array_info(ctx.layouts, value_mir_types, arguments[0]) {
                let stride = ctx.layouts.types[elem.0].size;
                let kind = element_kind(ctx.layouts, elem);
                if let Some(value_id) = value {
                    code.instruction(&Ins::I32Const(0));
                    set(code, value_id);
                    code.instruction(&Ins::I32Const(0));
                    code.instruction(&Ins::LocalSet(scratch));
                    code.instruction(&Ins::Block(BlockType::Empty));
                    code.instruction(&Ins::Loop(BlockType::Empty));
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(len as i32));
                    code.instruction(&Ins::I32GeU);
                    code.instruction(&Ins::BrIf(1));
                    get(code, arguments[0]);
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(stride as i32));
                    code.instruction(&Ins::I32Mul);
                    code.instruction(&Ins::I32Add);
                    push_as_i64(
                        code,
                        map,
                        arguments[1],
                        vty(&arguments[1]),
                        value_locals,
                        borrow_of,
                    );
                    code.instruction(&Ins::I32Const(kind));
                    code.instruction(&Ins::Call(ctx.rt.ls_elem_eq));
                    code.instruction(&Ins::If(BlockType::Empty));
                    code.instruction(&Ins::I32Const(1));
                    set(code, value_id);
                    code.instruction(&Ins::Br(2));
                    code.instruction(&Ins::End);
                    code.instruction(&Ins::LocalGet(scratch));
                    code.instruction(&Ins::I32Const(1));
                    code.instruction(&Ins::I32Add);
                    code.instruction(&Ins::LocalSet(scratch));
                    code.instruction(&Ins::Br(0));
                    code.instruction(&Ins::End);
                    code.instruction(&Ins::End);
                }
            }
        }
        B::MapIsEmpty => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.map_len));
            code.instruction(&Ins::I32Eqz);
            if let Some(value) = value {
                set(code, value);
            }
        }
        B::MapCapacity => {
            get(code, arguments[0]);
            code.instruction(&Ins::Call(ctx.rt.map_capacity));
            if let Some(value) = value {
                set_from_i32(code, value, vty(&value), value_locals);
            }
        }
        B::MapReserve => {
            get(code, arguments[0]);
            get(code, arguments[1]);
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::Call(ctx.rt.map_reserve));
        }
        B::MapKeys | B::MapValues => {
            let keys = matches!(function, B::MapKeys);
            if let Some(value_id) = value {
                let element = value_mir_types
                    .get(&arguments[0])
                    .and_then(|ty| ctx.layouts.maps.get(ty).copied())
                    .map(|(key, val)| if keys { key } else { val });
                let kind = element.map_or(super::rc::KIND_NONE, |element| {
                    element_kind(ctx.layouts, element)
                });
                let entry_offset = if keys { 0 } else { 8 };
                code.instruction(&Ins::I32Const(kind));
                get(code, arguments[0]);
                code.instruction(&Ins::Call(ctx.rt.map_len));
                code.instruction(&Ins::Call(ctx.rt.list_new));
                set(code, value_id);
                code.instruction(&Ins::I32Const(0));
                code.instruction(&Ins::LocalSet(scratch));
                code.instruction(&Ins::Block(BlockType::Empty));
                code.instruction(&Ins::Loop(BlockType::Empty));
                code.instruction(&Ins::LocalGet(scratch));
                get(code, arguments[0]);
                code.instruction(&Ins::Call(ctx.rt.map_len));
                code.instruction(&Ins::I32GeU);
                code.instruction(&Ins::BrIf(1));
                get(code, value_id);
                get(code, arguments[0]);
                code.instruction(&Ins::I32Load(memarg(12)));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(16));
                code.instruction(&Ins::I32Mul);
                code.instruction(&Ins::I32Add);
                code.instruction(&Ins::I64Load(memarg(entry_offset)));
                code.instruction(&Ins::Call(ctx.rt.list_push));
                code.instruction(&Ins::LocalGet(scratch));
                code.instruction(&Ins::I32Const(1));
                code.instruction(&Ins::I32Add);
                code.instruction(&Ins::LocalSet(scratch));
                code.instruction(&Ins::Br(0));
                code.instruction(&Ins::End);
                code.instruction(&Ins::End);
            }
        }
        other => {
            return Err(CodegenError::Backend(format!(
                "wasm backend: unsupported runtime call {other:?}"
            )));
        }
    }
    Ok(())
}

fn retain_call(rt: &super::Runtime, ownership: OwnershipKind) -> Option<u32> {
    Some(match ownership {
        OwnershipKind::RcString => rt.str_retain,
        OwnershipKind::RcBytes => rt.bytes_retain,
        OwnershipKind::RcList => rt.list_retain,
        OwnershipKind::RcMap => rt.map_retain,
        _ => return None,
    })
}

fn release_call(rt: &super::Runtime, ownership: OwnershipKind) -> Option<u32> {
    Some(match ownership {
        OwnershipKind::RcString => rt.str_release,
        OwnershipKind::RcBytes => rt.bytes_release,
        OwnershipKind::RcList => rt.list_release,
        OwnershipKind::RcMap => rt.map_release,
        _ => return None,
    })
}

fn is_managed_handle(layouts: &LayoutTable, ty: vut_hir::TypeId) -> bool {
    matches!(
        layouts.types[ty.0].ownership,
        OwnershipKind::RcString
            | OwnershipKind::RcBytes
            | OwnershipKind::RcList
            | OwnershipKind::RcMap
            | OwnershipKind::RcChannel
            | OwnershipKind::RcVutcon
            | OwnershipKind::Resource
            | OwnershipKind::Future
            | OwnershipKind::Interface
            | OwnershipKind::OpaqueManaged
            | OwnershipKind::RcClosure
    )
}

fn optional_payload_offset(layouts: &LayoutTable, inner: vut_hir::TypeId) -> usize {
    let align = layouts.types[inner.0].alignment.max(1);
    layouts.pointer_size.div_ceil(align) * align
}

fn array_info(
    layouts: &LayoutTable,
    value_mir_types: &HashMap<ValueId, vut_hir::TypeId>,
    value: ValueId,
) -> Option<(vut_hir::TypeId, usize)> {
    value_mir_types
        .get(&value)
        .and_then(|ty| layouts.arrays.get(ty).copied())
}

fn default_value(code: &mut WasmFunction, ty: ValType) {
    match ty {
        ValType::I32 => {
            code.instruction(&Ins::I32Const(0));
        }
        ValType::F32 => {
            code.instruction(&Ins::F32Const(0.0.into()));
        }
        ValType::F64 => {
            code.instruction(&Ins::F64Const(0.0.into()));
        }
        _ => {
            code.instruction(&Ins::I64Const(0));
        }
    }
}

fn widen_to_slot(code: &mut WasmFunction, from: ValType) {
    match from {
        ValType::I32 => {
            code.instruction(&Ins::I64ExtendI32U);
        }
        ValType::F32 => {
            code.instruction(&Ins::F64PromoteF32);
            code.instruction(&Ins::I64ReinterpretF64);
        }
        ValType::F64 => {
            code.instruction(&Ins::I64ReinterpretF64);
        }
        _ => {}
    }
}

fn element_kind(layouts: &LayoutTable, ty: vut_hir::TypeId) -> i32 {
    if matches!(layouts.types[ty.0].repr, vut_mir::ValueRepr::Float) {
        return super::rc::KIND_FLOAT;
    }
    match layouts.types[ty.0].ownership {
        OwnershipKind::RcString => super::rc::KIND_STRING,
        OwnershipKind::RcBytes => super::rc::KIND_BYTES,
        OwnershipKind::RcList => super::rc::KIND_LIST,
        OwnershipKind::RcMap => super::rc::KIND_MAP,
        _ => super::rc::KIND_NONE,
    }
}

/// Emits an implicit numeric coercion between wasm value types.
fn coerce(code: &mut WasmFunction, from: Option<ValType>, to: ValType) {
    match (from, to) {
        (Some(ValType::I64), ValType::I32) => {
            code.instruction(&Ins::I32WrapI64);
        }
        (Some(ValType::I32), ValType::I64) => {
            code.instruction(&Ins::I64ExtendI32U);
        }
        (Some(ValType::F64), ValType::F32) => {
            code.instruction(&Ins::F32DemoteF64);
        }
        (Some(ValType::F32), ValType::F64) => {
            code.instruction(&Ins::F64PromoteF32);
        }
        (Some(ValType::I64), ValType::F64) => {
            code.instruction(&Ins::F64ConvertI64S);
        }
        (Some(ValType::I32), ValType::F64) => {
            code.instruction(&Ins::F64ConvertI32S);
        }
        _ => {}
    }
}

/// Inclusive lower, exclusive upper bound for a float -> integer conversion.
fn float_int_range(to: NumericKind, pointer_bits: u32) -> (f64, f64) {
    let bits = if to.bits() == 0 {
        pointer_bits
    } else {
        to.bits()
    };
    if to.is_unsigned() {
        (0.0, 2_f64.powi(i32::try_from(bits).unwrap_or(64)))
    } else {
        (
            -2_f64.powi(i32::try_from(bits).unwrap_or(64) - 1),
            2_f64.powi(i32::try_from(bits).unwrap_or(64) - 1),
        )
    }
}

/// Lowers an explicit numeric conversion with Vut's checked semantics. Integer
/// narrowing and sign changes are range-checked and trap (`unreachable`); float
/// to integer rejects NaN and out-of-range values. No conversion silently wraps.
#[allow(clippy::too_many_arguments)]
#[expect(
    clippy::too_many_lines,
    reason = "one branch per numeric conversion family"
)]
fn emit_numeric_cast(
    code: &mut WasmFunction,
    from: NumericKind,
    to: NumericKind,
    pointer_bits: u32,
    source: u32,
    source_ty: ValType,
    dest_ty: ValType,
    dest: Option<u32>,
) {
    let to_bits = if to.bits() == 0 {
        pointer_bits
    } else {
        to.bits()
    };
    let store = |code: &mut WasmFunction| {
        if let Some(index) = dest {
            code.instruction(&Ins::LocalSet(index));
        }
    };
    if !from.is_float() && !to.is_float() {
        let unsigned = from.is_unsigned();
        let push_wide = |code: &mut WasmFunction| {
            code.instruction(&Ins::LocalGet(source));
            if source_ty == ValType::I32 {
                if unsigned {
                    code.instruction(&Ins::I64ExtendI32U);
                } else {
                    code.instruction(&Ins::I64ExtendI32S);
                }
            }
        };
        push_wide(code);
        if to_bits >= 64 {
            if to.is_unsigned() && !from.is_unsigned() {
                code.instruction(&Ins::I64Const(0));
                code.instruction(&Ins::I64GeS);
            } else {
                code.instruction(&Ins::I64Const(1));
            }
        } else if to.is_unsigned() {
            push_wide(code);
            code.instruction(&Ins::I64Const(0));
            code.instruction(&Ins::I64GeS);
            push_wide(code);
            code.instruction(&Ins::I64Const(1_i64 << to_bits));
            code.instruction(&Ins::I64LtS);
            code.instruction(&Ins::I32And);
        } else {
            let min = -(1_i64 << (to_bits - 1));
            let max = 1_i64 << (to_bits - 1);
            push_wide(code);
            code.instruction(&Ins::I64Const(min));
            code.instruction(&Ins::I64GeS);
            push_wide(code);
            code.instruction(&Ins::I64Const(max));
            code.instruction(&Ins::I64LtS);
            code.instruction(&Ins::I32And);
        }
        code.instruction(&Ins::If(BlockType::Result(dest_ty)));
        push_wide(code);
        if dest_ty == ValType::I32 {
            code.instruction(&Ins::I32WrapI64);
        }
        code.instruction(&Ins::Else);
        code.instruction(&Ins::Unreachable);
        code.instruction(&Ins::End);
        store(code);
        return;
    }
    if !from.is_float() && to.is_float() {
        code.instruction(&Ins::LocalGet(source));
        if source_ty == ValType::I32 {
            if from.is_unsigned() {
                code.instruction(&Ins::I64ExtendI32U);
            } else {
                code.instruction(&Ins::I64ExtendI32S);
            }
        }
        if to == NumericKind::F32 {
            if from.is_unsigned() {
                code.instruction(&Ins::F32ConvertI64U);
            } else {
                code.instruction(&Ins::F32ConvertI64S);
            }
        } else if from.is_unsigned() {
            code.instruction(&Ins::F64ConvertI64U);
        } else {
            code.instruction(&Ins::F64ConvertI64S);
        }
        store(code);
        return;
    }
    if from.is_float() && !to.is_float() {
        let push_float = |code: &mut WasmFunction| {
            code.instruction(&Ins::LocalGet(source));
            if source_ty == ValType::F32 {
                code.instruction(&Ins::F64PromoteF32);
            }
        };
        let (min, max) = float_int_range(to, pointer_bits);
        push_float(code);
        code.instruction(&Ins::F64Abs);
        code.instruction(&Ins::F64Const(f64::INFINITY.into()));
        code.instruction(&Ins::F64Lt);
        push_float(code);
        code.instruction(&Ins::F64Trunc);
        code.instruction(&Ins::F64Const(min.into()));
        code.instruction(&Ins::F64Ge);
        code.instruction(&Ins::I32And);
        push_float(code);
        code.instruction(&Ins::F64Trunc);
        code.instruction(&Ins::F64Const(max.into()));
        code.instruction(&Ins::F64Lt);
        code.instruction(&Ins::I32And);
        code.instruction(&Ins::If(BlockType::Result(dest_ty)));
        push_float(code);
        if to.is_unsigned() {
            code.instruction(&Ins::I64TruncF64U);
        } else {
            code.instruction(&Ins::I64TruncF64S);
        }
        if dest_ty == ValType::I32 {
            code.instruction(&Ins::I32WrapI64);
        }
        code.instruction(&Ins::Else);
        code.instruction(&Ins::Unreachable);
        code.instruction(&Ins::End);
        store(code);
        return;
    }
    code.instruction(&Ins::LocalGet(source));
    if source_ty == ValType::F32 && to != NumericKind::F32 {
        code.instruction(&Ins::F64PromoteF32);
    } else if source_ty != ValType::F32 && to == NumericKind::F32 {
        code.instruction(&Ins::F32DemoteF64);
    }
    store(code);
}

fn push_as_i64(
    code: &mut WasmFunction,
    map: &LocalMap,
    value: ValueId,
    vty: ValType,
    value_locals: &HashMap<ValueId, u32>,
    borrow_of: &HashMap<ValueId, LocalId>,
) {
    if let Some(local) = borrow_of.get(&value) {
        code.instruction(&Ins::LocalGet(map.get(*local)));
    } else {
        code.instruction(&Ins::LocalGet(value_locals[&value]));
    }
    match vty {
        ValType::I32 => {
            code.instruction(&Ins::I64ExtendI32U);
        }
        ValType::F32 => {
            code.instruction(&Ins::F64PromoteF32);
            code.instruction(&Ins::I64ReinterpretF64);
        }
        ValType::F64 => {
            code.instruction(&Ins::I64ReinterpretF64);
        }
        _ => {}
    }
}

fn set_from_i32(
    code: &mut WasmFunction,
    value: ValueId,
    vty: ValType,
    value_locals: &HashMap<ValueId, u32>,
) {
    match vty {
        ValType::I64 => {
            code.instruction(&Ins::I64ExtendI32U);
        }
        ValType::F64 => {
            code.instruction(&Ins::F64ConvertI32U);
        }
        _ => {}
    }
    if let Some(index) = value_locals.get(&value) {
        code.instruction(&Ins::LocalSet(*index));
    }
}

fn set_from_i64(
    code: &mut WasmFunction,
    value: ValueId,
    vty: ValType,
    value_locals: &HashMap<ValueId, u32>,
) {
    match vty {
        ValType::I32 => {
            code.instruction(&Ins::I32WrapI64);
        }
        ValType::F32 => {
            code.instruction(&Ins::I32WrapI64);
            code.instruction(&Ins::F32ReinterpretI32);
        }
        ValType::F64 => {
            code.instruction(&Ins::F64ReinterpretI64);
        }
        _ => {}
    }
    if let Some(index) = value_locals.get(&value) {
        code.instruction(&Ins::LocalSet(*index));
    }
}

fn binary_instruction(
    op: BinaryOp,
    ty: ValType,
    unsigned: bool,
) -> Result<(Ins<'static>, bool), CodegenError> {
    let comparison = matches!(
        op,
        BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
    );
    let instruction = match (ty, op) {
        (ValType::I64, BinaryOp::Add) => Ins::I64Add,
        (ValType::I64, BinaryOp::Subtract) => Ins::I64Sub,
        (ValType::I64, BinaryOp::Multiply) => Ins::I64Mul,
        (ValType::I64, BinaryOp::Divide) if unsigned => Ins::I64DivU,
        (ValType::I64, BinaryOp::Divide) => Ins::I64DivS,
        (ValType::I64, BinaryOp::Modulo) if unsigned => Ins::I64RemU,
        (ValType::I64, BinaryOp::Modulo) => Ins::I64RemS,
        (ValType::I64, BinaryOp::Equal) => Ins::I64Eq,
        (ValType::I64, BinaryOp::NotEqual) => Ins::I64Ne,
        (ValType::I64, BinaryOp::Less) if unsigned => Ins::I64LtU,
        (ValType::I64, BinaryOp::Less) => Ins::I64LtS,
        (ValType::I64, BinaryOp::LessEqual) if unsigned => Ins::I64LeU,
        (ValType::I64, BinaryOp::LessEqual) => Ins::I64LeS,
        (ValType::I64, BinaryOp::Greater) if unsigned => Ins::I64GtU,
        (ValType::I64, BinaryOp::Greater) => Ins::I64GtS,
        (ValType::I64, BinaryOp::GreaterEqual) if unsigned => Ins::I64GeU,
        (ValType::I64, BinaryOp::GreaterEqual) => Ins::I64GeS,
        (ValType::I32, BinaryOp::Add) => Ins::I32Add,
        (ValType::I32, BinaryOp::Subtract) => Ins::I32Sub,
        (ValType::I32, BinaryOp::Multiply) => Ins::I32Mul,
        (ValType::I32, BinaryOp::Divide) => Ins::I32DivS,
        (ValType::I32, BinaryOp::Modulo) => Ins::I32RemS,
        (ValType::I32, BinaryOp::And) => Ins::I32And,
        (ValType::I32, BinaryOp::Or) => Ins::I32Or,
        (ValType::I32, BinaryOp::Equal) => Ins::I32Eq,
        (ValType::I32, BinaryOp::NotEqual) => Ins::I32Ne,
        (ValType::I32, BinaryOp::Less) => Ins::I32LtS,
        (ValType::I32, BinaryOp::LessEqual) => Ins::I32LeS,
        (ValType::I32, BinaryOp::Greater) => Ins::I32GtS,
        (ValType::I32, BinaryOp::GreaterEqual) => Ins::I32GeS,
        (ValType::F64, BinaryOp::Add) => Ins::F64Add,
        (ValType::F64, BinaryOp::Subtract) => Ins::F64Sub,
        (ValType::F64, BinaryOp::Multiply) => Ins::F64Mul,
        (ValType::F64, BinaryOp::Divide) => Ins::F64Div,
        (ValType::F64, BinaryOp::Equal) => Ins::F64Eq,
        (ValType::F64, BinaryOp::NotEqual) => Ins::F64Ne,
        (ValType::F64, BinaryOp::Less) => Ins::F64Lt,
        (ValType::F64, BinaryOp::LessEqual) => Ins::F64Le,
        (ValType::F64, BinaryOp::Greater) => Ins::F64Gt,
        (ValType::F64, BinaryOp::GreaterEqual) => Ins::F64Ge,
        _ => {
            return Err(CodegenError::Backend(format!(
                "wasm backend: unsupported binary op {op:?} on {ty:?}"
            )));
        }
    };
    Ok((instruction, comparison))
}

fn load_instruction(layouts: &LayoutTable, ty: vut_hir::TypeId, offset: i64) -> Ins<'static> {
    let info = layouts.types[ty.0];
    match info.repr {
        vut_mir::ValueRepr::Float if info.size == 4 => Ins::F32Load(memarg(offset)),
        vut_mir::ValueRepr::Float => Ins::F64Load(memarg(offset)),
        vut_mir::ValueRepr::Integer | vut_mir::ValueRepr::Pointer => match info.size {
            1 => Ins::I32Load8S(memarg(offset)),
            2 => Ins::I32Load16S(memarg(offset)),
            4 => Ins::I32Load(memarg(offset)),
            _ => Ins::I64Load(memarg(offset)),
        },
        vut_mir::ValueRepr::Void => Ins::I32Load(memarg(offset)),
    }
}

fn store_instruction(layouts: &LayoutTable, ty: vut_hir::TypeId, offset: i64) -> Ins<'static> {
    let info = layouts.types[ty.0];
    match info.repr {
        vut_mir::ValueRepr::Float if info.size == 4 => Ins::F32Store(memarg(offset)),
        vut_mir::ValueRepr::Float => Ins::F64Store(memarg(offset)),
        vut_mir::ValueRepr::Integer | vut_mir::ValueRepr::Pointer => match info.size {
            1 => Ins::I32Store8(memarg(offset)),
            2 => Ins::I32Store16(memarg(offset)),
            4 => Ins::I32Store(memarg(offset)),
            _ => Ins::I64Store(memarg(offset)),
        },
        vut_mir::ValueRepr::Void => Ins::I32Store(memarg(offset)),
    }
}

fn defined_value(instruction: &Instruction) -> Option<ValueId> {
    match instruction {
        Instruction::ConstInt { value, .. }
        | Instruction::ConstBool { value, .. }
        | Instruction::ConstFloat { value, .. }
        | Instruction::ConstString { value, .. }
        | Instruction::ConstNull { value, .. }
        | Instruction::OptionalWrap { value, .. }
        | Instruction::OptionalUnwrap { value, .. }
        | Instruction::OptionalIsPresent { value, .. }
        | Instruction::OptionalFromValue { value, .. }
        | Instruction::ConstructResult { value, .. }
        | Instruction::ResultState { result: value, .. }
        | Instruction::ResultPayload { value, .. }
        | Instruction::Construct { value, .. }
        | Instruction::ConstructArray { value, .. }
        | Instruction::ConstructEnum { value, .. }
        | Instruction::EnumTag { result: value, .. }
        | Instruction::EnumPayload { value, .. }
        | Instruction::CopyAggregate { value, .. }
        | Instruction::ConstructVariadicBuffer { value, .. }
        | Instruction::VariadicAt { value, .. }
        | Instruction::MakeClosure { value, .. }
        | Instruction::CallIndirect {
            value: Some(value), ..
        }
        | Instruction::ReloadValue { value, .. }
        | Instruction::FrameState { value, .. }
        | Instruction::IteratorInit {
            iterator: value, ..
        }
        | Instruction::IteratorNext { value, .. }
        | Instruction::PollFuture {
            value: Some(value), ..
        }
        | Instruction::PollChannelRecv { value, .. }
        | Instruction::ConstructInterface { value, .. }
        | Instruction::Copy { value, .. }
        | Instruction::Move { value, .. }
        | Instruction::Borrow { value, .. }
        | Instruction::Binary { value, .. }
        | Instruction::Unary { value, .. }
        | Instruction::ConcatString { value, .. }
        | Instruction::FormatValue { value, .. }
        | Instruction::TypeRetain { value, .. }
        | Instruction::TypeRelease { value, .. }
        | Instruction::MakeUnique { value, .. }
        | Instruction::Call {
            value: Some(value), ..
        }
        | Instruction::RuntimeCall {
            value: Some(value), ..
        }
        | Instruction::StartFuture {
            value: Some(value), ..
        }
        | Instruction::AwaitFuture { value, .. }
        | Instruction::Spawn { value, .. }
        | Instruction::MakeFunction { value, .. }
        | Instruction::Allocate { value, .. }
        | Instruction::LoadRaw { value, .. }
        | Instruction::Field { value, .. } => Some(*value),
        _ => None,
    }
}

/// The MIR result type of a value-producing instruction, when it has one.
fn value_result_type(function: &Function, instruction: &Instruction) -> Option<vut_hir::TypeId> {
    match instruction {
        Instruction::Copy { local, .. }
        | Instruction::Move { local, .. }
        | Instruction::Borrow { local, .. } => function.locals.get(local.0).and_then(|l| l.ty),
        Instruction::ConstNull { ty, .. } => *ty,
        Instruction::OptionalWrap { ty, .. }
        | Instruction::OptionalFromValue { ty, .. }
        | Instruction::ConstructResult { ty, .. }
        | Instruction::Construct { ty, .. }
        | Instruction::ConstructArray { ty, .. }
        | Instruction::ConstructEnum { ty, .. }
        | Instruction::CopyAggregate { ty, .. }
        | Instruction::Allocate { ty, .. }
        | Instruction::LoadRaw { ty, .. }
        | Instruction::ConstructInterface { ty, .. } => Some(*ty),
        Instruction::OptionalUnwrap { inner, .. }
        | Instruction::IteratorNext {
            element_type: inner,
            ..
        } => Some(*inner),
        Instruction::Call { result_type, .. }
        | Instruction::RuntimeCall { result_type, .. }
        | Instruction::StartFuture { result_type, .. } => *result_type,
        Instruction::AwaitFuture { result_type, .. } | Instruction::Spawn { result_type, .. } => {
            Some(*result_type)
        }
        _ => None,
    }
}

#[expect(clippy::too_many_lines, reason = "one arm per producing instruction")]
fn infer_type(
    layouts: &LayoutTable,
    function: &Function,
    instruction: &Instruction,
    value_types: &HashMap<ValueId, ValType>,
    value_mir_types: &HashMap<ValueId, vut_hir::TypeId>,
) -> ValType {
    match instruction {
        Instruction::ConstInt { .. } => ValType::I64,
        Instruction::FrameState { .. } => ValType::I64,
        Instruction::PollFuture { result_type, .. } => valtype(layouts, *result_type),
        Instruction::ConstBool { .. } => ValType::I32,
        Instruction::ConstString { .. } => ValType::I32,
        Instruction::ConstFloat { ty, .. } => {
            if ty.is_some_and(|ty| layouts.types[ty.0].size == 4) {
                ValType::F32
            } else {
                ValType::F64
            }
        }
        Instruction::Binary {
            operand_type, op, ..
        } => {
            let comparison = matches!(
                op,
                BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
            );
            if comparison {
                ValType::I32
            } else {
                operand_type
                    .map(|ty| valtype(layouts, ty))
                    .unwrap_or(ValType::I64)
            }
        }
        Instruction::Call { result_type, .. } | Instruction::RuntimeCall { result_type, .. } => {
            result_type
                .map(|ty| valtype(layouts, ty))
                .unwrap_or(ValType::I32)
        }
        // A started future is a pointer-sized handle, not its result value.
        Instruction::StartFuture { .. } => ValType::I32,
        Instruction::CallIndirect { result_type, .. } => result_type
            .map(|ty| valtype(layouts, ty))
            .unwrap_or(ValType::I32),
        Instruction::AwaitFuture { result_type, .. } => valtype(layouts, *result_type),
        Instruction::Spawn { .. } => ValType::I32,
        Instruction::Copy { local, .. } | Instruction::Move { local, .. } => function
            .locals
            .get(local.0)
            .and_then(|local| local.ty)
            .map_or(ValType::I32, |ty| valtype(layouts, ty)),
        Instruction::Unary { op, operand, .. } => {
            if matches!(op, UnaryOp::Not) {
                ValType::I32
            } else {
                value_types.get(operand).copied().unwrap_or(ValType::I64)
            }
        }
        Instruction::OptionalUnwrap { inner, .. } => {
            if is_managed_handle(layouts, *inner) || layouts.is_aggregate(*inner) {
                ValType::I32
            } else {
                valtype(layouts, *inner)
            }
        }
        Instruction::ResultPayload { source, ok, .. } => value_mir_types
            .get(source)
            .and_then(|ty| layouts.results.get(ty))
            .map_or(ValType::I64, |result| {
                let payload = if *ok { result.ok } else { result.err };
                if is_managed_handle(layouts, payload) || layouts.is_aggregate(payload) {
                    ValType::I32
                } else {
                    valtype(layouts, payload)
                }
            }),
        Instruction::OptionalIsPresent { .. } | Instruction::OptionalWrap { .. } => ValType::I32,
        Instruction::EnumTag { .. } => ValType::I64,
        Instruction::IteratorNext { element_type, .. } => {
            if is_managed_handle(layouts, *element_type) || layouts.is_aggregate(*element_type) {
                ValType::I32
            } else {
                valtype(layouts, *element_type)
            }
        }
        Instruction::VariadicAt { element, .. } => {
            if is_managed_handle(layouts, *element) || layouts.is_aggregate(*element) {
                ValType::I32
            } else {
                valtype(layouts, *element)
            }
        }
        Instruction::EnumPayload {
            ty,
            variant_index,
            field_index,
            ..
        } => layouts
            .enums
            .get(ty)
            .and_then(|layout| layout.variants.get(*variant_index))
            .and_then(|variant| variant.fields.get(*field_index))
            .map_or(ValType::I64, |field| {
                if is_managed_handle(layouts, field.ty) || layouts.is_aggregate(field.ty) {
                    ValType::I32
                } else {
                    valtype(layouts, field.ty)
                }
            }),
        _ => ValType::I32,
    }
}

fn mem_store(code: &mut WasmFunction, ty: ValType, offset: i32) {
    let arg = memarg(i64::from(offset));
    let _ = match ty {
        ValType::I64 => code.instruction(&Ins::I64Store(arg)),
        ValType::F32 => code.instruction(&Ins::F32Store(arg)),
        ValType::F64 => code.instruction(&Ins::F64Store(arg)),
        _ => code.instruction(&Ins::I32Store(arg)),
    };
}

fn mem_load(code: &mut WasmFunction, ty: ValType, offset: i32) {
    let arg = memarg(i64::from(offset));
    let _ = match ty {
        ValType::I64 => code.instruction(&Ins::I64Load(arg)),
        ValType::F32 => code.instruction(&Ins::F32Load(arg)),
        ValType::F64 => code.instruction(&Ins::F64Load(arg)),
        _ => code.instruction(&Ins::I32Load(arg)),
    };
}

fn unsupported(function: &Function, what: &str) -> CodegenError {
    CodegenError::Backend(format!(
        "wasm backend: {what} is not supported yet (function {})",
        function.symbol.0
    ))
}
