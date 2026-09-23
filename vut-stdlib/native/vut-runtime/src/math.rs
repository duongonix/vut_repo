//! Scalar `f64` math primitives (`vut_rt_math_*`).
//!
//! The standard library exposes `float` (IEEE-754 binary64) math. These
//! functions delegate to the platform libm through Rust's `f64` methods, so
//! domain errors yield NaN, overflow yields `±Inf`, and underflow yields `0`,
//! matching IEEE-754. `f32` is not supported yet.

macro_rules! unary {
    ($($name:ident => $method:ident),+ $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            pub extern "C" fn $name(value: f64) -> f64 {
                value.$method()
            }
        )+
    };
}

unary!(
    vut_rt_math_sin_v1 => sin,
    vut_rt_math_cos_v1 => cos,
    vut_rt_math_tan_v1 => tan,
    vut_rt_math_asin_v1 => asin,
    vut_rt_math_acos_v1 => acos,
    vut_rt_math_atan_v1 => atan,
    vut_rt_math_sinh_v1 => sinh,
    vut_rt_math_cosh_v1 => cosh,
    vut_rt_math_tanh_v1 => tanh,
    vut_rt_math_exp_v1 => exp,
    vut_rt_math_exp2_v1 => exp2,
    vut_rt_math_log_v1 => ln,
    vut_rt_math_log2_v1 => log2,
    vut_rt_math_log10_v1 => log10,
    vut_rt_math_cbrt_v1 => cbrt,
);

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_math_atan2_v1(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Euclidean distance `sqrt(left^2 + right^2)` without intermediate overflow
/// (IEEE-754 `hypot`).
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_math_hypot_v1(left: f64, right: f64) -> f64 {
    left.hypot(right)
}
