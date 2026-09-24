//! `list` runtime helpers for the wasm backend.
//!
//! A list handle points at `[len: i32][cap: i32][elem_kind: i32][data: i32]`
//! (16-byte payload) after the shared eight-byte reference-count prefix. Elements
//! occupy eight-byte slots.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

use super::rc::{KIND_BYTES, KIND_FLOAT, KIND_STRING};
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

/// Signatures in [`idx`] order.
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32, I32], vec![]),         // LS_ENSURE
        (vec![I32], vec![I32]),           // LS_CAPACITY
        (vec![I32, I32], vec![]),         // LS_RESERVE
        (vec![I32, I32], vec![]),         // LS_TRUNCATE
        (vec![I32, I32, I32], vec![]),    // LS_SWAP
        (vec![I32], vec![]),              // LS_REVERSE
        (vec![I32], vec![]),              // LS_SHRINK
        (vec![I32], vec![I64]),           // LS_FIRST
        (vec![I32], vec![I64]),           // LS_LAST
        (vec![I32, I32, I64], vec![]),    // LS_INSERT
        (vec![I32, I32], vec![I64]),      // LS_REMOVE
        (vec![I32, I32], vec![]),         // LS_EXTEND
        (vec![I32, I32, I32], vec![I32]), // LS_SLICE
        (vec![I32, I64], vec![I32]),      // LS_CONTAINS
        (vec![I32, I64], vec![I64]),      // LS_FIND
        (vec![I32, I64, I32], vec![I32]), // LS_ELEM_EQ
        (vec![I32, I32], vec![I32]),      // LS_JOIN
    ]
}

/// Helper bodies in [`idx`] order.
#[must_use]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let alloc = base + idx::ALLOC;
    let rc_new = base + idx::RC_NEW;
    let elem_retain = base + idx::ELEM_RETAIN;
    let ensure = base + idx::LS_ENSURE;
    let elem_eq = base + idx::LS_ELEM_EQ;
    let str_eq = base + idx::STR_EQ;
    let bt_cmp = base + idx::BT_CMP;
    vec![
        ensure_body(alloc),
        capacity_body(),
        reserve_body(alloc),
        truncate_body(),
        swap_body(),
        reverse_body(),
        shrink_body(alloc),
        first_body(),
        last_body(),
        insert_body(ensure),
        remove_body(),
        extend_body(ensure, elem_retain),
        slice_body(rc_new, alloc, elem_retain),
        contains_body(elem_eq),
        find_body(elem_eq),
        elem_eq_body(str_eq, bt_cmp),
        join_body(rc_new),
    ]
}

/// `(h, needed) -> ()`: grows so the list holds at least `needed` elements.
fn ensure_body(alloc: u32) -> WasmFunction {
    // locals: 2 = cap, 3 = len, 4 = new_cap, 5 = new_data.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::End);
    f
}

fn capacity_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::End);
    f
}

/// `(h, n) -> ()`: grow so capacity is at least `n` (exact, matching native).
fn reserve_body(alloc: u32) -> WasmFunction {
    // locals: 2 = len, 3 = new_data.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::End);
    f
}

fn truncate_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(h, i, j) -> ()`: swap two 8-byte slots (bounds-checked).
fn swap_body() -> WasmFunction {
    // locals: 3 = tmp (i64).
    let mut f = WasmFunction::new([(1, ValType::I64)]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::I32Or);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Unreachable);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    // a[i] = a[j]
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Store(m(0)));
    // a[j] = tmp
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::End);
    f
}

fn reverse_body() -> WasmFunction {
    // locals: 1 = i, 2 = j.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    // swap slots i and j inline
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Store(m(0)));
    // load i into scratch local 3? reuse: recompute
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn shrink_body(alloc: u32) -> WasmFunction {
    // locals: 1 = len, 2 = new_data.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn first_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Unreachable);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::End);
    f
}

fn last_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Unreachable);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::End);
    f
}

/// `(h, i, v: i64) -> ()`: insert before index `i`.
fn insert_body(ensure: u32) -> WasmFunction {
    // locals: 3 = len, 4 = j.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ensure));
    // j = len; while j > i: a[j] = a[j-1]; j--
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

/// `(h, i) -> i64`: remove and return the element at `i`.
fn remove_body() -> WasmFunction {
    // locals: 2 = len, 3 = v (i64), 4 = j.
    let mut f = WasmFunction::new([(1, ValType::I32), (1, ValType::I64), (1, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Unreachable);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

/// `(h, other) -> ()`: append every element of `other`, retaining managed ones.
fn extend_body(ensure: u32, elem_retain: u32) -> WasmFunction {
    // locals: 2 = len, 3 = olen, 4 = base, 5 = k.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ensure));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(elem_retain));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

/// `(h, start, end) -> list`: clamped range copy with managed elements retained.
fn slice_body(rc_new: u32, alloc: u32, elem_retain: u32) -> WasmFunction {
    // locals: 3 = len, 4 = lo, 5 = hi, 6 = out, 7 = i.
    let mut f = WasmFunction::new([(5, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    clamp(&mut f, 1, 3, 4);
    clamp(&mut f, 2, 3, 5);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::I32Store(m(8)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    // retain copied managed elements
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(elem_retain));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::End);
    f
}

fn contains_body(elem_eq: u32) -> WasmFunction {
    // locals: 2 = len, 3 = kind, 4 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(elem_eq));
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

fn find_body(elem_eq: u32) -> WasmFunction {
    // locals: 2 = len, 3 = kind, 4 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(elem_eq));
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

/// `(slot: i32, value: i64, kind: i32) -> i32`: element equality by kind.
fn elem_eq_body(str_eq: u32, bt_cmp: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(KIND_FLOAT));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Load(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::F64ReinterpretI64);
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(KIND_STRING));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::Call(str_eq));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(KIND_BYTES));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::Call(bt_cmp));
    f.instruction(&Ins::I64Eqz);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Eq);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(h, sep) -> str`: concatenate `list[str]` elements with `sep`.
#[expect(
    clippy::too_many_lines,
    reason = "join builds the result in two passes"
)]
fn join_body(rc_new: u32) -> WasmFunction {
    // locals: 2 = len, 3 = total, 4 = i, 5 = out, 6 = cursor, 7 = elem, 8 = n, 9 = slen.
    let mut f = WasmFunction::new([(8, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // add separators
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(8));
    // copy element bytes to out+4+cursor
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    // separator if not last
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f
}

/// `local = clamp(local, 0, len_local)` (signed compares).
fn clamp(f: &mut WasmFunction, src: u32, len: u32, dest: u32) {
    f.instruction(&Ins::LocalGet(src));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(src));
    f.instruction(&Ins::LocalGet(len));
    f.instruction(&Ins::I32GtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(len));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(src));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(dest));
}
