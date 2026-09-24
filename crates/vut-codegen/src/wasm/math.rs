//! Portable `libm` subset for the wasm backend.
//!
//! Transcendental `math` primitives (`vut_rt_math_*`) are implemented here as
//! wasm functions using range reduction and minimax/Taylor polynomials, so they
//! run identically on Wasmtime, Wasmer, Node WASI, and browsers without a host
//! or JavaScript dependency. `sqrt`/`abs`/`floor`/`ceil`/`trunc`/`min`/`max`/
//! `copysign`/`fma` stay as native wasm instructions (lowered by the frontend).
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, ValType};

const F64: ValType = ValType::F64;
const I32: ValType = ValType::I32;

/// Number of math helper bodies.
pub(crate) const COUNT: usize = 17;

const INV_LN2: f64 = std::f64::consts::LOG2_E;
const LN2: f64 = std::f64::consts::LN_2;
const LN2_HI: f64 = 0.693_147_180_369_123_8;
const LN2_LO: f64 = 1.908_214_929_270_587_7e-10;
const LOG2E: f64 = std::f64::consts::LOG2_E;
const LOG10E: f64 = std::f64::consts::LOG10_E;
const FRAC_2_PI: f64 = std::f64::consts::FRAC_2_PI;
const PIO2: f64 = std::f64::consts::FRAC_PI_2;
const PIO2_HI: f64 = 1.570_796_326_734_125_6;
const PIO2_LO: f64 = 6.077_100_506_506_192e-11;
const PI: f64 = std::f64::consts::PI;
const FRAC_PI_4: f64 = std::f64::consts::FRAC_PI_4;
const TAN_3PI8: f64 = 0.414_213_562_373_095_03;
const DBL_MIN: f64 = 2.225_073_858_507_201_4e-308;
const TWO_POW_54: f64 = 1.801_439_850_948_198_4e16;

fn c(x: f64) -> Ins<'static> {
    Ins::F64Const(x.into())
}

/// Emits `coeffs[0] + x*(coeffs[1] + x*(...))`, leaving the result on the stack.
fn horner(f: &mut WasmFunction, x: u32, coeffs: &[f64]) {
    let last = coeffs.len() - 1;
    f.instruction(&c(coeffs[last]));
    for index in (0..last).rev() {
        f.instruction(&Ins::LocalGet(x));
        f.instruction(&Ins::F64Mul);
        f.instruction(&c(coeffs[index]));
        f.instruction(&Ins::F64Add);
    }
}

fn exp_coeffs() -> Vec<f64> {
    let mut factorial = 1.0_f64;
    let mut coeffs = vec![1.0];
    for k in 1..=13 {
        factorial *= f64::from(k);
        coeffs.push(1.0 / factorial);
    }
    coeffs
}

fn log_coeffs() -> Vec<f64> {
    (0..20).map(|k| 1.0 / f64::from(2 * k + 1)).collect()
}

fn sin_coeffs() -> Vec<f64> {
    let mut factorial = 1.0_f64;
    let mut coeffs = Vec::new();
    for k in 0..=6 {
        if k > 0 {
            factorial *= f64::from(2 * k) * f64::from(2 * k + 1);
        }
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        coeffs.push(sign / factorial);
    }
    coeffs
}

fn cos_coeffs() -> Vec<f64> {
    let mut factorial = 1.0_f64;
    let mut coeffs = Vec::new();
    for k in 0..=6 {
        if k > 0 {
            factorial *= f64::from(2 * k - 1) * f64::from(2 * k);
        }
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        coeffs.push(sign / factorial);
    }
    coeffs
}

fn atan_coeffs() -> Vec<f64> {
    (0..20)
        .map(|k| if k % 2 == 0 { 1.0 } else { -1.0 } / f64::from(2 * k + 1))
        .collect()
}

/// Math helper bodies in [`COUNT`] order, referencing `ldexp` and the block base.
#[must_use]
pub(crate) fn bodies(base: u32, ldexp: u32) -> Vec<WasmFunction> {
    let exp = base;
    let log = base + 2;
    let sin = base + 5;
    let cos = base + 6;
    let atan = base + 10;
    let asin = base + 8;
    vec![
        exp_body(ldexp),
        exp2_body(exp),
        log_body(),
        log2_body(log),
        log10_body(log),
        sin_body(),
        cos_body(),
        tan_body(sin, cos),
        asin_body(atan),
        acos_body(asin),
        atan_body(atan),
        atan2_body(atan),
        sinh_body(exp),
        cosh_body(exp),
        tanh_body(exp),
        cbrt_body(exp, log),
        hypot_body(),
    ]
}

fn exp_body(ldexp: u32) -> WasmFunction {
    // (x: f64) -> f64; locals: 1 = k, 2 = r, 3 = poly, 4 = ki.
    let mut f = WasmFunction::new([(3, F64), (1, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(709.782_712_893_384));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(f64::INFINITY));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(-745.133_219_101_941_1));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(0.0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(INV_LN2));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Nearest);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(LN2_HI));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(LN2_LO));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalSet(2));
    horner(&mut f, 2, &exp_coeffs());
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32TruncSatF64S);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::Call(ldexp));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn exp2_body(exp: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(LN2));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::End);
    f
}

fn log_body() -> WasmFunction {
    // (x) -> f64; locals: 1 = k, 2 = m, 3 = r, 4 = z, 5 = poly, 6 = adj.
    let mut f = WasmFunction::new([(1, I32), (4, F64), (1, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Le);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(f64::NAN));
    f.instruction(&Ins::Else);
    f.instruction(&c(f64::NEG_INFINITY));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(DBL_MIN));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(TWO_POW_54));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(0));
    f.instruction(&Ins::I32Const(-54));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::End);
    // k = exponent - 1023 + adj
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64ReinterpretF64);
    f.instruction(&Ins::I64Const(52));
    f.instruction(&Ins::I64ShrU);
    f.instruction(&Ins::I64Const(0x7ff));
    f.instruction(&Ins::I64And);
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::I32Const(1023));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(1));
    // m = mantissa in [1, 2)
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64ReinterpretF64);
    f.instruction(&Ins::I64Const(0xf_ffff_ffff_ffff));
    f.instruction(&Ins::I64And);
    f.instruction(&Ins::I64Const(1023));
    f.instruction(&Ins::I64Const(52));
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::I64Or);
    f.instruction(&Ins::F64ReinterpretI64);
    f.instruction(&Ins::LocalSet(2));
    // normalize
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&c(std::f64::consts::SQRT_2));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&c(0.5));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::End);
    // r = (m-1)/(m+1), z = r*r
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(4));
    horner(&mut f, 4, &log_coeffs());
    f.instruction(&Ins::LocalSet(5));
    // k*ln2 + 2*r*poly
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64ConvertI32S);
    f.instruction(&c(LN2));
    f.instruction(&Ins::F64Mul);
    f.instruction(&c(2.0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn log2_body(log: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(log));
    f.instruction(&c(LOG2E));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f
}

fn log10_body(log: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(log));
    f.instruction(&c(LOG10E));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f
}

/// Emits `sin(r)` for `r` in `[-pi/4, pi/4]` (locals `r`, scratch `3`).
fn sin_poly(f: &mut WasmFunction, r: u32) {
    f.instruction(&Ins::LocalGet(r));
    f.instruction(&Ins::LocalGet(r));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(r));
    horner(f, 3, &sin_coeffs());
    f.instruction(&Ins::F64Mul);
}

fn cos_poly(f: &mut WasmFunction, r: u32) {
    f.instruction(&Ins::LocalGet(r));
    f.instruction(&Ins::LocalGet(r));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(3));
    horner(f, 3, &cos_coeffs());
}

fn sin_body() -> WasmFunction {
    // (x) -> f64; locals: 1 = n, 2 = r, 3 = z, 4 = m.
    let mut f = WasmFunction::new([(3, F64), (1, I32)]);
    reduce(&mut f, 1, 2);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32TruncSatF64S);
    f.instruction(&Ins::I32Const(3));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    sin_poly(&mut f, 2);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    cos_poly(&mut f, 2);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    sin_poly(&mut f, 2);
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Else);
    cos_poly(&mut f, 2);
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn cos_body() -> WasmFunction {
    // (x) -> f64; locals: 1 = n, 2 = r, 3 = z, 4 = m.
    let mut f = WasmFunction::new([(3, F64), (1, I32)]);
    reduce(&mut f, 1, 2);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32TruncSatF64S);
    f.instruction(&Ins::I32Const(3));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    cos_poly(&mut f, 2);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    sin_poly(&mut f, 2);
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    cos_poly(&mut f, 2);
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Else);
    sin_poly(&mut f, 2);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `n = nearest(x * 2/pi)` (local `n`), `r = x - n*pi/2` (local `r`).
fn reduce(f: &mut WasmFunction, n: u32, r: u32) {
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(FRAC_2_PI));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Nearest);
    f.instruction(&Ins::LocalSet(n));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(n));
    f.instruction(&c(PIO2_HI));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalGet(n));
    f.instruction(&c(PIO2_LO));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalSet(r));
}

fn tan_body(sin: u32, cos: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(sin));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(cos));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::End);
    f
}

fn asin_body(atan: u32) -> WasmFunction {
    // (x) -> f64; locals: 1 = ax.
    let mut f = WasmFunction::new([(1, F64)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Abs);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(f64::NAN));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(PIO2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(1.0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::F64Sqrt);
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::Call(atan));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn acos_body(asin: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&c(PIO2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(asin));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::End);
    f
}

fn atan_body(atan: u32) -> WasmFunction {
    // (x) -> f64; locals: 1 = ax, 2 = t, 3 = z, 4 = poly, 5 = r.
    let mut f = WasmFunction::new([(5, F64)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Abs);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&c(PIO2));
    f.instruction(&c(1.0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::Call(atan));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(TAN_3PI8));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&c(FRAC_PI_4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(atan));
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(3));
    horner(&mut f, 3, &atan_coeffs());
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn atan2_body(atan: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::Call(atan));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::Call(atan));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Ge);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(PI));
    f.instruction(&Ins::Else);
    f.instruction(&c(-PI));
    f.instruction(&Ins::End);
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::Else);
    // x == 0
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(PIO2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(-PIO2));
    f.instruction(&Ins::Else);
    f.instruction(&c(0.0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn sinh_body(exp: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::F64Sub);
    f.instruction(&c(0.5));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f
}

fn cosh_body(exp: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::F64Add);
    f.instruction(&c(0.5));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f
}

fn tanh_body(exp: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(1, F64)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(20.0));
    f.instruction(&Ins::F64Gt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(1.0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(-20.0));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(-1.0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(2.0));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&c(1.0));
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn cbrt_body(exp: u32, log: u32) -> WasmFunction {
    // (x) -> f64; locals: 1 = a, 2 = r, 3 = i.
    let mut f = WasmFunction::new([(2, F64), (1, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(0.0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Abs);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(log));
    f.instruction(&c(1.0 / 3.0));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::Call(exp));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    // r = r - (r^3 - a)/(3 r^2)
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::F64Mul);
    f.instruction(&c(3.0));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::F64Neg);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn hypot_body() -> WasmFunction {
    // (x, y) -> f64; locals: 2 = ax, 3 = ay, 4 = m.
    let mut f = WasmFunction::new([(3, F64)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Abs);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Abs);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::F64Max);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&c(0.0));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&c(0.0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::F64Sqrt);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}
