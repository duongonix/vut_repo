//! MIR instruction selection.
use super::CodegenError;
use super::managed::{
    const_runtime_string, element_pointer, element_value, is_managed_handle, manage_value,
    optional_from_presence, optional_payload_offset, stack_slot_for_type, zero_stack_value,
};
use super::signatures::{
    c_call_conv, coerce_integer, copy_aggregate, indirect_signature, machine_type, runtime_function,
};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{DataId, FuncId, Module};
use cranelift_object::ObjectModule;
use std::collections::HashMap;
use vut_mir::{BinaryOp, Instruction, LayoutTable, ValueId, ValueRepr};

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "instruction selection shares one Cranelift function state"
)]
pub(super) fn lower_instruction(
    builder: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    declarations: &HashMap<vut_mir::SymbolId, (FuncId, Signature)>,
    instruction: &Instruction,
    variables: &[Variable],
    local_types: &[Option<vut_hir::TypeId>],
    storages: &[vut_mir::LocalStorage],
    frame_local: Option<vut_mir::LocalId>,
    values: &mut HashMap<ValueId, cranelift_codegen::ir::Value>,
    value_types: &mut HashMap<ValueId, vut_hir::TypeId>,
    spill_types: &mut HashMap<usize, (cranelift_codegen::ir::Type, Option<vut_hir::TypeId>)>,
    layouts: &LayoutTable,
    type_functions: &HashMap<(usize, bool), FuncId>,
    vtables: &HashMap<(usize, Option<vut_mir::SymbolId>), DataId>,
    frames: &HashMap<vut_mir::SymbolId, vut_mir::FrameLayout>,
    future_thunks: &HashMap<vut_mir::SymbolId, (FuncId, FuncId)>,
    next_data: &mut usize,
) -> Result<(), CodegenError> {
    let pair = match instruction {
        Instruction::ConstInt { value, literal } => {
            Some((*value, builder.ins().iconst(types::I64, *literal)))
        }
        Instruction::ConstBool { value, literal } => {
            Some((*value, builder.ins().iconst(types::I8, i64::from(*literal))))
        }
        Instruction::ConstFloat { value, literal } => {
            Some((*value, builder.ins().f64const(*literal)))
        }
        Instruction::ConstString { value, literal } => Some((
            *value,
            const_runtime_string(builder, module, next_data, literal)?,
        )),
        Instruction::ConstNull { value, ty } => {
            let result = match ty.and_then(|ty| layouts.optionals.get(&ty).copied()) {
                Some(_) => {
                    // A tagged optional absent value is a zeroed aggregate block
                    // (discriminant 0).
                    let ty = ty.expect("optional type is present");
                    let slot = stack_slot_for_type(builder, layouts, ty)?;
                    zero_stack_value(builder, layouts, ty, slot);
                    slot
                }
                None => builder.ins().iconst(types::I64, 0),
            };
            Some((*value, result))
        }
        Instruction::OptionalWrap {
            value,
            operand,
            ty,
            inner,
        } => {
            let operand = values.get(operand).copied().ok_or_else(|| {
                CodegenError::Backend("invalid typed MIR: optional operand was not produced".into())
            })?;
            let result = if is_managed_handle(layouts, *inner) {
                operand
            } else {
                // A tagged optional is `{ discriminant = 1, payload }`.
                let slot = stack_slot_for_type(builder, layouts, *ty)?;
                let present = builder.ins().iconst(types::I64, 1);
                builder
                    .ins()
                    .store(MemFlagsData::trusted(), present, slot, 0);
                let offset =
                    i32::try_from(optional_payload_offset(layouts, *inner)).map_err(|_| {
                        CodegenError::Backend(
                            "optional payload offset exceeds backend limit".into(),
                        )
                    })?;
                if layouts.is_aggregate(*inner) {
                    let delta =
                        i64::try_from(optional_payload_offset(layouts, *inner)).map_err(|_| {
                            CodegenError::Backend("optional payload offset exceeds limit".into())
                        })?;
                    let destination = builder.ins().iadd_imm_u(slot, delta);
                    copy_aggregate(builder, layouts, *inner, operand, destination)?;
                } else {
                    builder
                        .ins()
                        .store(MemFlagsData::trusted(), operand, slot, offset);
                }
                slot
            };
            value_types.insert(*value, *ty);
            Some((*value, result))
        }
        Instruction::OptionalUnwrap {
            value,
            operand,
            inner,
        } => {
            let operand = values.get(operand).copied().ok_or_else(|| {
                CodegenError::Backend("invalid typed MIR: optional operand was not produced".into())
            })?;
            let result = if is_managed_handle(layouts, *inner) {
                operand
            } else {
                let offset =
                    i32::try_from(optional_payload_offset(layouts, *inner)).map_err(|_| {
                        CodegenError::Backend(
                            "optional payload offset exceeds backend limit".into(),
                        )
                    })?;
                if layouts.is_aggregate(*inner) {
                    builder.ins().iadd_imm_u(operand, i64::from(offset))
                } else {
                    builder.ins().load(
                        machine_type(layouts, *inner),
                        MemFlagsData::trusted(),
                        operand,
                        offset,
                    )
                }
            };
            value_types.insert(*value, *inner);
            Some((*value, result))
        }
        Instruction::OptionalIsPresent {
            value,
            operand,
            inner,
        } => {
            let operand = values.get(operand).copied().ok_or_else(|| {
                CodegenError::Backend("invalid typed MIR: optional operand was not produced".into())
            })?;
            let result = if is_managed_handle(layouts, *inner) {
                builder.ins().icmp_imm_u(IntCC::NotEqual, operand, 0)
            } else {
                // Tagged optionals carry their discriminant in the first word.
                let tag = builder
                    .ins()
                    .load(types::I64, MemFlagsData::trusted(), operand, 0);
                builder.ins().icmp_imm_u(IntCC::NotEqual, tag, 0)
            };
            Some((*value, result))
        }
        Instruction::FormatValue {
            value,
            operand,
            ty,
            unsigned,
        } => {
            let (name, parameter) = if layouts.types[ty.0].repr == ValueRepr::Float {
                (vut_runtime::abi::FORMAT_F64, types::F64)
            } else {
                (vut_runtime::abi::FORMAT_I64, types::I64)
            };
            let target = runtime_function(module, name, &[parameter], &[types::I64])?;
            let reference = module.declare_func_in_func(target, builder.func);
            let operand = values.get(operand).copied().ok_or_else(|| {
                CodegenError::Backend(format!(
                    "invalid typed MIR: formatted value {} was not produced",
                    operand.0
                ))
            })?;
            let argument = if *unsigned
                && builder.func.dfg.value_type(operand).is_int()
                && builder.func.dfg.value_type(operand).bits() < parameter.bits()
            {
                builder.ins().uextend(parameter, operand)
            } else {
                coerce_integer(builder, operand, parameter)
            };
            let call = builder.ins().call(reference, &[argument]);
            Some((*value, builder.inst_results(call)[0]))
        }
        Instruction::ConcatString { value, left, right } => {
            let target = runtime_function(
                module,
                vut_runtime::abi::CONCAT,
                &[types::I64, types::I64],
                &[types::I64],
            )?;
            let reference = module.declare_func_in_func(target, builder.func);
            let call = builder
                .ins()
                .call(reference, &[values[left], values[right]]);
            Some((*value, builder.inst_results(call)[0]))
        }
        Instruction::Borrow { value, local } | Instruction::Move { value, local } => {
            if let Some(ty) = local_types[local.0] {
                value_types.insert(*value, ty);
            }
            Some((
                *value,
                local_read(
                    builder,
                    variables,
                    storages,
                    frame_local,
                    layouts,
                    local_types,
                    *local,
                )?,
            ))
        }
        Instruction::Copy { value, local } => {
            let source = local_read(
                builder,
                variables,
                storages,
                frame_local,
                layouts,
                local_types,
                *local,
            )?;
            match local_types[local.0] {
                // An untyped local is a raw 64-bit word, so its copy is still a
                // usable value.
                None => Some((*value, source)),
                Some(ty) => {
                    value_types.insert(*value, ty);
                    if layouts.arrays.contains_key(&ty) {
                        let layout = layouts.types[ty.0];
                        let size = u32::try_from(layout.size.max(1)).map_err(|_| {
                            CodegenError::Backend("array copy exceeds stack-slot limit".into())
                        })?;
                        let align_shift = u8::try_from(layout.alignment.trailing_zeros())
                            .map_err(|_| CodegenError::Backend("invalid array alignment".into()))?;
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            size,
                            align_shift,
                        ));
                        let destination = builder.ins().stack_addr(types::I64, slot, 0);
                        let mut offset = 0;
                        while offset + 8 <= layout.size {
                            let offset_i32 = i32::try_from(offset).map_err(|_| {
                                CodegenError::Backend(
                                    "array copy offset exceeds backend limit".into(),
                                )
                            })?;
                            let item = builder.ins().load(
                                types::I64,
                                MemFlagsData::trusted(),
                                source,
                                offset_i32,
                            );
                            builder.ins().store(
                                MemFlagsData::trusted(),
                                item,
                                destination,
                                offset_i32,
                            );
                            offset += 8;
                        }
                        while offset < layout.size {
                            let offset_i32 = i32::try_from(offset).map_err(|_| {
                                CodegenError::Backend(
                                    "array copy offset exceeds backend limit".into(),
                                )
                            })?;
                            let item = builder.ins().load(
                                types::I8,
                                MemFlagsData::trusted(),
                                source,
                                offset_i32,
                            );
                            builder.ins().store(
                                MemFlagsData::trusted(),
                                item,
                                destination,
                                offset_i32,
                            );
                            offset += 1;
                        }
                        Some((*value, destination))
                    } else {
                        Some((*value, source))
                    }
                }
            }
        }
        Instruction::Store { local, value } => {
            let target = local_types[local.0].map_or(types::I64, |ty| machine_type(layouts, ty));
            let stored = coerce_integer(builder, values[value], target);
            local_write(
                builder,
                variables,
                storages,
                frame_local,
                layouts,
                local_types,
                *local,
                stored,
            )?;
            None
        }
        Instruction::Binary {
            value,
            op,
            left,
            right,
        } => {
            let mut a = values[left];
            let mut b = values[right];
            let a_type = builder.func.dfg.value_type(a);
            let b_type = builder.func.dfg.value_type(b);
            if a_type.is_int() && b_type.is_int() && a_type != b_type {
                let target = if a_type.bits() < b_type.bits() {
                    a_type
                } else {
                    b_type
                };
                a = coerce_integer(builder, a, target);
                b = coerce_integer(builder, b, target);
            }
            let float = builder.func.dfg.value_type(a) == types::F64;
            let result = if float {
                match op {
                    BinaryOp::Add => builder.ins().fadd(a, b),
                    BinaryOp::Subtract => builder.ins().fsub(a, b),
                    BinaryOp::Multiply => builder.ins().fmul(a, b),
                    BinaryOp::Divide => builder.ins().fdiv(a, b),
                    BinaryOp::Modulo => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::F64_MOD,
                            &[types::F64, types::F64],
                            &[types::F64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[a, b]);
                        builder.inst_results(call)[0]
                    }
                    BinaryOp::Equal => builder.ins().fcmp(FloatCC::Equal, a, b),
                    BinaryOp::NotEqual => builder.ins().fcmp(FloatCC::NotEqual, a, b),
                    BinaryOp::Less => builder.ins().fcmp(FloatCC::LessThan, a, b),
                    BinaryOp::LessEqual => builder.ins().fcmp(FloatCC::LessThanOrEqual, a, b),
                    BinaryOp::Greater => builder.ins().fcmp(FloatCC::GreaterThan, a, b),
                    BinaryOp::GreaterEqual => builder.ins().fcmp(FloatCC::GreaterThanOrEqual, a, b),
                    BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::RangeExclusive
                    | BinaryOp::RangeInclusive => {
                        return Err(CodegenError::Backend(format!(
                            "invalid typed MIR: {op:?} cannot operate on floats"
                        )));
                    }
                }
            } else {
                match op {
                    BinaryOp::Add => builder.ins().iadd(a, b),
                    BinaryOp::Subtract => builder.ins().isub(a, b),
                    BinaryOp::Multiply => builder.ins().imul(a, b),
                    BinaryOp::Divide => builder.ins().sdiv(a, b),
                    BinaryOp::Modulo => builder.ins().srem(a, b),
                    BinaryOp::Equal => builder.ins().icmp(IntCC::Equal, a, b),
                    BinaryOp::NotEqual => builder.ins().icmp(IntCC::NotEqual, a, b),
                    BinaryOp::Less => builder.ins().icmp(IntCC::SignedLessThan, a, b),
                    BinaryOp::LessEqual => builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b),
                    BinaryOp::Greater => builder.ins().icmp(IntCC::SignedGreaterThan, a, b),
                    BinaryOp::GreaterEqual => {
                        builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b)
                    }
                    BinaryOp::And => builder.ins().band(a, b),
                    BinaryOp::Or => builder.ins().bor(a, b),
                    BinaryOp::RangeExclusive | BinaryOp::RangeInclusive => {
                        return Err(CodegenError::Backend(format!(
                            "invalid typed MIR: range operator {op:?} reached scalar lowering"
                        )));
                    }
                }
            };
            Some((*value, result))
        }
        Instruction::Unary { value, op, operand } => {
            let operand = values[operand];
            let result = match op {
                vut_ast::UnaryOp::Not => builder.ins().icmp_imm_s(IntCC::Equal, operand, 0),
                vut_ast::UnaryOp::Negate if builder.func.dfg.value_type(operand) == types::F64 => {
                    builder.ins().fneg(operand)
                }
                vut_ast::UnaryOp::Negate => builder.ins().ineg(operand),
                vut_ast::UnaryOp::Positive => operand,
            };
            Some((*value, result))
        }
        Instruction::Allocate { value, ty } => {
            let layout = layouts.types.get(ty.0).ok_or_else(|| {
                CodegenError::Backend(format!("missing layout for type {}", ty.0))
            })?;
            let size = u32::try_from(layout.size.max(1)).map_err(|_| {
                CodegenError::Backend("aggregate exceeds backend stack-slot limit".into())
            })?;
            let align_shift = u8::try_from(layout.alignment.trailing_zeros())
                .map_err(|_| CodegenError::Backend("invalid aggregate alignment".into()))?;
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                size,
                align_shift,
            ));
            value_types.insert(*value, *ty);
            Some((*value, builder.ins().stack_addr(types::I64, slot, 0)))
        }
        Instruction::Construct { value, ty, fields } => {
            let mut address = values[value];
            if !layouts.fields.contains_key(ty)
                && fields.iter().all(|(name, _)| name.parse::<usize>().is_ok())
            {
                let bytes = 8_usize
                    .saturating_mul(fields.len().saturating_add(1))
                    .max(16);
                let size = u32::try_from(bytes).map_err(|_| {
                    CodegenError::Backend("list literal exceeds backend stack-slot limit".into())
                })?;
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                ));
                address = builder.ins().stack_addr(types::I64, slot, 0);
                let len = builder.ins().iconst(
                    types::I64,
                    i64::try_from(fields.len()).map_err(|_| {
                        CodegenError::Backend("list literal exceeds target limit".into())
                    })?,
                );
                builder
                    .ins()
                    .store(MemFlagsData::trusted(), len, address, 0);
                values.insert(*value, address);
            }
            for (fallback_index, (name, field_value)) in fields.iter().enumerate() {
                let field = layouts.field(*ty, name);
                let offset_value = field
                    .map_or((fallback_index + 1) * layouts.pointer_size, |field| {
                        field.offset
                    });
                let offset = i32::try_from(offset_value).map_err(|_| {
                    CodegenError::Backend("field offset exceeds backend limit".into())
                })?;
                // Aggregate fields are stored inline so their bytes are copied
                // out of the producing frame; scalars are stored by value.
                if let Some(field) = field
                    && layouts.is_aggregate(field.ty)
                {
                    let delta = i64::try_from(offset_value).map_err(|_| {
                        CodegenError::Backend("field offset exceeds backend limit".into())
                    })?;
                    let destination = builder.ins().iadd_imm_u(address, delta);
                    copy_aggregate(builder, layouts, field.ty, values[field_value], destination)?;
                } else {
                    builder.ins().store(
                        MemFlagsData::trusted(),
                        values[field_value],
                        address,
                        offset,
                    );
                }
            }
            value_types.insert(*value, *ty);
            None
        }
        Instruction::ConstructArray {
            value,
            ty,
            elements,
        } => {
            let (element_type, length) = layouts.arrays[ty];
            let stride = layouts.types[element_type.0].size;
            debug_assert_eq!(length, elements.len());
            let address = values[value];
            for (index, element) in elements.iter().enumerate() {
                let offset = i32::try_from(index.saturating_mul(stride)).map_err(|_| {
                    CodegenError::Backend("array element offset exceeds backend limit".into())
                })?;
                if layouts.is_aggregate(element_type) {
                    let delta = i64::try_from(index.saturating_mul(stride)).map_err(|_| {
                        CodegenError::Backend("array element offset exceeds backend limit".into())
                    })?;
                    let destination = builder.ins().iadd_imm_u(address, delta);
                    copy_aggregate(builder, layouts, element_type, values[element], destination)?;
                } else {
                    let stored = coerce_integer(
                        builder,
                        values[element],
                        machine_type(layouts, element_type),
                    );
                    builder
                        .ins()
                        .store(MemFlagsData::trusted(), stored, address, offset);
                }
            }
            value_types.insert(*value, *ty);
            None
        }
        Instruction::ConstructVariadicBuffer {
            value,
            element,
            elements,
        } => {
            let stride = layouts.element_storage_size(*element);
            let size = stride.saturating_mul(elements.len()).max(1);
            let size = u32::try_from(size).map_err(|_| {
                CodegenError::Backend("variadic arguments exceed stack-slot limit".into())
            })?;
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                size,
                u8::try_from(layouts.types[element.0].alignment.max(1).trailing_zeros())
                    .unwrap_or(0),
            ));
            let address = builder.ins().stack_addr(types::I64, slot, 0);
            for (index, element_value) in elements.iter().enumerate() {
                let position = index.saturating_mul(stride);
                if layouts.is_aggregate(*element) {
                    let delta = i64::try_from(position).map_err(|_| {
                        CodegenError::Backend(
                            "variadic element offset exceeds backend limit".into(),
                        )
                    })?;
                    let destination = builder.ins().iadd_imm_u(address, delta);
                    copy_aggregate(
                        builder,
                        layouts,
                        *element,
                        values[element_value],
                        destination,
                    )?;
                } else {
                    let offset = i32::try_from(position).map_err(|_| {
                        CodegenError::Backend(
                            "variadic element offset exceeds backend limit".into(),
                        )
                    })?;
                    let stored = coerce_integer(
                        builder,
                        values[element_value],
                        machine_type(layouts, *element),
                    );
                    builder
                        .ins()
                        .store(MemFlagsData::trusted(), stored, address, offset);
                }
            }
            Some((*value, address))
        }
        Instruction::VariadicAt {
            value,
            data,
            len,
            index,
            element,
        } => {
            let data = values[data];
            let len = values[len];
            let index = values[index];
            let valid = builder.ins().icmp(IntCC::UnsignedLessThan, index, len);
            builder
                .ins()
                .trapz(valid, cranelift_codegen::ir::TrapCode::unwrap_user(2));
            let stride = layouts.element_storage_size(*element);
            let offset = builder.ins().imul_imm_u(
                index,
                i64::try_from(stride).map_err(|_| {
                    CodegenError::Backend("variadic stride exceeds target limit".into())
                })?,
            );
            let address = builder.ins().iadd(data, offset);
            value_types.insert(*value, *element);
            if layouts.is_aggregate(*element) {
                Some((*value, address))
            } else {
                Some((
                    *value,
                    builder.ins().load(
                        machine_type(layouts, *element),
                        MemFlagsData::trusted(),
                        address,
                        0,
                    ),
                ))
            }
        }
        Instruction::ConstructResult {
            value,
            ty,
            ok,
            payload,
        } => {
            let layout = layouts.results.get(ty).ok_or_else(|| {
                CodegenError::Backend(format!("missing result layout for type {}", ty.0))
            })?;
            let address = stack_slot_for_type(builder, layouts, *ty)?;
            let tag = builder.ins().iconst(types::I64, i64::from(!*ok));
            builder.ins().store(
                MemFlagsData::trusted(),
                tag,
                address,
                i32::try_from(layout.tag_offset).map_err(|_| {
                    CodegenError::Backend("result tag offset exceeds backend limit".into())
                })?,
            );
            let payload_ty = if *ok { layout.ok } else { layout.err };
            let payload_offset = if *ok {
                layout.ok_offset
            } else {
                layout.err_offset
            };
            let offset = i32::try_from(payload_offset).map_err(|_| {
                CodegenError::Backend("result payload offset exceeds backend limit".into())
            })?;
            // Aggregate payloads are stored inline so their bytes survive the
            // producing frame; scalars/managed handles are stored by value.
            if layouts.is_aggregate(payload_ty) {
                let delta = i64::try_from(payload_offset).map_err(|_| {
                    CodegenError::Backend("result payload offset exceeds backend limit".into())
                })?;
                let destination = builder.ins().iadd_imm_u(address, delta);
                copy_aggregate(builder, layouts, payload_ty, values[payload], destination)?;
            } else {
                let stored =
                    coerce_integer(builder, values[payload], machine_type(layouts, payload_ty));
                builder
                    .ins()
                    .store(MemFlagsData::trusted(), stored, address, offset);
            }
            value_types.insert(*value, *ty);
            Some((*value, address))
        }
        Instruction::ResultState { result, value, ok } => {
            let ty = value_types.get(value).ok_or_else(|| {
                CodegenError::Backend("missing result value type for state test".into())
            })?;
            let layout = layouts.results.get(ty).ok_or_else(|| {
                CodegenError::Backend(format!("missing result layout for type {}", ty.0))
            })?;
            let tag = builder.ins().load(
                types::I64,
                MemFlagsData::trusted(),
                values[value],
                i32::try_from(layout.tag_offset).map_err(|_| {
                    CodegenError::Backend("result tag offset exceeds backend limit".into())
                })?,
            );
            let expected = i64::from(!*ok);
            Some((
                *result,
                builder.ins().icmp_imm_s(IntCC::Equal, tag, expected),
            ))
        }
        Instruction::Spawn {
            value,
            callable,
            result_type,
            ..
        } => {
            let layout = layouts.types[result_type.0];
            let size = if layout.repr == ValueRepr::Void {
                0
            } else {
                layout.size
            };
            let align = layout.alignment.max(1);
            let pointer = module.target_config().pointer_type();
            let (poll, drop) = future_thunks.get(callable).copied().ok_or_else(|| {
                CodegenError::Backend(format!(
                    "spawn callable {} has no poll/drop thunk",
                    callable.0
                ))
            })?;
            let op = if let Some(frame_layout) = frames.get(callable) {
                // Async callable: allocate its frame and poll the frame-based body.
                let allocate = runtime_function(
                    module,
                    vut_runtime::abi::FRAME_ALLOC,
                    &[types::I64, types::I64],
                    &[types::I64],
                )?;
                let allocate = module.declare_func_in_func(allocate, builder.func);
                let frame_size = builder.ins().iconst(
                    types::I64,
                    i64::try_from(frame_layout.size).map_err(|_| {
                        CodegenError::Backend("future frame size exceeds target".into())
                    })?,
                );
                let frame_align = builder.ins().iconst(
                    types::I64,
                    i64::try_from(frame_layout.align).map_err(|_| {
                        CodegenError::Backend("future frame alignment exceeds target".into())
                    })?,
                );
                let allocation = builder.ins().call(allocate, &[frame_size, frame_align]);
                builder.inst_results(allocation)[0]
            } else {
                builder.ins().iconst(types::I64, 0)
            };
            let poll_ref = module.declare_func_in_func(poll, builder.func);
            let poll_address = builder.ins().func_addr(pointer, poll_ref);
            let drop_ref = module.declare_func_in_func(drop, builder.func);
            let drop_address = builder.ins().func_addr(pointer, drop_ref);
            let size = builder.ins().iconst(
                types::I64,
                i64::try_from(size).map_err(|_| {
                    CodegenError::Backend("vutcon result size exceeds target".into())
                })?,
            );
            let align = builder.ins().iconst(
                types::I64,
                i64::try_from(align).map_err(|_| {
                    CodegenError::Backend("vutcon result alignment exceeds target".into())
                })?,
            );
            let target = runtime_function(
                module,
                vut_runtime::abi::VUTCON_SPAWN,
                &[types::I64, types::I64, types::I64, types::I64, types::I64],
                &[types::I64],
            )?;
            let reference = module.declare_func_in_func(target, builder.func);
            let call = builder
                .ins()
                .call(reference, &[op, poll_address, drop_address, size, align]);
            Some((*value, builder.inst_results(call)[0]))
        }
        Instruction::AwaitFuture {
            value,
            handle,
            result_type,
        } => {
            let destination = stack_slot_for_type(builder, layouts, *result_type)?;
            let target = runtime_function(
                module,
                vut_runtime::abi::ASYNC_AWAIT,
                &[types::I64, types::I64],
                &[types::I32],
            )?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder
                .ins()
                .call(reference, &[values[handle], destination]);
            // Awaiting consumes the future handle.
            let drop_target =
                runtime_function(module, vut_runtime::abi::ASYNC_DROP, &[types::I64], &[])?;
            let drop_reference = module.declare_func_in_func(drop_target, builder.func);
            builder.ins().call(drop_reference, &[values[handle]]);
            value_types.insert(*value, *result_type);
            let layout = layouts.types[result_type.0];
            if layouts.is_aggregate(*result_type) {
                Some((*value, destination))
            } else if layout.repr == ValueRepr::Void {
                Some((*value, builder.ins().iconst(types::I64, 0)))
            } else {
                Some((
                    *value,
                    builder.ins().load(
                        machine_type(layouts, *result_type),
                        MemFlagsData::trusted(),
                        destination,
                        0,
                    ),
                ))
            }
        }
        Instruction::FrameState { value, frame } => {
            let _ = frame_local;
            let pointer = frame_pointer(builder, variables, Some(*frame))?;
            let word = builder.ins().load(
                types::I32,
                MemFlagsData::trusted(),
                pointer,
                frame_offset_i32(vut_mir::FRAME_STATE_OFFSET)?,
            );
            let widened = builder.ins().uextend(types::I64, word);
            Some((*value, widened))
        }
        Instruction::SetFrameState { frame, state } => {
            let _ = frame_local;
            let pointer = frame_pointer(builder, variables, Some(*frame))?;
            let word = builder.ins().iconst(types::I32, i64::from(*state));
            builder.ins().store(
                MemFlagsData::trusted(),
                word,
                pointer,
                frame_offset_i32(vut_mir::FRAME_STATE_OFFSET)?,
            );
            None
        }
        Instruction::SetFrameChild { frame, value } => {
            let _ = frame_local;
            let pointer = frame_pointer(builder, variables, Some(*frame))?;
            builder.ins().store(
                MemFlagsData::trusted(),
                values[value],
                pointer,
                frame_offset_i32(vut_mir::FRAME_CHILD_OFFSET)?,
            );
            None
        }
        Instruction::FieldStore { base, name, value } => {
            let base_ty = local_types[base.0].ok_or_else(|| {
                CodegenError::Backend("field assignment on an untyped local".into())
            })?;
            let field = layouts
                .fields
                .get(&base_ty)
                .and_then(|fields| fields.iter().find(|field| field.name == *name))
                .ok_or_else(|| {
                    CodegenError::Backend(format!("missing field layout for `{name}`"))
                })?;
            let base_value = local_read(
                builder,
                variables,
                storages,
                frame_local,
                layouts,
                local_types,
                *base,
            )?;
            let stored = values[value];
            if layouts.is_aggregate(field.ty) {
                let address = builder
                    .ins()
                    .iadd_imm_u(base_value, frame_offset_i64(field.offset)?);
                if layouts.types[field.ty.0].needs_drop {
                    manage_value(builder, module, layouts, field.ty, address, false)?;
                }
                copy_aggregate(builder, layouts, field.ty, stored, address)?;
            } else {
                if layouts.types[field.ty.0].needs_drop {
                    let old = builder.ins().load(
                        machine_type(layouts, field.ty),
                        MemFlagsData::trusted(),
                        base_value,
                        frame_offset_i32(field.offset)?,
                    );
                    manage_value(builder, module, layouts, field.ty, old, false)?;
                }
                let coerced = coerce_integer(builder, stored, machine_type(layouts, field.ty));
                builder.ins().store(
                    MemFlagsData::trusted(),
                    coerced,
                    base_value,
                    frame_offset_i32(field.offset)?,
                );
            }
            None
        }
        Instruction::ResourceDeref { value, handle } => {
            let target = runtime_function(
                module,
                vut_runtime::abi::RESOURCE_PTR,
                &[types::I64],
                &[types::I64],
            )?;
            let reference = module.declare_func_in_func(target, builder.func);
            let call = builder.ins().call(reference, &[values[handle]]);
            Some((*value, builder.inst_results(call)[0]))
        }
        Instruction::BorrowField { value, base, name } => {
            let base_ty = local_types[base.0]
                .ok_or_else(|| CodegenError::Backend("field borrow on an untyped local".into()))?;
            let field = layouts
                .fields
                .get(&base_ty)
                .and_then(|fields| fields.iter().find(|field| field.name == *name))
                .ok_or_else(|| {
                    CodegenError::Backend(format!("missing field layout for `{name}`"))
                })?;
            let base_value = local_read(
                builder,
                variables,
                storages,
                frame_local,
                layouts,
                local_types,
                *base,
            )?;
            let address = builder
                .ins()
                .iadd_imm_u(base_value, frame_offset_i64(field.offset)?);
            value_types.insert(*value, field.ty);
            Some((*value, address))
        }
        Instruction::SpillValue { slot, value } => {
            let pointer = frame_pointer(builder, variables, frame_local)?;
            let stored = values[value];
            let machine = builder.func.dfg.value_type(stored);
            builder.ins().store(
                MemFlagsData::trusted(),
                stored,
                pointer,
                frame_offset_i32(*slot)?,
            );
            spill_types.insert(*slot, (machine, value_types.get(value).copied()));
            None
        }
        Instruction::ReloadValue { value, slot } => {
            let pointer = frame_pointer(builder, variables, frame_local)?;
            let (machine, semantic) = spill_types.get(slot).copied().unwrap_or((types::I64, None));
            let loaded = builder.ins().load(
                machine,
                MemFlagsData::trusted(),
                pointer,
                frame_offset_i32(*slot)?,
            );
            if let Some(ty) = semantic {
                value_types.insert(*value, ty);
            }
            Some((*value, loaded))
        }
        Instruction::PollFuture {
            value,
            ready,
            frame,
            result_type,
        } => {
            let _ = frame_local;
            let pointer = frame_pointer(builder, variables, Some(*frame))?;
            let slot = builder
                .ins()
                .iadd_imm_u(pointer, frame_offset_i64(vut_mir::FRAME_CHILD_OFFSET)?);
            let destination = stack_slot_for_type(builder, layouts, *result_type)?;
            let target = runtime_function(
                module,
                vut_runtime::abi::ASYNC_AWAIT_CHILD,
                &[types::I64, types::I64],
                &[types::I32],
            )?;
            let reference = module.declare_func_in_func(target, builder.func);
            let call = builder.ins().call(reference, &[slot, destination]);
            let status = builder.inst_results(call)[0];
            let ready_value = builder.ins().icmp_imm_u(IntCC::NotEqual, status, 0);
            values.insert(*ready, ready_value);
            if let Some(id) = value {
                value_types.insert(*id, *result_type);
                let layout = layouts.types[result_type.0];
                let produced = if layouts.is_aggregate(*result_type) {
                    destination
                } else if layout.repr == ValueRepr::Void {
                    builder.ins().iconst(types::I64, 0)
                } else {
                    builder.ins().load(
                        machine_type(layouts, *result_type),
                        MemFlagsData::trusted(),
                        destination,
                        0,
                    )
                };
                values.insert(*id, produced);
            }
            None
        }
        Instruction::ResultPayload { value, source, ok } => {
            let ty = value_types.get(source).ok_or_else(|| {
                CodegenError::Backend("missing result value type for payload load".into())
            })?;
            let layout = layouts.results.get(ty).ok_or_else(|| {
                CodegenError::Backend(format!("missing result layout for type {}", ty.0))
            })?;
            let payload_ty = if *ok { layout.ok } else { layout.err };
            let offset = if *ok {
                layout.ok_offset
            } else {
                layout.err_offset
            };
            value_types.insert(*value, payload_ty);
            if layouts.is_aggregate(payload_ty) {
                // Inline aggregate payload: its value is the payload address.
                let delta = i64::try_from(offset).map_err(|_| {
                    CodegenError::Backend("result payload offset exceeds backend limit".into())
                })?;
                Some((*value, builder.ins().iadd_imm_u(values[source], delta)))
            } else {
                Some((
                    *value,
                    builder.ins().load(
                        machine_type(layouts, payload_ty),
                        MemFlagsData::trusted(),
                        values[source],
                        i32::try_from(offset).map_err(|_| {
                            CodegenError::Backend(
                                "result payload offset exceeds backend limit".into(),
                            )
                        })?,
                    ),
                ))
            }
        }
        Instruction::IteratorInit {
            iterator,
            iterable,
            length,
            length_value,
            stride,
            slot,
        } => {
            let state = if let Some(offset) = slot {
                // The state lives in the future frame so it survives a
                // suspension.
                let pointer = frame_pointer(builder, variables, frame_local)?;
                builder
                    .ins()
                    .iadd_imm_u(pointer, frame_offset_i64(*offset)?)
            } else {
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    32,
                    3,
                ));
                builder.ins().stack_addr(types::I64, slot, 0)
            };
            let collection = values[iterable];
            let is_list = value_types
                .get(iterable)
                .is_some_and(|ty| layouts.lists.contains_key(ty));
            let data = if is_list {
                let target = runtime_function(
                    module,
                    vut_runtime::abi::LIST_DATA,
                    &[types::I64],
                    &[types::I64],
                )?;
                let reference = module.declare_func_in_func(target, builder.func);
                let call = builder.ins().call(reference, &[collection]);
                builder.inst_results(call)[0]
            } else if length.is_some() || length_value.is_some() {
                collection
            } else {
                builder.ins().iadd_imm_u(collection, 8)
            };
            builder.ins().store(MemFlagsData::trusted(), data, state, 0);
            let zero = builder.ins().iconst(types::I64, 0);
            builder.ins().store(MemFlagsData::trusted(), zero, state, 8);
            let length = if is_list {
                let target = runtime_function(
                    module,
                    vut_runtime::abi::LIST_LEN,
                    &[types::I64],
                    &[types::I64],
                )?;
                let reference = module.declare_func_in_func(target, builder.func);
                let call = builder.ins().call(reference, &[collection]);
                builder.inst_results(call)[0]
            } else if let Some(length_value) = length_value {
                values[length_value]
            } else if let Some(length) = length {
                builder.ins().iconst(
                    types::I64,
                    i64::try_from(*length)
                        .map_err(|_| CodegenError::Backend("array length exceeds target".into()))?,
                )
            } else {
                builder
                    .ins()
                    .load(types::I64, MemFlagsData::trusted(), collection, 0)
            };
            builder
                .ins()
                .store(MemFlagsData::trusted(), length, state, 16);
            let stride = builder.ins().iconst(
                types::I64,
                i64::try_from(stride.unwrap_or(layouts.pointer_size))
                    .map_err(|_| CodegenError::Backend("element stride exceeds target".into()))?,
            );
            builder
                .ins()
                .store(MemFlagsData::trusted(), stride, state, 24);
            Some((*iterator, state))
        }
        Instruction::IteratorNext {
            has_value,
            value,
            index,
            iterator,
            element_type,
        } => {
            let state = values[iterator];
            let data = builder
                .ins()
                .load(types::I64, MemFlagsData::trusted(), state, 0);
            let current = builder
                .ins()
                .load(types::I64, MemFlagsData::trusted(), state, 8);
            let len = builder
                .ins()
                .load(types::I64, MemFlagsData::trusted(), state, 16);
            let has = builder.ins().icmp(IntCC::UnsignedLessThan, current, len);
            let stride = builder
                .ins()
                .load(types::I64, MemFlagsData::trusted(), state, 24);
            let offset = builder.ins().imul(current, stride);
            let element_address = builder.ins().iadd(data, offset);
            // The exhausted case must not read through the collection data
            // pointer: an empty list reports a null data pointer, so fall back
            // to a zeroed slot.
            let fallback = stack_slot_for_type(builder, layouts, *element_type)?;
            zero_stack_value(builder, layouts, *element_type, fallback);
            let safe_address = builder.ins().select(has, element_address, fallback);
            let element = element_value(builder, layouts, *element_type, safe_address);
            let next = builder.ins().iadd_imm_u(current, 1);
            builder.ins().store(MemFlagsData::trusted(), next, state, 8);
            values.insert(*has_value, has);
            values.insert(*value, element);
            Some((*index, current))
        }
        Instruction::CopyAggregate { value, source, ty } => {
            let slot = stack_slot_for_type(builder, layouts, *ty)?;
            copy_aggregate(builder, layouts, *ty, values[source], slot)?;
            manage_value(builder, module, layouts, *ty, slot, true)?;
            value_types.insert(*value, *ty);
            Some((*value, slot))
        }
        Instruction::Field { value, base, name } => {
            let ty = value_types.get(base).ok_or_else(|| {
                CodegenError::Backend(format!("missing aggregate type for field `{name}`"))
            })?;
            let field = layouts.field(*ty, name).ok_or_else(|| {
                CodegenError::Backend(format!("missing layout for field `{name}`"))
            })?;
            let field_ty = layouts
                .types
                .get(field.ty.0)
                .ok_or_else(|| CodegenError::Backend("missing field type layout".into()))?;
            let machine_ty = if field_ty.size == 1 {
                types::I8
            } else if field_ty.size == 2 {
                types::I16
            } else if field_ty.size == 4 {
                types::I32
            } else {
                types::I64
            };
            let offset = i32::try_from(field.offset)
                .map_err(|_| CodegenError::Backend("field offset exceeds backend limit".into()))?;
            value_types.insert(*value, field.ty);
            if layouts.is_aggregate(field.ty) {
                // The field is stored inline; its value is the field address.
                let delta = i64::try_from(field.offset).map_err(|_| {
                    CodegenError::Backend("field offset exceeds backend limit".into())
                })?;
                Some((*value, builder.ins().iadd_imm_u(values[base], delta)))
            } else {
                Some((
                    *value,
                    builder
                        .ins()
                        .load(machine_ty, MemFlagsData::trusted(), values[base], offset),
                ))
            }
        }
        Instruction::ConstructEnum {
            value,
            ty,
            variant_index,
            payload,
        } => {
            let layout = layouts.enums.get(ty).ok_or_else(|| {
                CodegenError::Backend(format!("missing enum layout for type {}", ty.0))
            })?;
            let address = stack_slot_for_type(builder, layouts, *ty)?;
            let tag_value = builder
                .ins()
                .iconst(types::I64, i64::try_from(*variant_index).unwrap_or(0));
            let tag_type = match layout.tag_size {
                1 => types::I8,
                2 => types::I16,
                4 => types::I32,
                _ => types::I64,
            };
            let tag = coerce_integer(builder, tag_value, tag_type);
            let tag_offset = i32::try_from(layout.tag_offset).map_err(|_| {
                CodegenError::Backend("enum tag offset exceeds backend limit".into())
            })?;
            builder
                .ins()
                .store(MemFlagsData::trusted(), tag, address, tag_offset);
            let variant = layout
                .variants
                .get(*variant_index)
                .ok_or_else(|| CodegenError::Backend("missing enum variant layout".into()))?;
            for (index, field) in variant.fields.iter().enumerate() {
                let Some(field_value) = payload.get(index) else {
                    continue;
                };
                if layouts.is_aggregate(field.ty) {
                    let delta = i64::try_from(field.offset).map_err(|_| {
                        CodegenError::Backend("enum field offset exceeds backend limit".into())
                    })?;
                    let destination = builder.ins().iadd_imm_u(address, delta);
                    copy_aggregate(builder, layouts, field.ty, values[field_value], destination)?;
                } else {
                    let offset = i32::try_from(field.offset).map_err(|_| {
                        CodegenError::Backend("enum field offset exceeds backend limit".into())
                    })?;
                    let stored = coerce_integer(
                        builder,
                        values[field_value],
                        machine_type(layouts, field.ty),
                    );
                    builder
                        .ins()
                        .store(MemFlagsData::trusted(), stored, address, offset);
                }
            }
            values.insert(*value, address);
            value_types.insert(*value, *ty);
            None
        }
        Instruction::EnumTag { result, value, ty } => {
            let layout = layouts.enums.get(ty).ok_or_else(|| {
                CodegenError::Backend(format!("missing enum layout for type {}", ty.0))
            })?;
            let tag_type = match layout.tag_size {
                1 => types::I8,
                2 => types::I16,
                4 => types::I32,
                _ => types::I64,
            };
            let offset = i32::try_from(layout.tag_offset).map_err(|_| {
                CodegenError::Backend("enum tag offset exceeds backend limit".into())
            })?;
            let tag = builder
                .ins()
                .load(tag_type, MemFlagsData::trusted(), values[value], offset);
            Some((*result, coerce_integer(builder, tag, types::I64)))
        }
        Instruction::EnumPayload {
            value,
            source,
            ty,
            variant_index,
            field_index,
        } => {
            let layout = layouts.enums.get(ty).ok_or_else(|| {
                CodegenError::Backend(format!("missing enum layout for type {}", ty.0))
            })?;
            let variant = layout
                .variants
                .get(*variant_index)
                .ok_or_else(|| CodegenError::Backend("missing enum variant layout".into()))?;
            let field = variant
                .fields
                .get(*field_index)
                .ok_or_else(|| CodegenError::Backend("missing enum field layout".into()))?;
            value_types.insert(*value, field.ty);
            if layouts.is_aggregate(field.ty) {
                let delta = i64::try_from(field.offset).map_err(|_| {
                    CodegenError::Backend("enum payload offset exceeds backend limit".into())
                })?;
                Some((*value, builder.ins().iadd_imm_u(values[source], delta)))
            } else {
                let offset = i32::try_from(field.offset).map_err(|_| {
                    CodegenError::Backend("enum payload offset exceeds backend limit".into())
                })?;
                let machine = machine_type(layouts, field.ty);
                let loaded =
                    builder
                        .ins()
                        .load(machine, MemFlagsData::trusted(), values[source], offset);
                Some((*value, loaded))
            }
        }
        Instruction::TypeRetain { value, ty } => {
            let address = type_function_address(module, builder, type_functions, ty.0, true);
            Some((*value, address))
        }
        Instruction::TypeRelease { value, ty } => {
            let address = type_function_address(module, builder, type_functions, ty.0, false);
            Some((*value, address))
        }
        Instruction::Call {
            value,
            result_type,
            target,
            arguments,
        } => {
            let (id, signature) = declarations.get(target).ok_or_else(|| {
                CodegenError::Backend(format!(
                    "invalid typed MIR: call target {} has no declaration",
                    target.0
                ))
            })?;
            let reference = module.declare_func_in_func(*id, builder.func);
            let aggregate_return = result_type.filter(|ty| layouts.is_aggregate(*ty));
            let destination = aggregate_return
                .map(|ty| stack_slot_for_type(builder, layouts, ty))
                .transpose()?;
            let mut args = Vec::with_capacity(arguments.len() + usize::from(destination.is_some()));
            if let Some(destination) = destination {
                args.push(destination);
            }
            args.extend(
                arguments
                    .iter()
                    .zip(
                        signature
                            .params
                            .iter()
                            .skip(usize::from(destination.is_some())),
                    )
                    .map(|(id, parameter)| {
                        coerce_integer(builder, values[id], parameter.value_type)
                    }),
            );
            let call = builder.ins().call(reference, &args);
            if let Some((id, ty)) = value.zip(*result_type) {
                value_types.insert(id, ty);
            }
            if let Some(destination) = destination {
                value.map(|id| (id, destination))
            } else {
                value.map(|id| (id, builder.inst_results(call)[0]))
            }
        }
        Instruction::StartFuture {
            value,
            target,
            arguments,
            ..
        } => {
            let layout = frames.get(target).ok_or_else(|| {
                CodegenError::Backend(format!("async function {} has no frame layout", target.0))
            })?;
            let (resume, drop) = future_thunks.get(target).copied().ok_or_else(|| {
                CodegenError::Backend(format!("async function {} has no resume thunk", target.0))
            })?;
            let alloc = runtime_function(
                module,
                vut_runtime::abi::FRAME_ALLOC,
                &[types::I64, types::I64],
                &[types::I64],
            )?;
            let alloc_ref = module.declare_func_in_func(alloc, builder.func);
            let size = builder
                .ins()
                .iconst(types::I64, i64::try_from(layout.size).unwrap_or(i64::MAX));
            let align = builder
                .ins()
                .iconst(types::I64, i64::try_from(layout.align).unwrap_or(i64::MAX));
            let allocation = builder.ins().call(alloc_ref, &[size, align]);
            let frame = builder.inst_results(allocation)[0];
            for (argument, slot) in arguments.iter().zip(layout.parameters.iter()) {
                let source = values[argument];
                if layouts.is_aggregate(slot.ty) {
                    let delta = i64::try_from(slot.offset).map_err(|_| {
                        CodegenError::Backend("future parameter offset exceeds limit".into())
                    })?;
                    let destination = builder.ins().iadd_imm_u(frame, delta);
                    copy_aggregate(builder, layouts, slot.ty, source, destination)?;
                } else {
                    let offset = i32::try_from(slot.offset).map_err(|_| {
                        CodegenError::Backend("future parameter offset exceeds limit".into())
                    })?;
                    builder
                        .ins()
                        .store(MemFlagsData::trusted(), source, frame, offset);
                }
            }
            let create = runtime_function(
                module,
                vut_runtime::abi::ASYNC_NEW,
                &[types::I64, types::I64, types::I64],
                &[types::I64],
            )?;
            let create_ref = module.declare_func_in_func(create, builder.func);
            let resume_ref = module.declare_func_in_func(resume, builder.func);
            let drop_ref = module.declare_func_in_func(drop, builder.func);
            let pointer = module.target_config().pointer_type();
            let resume_address = builder.ins().func_addr(pointer, resume_ref);
            let drop_address = builder.ins().func_addr(pointer, drop_ref);
            let created = builder
                .ins()
                .call(create_ref, &[frame, resume_address, drop_address]);
            value.map(|id| (id, builder.inst_results(created)[0]))
        }
        Instruction::MakeFunction { value, symbol } => {
            let (id, _) = declarations.get(symbol).ok_or_else(|| {
                CodegenError::Backend(format!(
                    "invalid typed MIR: function {} has no declaration",
                    symbol.0
                ))
            })?;
            let reference = module.declare_func_in_func(*id, builder.func);
            let pointer = module.target_config().pointer_type();
            Some((*value, builder.ins().func_addr(pointer, reference)))
        }
        Instruction::CallIndirect {
            value,
            result_type,
            callable_ty,
            callee,
            arguments,
        } => {
            let signature = indirect_signature(module, layouts, *callable_ty)?;
            let aggregate_return = callable_ty
                .and_then(|ty| layouts.callables.get(&ty))
                .map(|(_, result)| *result)
                .filter(|result| layouts.is_aggregate(*result));
            let destination = aggregate_return
                .map(|ty| stack_slot_for_type(builder, layouts, ty))
                .transpose()?;
            let mut args = Vec::with_capacity(arguments.len() + usize::from(destination.is_some()));
            if let Some(destination) = destination {
                args.push(destination);
            }
            args.extend(
                arguments
                    .iter()
                    .zip(
                        signature
                            .params
                            .iter()
                            .skip(usize::from(destination.is_some())),
                    )
                    .map(|(id, parameter)| {
                        coerce_integer(builder, values[id], parameter.value_type)
                    }),
            );
            let signature_ref = builder.func.import_signature(signature);
            let call = builder
                .ins()
                .call_indirect(signature_ref, values[callee], &args);
            if let Some((id, ty)) = value.zip(*result_type) {
                value_types.insert(id, ty);
            }
            if let Some(destination) = destination {
                value.map(|id| (id, destination))
            } else {
                value.map(|id| (id, builder.inst_results(call)[0]))
            }
        }
        Instruction::RuntimeCall {
            value,
            result_type,
            function,
            arguments,
        } => {
            if let Some((id, ty)) = value.zip(*result_type) {
                value_types.insert(id, ty);
            }
            if matches!(
                function,
                vut_mir::BuiltinFunction::ArrayLen
                    | vut_mir::BuiltinFunction::ArrayAt
                    | vut_mir::BuiltinFunction::ArraySet
                    | vut_mir::BuiltinFunction::ArrayFirst
                    | vut_mir::BuiltinFunction::ArrayLast
                    | vut_mir::BuiltinFunction::ArrayFill
                    | vut_mir::BuiltinFunction::ArrayContains
                    | vut_mir::BuiltinFunction::ArrayReverse
                    | vut_mir::BuiltinFunction::ArraySort
            ) {
                let receiver_id = arguments[0];
                let receiver = values[&receiver_id];
                let array_ty = value_types[&receiver_id];
                let (element_ty, length) = *layouts.arrays.get(&array_ty).ok_or_else(|| {
                    CodegenError::Backend(format!(
                        "array method receiver type {} is not an array",
                        array_ty.0
                    ))
                })?;
                let element_machine_ty = machine_type(layouts, element_ty);
                let stride = layouts.types[element_ty.0].size;
                let result = match function {
                    vut_mir::BuiltinFunction::ArrayLen => Some(builder.ins().iconst(
                        types::I64,
                        i64::try_from(length).map_err(|_| {
                            CodegenError::Backend("array length exceeds target".into())
                        })?,
                    )),
                    vut_mir::BuiltinFunction::ArrayAt | vut_mir::BuiltinFunction::ArraySet => {
                        let index = values[&arguments[1]];
                        let op = if *function == vut_mir::BuiltinFunction::ArrayAt {
                            vut_runtime::abi::bounds_op::ARRAY_AT
                        } else {
                            vut_runtime::abi::bounds_op::ARRAY_SET
                        };
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BOUNDS_CHECK,
                            &[types::I64, types::I64, types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let op_value = builder.ins().iconst(types::I64, op);
                        let length_value = builder.ins().iconst(
                            types::I64,
                            i64::try_from(length).map_err(|_| {
                                CodegenError::Backend("array length exceeds target".into())
                            })?,
                        );
                        let call = builder
                            .ins()
                            .call(reference, &[op_value, index, length_value]);
                        let index = builder.inst_results(call)[0];
                        let offset = builder.ins().imul_imm_u(
                            index,
                            i64::try_from(stride).map_err(|_| {
                                CodegenError::Backend("array stride exceeds target".into())
                            })?,
                        );
                        let address = builder.ins().iadd(receiver, offset);
                        if *function == vut_mir::BuiltinFunction::ArrayAt {
                            Some(read_owned_array_element(
                                builder, module, layouts, element_ty, address,
                            )?)
                        } else {
                            // `set` releases the previous element and keeps its
                            // own reference to the new one; the caller still owns
                            // and releases the argument temporary.
                            let previous = element_value(builder, layouts, element_ty, address);
                            manage_value(builder, module, layouts, element_ty, previous, false)?;
                            if layouts.is_aggregate(element_ty) {
                                copy_aggregate(
                                    builder,
                                    layouts,
                                    element_ty,
                                    values[&arguments[2]],
                                    address,
                                )?;
                                manage_value(builder, module, layouts, element_ty, address, true)?;
                            } else {
                                let stored = coerce_integer(
                                    builder,
                                    values[&arguments[2]],
                                    element_machine_ty,
                                );
                                builder
                                    .ins()
                                    .store(MemFlagsData::trusted(), stored, address, 0);
                                manage_value(builder, module, layouts, element_ty, stored, true)?;
                            }
                            None
                        }
                    }
                    vut_mir::BuiltinFunction::ArrayFirst | vut_mir::BuiltinFunction::ArrayLast => {
                        if length == 0 {
                            let op = if *function == vut_mir::BuiltinFunction::ArrayFirst {
                                vut_runtime::abi::bounds_op::ARRAY_FIRST
                            } else {
                                vut_runtime::abi::bounds_op::ARRAY_LAST
                            };
                            let target = runtime_function(
                                module,
                                vut_runtime::abi::BOUNDS_PANIC,
                                &[types::I64, types::I64, types::I64],
                                &[],
                            )?;
                            let reference = module.declare_func_in_func(target, builder.func);
                            let op_value = builder.ins().iconst(types::I64, op);
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().call(reference, &[op_value, zero, zero]);
                        }
                        let offset = if *function == vut_mir::BuiltinFunction::ArrayFirst {
                            0
                        } else {
                            i32::try_from(length.saturating_sub(1).saturating_mul(stride)).map_err(
                                |_| CodegenError::Backend("array offset exceeds target".into()),
                            )?
                        };
                        let address = builder.ins().iadd_imm_u(receiver, i64::from(offset));
                        Some(read_owned_array_element(
                            builder, module, layouts, element_ty, address,
                        )?)
                    }
                    vut_mir::BuiltinFunction::ArrayFill => {
                        if layouts.is_aggregate(element_ty) {
                            for index in 0..length {
                                let delta =
                                    i64::try_from(index.saturating_mul(stride)).map_err(|_| {
                                        CodegenError::Backend("array offset exceeds target".into())
                                    })?;
                                let destination = builder.ins().iadd_imm_u(receiver, delta);
                                manage_value(
                                    builder,
                                    module,
                                    layouts,
                                    element_ty,
                                    destination,
                                    false,
                                )?;
                                copy_aggregate(
                                    builder,
                                    layouts,
                                    element_ty,
                                    values[&arguments[1]],
                                    destination,
                                )?;
                                manage_value(
                                    builder,
                                    module,
                                    layouts,
                                    element_ty,
                                    destination,
                                    true,
                                )?;
                            }
                        } else {
                            let stored =
                                coerce_integer(builder, values[&arguments[1]], element_machine_ty);
                            for index in 0..length {
                                let offset =
                                    i32::try_from(index.saturating_mul(stride)).map_err(|_| {
                                        CodegenError::Backend("array offset exceeds target".into())
                                    })?;
                                if layouts.types[element_ty.0].needs_drop {
                                    let previous = builder.ins().load(
                                        machine_type(layouts, element_ty),
                                        MemFlagsData::trusted(),
                                        receiver,
                                        offset,
                                    );
                                    manage_value(
                                        builder, module, layouts, element_ty, previous, false,
                                    )?;
                                }
                                builder.ins().store(
                                    MemFlagsData::trusted(),
                                    stored,
                                    receiver,
                                    offset,
                                );
                                manage_value(builder, module, layouts, element_ty, stored, true)?;
                            }
                        }
                        None
                    }
                    vut_mir::BuiltinFunction::ArrayContains => {
                        let value_ptr =
                            element_pointer(builder, layouts, element_ty, values[&arguments[1]])?;
                        let len = builder.ins().iconst(
                            types::I64,
                            i64::try_from(length).map_err(|_| {
                                CodegenError::Backend("array length exceeds target".into())
                            })?,
                        );
                        let stride_value = builder.ins().iconst(
                            types::I64,
                            i64::try_from(stride).map_err(|_| {
                                CodegenError::Backend("array stride exceeds target".into())
                            })?,
                        );
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::ARRAY_CONTAINS,
                            &[types::I64, types::I64, types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder
                            .ins()
                            .call(reference, &[receiver, len, stride_value, value_ptr]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::ArrayReverse
                    | vut_mir::BuiltinFunction::ArraySort => {
                        let len = builder.ins().iconst(
                            types::I64,
                            i64::try_from(length).map_err(|_| {
                                CodegenError::Backend("array length exceeds target".into())
                            })?,
                        );
                        let stride_value = builder.ins().iconst(
                            types::I64,
                            i64::try_from(stride).map_err(|_| {
                                CodegenError::Backend("array stride exceeds target".into())
                            })?,
                        );
                        if *function == vut_mir::BuiltinFunction::ArrayReverse {
                            let target = runtime_function(
                                module,
                                vut_runtime::abi::ARRAY_REVERSE,
                                &[types::I64, types::I64, types::I64],
                                &[],
                            )?;
                            let reference = module.declare_func_in_func(target, builder.func);
                            builder
                                .ins()
                                .call(reference, &[receiver, len, stride_value]);
                        } else {
                            let target = runtime_function(
                                module,
                                vut_runtime::abi::ARRAY_SORT,
                                &[types::I64, types::I64, types::I64, types::I64],
                                &[],
                            )?;
                            let reference = module.declare_func_in_func(target, builder.func);
                            builder.ins().call(
                                reference,
                                &[receiver, len, stride_value, values[&arguments[1]]],
                            );
                        }
                        None
                    }
                    _ => unreachable!(),
                };
                if let Some((id, result)) = value.zip(result) {
                    values.insert(id, result);
                    if matches!(
                        function,
                        vut_mir::BuiltinFunction::ArrayAt
                            | vut_mir::BuiltinFunction::ArrayFirst
                            | vut_mir::BuiltinFunction::ArrayLast
                    ) {
                        value_types.insert(id, element_ty);
                    }
                }
                return Ok(());
            }
            if matches!(
                function,
                vut_mir::BuiltinFunction::ListLen | vut_mir::BuiltinFunction::ListIsEmpty
            ) {
                let name = if *function == vut_mir::BuiltinFunction::ListLen {
                    vut_runtime::abi::LIST_LEN
                } else {
                    vut_runtime::abi::LIST_IS_EMPTY
                };
                let returns = if *function == vut_mir::BuiltinFunction::ListLen {
                    &[types::I64][..]
                } else {
                    &[types::I8][..]
                };
                let target = runtime_function(module, name, &[types::I64], returns)?;
                let reference = module.declare_func_in_func(target, builder.func);
                let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                let result = builder.inst_results(call)[0];
                if let Some(id) = value {
                    values.insert(*id, result);
                }
                return Ok(());
            }
            if matches!(
                function,
                vut_mir::BuiltinFunction::BytesNew
                    | vut_mir::BuiltinFunction::BytesClone
                    | vut_mir::BuiltinFunction::StringToBytes
                    | vut_mir::BuiltinFunction::BytesLen
                    | vut_mir::BuiltinFunction::BytesIsEmpty
                    | vut_mir::BuiltinFunction::BytesCapacity
                    | vut_mir::BuiltinFunction::BytesReserve
                    | vut_mir::BuiltinFunction::BytesAt
                    | vut_mir::BuiltinFunction::BytesSet
                    | vut_mir::BuiltinFunction::BytesFirst
                    | vut_mir::BuiltinFunction::BytesLast
                    | vut_mir::BuiltinFunction::BytesSlice
                    | vut_mir::BuiltinFunction::BytesClear
                    | vut_mir::BuiltinFunction::BytesToList
                    | vut_mir::BuiltinFunction::BytesFromList
                    | vut_mir::BuiltinFunction::BytesToStr
                    | vut_mir::BuiltinFunction::BytesFromHex
                    | vut_mir::BuiltinFunction::BytesReadInt { .. }
                    | vut_mir::BuiltinFunction::BytesWriteInt { .. }
            ) {
                let call_result = match function {
                    vut_mir::BuiltinFunction::BytesNew => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_NEW,
                            &[],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::BytesClone => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_CLONE,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::StringToBytes
                    | vut_mir::BuiltinFunction::BytesSlice
                    | vut_mir::BuiltinFunction::BytesToList
                    | vut_mir::BuiltinFunction::BytesFromList => {
                        let (name, params) = match function {
                            vut_mir::BuiltinFunction::StringToBytes => {
                                (vut_runtime::abi::STRING_TO_BYTES, vec![types::I64])
                            }
                            vut_mir::BuiltinFunction::BytesSlice => (
                                vut_runtime::abi::BYTES_SLICE,
                                vec![types::I64, types::I64, types::I64],
                            ),
                            vut_mir::BuiltinFunction::BytesToList => {
                                (vut_runtime::abi::BYTES_TO_LIST, vec![types::I64])
                            }
                            _ => (vut_runtime::abi::BYTES_FROM_LIST, vec![types::I64]),
                        };
                        let target = runtime_function(module, name, &params, &[types::I64])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let args: Vec<_> = arguments.iter().map(|id| values[id]).collect();
                        let call = builder.ins().call(reference, &args);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::BytesLen
                    | vut_mir::BuiltinFunction::BytesIsEmpty
                    | vut_mir::BuiltinFunction::BytesCapacity => {
                        let (name, returns) = match function {
                            vut_mir::BuiltinFunction::BytesLen => {
                                (vut_runtime::abi::BYTES_LEN, &[types::I64][..])
                            }
                            vut_mir::BuiltinFunction::BytesIsEmpty => {
                                (vut_runtime::abi::BYTES_IS_EMPTY, &[types::I8][..])
                            }
                            _ => (vut_runtime::abi::BYTES_CAPACITY, &[types::I64][..]),
                        };
                        let target = runtime_function(module, name, &[types::I64], returns)?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::BytesReserve
                    | vut_mir::BuiltinFunction::BytesClear => {
                        let (name, args) = if *function == vut_mir::BuiltinFunction::BytesReserve {
                            (
                                vut_runtime::abi::BYTES_RESERVE,
                                vec![values[&arguments[0]], values[&arguments[1]]],
                            )
                        } else {
                            (vut_runtime::abi::BYTES_CLEAR, vec![values[&arguments[0]]])
                        };
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, &[])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &args);
                        None
                    }
                    vut_mir::BuiltinFunction::BytesAt
                    | vut_mir::BuiltinFunction::BytesFirst
                    | vut_mir::BuiltinFunction::BytesLast => {
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            1,
                            0,
                        ));
                        let out = builder.ins().stack_addr(types::I64, slot, 0);
                        let (name, args) = match function {
                            vut_mir::BuiltinFunction::BytesAt => (
                                vut_runtime::abi::BYTES_AT,
                                vec![values[&arguments[0]], values[&arguments[1]], out],
                            ),
                            vut_mir::BuiltinFunction::BytesFirst => (
                                vut_runtime::abi::BYTES_FIRST,
                                vec![values[&arguments[0]], out],
                            ),
                            _ => (
                                vut_runtime::abi::BYTES_LAST,
                                vec![values[&arguments[0]], out],
                            ),
                        };
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, &[types::I8])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &args);
                        Some(
                            builder
                                .ins()
                                .load(types::I8, MemFlagsData::trusted(), out, 0),
                        )
                    }
                    vut_mir::BuiltinFunction::BytesSet => {
                        let byte = coerce_integer(builder, values[&arguments[2]], types::I8);
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_SET,
                            &[types::I64, types::I64, types::I8],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(
                            reference,
                            &[values[&arguments[0]], values[&arguments[1]], byte],
                        );
                        None
                    }
                    vut_mir::BuiltinFunction::BytesToStr => {
                        let result_ty = result_type.ok_or_else(|| {
                            CodegenError::Backend("bytes.to_str missing result type".into())
                        })?;
                        let layout = layouts.results.get(&result_ty).ok_or_else(|| {
                            CodegenError::Backend("bytes.to_str missing result layout".into())
                        })?;
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_TO_STR,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                        let text = builder.inst_results(call)[0];
                        let address = stack_slot_for_type(builder, layouts, result_ty)?;
                        let ok_block = builder.create_block();
                        let err_block = builder.create_block();
                        let join = builder.create_block();
                        let is_ok = builder.ins().icmp_imm_s(IntCC::NotEqual, text, 0);
                        builder.ins().brif(is_ok, ok_block, &[], err_block, &[]);
                        builder.switch_to_block(ok_block);
                        let ok_tag = builder.ins().iconst(types::I64, 0);
                        builder.ins().store(
                            MemFlagsData::trusted(),
                            ok_tag,
                            address,
                            i32::try_from(layout.tag_offset).map_err(|_| {
                                CodegenError::Backend(
                                    "result tag offset exceeds backend limit".into(),
                                )
                            })?,
                        );
                        builder.ins().store(
                            MemFlagsData::trusted(),
                            text,
                            address,
                            i32::try_from(layout.ok_offset).map_err(|_| {
                                CodegenError::Backend(
                                    "result ok offset exceeds backend limit".into(),
                                )
                            })?,
                        );
                        builder.ins().jump(join, &[]);
                        builder.switch_to_block(err_block);
                        let bytes_value = values[&arguments[0]];
                        let valid_target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_UTF8_VALID_UP_TO,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let valid_ref = module.declare_func_in_func(valid_target, builder.func);
                        let valid_call = builder.ins().call(valid_ref, &[bytes_value]);
                        let valid_up_to = builder.inst_results(valid_call)[0];
                        let error_len_target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_UTF8_ERROR_LEN,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let error_len_ref =
                            module.declare_func_in_func(error_len_target, builder.func);
                        let error_len_call = builder.ins().call(error_len_ref, &[bytes_value]);
                        let error_len = builder.inst_results(error_len_call)[0];
                        let err_tag = builder.ins().iconst(types::I64, 1);
                        builder.ins().store(
                            MemFlagsData::trusted(),
                            err_tag,
                            address,
                            i32::try_from(layout.tag_offset).map_err(|_| {
                                CodegenError::Backend(
                                    "result tag offset exceeds backend limit".into(),
                                )
                            })?,
                        );
                        let err_fields = layouts.fields.get(&layout.err).ok_or_else(|| {
                            CodegenError::Backend("bytes.to_str missing Utf8Error layout".into())
                        })?;
                        // The error payload is stored inline in the result block,
                        // matching how aggregate result payloads are read.
                        let err_delta = i64::try_from(layout.err_offset).map_err(|_| {
                            CodegenError::Backend("result err offset exceeds backend limit".into())
                        })?;
                        let err_base = builder.ins().iadd_imm_u(address, err_delta);
                        for field in err_fields {
                            let field_value = match field.name.as_str() {
                                "valid_up_to" => valid_up_to,
                                "error_len" => error_len,
                                _ => continue,
                            };
                            builder.ins().store(
                                MemFlagsData::trusted(),
                                field_value,
                                err_base,
                                i32::try_from(field.offset).map_err(|_| {
                                    CodegenError::Backend(
                                        "Utf8Error field offset exceeds backend limit".into(),
                                    )
                                })?,
                            );
                        }
                        builder.ins().jump(join, &[]);
                        builder.switch_to_block(join);
                        if let Some(id) = value {
                            value_types.insert(*id, result_ty);
                        }
                        Some(address)
                    }
                    vut_mir::BuiltinFunction::BytesFromHex => {
                        let result_ty = result_type.ok_or_else(|| {
                            CodegenError::Backend("bytes.from_hex missing result type".into())
                        })?;
                        let layout = layouts.results.get(&result_ty).ok_or_else(|| {
                            CodegenError::Backend("bytes.from_hex missing result layout".into())
                        })?;
                        let tag_offset = i32::try_from(layout.tag_offset).map_err(|_| {
                            CodegenError::Backend("result tag offset exceeds backend limit".into())
                        })?;
                        let ok_offset = i32::try_from(layout.ok_offset).map_err(|_| {
                            CodegenError::Backend("result ok offset exceeds backend limit".into())
                        })?;
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_FROM_HEX,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                        let bytes = builder.inst_results(call)[0];
                        let address = stack_slot_for_type(builder, layouts, result_ty)?;
                        let ok_block = builder.create_block();
                        let err_block = builder.create_block();
                        let join = builder.create_block();
                        let is_ok = builder.ins().icmp_imm_s(IntCC::NotEqual, bytes, 0);
                        builder.ins().brif(is_ok, ok_block, &[], err_block, &[]);
                        builder.switch_to_block(ok_block);
                        let ok_tag = builder.ins().iconst(types::I64, 0);
                        builder
                            .ins()
                            .store(MemFlagsData::trusted(), ok_tag, address, tag_offset);
                        builder
                            .ins()
                            .store(MemFlagsData::trusted(), bytes, address, ok_offset);
                        builder.ins().jump(join, &[]);
                        builder.switch_to_block(err_block);
                        let index_target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_FROM_HEX_ERROR_INDEX,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let index_ref = module.declare_func_in_func(index_target, builder.func);
                        let index_call = builder.ins().call(index_ref, &[values[&arguments[0]]]);
                        let index = builder.inst_results(index_call)[0];
                        let err_tag = builder.ins().iconst(types::I64, 1);
                        builder
                            .ins()
                            .store(MemFlagsData::trusted(), err_tag, address, tag_offset);
                        let err_fields = layouts.fields.get(&layout.err).ok_or_else(|| {
                            CodegenError::Backend("bytes.from_hex missing HexError layout".into())
                        })?;
                        let err_delta = i64::try_from(layout.err_offset).map_err(|_| {
                            CodegenError::Backend("result err offset exceeds backend limit".into())
                        })?;
                        let err_base = builder.ins().iadd_imm_u(address, err_delta);
                        for field in err_fields {
                            if field.name != "index" {
                                continue;
                            }
                            builder.ins().store(
                                MemFlagsData::trusted(),
                                index,
                                err_base,
                                i32::try_from(field.offset).map_err(|_| {
                                    CodegenError::Backend(
                                        "HexError field offset exceeds backend limit".into(),
                                    )
                                })?,
                            );
                        }
                        builder.ins().jump(join, &[]);
                        builder.switch_to_block(join);
                        if let Some(id) = value {
                            value_types.insert(*id, result_ty);
                        }
                        Some(address)
                    }
                    vut_mir::BuiltinFunction::BytesReadInt {
                        width,
                        big_endian,
                        signed,
                    } => {
                        let width_value = builder.ins().iconst(types::I64, i64::from(*width));
                        let endian_value = builder.ins().iconst(types::I64, i64::from(*big_endian));
                        let signed_value = builder.ins().iconst(types::I64, i64::from(*signed));
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_READ_INT,
                            &[types::I64, types::I64, types::I64, types::I64, types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(
                            reference,
                            &[
                                values[&arguments[0]],
                                values[&arguments[1]],
                                width_value,
                                endian_value,
                                signed_value,
                            ],
                        );
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::BytesWriteInt { width, big_endian } => {
                        let width_value = builder.ins().iconst(types::I64, i64::from(*width));
                        let endian_value = builder.ins().iconst(types::I64, i64::from(*big_endian));
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::BYTES_WRITE_INT,
                            &[types::I64, types::I64, types::I64, types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(
                            reference,
                            &[
                                values[&arguments[0]],
                                values[&arguments[1]],
                                width_value,
                                endian_value,
                                values[&arguments[2]],
                            ],
                        );
                        None
                    }
                    _ => unreachable!(),
                };
                if let Some((id, result)) = value.zip(call_result) {
                    values.insert(id, result);
                }
                return Ok(());
            }
            if matches!(
                function,
                vut_mir::BuiltinFunction::ListNew
                    | vut_mir::BuiltinFunction::ListCapacity
                    | vut_mir::BuiltinFunction::ListReserve
                    | vut_mir::BuiltinFunction::ListPush
                    | vut_mir::BuiltinFunction::ListAt
                    | vut_mir::BuiltinFunction::ListSet
                    | vut_mir::BuiltinFunction::ListInsert
                    | vut_mir::BuiltinFunction::ListRemove
                    | vut_mir::BuiltinFunction::ListClear
                    | vut_mir::BuiltinFunction::ListSlice
                    | vut_mir::BuiltinFunction::ListContains
                    | vut_mir::BuiltinFunction::ListPop
                    | vut_mir::BuiltinFunction::ListFirst
                    | vut_mir::BuiltinFunction::ListLast
                    | vut_mir::BuiltinFunction::ListFindIndex
                    | vut_mir::BuiltinFunction::ListExtend
                    | vut_mir::BuiltinFunction::ListReverse
                    | vut_mir::BuiltinFunction::ListSort
                    | vut_mir::BuiltinFunction::ListTruncate
                    | vut_mir::BuiltinFunction::ListSwap
                    | vut_mir::BuiltinFunction::ListShrinkToFit
            ) {
                let call_result = match function {
                    vut_mir::BuiltinFunction::ListNew => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::LIST_NEW,
                            &[types::I64, types::I64, types::I64, types::I64, types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let args: Vec<_> = arguments.iter().map(|id| values[id]).collect();
                        let call = builder.ins().call(reference, &args);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::ListCapacity => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::LIST_CAPACITY,
                            &[types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::ListReserve | vut_mir::BuiltinFunction::ListClear => {
                        let (name, args) = if *function == vut_mir::BuiltinFunction::ListReserve {
                            (
                                vut_runtime::abi::LIST_RESERVE,
                                vec![values[&arguments[0]], values[&arguments[1]]],
                            )
                        } else {
                            (vut_runtime::abi::LIST_CLEAR, vec![values[&arguments[0]]])
                        };
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, &[])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &args);
                        None
                    }
                    vut_mir::BuiltinFunction::ListPush
                    | vut_mir::BuiltinFunction::ListSet
                    | vut_mir::BuiltinFunction::ListInsert
                    | vut_mir::BuiltinFunction::ListContains => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing list receiver type".into())
                        })?;
                        let element = *layouts.lists.get(&receiver_ty).ok_or_else(|| {
                            CodegenError::Backend("missing list element type".into())
                        })?;
                        let element_ptr = element_pointer(
                            builder,
                            layouts,
                            element,
                            values[&arguments[arguments.len() - 1]],
                        )?;
                        let (name, args, returns) = match function {
                            vut_mir::BuiltinFunction::ListPush => (
                                vut_runtime::abi::LIST_PUSH,
                                vec![values[&arguments[0]], element_ptr],
                                &[][..],
                            ),
                            vut_mir::BuiltinFunction::ListSet => (
                                vut_runtime::abi::LIST_SET,
                                vec![values[&arguments[0]], values[&arguments[1]], element_ptr],
                                &[types::I8][..],
                            ),
                            vut_mir::BuiltinFunction::ListInsert => (
                                vut_runtime::abi::LIST_INSERT,
                                vec![values[&arguments[0]], values[&arguments[1]], element_ptr],
                                &[types::I8][..],
                            ),
                            vut_mir::BuiltinFunction::ListContains => (
                                vut_runtime::abi::LIST_CONTAINS,
                                vec![values[&arguments[0]], element_ptr],
                                &[types::I8][..],
                            ),
                            _ => unreachable!(),
                        };
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, returns)?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &args);
                        if returns.is_empty() {
                            None
                        } else {
                            Some(builder.inst_results(call)[0])
                        }
                    }
                    vut_mir::BuiltinFunction::ListAt | vut_mir::BuiltinFunction::ListRemove => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing list receiver type".into())
                        })?;
                        let element_ty = *layouts.lists.get(&receiver_ty).ok_or_else(|| {
                            CodegenError::Backend("missing list element type".into())
                        })?;
                        let out = stack_slot_for_type(builder, layouts, element_ty)?;
                        zero_stack_value(builder, layouts, element_ty, out);
                        let name = if *function == vut_mir::BuiltinFunction::ListAt {
                            vut_runtime::abi::LIST_AT
                        } else {
                            vut_runtime::abi::LIST_REMOVE
                        };
                        let target = runtime_function(
                            module,
                            name,
                            &[types::I64, types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(
                            reference,
                            &[values[&arguments[0]], values[&arguments[1]], out],
                        );
                        let loaded = element_value(builder, layouts, element_ty, out);
                        if let Some(id) = value {
                            value_types.insert(*id, element_ty);
                        }
                        Some(loaded)
                    }
                    vut_mir::BuiltinFunction::ListSlice => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::LIST_SLICE,
                            &[types::I64, types::I64, types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let args: Vec<_> = arguments.iter().map(|id| values[id]).collect();
                        let call = builder.ins().call(reference, &args);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::ListPop
                    | vut_mir::BuiltinFunction::ListFirst
                    | vut_mir::BuiltinFunction::ListLast => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing list receiver type".into())
                        })?;
                        let element_ty = *layouts.lists.get(&receiver_ty).ok_or_else(|| {
                            CodegenError::Backend("missing list element type".into())
                        })?;
                        let out = stack_slot_for_type(builder, layouts, element_ty)?;
                        zero_stack_value(builder, layouts, element_ty, out);
                        let name = match function {
                            vut_mir::BuiltinFunction::ListPop => vut_runtime::abi::LIST_POP,
                            vut_mir::BuiltinFunction::ListFirst => vut_runtime::abi::LIST_FIRST,
                            _ => vut_runtime::abi::LIST_LAST,
                        };
                        let target = runtime_function(
                            module,
                            name,
                            &[types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &[values[&arguments[0]], out]);
                        let loaded = element_value(builder, layouts, element_ty, out);
                        if let Some(id) = value {
                            value_types.insert(*id, element_ty);
                        }
                        Some(loaded)
                    }
                    vut_mir::BuiltinFunction::ListFindIndex => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing list receiver type".into())
                        })?;
                        let element = *layouts.lists.get(&receiver_ty).ok_or_else(|| {
                            CodegenError::Backend("missing list element type".into())
                        })?;
                        let element_ptr = element_pointer(
                            builder,
                            layouts,
                            element,
                            values[&arguments[arguments.len() - 1]],
                        )?;
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::LIST_FIND_INDEX,
                            &[types::I64, types::I64],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder
                            .ins()
                            .call(reference, &[values[&arguments[0]], element_ptr]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::ListExtend => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::LIST_EXTEND,
                            &[types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder
                            .ins()
                            .call(reference, &[values[&arguments[0]], values[&arguments[1]]]);
                        None
                    }
                    vut_mir::BuiltinFunction::ListReverse
                    | vut_mir::BuiltinFunction::ListShrinkToFit => {
                        let name = if *function == vut_mir::BuiltinFunction::ListReverse {
                            vut_runtime::abi::LIST_REVERSE
                        } else {
                            vut_runtime::abi::LIST_SHRINK_TO_FIT
                        };
                        let target = runtime_function(module, name, &[types::I64], &[])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &[values[&arguments[0]]]);
                        None
                    }
                    vut_mir::BuiltinFunction::ListSort => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::LIST_SORT,
                            &[types::I64, types::I64],
                            &[],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder
                            .ins()
                            .call(reference, &[values[&arguments[0]], values[&arguments[1]]]);
                        None
                    }
                    vut_mir::BuiltinFunction::ListTruncate | vut_mir::BuiltinFunction::ListSwap => {
                        let name = if *function == vut_mir::BuiltinFunction::ListTruncate {
                            vut_runtime::abi::LIST_TRUNCATE
                        } else {
                            vut_runtime::abi::LIST_SWAP
                        };
                        let args: Vec<_> = arguments.iter().map(|id| values[id]).collect();
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, &[])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &args);
                        None
                    }
                    _ => unreachable!(),
                };
                if let Some((id, result)) = value.zip(call_result) {
                    values.insert(id, result);
                }
                return Ok(());
            }
            if matches!(
                function,
                vut_mir::BuiltinFunction::MapNew
                    | vut_mir::BuiltinFunction::MapLen
                    | vut_mir::BuiltinFunction::MapIsEmpty
                    | vut_mir::BuiltinFunction::MapCapacity
                    | vut_mir::BuiltinFunction::MapReserve
                    | vut_mir::BuiltinFunction::MapGet
                    | vut_mir::BuiltinFunction::MapSet
                    | vut_mir::BuiltinFunction::MapContainsKey
                    | vut_mir::BuiltinFunction::MapRemove
                    | vut_mir::BuiltinFunction::MapClear
                    | vut_mir::BuiltinFunction::MapGetOr
            ) {
                let call_result = match function {
                    vut_mir::BuiltinFunction::MapNew => {
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::MAP_NEW,
                            &[
                                types::I64,
                                types::I64,
                                types::I64,
                                types::I64,
                                types::I64,
                                types::I64,
                                types::I64,
                                types::I64,
                            ],
                            &[types::I64],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let args: Vec<_> = arguments.iter().map(|id| values[id]).collect();
                        let call = builder.ins().call(reference, &args);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::MapLen
                    | vut_mir::BuiltinFunction::MapIsEmpty
                    | vut_mir::BuiltinFunction::MapCapacity => {
                        let (name, returns) = match function {
                            vut_mir::BuiltinFunction::MapLen => {
                                (vut_runtime::abi::MAP_LEN, &[types::I64][..])
                            }
                            vut_mir::BuiltinFunction::MapIsEmpty => {
                                (vut_runtime::abi::MAP_IS_EMPTY, &[types::I8][..])
                            }
                            _ => (vut_runtime::abi::MAP_CAPACITY, &[types::I64][..]),
                        };
                        let target = runtime_function(module, name, &[types::I64], returns)?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &[values[&arguments[0]]]);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::MapReserve | vut_mir::BuiltinFunction::MapClear => {
                        let (name, args) = if *function == vut_mir::BuiltinFunction::MapReserve {
                            (
                                vut_runtime::abi::MAP_RESERVE,
                                vec![values[&arguments[0]], values[&arguments[1]]],
                            )
                        } else {
                            (vut_runtime::abi::MAP_CLEAR, vec![values[&arguments[0]]])
                        };
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, &[])?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(reference, &args);
                        None
                    }
                    vut_mir::BuiltinFunction::MapSet | vut_mir::BuiltinFunction::MapContainsKey => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing map receiver type".into())
                        })?;
                        let (key_ty, value_ty) =
                            *layouts.maps.get(&receiver_ty).ok_or_else(|| {
                                CodegenError::Backend("missing map key/value type".into())
                            })?;
                        let key_ptr =
                            element_pointer(builder, layouts, key_ty, values[&arguments[1]])?;
                        let (name, args, returns) = if *function == vut_mir::BuiltinFunction::MapSet
                        {
                            let value_ptr =
                                element_pointer(builder, layouts, value_ty, values[&arguments[2]])?;
                            (
                                vut_runtime::abi::MAP_SET,
                                vec![values[&arguments[0]], key_ptr, value_ptr],
                                &[types::I8][..],
                            )
                        } else {
                            (
                                vut_runtime::abi::MAP_CONTAINS_KEY,
                                vec![values[&arguments[0]], key_ptr],
                                &[types::I8][..],
                            )
                        };
                        let params = vec![types::I64; args.len()];
                        let target = runtime_function(module, name, &params, returns)?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder.ins().call(reference, &args);
                        Some(builder.inst_results(call)[0])
                    }
                    vut_mir::BuiltinFunction::MapGet | vut_mir::BuiltinFunction::MapRemove => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing map receiver type".into())
                        })?;
                        let (key_ty, value_ty) =
                            *layouts.maps.get(&receiver_ty).ok_or_else(|| {
                                CodegenError::Backend("missing map key/value type".into())
                            })?;
                        let key_ptr =
                            element_pointer(builder, layouts, key_ty, values[&arguments[1]])?;
                        let out = stack_slot_for_type(builder, layouts, value_ty)?;
                        zero_stack_value(builder, layouts, value_ty, out);
                        let name = if *function == vut_mir::BuiltinFunction::MapGet {
                            vut_runtime::abi::MAP_GET
                        } else {
                            vut_runtime::abi::MAP_REMOVE
                        };
                        let target = runtime_function(
                            module,
                            name,
                            &[types::I64, types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        let call = builder
                            .ins()
                            .call(reference, &[values[&arguments[0]], key_ptr, out]);
                        let present = builder.inst_results(call)[0];
                        let optional_ty = (*result_type).ok_or_else(|| {
                            CodegenError::Backend(
                                "map lookup without an optional result type".into(),
                            )
                        })?;
                        let result = optional_from_presence(
                            builder,
                            layouts,
                            optional_ty,
                            value_ty,
                            present,
                            out,
                        )?;
                        Some(result)
                    }
                    vut_mir::BuiltinFunction::MapGetOr => {
                        let receiver_ty = *value_types.get(&arguments[0]).ok_or_else(|| {
                            CodegenError::Backend("missing map receiver type".into())
                        })?;
                        let (key_ty, value_ty) =
                            *layouts.maps.get(&receiver_ty).ok_or_else(|| {
                                CodegenError::Backend("missing map key/value type".into())
                            })?;
                        let key_ptr =
                            element_pointer(builder, layouts, key_ty, values[&arguments[1]])?;
                        let out = stack_slot_for_type(builder, layouts, value_ty)?;
                        let default_ptr =
                            element_pointer(builder, layouts, value_ty, values[&arguments[2]])?;
                        let target = runtime_function(
                            module,
                            vut_runtime::abi::MAP_GET_OR,
                            &[types::I64, types::I64, types::I64, types::I64],
                            &[types::I8],
                        )?;
                        let reference = module.declare_func_in_func(target, builder.func);
                        builder.ins().call(
                            reference,
                            &[values[&arguments[0]], key_ptr, out, default_ptr],
                        );
                        let loaded = element_value(builder, layouts, value_ty, out);
                        if let Some(id) = value {
                            value_types.insert(*id, value_ty);
                        }
                        Some(loaded)
                    }
                    _ => unreachable!(),
                };
                if let Some((id, result)) = value.zip(call_result) {
                    values.insert(id, result);
                }
                return Ok(());
            }
            if matches!(
                function,
                vut_mir::BuiltinFunction::FloatAbs
                    | vut_mir::BuiltinFunction::FloatFloor
                    | vut_mir::BuiltinFunction::FloatCeil
                    | vut_mir::BuiltinFunction::FloatRound
                    | vut_mir::BuiltinFunction::FloatTrunc
                    | vut_mir::BuiltinFunction::FloatSqrt
                    | vut_mir::BuiltinFunction::FloatPow
                    | vut_mir::BuiltinFunction::FloatMin
                    | vut_mir::BuiltinFunction::FloatMax
                    | vut_mir::BuiltinFunction::FloatClamp
                    | vut_mir::BuiltinFunction::FloatToInt
                    | vut_mir::BuiltinFunction::FloatIsNan
                    | vut_mir::BuiltinFunction::FloatIsFinite
            ) {
                let (name, returns) = match function {
                    vut_mir::BuiltinFunction::FloatAbs => {
                        (vut_runtime::abi::F64_ABS, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatFloor => {
                        (vut_runtime::abi::F64_FLOOR, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatCeil => {
                        (vut_runtime::abi::F64_CEIL, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatRound => {
                        (vut_runtime::abi::F64_ROUND, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatTrunc => {
                        (vut_runtime::abi::F64_TRUNC, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatSqrt => {
                        (vut_runtime::abi::F64_SQRT, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatPow => {
                        (vut_runtime::abi::F64_POW, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatMin => {
                        (vut_runtime::abi::F64_MIN, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatMax => {
                        (vut_runtime::abi::F64_MAX, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatClamp => {
                        (vut_runtime::abi::F64_CLAMP, &[types::F64][..])
                    }
                    vut_mir::BuiltinFunction::FloatToInt => {
                        (vut_runtime::abi::F64_TO_INT, &[types::I64][..])
                    }
                    vut_mir::BuiltinFunction::FloatIsNan => {
                        (vut_runtime::abi::F64_IS_NAN, &[types::I8][..])
                    }
                    _ => (vut_runtime::abi::F64_IS_FINITE, &[types::I8][..]),
                };
                let parameters = vec![types::F64; arguments.len()];
                let target = runtime_function(module, name, &parameters, returns)?;
                let reference = module.declare_func_in_func(target, builder.func);
                let args: Vec<_> = arguments.iter().map(|id| values[id]).collect();
                let call = builder.ins().call(reference, &args);
                if let Some((id, result)) = value.zip(Some(builder.inst_results(call)[0])) {
                    values.insert(id, result);
                }
                return Ok(());
            }
            let (name, returns) = match function {
                vut_mir::BuiltinFunction::Print => (vut_runtime::abi::PRINT, &[][..]),
                vut_mir::BuiltinFunction::Out => (vut_runtime::abi::OUT, &[][..]),
                vut_mir::BuiltinFunction::Input => (vut_runtime::abi::INPUT, &[types::I64][..]),
                vut_mir::BuiltinFunction::StringByteLen => {
                    (vut_runtime::abi::STRING_BYTE_LEN, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringCharLen => {
                    (vut_runtime::abi::STRING_CHAR_LEN, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringIsEmpty => {
                    (vut_runtime::abi::STRING_IS_EMPTY, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::StringContains => {
                    (vut_runtime::abi::STRING_CONTAINS, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::StringStartsWith => {
                    (vut_runtime::abi::STRING_STARTS_WITH, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::StringEndsWith => {
                    (vut_runtime::abi::STRING_ENDS_WITH, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::StringTrim => {
                    (vut_runtime::abi::STRING_TRIM, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringTrimStart => {
                    (vut_runtime::abi::STRING_TRIM_START, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringTrimEnd => {
                    (vut_runtime::abi::STRING_TRIM_END, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringToLower => {
                    (vut_runtime::abi::STRING_TO_LOWER, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringToUpper => {
                    (vut_runtime::abi::STRING_TO_UPPER, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringReplace => {
                    (vut_runtime::abi::STRING_REPLACE, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringFind => {
                    (vut_runtime::abi::STRING_FIND, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringSplit => {
                    (vut_runtime::abi::STRING_SPLIT, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringSubstring => {
                    (vut_runtime::abi::STRING_SUBSTRING, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringLines => {
                    (vut_runtime::abi::STRING_LINES, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringSplitWhitespace => {
                    (vut_runtime::abi::STRING_SPLIT_WHITESPACE, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringChars => {
                    (vut_runtime::abi::STRING_CHARS, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringCharAt => {
                    (vut_runtime::abi::STRING_CHAR_AT, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringRepeat => {
                    (vut_runtime::abi::STRING_REPEAT, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringPadLeft => {
                    (vut_runtime::abi::STRING_PAD_LEFT, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringPadRight => {
                    (vut_runtime::abi::STRING_PAD_RIGHT, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringStripPrefix => {
                    (vut_runtime::abi::STRING_STRIP_PREFIX, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringStripSuffix => {
                    (vut_runtime::abi::STRING_STRIP_SUFFIX, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringRfind => {
                    (vut_runtime::abi::STRING_RFIND, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringCompare => {
                    (vut_runtime::abi::STRING_COMPARE, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringEquals => {
                    (vut_runtime::abi::STRING_EQ, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::StringToI64 => {
                    (vut_runtime::abi::STRING_TO_I64, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::StringToF64 => {
                    (vut_runtime::abi::STRING_TO_F64, &[types::F64][..])
                }
                vut_mir::BuiltinFunction::MapKeys => {
                    (vut_runtime::abi::MAP_KEYS, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::MapValues => {
                    (vut_runtime::abi::MAP_VALUES, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::ArrayToList => {
                    (vut_runtime::abi::ARRAY_TO_LIST, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::ListData => {
                    (vut_runtime::abi::LIST_DATA, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::ListJoin => {
                    (vut_runtime::abi::LIST_JOIN, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::BytesPush => (vut_runtime::abi::BYTES_PUSH, &[][..]),
                vut_mir::BuiltinFunction::BytesExtend => (vut_runtime::abi::BYTES_EXTEND, &[][..]),
                vut_mir::BuiltinFunction::BytesTruncate => {
                    (vut_runtime::abi::BYTES_TRUNCATE, &[][..])
                }
                vut_mir::BuiltinFunction::BytesResize => (vut_runtime::abi::BYTES_RESIZE, &[][..]),
                vut_mir::BuiltinFunction::BytesFind => {
                    (vut_runtime::abi::BYTES_FIND, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::BytesStartsWith => {
                    (vut_runtime::abi::BYTES_STARTS_WITH, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::BytesEndsWith => {
                    (vut_runtime::abi::BYTES_ENDS_WITH, &[types::I8][..])
                }
                vut_mir::BuiltinFunction::BytesCompare => {
                    (vut_runtime::abi::BYTES_COMPARE, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::BytesToHex => {
                    (vut_runtime::abi::BYTES_TO_HEX, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::BytesByteAt => {
                    (vut_runtime::abi::BYTES_BYTE_AT, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::IntToChar => {
                    (vut_runtime::abi::CHAR_FROM_CODE, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::IntToFloat => {
                    (vut_runtime::abi::INT_TO_FLOAT, &[types::F64][..])
                }
                vut_mir::BuiltinFunction::IntAbs => (vut_runtime::abi::INT_ABS, &[types::I64][..]),
                vut_mir::BuiltinFunction::IntPow => (vut_runtime::abi::INT_POW, &[types::I64][..]),
                vut_mir::BuiltinFunction::IntMin => (vut_runtime::abi::INT_MIN, &[types::I64][..]),
                vut_mir::BuiltinFunction::IntMax => (vut_runtime::abi::INT_MAX, &[types::I64][..]),
                vut_mir::BuiltinFunction::IntClamp => {
                    (vut_runtime::abi::INT_CLAMP, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::BoolToStr => {
                    (vut_runtime::abi::BOOL_TO_STR, &[types::I64][..])
                }
                vut_mir::BuiltinFunction::NumericToStr => {
                    let argument = values[&arguments[0]];
                    if builder.func.dfg.value_type(argument).is_float() {
                        (vut_runtime::abi::FORMAT_F64, &[types::I64][..])
                    } else {
                        (vut_runtime::abi::FORMAT_I64, &[types::I64][..])
                    }
                }
                vut_mir::BuiltinFunction::ListNew
                | vut_mir::BuiltinFunction::StringToBytes
                | vut_mir::BuiltinFunction::BytesNew
                | vut_mir::BuiltinFunction::BytesClone
                | vut_mir::BuiltinFunction::BytesLen
                | vut_mir::BuiltinFunction::BytesIsEmpty
                | vut_mir::BuiltinFunction::BytesCapacity
                | vut_mir::BuiltinFunction::BytesReserve
                | vut_mir::BuiltinFunction::BytesAt
                | vut_mir::BuiltinFunction::BytesSet
                | vut_mir::BuiltinFunction::BytesFirst
                | vut_mir::BuiltinFunction::BytesLast
                | vut_mir::BuiltinFunction::BytesSlice
                | vut_mir::BuiltinFunction::BytesClear
                | vut_mir::BuiltinFunction::BytesToList
                | vut_mir::BuiltinFunction::BytesFromList
                | vut_mir::BuiltinFunction::BytesToStr
                | vut_mir::BuiltinFunction::BytesFromHex
                | vut_mir::BuiltinFunction::BytesReadInt { .. }
                | vut_mir::BuiltinFunction::BytesWriteInt { .. }
                | vut_mir::BuiltinFunction::ListLen
                | vut_mir::BuiltinFunction::ListIsEmpty
                | vut_mir::BuiltinFunction::ListCapacity
                | vut_mir::BuiltinFunction::ListReserve
                | vut_mir::BuiltinFunction::ListPush
                | vut_mir::BuiltinFunction::ListAt
                | vut_mir::BuiltinFunction::ListSet
                | vut_mir::BuiltinFunction::ListInsert
                | vut_mir::BuiltinFunction::ListRemove
                | vut_mir::BuiltinFunction::ListClear
                | vut_mir::BuiltinFunction::ListSlice
                | vut_mir::BuiltinFunction::ListContains
                | vut_mir::BuiltinFunction::ListPop
                | vut_mir::BuiltinFunction::ListFirst
                | vut_mir::BuiltinFunction::ListLast
                | vut_mir::BuiltinFunction::ListFindIndex
                | vut_mir::BuiltinFunction::ListExtend
                | vut_mir::BuiltinFunction::ListReverse
                | vut_mir::BuiltinFunction::ListSort
                | vut_mir::BuiltinFunction::ListTruncate
                | vut_mir::BuiltinFunction::ListSwap
                | vut_mir::BuiltinFunction::ListShrinkToFit
                | vut_mir::BuiltinFunction::MapNew
                | vut_mir::BuiltinFunction::MapLen
                | vut_mir::BuiltinFunction::MapIsEmpty
                | vut_mir::BuiltinFunction::MapCapacity
                | vut_mir::BuiltinFunction::MapReserve
                | vut_mir::BuiltinFunction::MapGet
                | vut_mir::BuiltinFunction::MapSet
                | vut_mir::BuiltinFunction::MapContainsKey
                | vut_mir::BuiltinFunction::MapRemove
                | vut_mir::BuiltinFunction::MapClear
                | vut_mir::BuiltinFunction::MapGetOr
                | vut_mir::BuiltinFunction::ArrayLen
                | vut_mir::BuiltinFunction::ArrayAt
                | vut_mir::BuiltinFunction::ArraySet
                | vut_mir::BuiltinFunction::ArrayFirst
                | vut_mir::BuiltinFunction::ArrayLast
                | vut_mir::BuiltinFunction::ArrayFill
                | vut_mir::BuiltinFunction::ArrayContains
                | vut_mir::BuiltinFunction::ArrayReverse
                | vut_mir::BuiltinFunction::ArraySort
                | vut_mir::BuiltinFunction::FloatAbs
                | vut_mir::BuiltinFunction::FloatFloor
                | vut_mir::BuiltinFunction::FloatCeil
                | vut_mir::BuiltinFunction::FloatRound
                | vut_mir::BuiltinFunction::FloatTrunc
                | vut_mir::BuiltinFunction::FloatSqrt
                | vut_mir::BuiltinFunction::FloatPow
                | vut_mir::BuiltinFunction::FloatMin
                | vut_mir::BuiltinFunction::FloatMax
                | vut_mir::BuiltinFunction::FloatClamp
                | vut_mir::BuiltinFunction::FloatToInt
                | vut_mir::BuiltinFunction::FloatIsNan
                | vut_mir::BuiltinFunction::FloatIsFinite
                | vut_mir::BuiltinFunction::ResultIsOk
                | vut_mir::BuiltinFunction::ResultIsErr
                | vut_mir::BuiltinFunction::ResultUnwrapOr
                | vut_mir::BuiltinFunction::VariadicLen
                | vut_mir::BuiltinFunction::VariadicAt
                | vut_mir::BuiltinFunction::ListMap
                | vut_mir::BuiltinFunction::ListFilter
                | vut_mir::BuiltinFunction::ListAny
                | vut_mir::BuiltinFunction::ListAll
                | vut_mir::BuiltinFunction::ListFold
                | vut_mir::BuiltinFunction::ListFindIndexBy
                | vut_mir::BuiltinFunction::ListSortBy => unreachable!(),
            };
            let parameters = if *function == vut_mir::BuiltinFunction::NumericToStr
                && builder
                    .func
                    .dfg
                    .value_type(values[&arguments[0]])
                    .is_float()
            {
                vec![types::F64]
            } else {
                vec![types::I64; arguments.len()]
            };
            let target = runtime_function(module, name, &parameters, returns)?;
            let reference = module.declare_func_in_func(target, builder.func);
            let args: Vec<_> = arguments
                .iter()
                .map(|id| coerce_integer(builder, values[id], types::I64))
                .collect();
            let call = builder.ins().call(reference, &args);
            value.map(|id| (id, builder.inst_results(call)[0]))
        }
        Instruction::Drop(local) => {
            if let Some(ty) = local_types[local.0] {
                let value = local_read(
                    builder,
                    variables,
                    storages,
                    frame_local,
                    layouts,
                    local_types,
                    *local,
                )?;
                manage_value(builder, module, layouts, ty, value, false)?;
            }
            None
        }
        Instruction::Retain { value, ty } => {
            manage_value(builder, module, layouts, *ty, values[value], true)?;
            None
        }
        Instruction::Release { value, ty } => {
            manage_value(builder, module, layouts, *ty, values[value], false)?;
            None
        }
        Instruction::ConstructInterface {
            value,
            ty,
            concrete_ty,
            interface,
            source,
        } => {
            let layout = layouts.types[concrete_ty.0];
            let new = runtime_function(
                module,
                vut_runtime::abi::INTERFACE_NEW,
                &[types::I64, types::I64],
                &[types::I64],
            )?;
            let new_ref = module.declare_func_in_func(new, builder.func);
            let size = builder
                .ins()
                .iconst(types::I64, i64::try_from(layout.size).unwrap_or(i64::MAX));
            let align = builder.ins().iconst(
                types::I64,
                i64::try_from(layout.alignment.max(1)).unwrap_or(1),
            );
            let call = builder.ins().call(new_ref, &[size, align]);
            let box_ptr = builder.inst_results(call)[0];
            let data_fn = runtime_function(
                module,
                vut_runtime::abi::INTERFACE_DATA,
                &[types::I64],
                &[types::I64],
            )?;
            let data_ref = module.declare_func_in_func(data_fn, builder.func);
            let call = builder.ins().call(data_ref, &[box_ptr]);
            let data_ptr = builder.inst_results(call)[0];
            if layouts.is_aggregate(*concrete_ty) {
                copy_aggregate(builder, layouts, *concrete_ty, values[source], data_ptr)?;
            } else {
                let stored =
                    coerce_integer(builder, values[source], machine_type(layouts, *concrete_ty));
                builder
                    .ins()
                    .store(MemFlagsData::trusted(), stored, data_ptr, 0);
            }
            let vtable_id = vtables
                .get(&(concrete_ty.0, *interface))
                .copied()
                .ok_or_else(|| {
                    CodegenError::Backend("invalid typed MIR: missing interface vtable".into())
                })?;
            let global = module.declare_data_in_func(vtable_id, builder.func);
            let pointer = module.target_config().pointer_type();
            let vtable_ptr = builder.ins().symbol_value(pointer, global);
            let set_fn = runtime_function(
                module,
                vut_runtime::abi::INTERFACE_SET_VTABLE,
                &[types::I64, types::I64],
                &[],
            )?;
            let set_ref = module.declare_func_in_func(set_fn, builder.func);
            builder.ins().call(set_ref, &[box_ptr, vtable_ptr]);
            value_types.insert(*value, *ty);
            Some((*value, box_ptr))
        }
        Instruction::InterfaceCall {
            value,
            result_type,
            callee,
            method_index,
            arguments,
        } => {
            let data_fn = runtime_function(
                module,
                vut_runtime::abi::INTERFACE_DATA,
                &[types::I64],
                &[types::I64],
            )?;
            let data_ref = module.declare_func_in_func(data_fn, builder.func);
            let call = builder.ins().call(data_ref, &[values[callee]]);
            let data_ptr = builder.inst_results(call)[0];
            let vtable_fn = runtime_function(
                module,
                vut_runtime::abi::INTERFACE_VTABLE,
                &[types::I64],
                &[types::I64],
            )?;
            let vtable_ref = module.declare_func_in_func(vtable_fn, builder.func);
            let call = builder.ins().call(vtable_ref, &[values[callee]]);
            let vtable_ptr = builder.inst_results(call)[0];
            let slot = method_index.saturating_add(1).saturating_mul(8);
            let offset = i32::try_from(slot).map_err(|_| {
                CodegenError::Backend("interface method offset exceeds backend limit".into())
            })?;
            let method =
                builder
                    .ins()
                    .load(types::I64, MemFlagsData::trusted(), vtable_ptr, offset);
            let aggregate_return = result_type.filter(|ty| layouts.is_aggregate(*ty));
            let destination = aggregate_return
                .map(|ty| stack_slot_for_type(builder, layouts, ty))
                .transpose()?;
            let mut signature = Signature::new(c_call_conv(module));
            if destination.is_some() {
                signature.params.push(AbiParam::new(types::I64));
            }
            signature.params.push(AbiParam::new(types::I64));
            for argument in arguments {
                let ty = *value_types.get(argument).ok_or_else(|| {
                    CodegenError::Backend(
                        "invalid typed MIR: interface argument has no type".into(),
                    )
                })?;
                signature
                    .params
                    .push(AbiParam::new(machine_type(layouts, ty)));
            }
            if let Some(ty) = result_type
                && layouts.types[ty.0].repr != ValueRepr::Void
                && !layouts.is_aggregate(*ty)
            {
                signature
                    .returns
                    .push(AbiParam::new(machine_type(layouts, *ty)));
            }
            let mut args = Vec::new();
            if let Some(destination) = destination {
                args.push(destination);
            }
            args.push(data_ptr);
            let parameter_offset = usize::from(destination.is_some()).saturating_add(1);
            for (index, argument) in arguments.iter().enumerate() {
                let parameter = signature.params[parameter_offset + index];
                args.push(coerce_integer(
                    builder,
                    values[argument],
                    parameter.value_type,
                ));
            }
            let signature_ref = builder.func.import_signature(signature);
            let call = builder.ins().call_indirect(signature_ref, method, &args);
            if let Some((id, ty)) = value.zip(*result_type) {
                value_types.insert(id, ty);
            }
            if let Some(destination) = destination {
                value.map(|id| (id, destination))
            } else {
                value.and_then(|id| {
                    builder
                        .inst_results(call)
                        .first()
                        .map(|result| (id, *result))
                })
            }
        }
    };
    if let Some((id, value)) = pair {
        values.insert(id, value);
    }
    Ok(())
}

/// Produces the address of a compiler-generated per-type retain/release
/// callback, or a null pointer when the type needs no management.
fn type_function_address(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    type_functions: &HashMap<(usize, bool), FuncId>,
    type_index: usize,
    retain: bool,
) -> cranelift_codegen::ir::Value {
    match type_functions.get(&(type_index, retain)) {
        Some(id) => {
            let reference = module.declare_func_in_func(*id, builder.func);
            builder.ins().func_addr(types::I64, reference)
        }
        None => builder.ins().iconst(types::I64, 0),
    }
}

fn frame_offset_i32(offset: usize) -> Result<i32, CodegenError> {
    i32::try_from(offset)
        .map_err(|_| CodegenError::Backend("frame offset exceeds backend limit".into()))
}

fn frame_offset_i64(offset: usize) -> Result<i64, CodegenError> {
    i64::try_from(offset)
        .map_err(|_| CodegenError::Backend("frame offset exceeds backend limit".into()))
}

fn frame_pointer(
    builder: &mut FunctionBuilder<'_>,
    variables: &[Variable],
    frame_local: Option<vut_mir::LocalId>,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    let frame = frame_local
        .ok_or_else(|| CodegenError::Backend("frame-backed local without a frame".into()))?;
    Ok(builder.use_var(variables[frame.0]))
}

/// Reads a local, transparently loading frame-resident locals.
fn local_read(
    builder: &mut FunctionBuilder<'_>,
    variables: &[Variable],
    storages: &[vut_mir::LocalStorage],
    frame_local: Option<vut_mir::LocalId>,
    layouts: &LayoutTable,
    local_types: &[Option<vut_hir::TypeId>],
    local: vut_mir::LocalId,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    match storages
        .get(local.0)
        .copied()
        .unwrap_or(vut_mir::LocalStorage::Stack)
    {
        vut_mir::LocalStorage::Stack => Ok(builder.use_var(variables[local.0])),
        vut_mir::LocalStorage::Frame(offset) => {
            let pointer = frame_pointer(builder, variables, frame_local)?;
            match local_types[local.0] {
                Some(ty) if layouts.is_aggregate(ty) => {
                    Ok(builder.ins().iadd_imm_u(pointer, frame_offset_i64(offset)?))
                }
                Some(ty) => Ok(builder.ins().load(
                    machine_type(layouts, ty),
                    MemFlagsData::trusted(),
                    pointer,
                    frame_offset_i32(offset)?,
                )),
                None => Ok(builder.ins().load(
                    types::I64,
                    MemFlagsData::trusted(),
                    pointer,
                    frame_offset_i32(offset)?,
                )),
            }
        }
    }
}

/// Writes a local, transparently storing frame-resident locals.
#[expect(
    clippy::too_many_arguments,
    reason = "local access needs the backend function state"
)]
fn local_write(
    builder: &mut FunctionBuilder<'_>,
    variables: &[Variable],
    storages: &[vut_mir::LocalStorage],
    frame_local: Option<vut_mir::LocalId>,
    layouts: &LayoutTable,
    local_types: &[Option<vut_hir::TypeId>],
    local: vut_mir::LocalId,
    value: cranelift_codegen::ir::Value,
) -> Result<(), CodegenError> {
    match storages
        .get(local.0)
        .copied()
        .unwrap_or(vut_mir::LocalStorage::Stack)
    {
        vut_mir::LocalStorage::Stack => {
            builder.def_var(variables[local.0], value);
            Ok(())
        }
        vut_mir::LocalStorage::Frame(offset) => {
            let pointer = frame_pointer(builder, variables, frame_local)?;
            match local_types[local.0] {
                Some(ty) if layouts.is_aggregate(ty) => {
                    let destination = builder.ins().iadd_imm_u(pointer, frame_offset_i64(offset)?);
                    copy_aggregate(builder, layouts, ty, value, destination)
                }
                Some(ty) => {
                    let stored = coerce_integer(builder, value, machine_type(layouts, ty));
                    builder.ins().store(
                        MemFlagsData::trusted(),
                        stored,
                        pointer,
                        frame_offset_i32(offset)?,
                    );
                    Ok(())
                }
                None => {
                    builder.ins().store(
                        MemFlagsData::trusted(),
                        value,
                        pointer,
                        frame_offset_i32(offset)?,
                    );
                    Ok(())
                }
            }
        }
    }
}

/// Loads an array element as an owned value: non-aggregate managed elements are
/// retained, and aggregate elements are copied to a fresh slot whose managed
/// fields are retained. This matches `list.at`, whose result the caller owns.
fn read_owned_array_element(
    builder: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    layouts: &LayoutTable,
    element_ty: vut_hir::TypeId,
    address: cranelift_codegen::ir::Value,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    if layouts.is_aggregate(element_ty) {
        let slot = stack_slot_for_type(builder, layouts, element_ty)?;
        copy_aggregate(builder, layouts, element_ty, address, slot)?;
        manage_value(builder, module, layouts, element_ty, slot, true)?;
        Ok(slot)
    } else {
        let value = element_value(builder, layouts, element_ty, address);
        manage_value(builder, module, layouts, element_ty, value, true)?;
        Ok(value)
    }
}
