//! String runtime helpers for the wasm backend.
//!
//! A string handle points at `[len: i32][utf-8 bytes]`, preceded by the shared
//! eight-byte reference-count prefix (see `rc`). `find`/`rfind`/`substring` use
//! **byte** indices per `specs/collections.md`; `trim` and case conversion here
//! are ASCII (Unicode case/whitespace tables are a later C3 increment).
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

/// Signatures in [`idx`] order.
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32, I32, I32], vec![I32]), // STR_COPY
        (vec![I32, I32, I32], vec![I32]), // STR_EQ_AT
        (vec![I32], vec![I32]),           // STR_IS_WS
        (vec![I32, I32], vec![I32]),      // STR_EQ
        (vec![I32, I32], vec![I64]),      // STR_CMP
        (vec![I32, I32], vec![I32]),      // STR_CONTAINS
        (vec![I32, I32], vec![I32]),      // STR_STARTS
        (vec![I32, I32], vec![I32]),      // STR_ENDS
        (vec![I32, I32], vec![I64]),      // STR_FIND
        (vec![I32, I32], vec![I64]),      // STR_RFIND
        (vec![I32, I32, I32], vec![I32]), // STR_SUBSTRING
        (vec![I32, I32], vec![I32]),      // STR_REPEAT
        (vec![I32, I32], vec![I32]),      // STR_STRIP_PREFIX
        (vec![I32, I32], vec![I32]),      // STR_STRIP_SUFFIX
        (vec![I32], vec![I32]),           // STR_TRIM_START
        (vec![I32], vec![I32]),           // STR_TRIM_END
        (vec![I32], vec![I32]),           // STR_LOWER
        (vec![I32], vec![I32]),           // STR_UPPER
    ]
}

/// Helper bodies in [`idx`] order.
#[must_use]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let rc_new = base + idx::RC_NEW;
    let copy = base + idx::STR_COPY;
    let eq_at = base + idx::STR_EQ_AT;
    let is_ws = base + idx::SO_IS_WS;
    vec![
        copy_body(rc_new),
        eq_at_body(),
        is_ws_body(),
        eq_body(eq_at),
        cmp_body(),
        contains_body(eq_at),
        starts_body(eq_at),
        ends_body(eq_at),
        find_body(eq_at),
        rfind_body(eq_at),
        substring_body(copy),
        repeat_body(rc_new),
        strip_prefix_body(eq_at, copy),
        strip_suffix_body(eq_at, copy),
        trim_start_body(copy, is_ws),
        trim_end_body(copy, is_ws),
        map_bytes_body(rc_new, 32),
        map_bytes_body(rc_new, -32),
    ]
}

/// `(b: i32) -> i32`: whether `b` is ASCII whitespace.
fn is_ws_body() -> WasmFunction {
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    for byte in [32, 9, 10, 13, 12, 11] {
        f.instruction(&Ins::LocalGet(0));
        f.instruction(&Ins::I32Const(byte));
        f.instruction(&Ins::I32Eq);
        f.instruction(&Ins::If(BlockType::Empty));
        f.instruction(&Ins::I32Const(1));
        f.instruction(&Ins::Return);
        f.instruction(&Ins::End);
    }
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f
}

/// `(haystack, needle) -> i32`: substring presence.
fn contains_body(eq_at: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = nl, 4 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f
}

/// `(src, start, len) -> handle`: copies `len` bytes of `src` from `start`.
fn copy_body(rc_new: u32) -> WasmFunction {
    // locals: 3 = dest, 4 = i.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

/// `(haystack, offset, needle) -> i32`: whether `needle` matches at `offset`.
/// Assumes `offset + len(needle) <= len(haystack)`.
fn eq_at_body() -> WasmFunction {
    // locals: 3 = i, 4 = nlen.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::End);
    f
}

/// `(a, b) -> i32`: byte equality.
fn eq_body(eq_at: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    // lengths differ -> 0
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::End);
    f
}

/// `(a, b) -> i64`: lexicographic comparison (-1, 0, 1).
fn cmp_body() -> WasmFunction {
    // locals: 2 = la, 3 = lb, 4 = i, 5 = n.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    // x, y
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::I64Const(-1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // lengths decide
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::I64Const(-1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(haystack, needle) -> i64`: first byte index, or -1.
fn find_body(eq_at: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = nl, 4 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I64Const(-1));
    f.instruction(&Ins::End);
    f
}

/// `(haystack, needle) -> i64`: first byte index, or -1.
fn starts_body(eq_at: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = nl.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::End);
    f
}

fn ends_body(eq_at: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = nl.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::End);
    f
}

fn rfind_body(eq_at: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = nl, 4 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    push_len(&mut f, 0);
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    // i = hl - nl (or -1 if nl > hl)
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(-1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I64Const(-1));
    f.instruction(&Ins::End);
    f
}

/// `(s, start, end) -> handle`: byte-range slice, clamped.
fn substring_body(copy: u32) -> WasmFunction {
    // locals: 3 = len, 4 = lo, 5 = hi.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(3));
    // lo = clamp(start, 0, len)
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(4));
    // hi = clamp(end, lo, len)
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::End);
    f
}

/// `(s, n) -> handle`: repeat the string `n` times (n >= 0).
fn repeat_body(rc_new: u32) -> WasmFunction {
    // locals: 2 = slen, 3 = dest, 4 = i, 5 = j.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

/// `(s, prefix) -> handle`: `s` without `prefix`, or a copy of `s`.
fn strip_prefix_body(eq_at: u32, copy: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = pl.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::End);
    f
}

fn strip_suffix_body(eq_at: u32, copy: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = sl.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    push_len(&mut f, 1);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(eq_at));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::End);
    f
}

/// `(s) -> handle`: drop leading ASCII whitespace.
fn trim_start_body(copy: u32, is_ws: u32) -> WasmFunction {
    // locals: 2 = len, 3 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(is_ws));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // copy(s, i, len - i)
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::End);
    f
}

/// `(s) -> handle`: drop trailing ASCII whitespace.
fn trim_end_body(copy: u32, is_ws: u32) -> WasmFunction {
    // locals: 2 = len, 3 = end.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(is_ws));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(copy));
    f.instruction(&Ins::End);
    f
}

/// `(s, delta) -> handle`: copy with each ASCII letter shifted (`delta` = ±32).
fn map_bytes_body(rc_new: u32, delta: i32) -> WasmFunction {
    // locals: 2 = len, 3 = dest, 4 = i, 5 = b.
    let mut f = WasmFunction::new([(5, ValType::I32)]);
    push_len(&mut f, 0);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(5));
    // if 'A'..='Z' (for lower) or 'a'..='z' (for upper) shift
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(if delta == 32 { 65 } else { 97 }));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(if delta == 32 { 90 } else { 122 }));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(delta));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

fn push_len(f: &mut WasmFunction, local: u32) {
    f.instruction(&Ins::LocalGet(local));
    f.instruction(&Ins::I32Load(m(0)));
}

fn push_byte(f: &mut WasmFunction, base: u32, offset: u32) {
    f.instruction(&Ins::LocalGet(base));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(offset));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
}

/// Signatures for the trailing helpers (appended after the other modules).
#[must_use]
pub(crate) fn extra_signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32], vec![I64]), // STR_TO_I64
        (vec![I32], vec![F64]), // STR_TO_F64
    ]
}

/// `base` is the runtime base; both helpers take `is_ws` = `STR_IS_WS`.
#[must_use]
pub(crate) fn extra_bodies(base: u32) -> Vec<WasmFunction> {
    let is_ws = base + idx::SO_IS_WS;
    vec![to_i64_body(is_ws), to_f64_body(is_ws)]
}

/// Locals used by the parsers: 1 = len, 2 = start, 3 = end.
fn trim_bounds(f: &mut WasmFunction, is_ws: u32) {
    // end = len
    push_len(f, 0);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalSet(3));
    // start = 0
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(2));
    // while start < len && ws(byte[start]) start++
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(is_ws));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // while end > start && ws(byte[end-1]) end--
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(is_ws));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
}

#[expect(clippy::too_many_lines, reason = "one branch per parse step")]
fn to_i64_body(is_ws: u32) -> WasmFunction {
    // locals: 1 = len, 2 = start, 3 = end, 4 = neg, 5 = acc(i64), 6 = b, 7 = d(i64), 8 = overflow.
    let mut f = WasmFunction::new([
        (4, ValType::I32),
        (1, ValType::I64),
        (1, ValType::I32),
        (1, ValType::I64),
        (1, ValType::I32),
    ]);
    trim_bounds(&mut f, is_ws);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    push_byte(&mut f, 0, 2);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(45));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(43));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    push_byte(&mut f, 0, 2);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(57));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::LocalSet(7));
    // overflow: acc > (MAX - d)/10
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I64Const(9_223_372_036_854_775_807));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::I64Const(10));
    f.instruction(&Ins::I64DivS);
    f.instruction(&Ins::I64GtS);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::Br(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I64Const(10));
    f.instruction(&Ins::I64Mul);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I64Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

#[expect(clippy::too_many_lines, reason = "one branch per numeric parse step")]
fn to_f64_body(is_ws: u32) -> WasmFunction {
    // locals: 1 = len, 2 = start, 3 = end, 4 = neg, 5 = acc(f64),
    // 6 = scale(f64), 7 = b(i32), 8 = exp(i64), 9 = esign(i32), 10 = digit(i64).
    let mut f = WasmFunction::new([
        (3, ValType::I32),
        (1, ValType::I32),
        (2, ValType::F64),
        (1, ValType::I32),
        (1, ValType::I64),
        (1, ValType::I32),
        (1, ValType::I64),
    ]);
    trim_bounds(&mut f, is_ws);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    push_byte(&mut f, 0, 2);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(45));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(43));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::F64Const(0.0.into()));
    f.instruction(&Ins::LocalSet(5));
    // integer digits
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    digit_or_break(&mut f);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::LocalSet(10));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Const(10.0.into()));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::F64ConvertI64S);
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // fraction
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(46));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::F64Const(0.1.into()));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    digit_or_break(&mut f);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::F64ConvertI64S);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::F64Const(10.0.into()));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // exponent
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(101));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(69));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(9));
    push_byte(&mut f, 0, 2);
    f.instruction(&Ins::I32Const(45));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Else);
    push_byte(&mut f, 0, 2);
    f.instruction(&Ins::I32Const(43));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    digit_or_break(&mut f);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::LocalSet(10));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I64Const(10));
    f.instruction(&Ins::I64Mul);
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::I64Add);
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::End);
    // scale by 10^exp
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I64Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64GtS);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Const(10.0.into()));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Const(10.0.into()));
    f.instruction(&Ins::F64Div);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::I64Add);
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::If(BlockType::Result(ValType::F64)));
    f.instruction(&Ins::F64Const(0.0.into()));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::F64Sub);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// Leaves `1` on the stack (branch out) when `byte[start]` (local 2) is not a digit.
fn digit_or_break(f: &mut WasmFunction) {
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(57));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::I32Or);
}
