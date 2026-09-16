//! Native signatures and ABI type mapping.
use super::CodegenError;

use cranelift_codegen::{
    ir::{AbiParam, InstBuilder, MemFlagsData, Signature, types},
    isa,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use vut_mir::{LayoutTable, ValueRepr};

pub(super) fn runtime_function(
    module: &mut ObjectModule,
    name: &str,
    params: &[cranelift_codegen::ir::Type],
    returns: &[cranelift_codegen::ir::Type],
) -> Result<FuncId, CodegenError> {
    let mut signature = Signature::new(c_call_conv(module));
    signature
        .params
        .extend(params.iter().copied().map(AbiParam::new));
    signature
        .returns
        .extend(returns.iter().copied().map(AbiParam::new));
    module
        .declare_function(name, Linkage::Import, &signature)
        .map_err(|error| CodegenError::Backend(error.to_string()))
}
/// Canonical C calling convention for the active target. Centralizes the
/// backend's target ABI selection so extern declarations, callbacks, and
/// indirect calls all agree.
pub(super) fn c_call_conv(module: &ObjectModule) -> isa::CallConv {
    module.isa().default_call_conv()
}
pub(super) fn signature_for(
    module: &ObjectModule,
    mir: &vut_mir::Function,
    layouts: &LayoutTable,
) -> Signature {
    let mut signature = Signature::new(c_call_conv(module));
    if mir.is_poll {
        // State-machine poll entry: `(frame, out) -> i32`.
        signature.params.push(AbiParam::new(types::I64));
        signature.params.push(AbiParam::new(types::I64));
        signature.returns.push(AbiParam::new(types::I32));
        return signature;
    }
    if mir.return_type.is_some_and(|ty| layouts.is_aggregate(ty)) {
        // Aggregate return uses a caller-provided destination pointer (sret).
        signature.params.push(AbiParam::new(types::I64));
    }
    if mir.frame_param.is_some() {
        // Async body: a single frame-pointer parameter; declared parameters are
        // read from frame slots.
        signature.params.push(AbiParam::new(types::I64));
    } else {
        signature
            .params
            .extend(mir.locals.iter().take(mir.parameter_count).map(|local| {
                AbiParam::new(local.ty.map_or(types::I64, |ty| machine_type(layouts, ty)))
            }));
    }
    if let Some(ty) = mir.return_type
        && layouts.types[ty.0].repr != ValueRepr::Void
    {
        signature
            .returns
            .push(AbiParam::new(machine_type(layouts, ty)));
    }
    signature
}
pub(super) fn external_signature_for(
    module: &ObjectModule,
    function: &vut_mir::ExternalFunction,
    layouts: &LayoutTable,
) -> Signature {
    let mut signature = Signature::new(c_call_conv(module));
    signature.params.extend(
        function
            .parameters
            .iter()
            .copied()
            .map(|ty| AbiParam::new(machine_type(layouts, ty))),
    );
    if let Some(ty) = function.return_type
        && layouts.types[ty.0].repr != ValueRepr::Void
    {
        signature
            .returns
            .push(AbiParam::new(machine_type(layouts, ty)));
    }
    signature
}
pub(super) fn indirect_signature(
    module: &ObjectModule,
    layouts: &LayoutTable,
    callable_ty: Option<vut_hir::TypeId>,
) -> Result<Signature, CodegenError> {
    let mut signature = Signature::new(c_call_conv(module));
    let Some(ty) = callable_ty else {
        return Err(CodegenError::Backend(
            "invalid typed MIR: indirect call without a callable type".into(),
        ));
    };
    let (parameters, result) = layouts.callables.get(&ty).cloned().ok_or_else(|| {
        CodegenError::Backend(format!("invalid typed MIR: type {} is not callable", ty.0))
    })?;
    if layouts.is_aggregate(result) {
        // Aggregate return uses a caller-provided destination pointer (sret).
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.params.extend(
        parameters
            .iter()
            .map(|ty| AbiParam::new(machine_type(layouts, *ty))),
    );
    if layouts.types[result.0].repr != ValueRepr::Void {
        signature
            .returns
            .push(AbiParam::new(machine_type(layouts, result)));
    }
    Ok(signature)
}

pub(super) fn machine_type(
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
) -> cranelift_codegen::ir::Type {
    match layouts.types[ty.0].repr {
        ValueRepr::Float if layouts.types[ty.0].size == 4 => types::F32,
        ValueRepr::Float => types::F64,
        ValueRepr::Integer if layouts.types[ty.0].size == 1 => types::I8,
        ValueRepr::Integer if layouts.types[ty.0].size == 2 => types::I16,
        ValueRepr::Integer if layouts.types[ty.0].size == 4 => types::I32,
        ValueRepr::Void | ValueRepr::Integer | ValueRepr::Pointer => types::I64,
    }
}

/// Copies an aggregate block from `source` to `destination`.
///
/// # Errors
/// Returns an error when a byte offset cannot be represented in the backend.
pub(super) fn copy_aggregate(
    builder: &mut FunctionBuilder<'_>,
    layouts: &LayoutTable,
    ty: vut_hir::TypeId,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
) -> Result<(), CodegenError> {
    let size = layouts.types[ty.0].size;
    let mut offset = 0_usize;
    while offset + 8 <= size {
        let position = i32::try_from(offset)
            .map_err(|_| CodegenError::Backend("aggregate offset exceeds backend limit".into()))?;
        let item = builder
            .ins()
            .load(types::I64, MemFlagsData::trusted(), source, position);
        builder
            .ins()
            .store(MemFlagsData::trusted(), item, destination, position);
        offset += 8;
    }
    while offset < size {
        let position = i32::try_from(offset)
            .map_err(|_| CodegenError::Backend("aggregate offset exceeds backend limit".into()))?;
        let item = builder
            .ins()
            .load(types::I8, MemFlagsData::trusted(), source, position);
        builder
            .ins()
            .store(MemFlagsData::trusted(), item, destination, position);
        offset += 1;
    }
    Ok(())
}

pub(super) fn coerce_integer(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    target: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    let source = builder.func.dfg.value_type(value);
    if source == target || !source.is_int() || !target.is_int() {
        value
    } else if source.bits() > target.bits() {
        builder.ins().ireduce(target, value)
    } else {
        builder.ins().sextend(target, value)
    }
}
