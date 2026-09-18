//! Numeric and boolean scalar intrinsics (`abs`, `pow`, rounding,
//! comparisons) plus `bool.to_str`.
//!
//! These are pure scalar helpers: no allocation, no managed values. Signed
//! overflow saturates rather than trapping, keeping the helpers total.

use super::string::{ManagedString, managed_string};

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_int_abs_v1(value: i64) -> i64 {
    value.wrapping_abs()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed. A negative exponent yields `1`
/// and overflow saturates to `i64::MAX`.
pub extern "C" fn vut_rt_int_pow_v1(base: i64, exponent: i64) -> i64 {
    let Ok(exponent) = u32::try_from(exponent) else {
        return 0;
    };
    base.checked_pow(exponent).unwrap_or(i64::MAX)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_int_min_v1(left: i64, right: i64) -> i64 {
    left.min(right)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_int_max_v1(left: i64, right: i64) -> i64 {
    left.max(right)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_int_clamp_v1(value: i64, low: i64, high: i64) -> i64 {
    if low > high {
        return value;
    }
    value.clamp(low, high)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_abs_v1(value: f64) -> f64 {
    value.abs()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_floor_v1(value: f64) -> f64 {
    value.floor()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_ceil_v1(value: f64) -> f64 {
    value.ceil()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_round_v1(value: f64) -> f64 {
    value.round()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_trunc_v1(value: f64) -> f64 {
    value.trunc()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed. Negative input yields NaN.
pub extern "C" fn vut_rt_f64_sqrt_v1(value: f64) -> f64 {
    value.sqrt()
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_pow_v1(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed. NaN yields `0`; out-of-range and
/// non-finite values saturate.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "float.to_int is an explicit, saturating truncating conversion"
)]
pub extern "C" fn vut_rt_f64_to_int_v1(value: f64) -> i64 {
    if value.is_nan() {
        return 0;
    }
    let truncated = value.trunc();
    if truncated >= i64::MAX as f64 {
        i64::MAX
    } else if truncated <= i64::MIN as f64 {
        i64::MIN
    } else {
        truncated as i64
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_is_nan_v1(value: f64) -> u8 {
    u8::from(value.is_nan())
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_is_finite_v1(value: f64) -> u8 {
    u8::from(value.is_finite())
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed. NaN is ignored, like `f64::min`.
pub extern "C" fn vut_rt_f64_min_v1(left: f64, right: f64) -> f64 {
    left.min(right)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_max_v1(left: f64, right: f64) -> f64 {
    left.max(right)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper; no memory is accessed.
pub extern "C" fn vut_rt_f64_clamp_v1(value: f64, low: f64, high: f64) -> f64 {
    if low > high {
        return value;
    }
    value.clamp(low, high)
}

#[unsafe(no_mangle)]
/// # Safety
/// Pure scalar helper. The caller owns one reference to the returned string.
pub extern "C" fn vut_rt_bool_to_str_v1(value: usize) -> *mut ManagedString {
    managed_string(if value == 0 { "false" } else { "true" })
}
