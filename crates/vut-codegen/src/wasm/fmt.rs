//! `float` → string formatting for the wasm backend.
//!
//! Fixed-point decimal: the integer part and a six-digit fractional part are
//! each formatted with the shared i64 formatter, then joined with a `.` and
//! trailing zeros trimmed. No exponent notation, matching `f64::to_string` for
//! values whose fraction needs at most six digits.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

use super::runtime::idx;

const I32: ValType = ValType::I32;
const I64: ValType = ValType::I64;
const F64: ValType = ValType::F64;

fn m(offset: i64) -> MemArg {
    MemArg {
        offset: u64::try_from(offset).unwrap_or(0),
        align: 0,
        memory_index: 0,
    }
}

/// Signature: `(v: f64) -> str`.
#[must_use]
pub(crate) fn signature() -> (Vec<ValType>, Vec<ValType>) {
    (vec![F64], vec![I32])
}

/// Helper body.
#[must_use]
pub(crate) fn body(base: u32) -> WasmFunction {
    float_body(base + idx::RC_NEW, base + idx::FORMAT)
}

#[expect(
    clippy::too_many_lines,
    reason = "one linear pass over the fixed-point construction"
)]
fn float_body(rc_new: u32, format: u32) -> WasmFunction {
    // 1 = a(f64), 2 = neg, 3 = ip(i64), 4 = frac(f64), 5 = fs(i64),
    // 6..14 = ip_str, len_i, fs_str, len_f, out, cursor, total, lead, k.
    let mut f = WasmFunction::new([(1, F64), (1, I32), (1, I64), (1, F64), (1, I64), (9, I32)]);
    // NaN / ±inf
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    make(&mut f, rc_new, b"NaN");
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Const(f64::INFINITY.into()));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    make(&mut f, rc_new, b"inf");
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Const(f64::NEG_INFINITY.into()));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    make(&mut f, rc_new, b"-inf");
    f.instruction(&Ins::End);
    // neg, a
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64ReinterpretF64);
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64LtS);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Abs);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64Const(0.0.into()));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::If(BlockType::Empty));
    make(&mut f, rc_new, b"-0");
    f.instruction(&Ins::End);
    make(&mut f, rc_new, b"0");
    f.instruction(&Ins::End);
    // ip = trunc(a)
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64TruncSatF64S);
    f.instruction(&Ins::LocalSet(3));
    // frac = clamp(a - ip, 0, 1)
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::F64ConvertI64S);
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Const(0.0.into()));
    f.instruction(&Ins::F64Lt);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Const(1.0.into()));
    f.instruction(&Ins::F64Ge);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::F64Const(0.0.into()));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::End);
    // fs = round(frac * 1e6)
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Const(1_000_000.0.into()));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::F64Const(0.5.into()));
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::I64TruncSatF64S);
    f.instruction(&Ins::LocalSet(5));
    // carry: if fs >= 1e6 { ip += 1; fs -= 1e6 }
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I64Const(1_000_000));
    f.instruction(&Ins::I64GeS);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::I64Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I64Const(1_000_000));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::End);
    // ip_str (after carry)
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(format));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    // fs_str + trim
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::Call(format));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(9));
    // lead = len_f >= 6 ? 0 : 6 - len_f
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Const(6));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(6));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(13));
    // k = len_f; strip trailing zeros
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::LocalSet(14));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(14));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // total = neg + len_i + (k>0 ? 1 + lead + k : 0)
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(13));
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(12));
    // out
    f.instruction(&Ins::LocalGet(12));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(10));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(12));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::LocalSet(11));
    // sign
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(45));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(11));
    f.instruction(&Ins::End);
    // integer digits
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(11));
    // fraction: '.' then lead zeros then fs_str[0..k]
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(46));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(11));
    // lead zeros
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::LocalGet(13));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(11));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // fs_str[0..k]
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(14));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::End);
    f
}

/// Emits and returns a string from `bytes`.
fn make(f: &mut WasmFunction, rc_new: u32, bytes: &[u8]) {
    f.instruction(&Ins::I32Const(bytes.len() as i32 + 4));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalTee(10));
    f.instruction(&Ins::I32Const(bytes.len() as i32));
    f.instruction(&Ins::I32Store(m(0)));
    for (index, byte) in bytes.iter().enumerate() {
        f.instruction(&Ins::LocalGet(10));
        f.instruction(&Ins::I32Const(index as i32 + 4));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::I32Const(i32::from(*byte)));
        f.instruction(&Ins::I32Store8(m(0)));
    }
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::Return);
}
