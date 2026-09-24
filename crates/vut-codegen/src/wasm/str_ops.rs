//! Remaining `str` operations for the wasm backend: byte-based `split`/`lines`/
//! `replace`, and Unicode-scalar `char_at`/`chars`/`pad_left`/`pad_right`/
//! `split_whitespace`. Trailing runtime helpers (see `runtime`).
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

use super::rc::KIND_STRING;
use super::runtime::idx;

const I32: ValType = ValType::I32;
const I64: ValType = ValType::I64;

fn m(offset: i64) -> MemArg {
    MemArg {
        offset: u64::try_from(offset).unwrap_or(0),
        align: 0,
        memory_index: 0,
    }
}

/// Signatures, in the trailing [`idx`] order.
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32, I32], vec![I32]),      // SO_SPLIT
        (vec![I32], vec![I32]),           // SO_LINES
        (vec![I32, I32, I32], vec![I32]), // SO_REPLACE
        (vec![I32, I64], vec![I32]),      // SO_CHAR_AT
        (vec![I32], vec![I32]),           // SO_CHARS
        (vec![I32, I64, I32], vec![I32]), // SO_PAD_LEFT
        (vec![I32, I64, I32], vec![I32]), // SO_PAD_RIGHT
        (vec![I32], vec![I32]),           // SO_SPLIT_WS
        (vec![I32], vec![I32]),           // SO_UTF8_LEN
        (vec![I32], vec![I32]),           // SO_SCALAR_COUNT
        (vec![I32], vec![I32]),           // SO_IS_WS
    ]
}

/// Helper bodies in the trailing [`idx`] order.
#[must_use]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let rc_new = base + idx::RC_NEW;
    let list_new = base + idx::LIST_NEW;
    let list_push = base + idx::LIST_PUSH;
    let copy = base + idx::STR_COPY;
    let eq_at = base + idx::STR_EQ_AT;
    let utf8_len = base + idx::SO_UTF8_LEN;
    let scalar_count = base + idx::SO_SCALAR_COUNT;
    let is_ws = base + idx::SO_IS_WS;
    vec![
        split_body(list_new, list_push, copy, eq_at),
        lines_body(list_new, list_push, copy),
        replace_body(rc_new, eq_at),
        char_at_body(rc_new, copy, utf8_len, scalar_count),
        chars_body(list_new, list_push, copy, utf8_len, scalar_count),
        pad_left_body(rc_new, scalar_count),
        pad_right_body(rc_new, scalar_count),
        split_ws_body(list_new, list_push, copy, utf8_len, is_ws),
        utf8_len_body(),
        scalar_count_body(utf8_len),
        is_ws_body(),
    ]
}

/// `(h, sep) -> list`: byte-based split.
fn split_body(list_new: u32, list_push: u32, copy: u32, eq_at: u32) -> WasmFunction {
    // locals: 2 = hlen, 3 = slen, 4 = out, 5 = i, 6 = start, 7 = scratch.
    let mut f = WasmFunction::new([(6, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(KIND_STRING));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Call(list_new));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    push_segment(&mut f, copy, list_push, 4, 7, 6, 2);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    push_segment(&mut f, copy, list_push, 4, 7, 6, 5);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    push_segment(&mut f, copy, list_push, 4, 7, 6, 2);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::End);
    f
}

/// `(h) -> list`: split on `\n`, dropping a trailing `\r` per line.
fn lines_body(list_new: u32, list_push: u32, copy: u32) -> WasmFunction {
    // locals: 1 = len, 2 = out, 3 = i, 4 = start, 7 = scratch.
    let mut f = WasmFunction::new([(7, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(KIND_STRING));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Call(list_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    push_byte(&mut f, 0, 3);
    f.instruction(&Ins::I32Const(10));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalSet(5));
    push_line(&mut f, copy, list_push, 2, 7, 4, 5);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalSet(5));
    push_line(&mut f, copy, list_push, 2, 7, 4, 5);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// `(h, from, to) -> str`: replace all non-overlapping occurrences.
#[expect(clippy::too_many_lines, reason = "two-pass replace over raw bytes")]
fn replace_body(rc_new: u32, eq_at: u32) -> WasmFunction {
    // locals: 3 = hlen, 4 = flen, 5 = tlen, 6 = i, 7 = out_len, 8 = out, 9 = cursor.
    let mut f = WasmFunction::new([(7, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(3));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(4));
    push_len(&mut f, 2);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::End);
    f
}

/// `(h, idx) -> str`: the `idx`-th Unicode scalar, or `""` when out of range.
fn char_at_body(rc_new: u32, copy: u32, utf8_len: u32, scalar_count: u32) -> WasmFunction {
    // locals: 2 = count, 3 = off, 4 = k, 5 = ch_len, 6 = scratch.
    let mut f = WasmFunction::new([(5, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(scalar_count));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64LtS);
    f.instruction(&Ins::If(BlockType::Empty));
    empty_str(&mut f, rc_new, 6);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64GeS);
    f.instruction(&Ins::If(BlockType::Empty));
    empty_str(&mut f, rc_new, 6);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    push_at(&mut f, 0, 3);
    f.instruction(&Ins::Call(utf8_len));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    push_at(&mut f, 0, 3);
    f.instruction(&Ins::Call(utf8_len));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::End);
    f
}

fn chars_body(
    list_new: u32,
    list_push: u32,
    copy: u32,
    utf8_len: u32,
    scalar_count: u32,
) -> WasmFunction {
    // locals: 1 = len, 2 = count, 3 = out, 4 = off, 5 = i, 6 = ch_len, 7 = scratch.
    let mut f = WasmFunction::new([(7, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(scalar_count));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(KIND_STRING));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(list_new));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    push_at(&mut f, 0, 4);
    f.instruction(&Ins::Call(utf8_len));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::Call(copy));
    list_into(&mut f, list_push, 3, 7);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

fn pad_left_body(rc_new: u32, scalar_count: u32) -> WasmFunction {
    // locals: 3 = count, 4 = hlen, 5 = flen, 6 = pad, 7 = need, 8 = out, 9 = i.
    let mut f = WasmFunction::new([(7, ValType::I32)]);
    emit_pad(&mut f, rc_new, scalar_count, true);
    f
}

fn pad_right_body(rc_new: u32, scalar_count: u32) -> WasmFunction {
    // locals: 3 = count, 4 = hlen, 5 = flen, 6 = pad, 7 = need, 8 = out, 9 = i.
    let mut f = WasmFunction::new([(7, ValType::I32)]);
    emit_pad(&mut f, rc_new, scalar_count, false);
    f
}

#[expect(clippy::too_many_lines, reason = "shared left/right padding emission")]
fn emit_pad(f: &mut WasmFunction, rc_new: u32, scalar_count: u32, left: bool) {
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(scalar_count));
    f.instruction(&Ins::LocalSet(3));
    push_len(f, 0);
    f.instruction(&Ins::LocalSet(4));
    push_len(f, 2);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Store(m(0)));
    // Copy the value either before (right) or after (left) the padding.
    let value_at = if left {
        // value after pad: dst = out+4+pad*flen
        f.instruction(&Ins::LocalGet(8));
        f.instruction(&Ins::I32Const(4));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::LocalGet(6));
        f.instruction(&Ins::LocalGet(5));
        f.instruction(&Ins::I32Mul);
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::LocalGet(0));
        f.instruction(&Ins::I32Const(4));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::LocalGet(4));
        f.instruction(&Ins::MemoryCopy {
            src_mem: 0,
            dst_mem: 0,
        });
        0
    } else {
        // value first
        f.instruction(&Ins::LocalGet(8));
        f.instruction(&Ins::I32Const(4));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::LocalGet(0));
        f.instruction(&Ins::I32Const(4));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::LocalGet(4));
        f.instruction(&Ins::MemoryCopy {
            src_mem: 0,
            dst_mem: 0,
        });
        f.instruction(&Ins::LocalGet(4));
        f.instruction(&Ins::LocalSet(9));
        4
    };
    let _ = value_at;
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(9));
    // pad loop
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    if left {
        f.instruction(&Ins::LocalGet(9));
        f.instruction(&Ins::LocalGet(5));
        f.instruction(&Ins::I32Mul);
    } else {
        f.instruction(&Ins::LocalGet(4));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::LocalGet(9));
        f.instruction(&Ins::LocalGet(5));
        f.instruction(&Ins::I32Mul);
    }
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::End);
}

fn split_ws_body(
    list_new: u32,
    list_push: u32,
    copy: u32,
    utf8_len: u32,
    is_ws: u32,
) -> WasmFunction {
    // locals: 1 = len, 2 = out, 3 = i, 4 = start, 5 = ch, 6 = in_run, 7 = scratch.
    let mut f = WasmFunction::new([(7, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(KIND_STRING));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Call(list_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    push_at(&mut f, 0, 3);
    f.instruction(&Ins::Call(is_ws));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::If(BlockType::Empty));
    push_segment(&mut f, copy, list_push, 2, 7, 4, 3);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    push_at(&mut f, 0, 3);
    f.instruction(&Ins::Call(utf8_len));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::If(BlockType::Empty));
    push_segment(&mut f, copy, list_push, 2, 7, 4, 1);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// `(ptr: i32) -> i32`: UTF-8 sequence length from the lead byte.
fn utf8_len_body() -> WasmFunction {
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(128));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(224));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(192));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(240));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(224));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(3));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(h) -> i32`: number of Unicode scalars.
fn scalar_count_body(utf8_len: u32) -> WasmFunction {
    // locals: 1 = len, 2 = off, 3 = count.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    push_at(&mut f, 0, 2);
    f.instruction(&Ins::Call(utf8_len));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

/// `(ptr: i32) -> i32`: whether the UTF-8 scalar at `ptr` is Unicode `White_Space`.
fn is_ws_body() -> WasmFunction {
    // locals: 1 = lead, 2 = cp.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(9));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(10));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(11));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(12));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(13));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(32));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(194));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(1)));
    f.instruction(&Ins::I32Const(133));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(1)));
    f.instruction(&Ins::I32Const(160));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::Else);
    decode_cp(&mut f);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x1680));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x2000));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x200A));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x2028));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x2029));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x202F));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x205F));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0x3000));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn decode_cp(f: &mut WasmFunction) {
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(224));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(192));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(31));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(6));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(1)));
    f.instruction(&Ins::I32Const(63));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(240));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(224));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(15));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(12));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(1)));
    f.instruction(&Ins::I32Const(63));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Const(6));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load8U(m(2)));
    f.instruction(&Ins::I32Const(63));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(2));
}

fn empty_str(f: &mut WasmFunction, rc_new: u32, scratch: u32) {
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalTee(scratch));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(scratch));
    f.instruction(&Ins::Return);
}

/// Copies `[start, end)` of `local0` and pushes the resulting string into the
/// list at `out_local` (transferring ownership).
fn push_segment(
    f: &mut WasmFunction,
    copy: u32,
    list_push: u32,
    out_local: u32,
    scratch: u32,
    start: u32,
    end: u32,
) {
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(start));
    f.instruction(&Ins::LocalGet(end));
    f.instruction(&Ins::LocalGet(start));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(copy));
    list_into(f, list_push, out_local, scratch);
}

/// Pushes a `\n`-delimited line `[start, end)` (trimming a trailing `\r`).
fn push_line(
    f: &mut WasmFunction,
    copy: u32,
    list_push: u32,
    out_local: u32,
    scratch: u32,
    start: u32,
    end: u32,
) {
    f.instruction(&Ins::LocalGet(end));
    f.instruction(&Ins::LocalGet(start));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(end));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(13));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(end));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(end));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    push_segment(f, copy, list_push, out_local, scratch, start, end);
}

/// Pushes the i32 handle on the stack into the list at `out_local`.
fn list_into(f: &mut WasmFunction, list_push: u32, out_local: u32, scratch: u32) {
    f.instruction(&Ins::LocalSet(scratch));
    f.instruction(&Ins::LocalGet(out_local));
    f.instruction(&Ins::LocalGet(scratch));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::Call(list_push));
}

fn push_at(f: &mut WasmFunction, base: u32, offset: u32) {
    f.instruction(&Ins::LocalGet(base));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(offset));
    f.instruction(&Ins::I32Add);
}

fn push_len(f: &mut WasmFunction, local: u32) {
    f.instruction(&Ins::LocalGet(local));
    f.instruction(&Ins::I32Load(m(0)));
}

fn push_byte(f: &mut WasmFunction, base: u32, offset: u32) {
    push_at(f, base, offset);
    f.instruction(&Ins::I32Load8U(m(0)));
}
