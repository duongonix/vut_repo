//! Explicit numeric conversions with checked (non-wrapping) semantics.
//!
//! Integer -> integer narrows/sign-changes are range-checked and trap through
//! the shared numeric panic path; float -> integer rejects NaN and out-of-range
//! values. Float widening/narrowing follows IEEE-754. No conversion silently
//! wraps or reinterprets bits. Fractional float conversion truncates toward zero.
use cranelift_codegen::ir::{
    InstBuilder, TrapCode, Value,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;
use vut_mir::NumericKind;

use super::CodegenError;
use super::signatures::runtime_function;

fn machine_of(
    kind: NumericKind,
    pointer: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Type {
    match kind {
        NumericKind::I8 | NumericKind::U8 => types::I8,
        NumericKind::I16 | NumericKind::U16 => types::I16,
        NumericKind::I32 | NumericKind::U32 => types::I32,
        NumericKind::I64 | NumericKind::U64 | NumericKind::Int => types::I64,
        NumericKind::Isize | NumericKind::Usize => pointer,
        NumericKind::F32 => types::F32,
        NumericKind::F64 | NumericKind::Float => types::F64,
    }
}

/// Sign/zero-extends `value` to an `I64` (identity when already 64-bit).
fn widen(builder: &mut FunctionBuilder<'_>, kind: NumericKind, value: Value) -> Value {
    if builder.func.dfg.value_type(value).bits() >= 64 {
        return value;
    }
    if kind.is_unsigned() {
        builder.ins().uextend(types::I64, value)
    } else {
        builder.ins().sextend(types::I64, value)
    }
}

/// Branches to a panic path (and traps) unless `valid` holds.
fn trap_unless(
    builder: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    valid: Value,
) -> Result<(), CodegenError> {
    let pass = builder.create_block();
    let fail = builder.create_block();
    builder.ins().brif(valid, pass, &[], fail, &[]);
    builder.switch_to_block(fail);
    let target = runtime_function(module, vut_runtime::abi::NUMERIC_PANIC, &[types::I64], &[])?;
    let reference = module.declare_func_in_func(target, builder.func);
    let marker = builder.ins().iconst(types::I64, 0);
    builder.ins().call(reference, &[marker]);
    builder.ins().trap(TrapCode::unwrap_user(3));
    builder.switch_to_block(pass);
    Ok(())
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

/// Converts `value` from `from` to `to`, trapping on any lossy integer change.
pub(super) fn numeric_cast(
    builder: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    pointer: cranelift_codegen::ir::Type,
    from: NumericKind,
    to: NumericKind,
    value: Value,
) -> Result<Value, CodegenError> {
    if !from.is_float() && !to.is_float() {
        let wide = widen(builder, from, value);
        let destination = machine_of(to, pointer);
        let narrowed = if builder.func.dfg.value_type(wide) == destination {
            wide
        } else {
            builder.ins().ireduce(destination, wide)
        };
        let back = widen(builder, to, narrowed);
        let mut valid = builder.ins().icmp(IntCC::Equal, back, wide);
        if from.is_unsigned() != to.is_unsigned() {
            // A large unsigned value has the top bit set; reject it for a signed
            // destination even when the bit pattern round-trips.
            let non_negative = builder
                .ins()
                .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, wide, 0);
            valid = builder.ins().band(valid, non_negative);
        }
        trap_unless(builder, module, valid)?;
        return Ok(narrowed);
    }
    if !from.is_float() && to.is_float() {
        let wide = widen(builder, from, value);
        // Convert directly to the destination precision to avoid double
        // rounding (integer -> f64 -> f32 can pick the wrong adjacent f32).
        let destination = machine_of(to, pointer);
        return Ok(if from.is_unsigned() {
            builder.ins().fcvt_from_uint(destination, wide)
        } else {
            builder.ins().fcvt_from_sint(destination, wide)
        });
    }
    if from.is_float() && !to.is_float() {
        let float = if from == NumericKind::F32 {
            builder.ins().fpromote(types::F64, value)
        } else {
            value
        };
        // Check finiteness before truncation, then range-check the integral value.
        let magnitude = builder.ins().fabs(float);
        let infinity = builder.ins().f64const(f64::INFINITY);
        let finite = builder.ins().fcmp(FloatCC::LessThan, magnitude, infinity);
        trap_unless(builder, module, finite)?;
        let float = builder.ins().trunc(float);
        let (min, max) = float_int_range(to, pointer.bits());
        let low = builder.ins().f64const(min);
        let high = builder.ins().f64const(max);
        let at_least_min = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, float, low);
        let below_max = builder.ins().fcmp(FloatCC::LessThan, float, high);
        let in_range = builder.ins().band(at_least_min, below_max);
        trap_unless(builder, module, in_range)?;
        let destination = machine_of(to, pointer);
        // Cranelift float conversions produce native 32/64-bit integers, not
        // i8/i16. The checked value can safely be reduced after conversion.
        let converted = if to.is_unsigned() {
            builder.ins().fcvt_to_uint(types::I64, float)
        } else {
            builder.ins().fcvt_to_sint(types::I64, float)
        };
        return Ok(if destination == types::I64 {
            converted
        } else {
            builder.ins().ireduce(destination, converted)
        });
    }
    let result = if from == NumericKind::F32 && to != NumericKind::F32 {
        builder.ins().fpromote(types::F64, value)
    } else if from != NumericKind::F32 && to == NumericKind::F32 {
        builder.ins().fdemote(types::F32, value)
    } else {
        value
    };
    Ok(result)
}
