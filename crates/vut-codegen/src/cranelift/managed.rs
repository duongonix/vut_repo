//! Managed-value retain/release and temporary storage helpers.
use super::CodegenError;
use super::signatures::{coerce_integer, copy_aggregate, machine_type, runtime_function};

use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{DataDescription, Linkage, Module};
use cranelift_object::ObjectModule;
use vut_mir::{LayoutTable, OwnershipKind};

#[expect(
    clippy::too_many_lines,
    reason = "ownership dispatch enumerates every managed layout in one place"
)]
pub(super) fn manage_value(
    builder: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
    value: cranelift_codegen::ir::Value,
    retain: bool,
) -> Result<(), CodegenError> {
    match layouts.types[ty.0].ownership {
        OwnershipKind::None | OwnershipKind::OpaqueManaged => {}
        OwnershipKind::RcMap => {
            let name = if retain {
                vut_runtime::abi::MAP_RETAIN
            } else {
                vut_runtime::abi::MAP_RELEASE
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::RcBytes => {
            let name = if retain {
                vut_runtime::abi::BYTES_RETAIN
            } else {
                vut_runtime::abi::BYTES_RELEASE
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::RcList => {
            let name = if retain {
                vut_runtime::abi::LIST_RETAIN
            } else {
                vut_runtime::abi::LIST_RELEASE
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::RcChannel => {
            let name = if retain {
                vut_runtime::abi::CHANNEL_RETAIN
            } else {
                vut_runtime::abi::CHANNEL_RELEASE
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::RcString => {
            let name = if retain {
                vut_runtime::abi::RETAIN_STRING
            } else {
                vut_runtime::abi::RELEASE_STRING
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::RcVutcon => {
            if retain {
                return Err(CodegenError::Backend(
                    "cannot retain a move-only Vutcon handle".into(),
                ));
            }
            let target =
                runtime_function(module, vut_runtime::abi::ASYNC_DROP, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::Resource => {
            if retain {
                return Err(CodegenError::Backend(
                    "cannot retain a move-only resource handle".into(),
                ));
            }
            let target = runtime_function(
                module,
                vut_runtime::abi::RESOURCE_RELEASE,
                &[types::I64],
                &[],
            )?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::Future => {
            if retain {
                return Err(CodegenError::Backend(
                    "cannot retain a move-only future handle".into(),
                ));
            }
            let target =
                runtime_function(module, vut_runtime::abi::ASYNC_DROP, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::Interface => {
            let name = if retain {
                vut_runtime::abi::INTERFACE_RETAIN
            } else {
                vut_runtime::abi::INTERFACE_RELEASE
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::RcClosure => {
            let name = if retain {
                vut_runtime::abi::CLOSURE_RETAIN
            } else {
                vut_runtime::abi::CLOSURE_RELEASE
            };
            let target = runtime_function(module, name, &[types::I64], &[])?;
            let reference = module.declare_func_in_func(target, builder.func);
            builder.ins().call(reference, &[value]);
        }
        OwnershipKind::Aggregate => {
            if let Some(inner) = layouts.optionals.get(&ty).copied() {
                // A tagged optional owns its payload only when present.
                if layouts.types[inner.0].needs_drop {
                    let tag = builder
                        .ins()
                        .load(types::I64, MemFlagsData::trusted(), value, 0);
                    let present_block = builder.create_block();
                    let join = builder.create_block();
                    let is_present = builder.ins().icmp_imm_s(IntCC::NotEqual, tag, 0);
                    builder
                        .ins()
                        .brif(is_present, present_block, &[], join, &[]);
                    builder.switch_to_block(present_block);
                    let payload_offset = optional_payload_offset(layouts, inner);
                    let inner_value = if layouts.is_aggregate(inner) {
                        let delta = i64::try_from(payload_offset).map_err(|_| {
                            CodegenError::Backend("optional payload offset exceeds limit".into())
                        })?;
                        builder.ins().iadd_imm_u(value, delta)
                    } else {
                        let offset = i32::try_from(payload_offset).map_err(|_| {
                            CodegenError::Backend("optional payload offset exceeds limit".into())
                        })?;
                        builder.ins().load(
                            machine_type(layouts, inner),
                            MemFlagsData::trusted(),
                            value,
                            offset,
                        )
                    };
                    manage_value(builder, module, layouts, inner, inner_value, retain)?;
                    builder.ins().jump(join, &[]);
                    builder.switch_to_block(join);
                }
                return Ok(());
            }
            if let Some(result) = layouts.results.get(&ty) {
                let tag = builder.ins().load(
                    types::I64,
                    MemFlagsData::trusted(),
                    value,
                    i32::try_from(result.tag_offset).map_err(|_| {
                        CodegenError::Backend("result tag offset exceeds backend limit".into())
                    })?,
                );
                let ok_block = builder.create_block();
                let err_block = builder.create_block();
                let join = builder.create_block();
                let is_ok = builder.ins().icmp_imm_s(IntCC::Equal, tag, 0);
                builder.ins().brif(is_ok, ok_block, &[], err_block, &[]);
                builder.switch_to_block(ok_block);
                if layouts.types[result.ok.0].needs_drop {
                    let payload = load_aggregate_payload(
                        builder,
                        layouts,
                        result.ok,
                        value,
                        result.ok_offset,
                    )?;
                    manage_value(builder, module, layouts, result.ok, payload, retain)?;
                }
                builder.ins().jump(join, &[]);
                builder.switch_to_block(err_block);
                if layouts.types[result.err.0].needs_drop {
                    let payload = load_aggregate_payload(
                        builder,
                        layouts,
                        result.err,
                        value,
                        result.err_offset,
                    )?;
                    manage_value(builder, module, layouts, result.err, payload, retain)?;
                }
                builder.ins().jump(join, &[]);
                builder.switch_to_block(join);
                return Ok(());
            }
            if let Some(enum_layout) = layouts.enums.get(&ty).cloned() {
                // Retain/release only the active variant's managed payload.
                let tag_machine = match enum_layout.tag_size {
                    1 => types::I8,
                    2 => types::I16,
                    4 => types::I32,
                    _ => types::I64,
                };
                let tag = builder.ins().load(
                    tag_machine,
                    MemFlagsData::trusted(),
                    value,
                    i32::try_from(enum_layout.tag_offset).map_err(|_| {
                        CodegenError::Backend("enum tag offset exceeds backend limit".into())
                    })?,
                );
                let tag = coerce_integer(builder, tag, types::I64);
                let join = builder.create_block();
                let mut current_test = builder.create_block();
                builder.ins().jump(current_test, &[]);
                for (index, variant) in enum_layout.variants.iter().enumerate() {
                    builder.switch_to_block(current_test);
                    let body = builder.create_block();
                    let after = builder.create_block();
                    let expected = builder
                        .ins()
                        .iconst(types::I64, i64::try_from(index).unwrap_or(0));
                    let is_variant = builder.ins().icmp(IntCC::Equal, tag, expected);
                    builder.ins().brif(is_variant, body, &[], after, &[]);
                    builder.switch_to_block(body);
                    for field in &variant.fields {
                        if !layouts.types[field.ty.0].needs_drop {
                            continue;
                        }
                        let field_value = if layouts.is_aggregate(field.ty) {
                            let delta = i64::try_from(field.offset).map_err(|_| {
                                CodegenError::Backend(
                                    "enum payload offset exceeds backend limit".into(),
                                )
                            })?;
                            builder.ins().iadd_imm_u(value, delta)
                        } else {
                            let offset = i32::try_from(field.offset).map_err(|_| {
                                CodegenError::Backend(
                                    "enum payload offset exceeds backend limit".into(),
                                )
                            })?;
                            builder.ins().load(
                                machine_type(layouts, field.ty),
                                MemFlagsData::trusted(),
                                value,
                                offset,
                            )
                        };
                        manage_value(builder, module, layouts, field.ty, field_value, retain)?;
                    }
                    builder.ins().jump(join, &[]);
                    current_test = after;
                }
                builder.switch_to_block(current_test);
                builder.ins().jump(join, &[]);
                builder.switch_to_block(join);
                return Ok(());
            }
            for field in layouts.fields.get(&ty).into_iter().flatten() {
                if !layouts.types[field.ty.0].needs_drop {
                    continue;
                }
                let offset = i32::try_from(field.offset).map_err(|_| {
                    CodegenError::Backend("managed field offset exceeds backend limit".into())
                })?;
                let field_value = if layouts.is_aggregate(field.ty) {
                    // Aggregate fields are stored inline; operate on their address.
                    let delta = i64::try_from(field.offset).map_err(|_| {
                        CodegenError::Backend("managed field offset exceeds backend limit".into())
                    })?;
                    builder.ins().iadd_imm_u(value, delta)
                } else {
                    builder.ins().load(
                        machine_type(layouts, field.ty),
                        MemFlagsData::trusted(),
                        value,
                        offset,
                    )
                };
                manage_value(builder, module, layouts, field.ty, field_value, retain)?;
            }
            if let Some((element_ty, length)) = layouts.arrays.get(&ty) {
                let (element_ty, length) = (*element_ty, *length);
                if layouts.types[element_ty.0].needs_drop {
                    let stride = layouts.types[element_ty.0].size;
                    for index in 0..length {
                        let offset = i32::try_from(index.saturating_mul(stride)).map_err(|_| {
                            CodegenError::Backend(
                                "managed array element offset exceeds backend limit".into(),
                            )
                        })?;
                        if layouts.is_aggregate(element_ty) {
                            let element_address =
                                builder.ins().iadd_imm_u(value, i64::from(offset));
                            manage_value(
                                builder,
                                module,
                                layouts,
                                element_ty,
                                element_address,
                                retain,
                            )?;
                        } else {
                            let element = builder.ins().load(
                                machine_type(layouts, element_ty),
                                MemFlagsData::trusted(),
                                value,
                                offset,
                            );
                            manage_value(builder, module, layouts, element_ty, element, retain)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Loads a `result` payload slot. Aggregate payloads are stored inline, so the
/// value is the address of the payload; other payloads are loaded by value.
fn load_aggregate_payload(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    payload_ty: vut_hir::TypeId,
    base: cranelift_codegen::ir::Value,
    offset: usize,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    if layouts.is_aggregate(payload_ty) {
        let delta = i64::try_from(offset)
            .map_err(|_| CodegenError::Backend("result payload offset exceeds limit".into()))?;
        Ok(builder.ins().iadd_imm_u(base, delta))
    } else {
        Ok(builder.ins().load(
            machine_type(layouts, payload_ty),
            MemFlagsData::trusted(),
            base,
            i32::try_from(offset)
                .map_err(|_| CodegenError::Backend("result payload offset exceeds limit".into()))?,
        ))
    }
}

/// Returns true when `ty` is a managed handle whose optional form is a nullable
/// handle rather than a boxed scalar.
#[must_use]
pub(super) fn is_managed_handle(layouts: &LayoutTable, ty: vut_hir::TypeId) -> bool {
    matches!(
        layouts.types[ty.0].ownership,
        OwnershipKind::RcString
            | OwnershipKind::RcBytes
            | OwnershipKind::RcList
            | OwnershipKind::RcMap
            | OwnershipKind::RcVutcon
            | OwnershipKind::Resource
            | OwnershipKind::Future
            | OwnershipKind::Interface
            | OwnershipKind::OpaqueManaged
    )
}

/// Byte offset of the payload inside a tagged optional aggregate: the
/// discriminant occupies one pointer-sized word, aligned for the inner type.
#[must_use]
pub(super) fn optional_payload_offset(layouts: &LayoutTable, inner: vut_hir::TypeId) -> usize {
    let align = layouts.types[inner.0].alignment.max(1);
    let pointer = layouts.pointer_size;
    pointer.div_ceil(align) * align
}

pub(super) fn stack_slot_for_type(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    let layout = layouts.types[ty.0];
    let size = u32::try_from(layout.size.max(1))
        .map_err(|_| CodegenError::Backend("temporary value exceeds backend limit".into()))?;
    let align_shift = u8::try_from(layout.alignment.trailing_zeros())
        .map_err(|_| CodegenError::Backend("invalid temporary alignment".into()))?;
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size,
        align_shift,
    ));
    Ok(builder.ins().stack_addr(types::I64, slot, 0))
}

pub(super) fn store_temp_value(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
    value: cranelift_codegen::ir::Value,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    let address = stack_slot_for_type(builder, layouts, ty)?;
    if layouts.types[ty.0].size != 0 {
        let value = coerce_integer(builder, value, machine_type(layouts, ty));
        builder
            .ins()
            .store(MemFlagsData::trusted(), value, address, 0);
    }
    Ok(address)
}

/// Returns a pointer to storage holding one element value suitable for passing
/// to a collection ABI (which copies `element_storage_size` bytes from it).
///
/// Aggregate values are already represented by the address of their storage, so
/// they are passed through unchanged; other values are spilled to a temporary.
pub(super) fn element_pointer(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
    value: cranelift_codegen::ir::Value,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    if layouts.is_aggregate(ty) {
        Ok(value)
    } else {
        store_temp_value(builder, layouts, ty, value)
    }
}

/// Loads one element value from storage at `address`, mirroring
/// [`element_pointer`]: aggregate elements are referenced in place and other
/// elements are loaded by value.
pub(super) fn element_value(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
    address: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    if layouts.types[ty.0].size == 0 {
        builder.ins().iconst(types::I64, 0)
    } else if layouts.is_aggregate(ty) {
        address
    } else {
        builder.ins().load(
            machine_type(layouts, ty),
            MemFlagsData::trusted(),
            address,
            0,
        )
    }
}

/// Builds an optional value of type `optional_ty` from a presence flag and a
/// payload stored at `payload`.
///
/// Managed payloads become a nullable handle (`select(present, handle, 0)`);
/// other payloads become a tagged aggregate `{ discriminant, payload }`.
pub(super) fn optional_from_presence(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    optional_ty: vut_hir::TypeId,
    inner: vut_hir::TypeId,
    present: cranelift_codegen::ir::Value,
    payload: cranelift_codegen::ir::Value,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    if is_managed_handle(layouts, inner) {
        let loaded = element_value(builder, layouts, inner, payload);
        let condition = builder.ins().icmp_imm_u(IntCC::NotEqual, present, 0);
        let zero = builder.ins().iconst(types::I64, 0);
        return Ok(builder.ins().select(condition, loaded, zero));
    }
    let slot = stack_slot_for_type(builder, layouts, optional_ty)?;
    let tag = coerce_integer(builder, present, types::I64);
    builder.ins().store(MemFlagsData::trusted(), tag, slot, 0);
    let offset = optional_payload_offset(layouts, inner);
    if layouts.is_aggregate(inner) {
        let delta = i64::try_from(offset)
            .map_err(|_| CodegenError::Backend("optional payload offset exceeds limit".into()))?;
        let destination = builder.ins().iadd_imm_u(slot, delta);
        copy_aggregate(builder, layouts, inner, payload, destination)?;
    } else if layouts.types[inner.0].size != 0 {
        let loaded = element_value(builder, layouts, inner, payload);
        let index = i32::try_from(offset)
            .map_err(|_| CodegenError::Backend("optional payload offset exceeds limit".into()))?;
        builder
            .ins()
            .store(MemFlagsData::trusted(), loaded, slot, index);
    }
    Ok(slot)
}

/// Zeroes a stack slot so that a subsequent unconditional load is defined even
/// when the runtime reports an out-of-bounds access and does not write it.
pub(super) fn zero_stack_value(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
    address: cranelift_codegen::ir::Value,
) {
    let size = layouts.types[ty.0].size.max(1);
    let zero = builder.ins().iconst(types::I8, 0);
    if let Ok(offset) = i32::try_from(size) {
        for index in 0..offset {
            builder
                .ins()
                .store(MemFlagsData::trusted(), zero, address, index);
        }
    }
}

pub(super) fn const_runtime_string(
    builder: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    next_data: &mut usize,
    literal: &str,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    let name = format!("vut_string_{}", *next_data);
    *next_data += 1;
    let data = module
        .declare_data(&name, Linkage::Local, false, false)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    let mut description = DataDescription::new();
    description.define(literal.as_bytes().to_vec().into_boxed_slice());
    module
        .define_data(data, &description)
        .map_err(|error| CodegenError::Backend(error.to_string()))?;
    let global = module.declare_data_in_func(data, builder.func);
    let pointer = module.target_config().pointer_type();
    let address = builder.ins().symbol_value(pointer, global);
    let literal_len = i64::try_from(literal.len())
        .map_err(|_| CodegenError::Backend("string literal exceeds target limit".into()))?;
    let len = builder.ins().iconst(types::I64, literal_len);
    let target = runtime_function(
        module,
        vut_runtime::abi::STRING_FROM_UTF8,
        &[types::I64, types::I64],
        &[types::I64],
    )?;
    let reference = module.declare_func_in_func(target, builder.func);
    let call = builder.ins().call(reference, &[address, len]);
    Ok(builder.inst_results(call)[0])
}
