//! `bytes` runtime helpers for the wasm backend.
//!
//! A bytes handle points at `[len: i32][cap: i32][unused: i32][data: i32]`
//! (16-byte payload) after the shared eight-byte reference-count prefix.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

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
        (vec![I32, I32], vec![]),         // BT_ENSURE
        (vec![I32], vec![]),              // BT_CLEAR
        (vec![I32], vec![I32]),           // BT_CAPACITY
        (vec![I32, I32], vec![]),         // BT_RESERVE
        (vec![I32, I32], vec![]),         // BT_PUSH
        (vec![I32, I32], vec![]),         // BT_EXTEND
        (vec![I32, I32], vec![]),         // BT_TRUNCATE
        (vec![I32, I32, I32], vec![]),    // BT_RESIZE
        (vec![I32], vec![I32]),           // BT_FIRST
        (vec![I32], vec![I32]),           // BT_LAST
        (vec![I32, I32, I32], vec![I32]), // BT_SLICE
        (vec![I32, I32], vec![I64]),      // BT_FIND
        (vec![I32, I32], vec![I32]),      // BT_STARTS
        (vec![I32, I32], vec![I32]),      // BT_ENDS
        (vec![I32, I32], vec![I64]),      // BT_CMP
        (vec![I32, I32, I32], vec![I32]), // BT_MEMCMP
    ]
}

/// Helper bodies in [`idx`] order.
#[must_use]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let alloc = base + idx::ALLOC;
    let rc_new = base + idx::RC_NEW;
    let ensure = base + idx::BT_ENSURE;
    let mem_cmp = base + idx::BT_MEMCMP;
    vec![
        ensure_body(alloc),
        clear_body(),
        capacity_body(),
        reserve_body(ensure),
        push_body(ensure),
        extend_body(ensure),
        truncate_body(),
        resize_body(ensure),
        first_body(),
        last_body(),
        slice_body(rc_new, alloc),
        find_body(),
        starts_body(mem_cmp),
        ends_body(mem_cmp),
        cmp_body(mem_cmp),
        mem_cmp_body(),
    ]
}

/// Signatures for the trailing bytes helpers (appended after the other modules).
#[must_use]
pub(crate) fn extra_signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32], vec![I32]),                  // BT_TO_HEX
        (vec![I32], vec![I32]),                  // BT_TO_LIST
        (vec![I32], vec![I32]),                  // BT_FROM_LIST
        (vec![I32, I32, I32, I32], vec![I64]),   // BT_READ_INT
        (vec![I32, I32, I32, I64, I32], vec![]), // BT_WRITE_INT
    ]
}

/// Helper bodies in the trailing [`idx`] order.
#[must_use]
pub(crate) fn extra_bodies(base: u32) -> Vec<WasmFunction> {
    let rc_new = base + idx::RC_NEW;
    let list_new = base + idx::LIST_NEW;
    let list_push = base + idx::LIST_PUSH;
    let bytes_new = base + idx::BYTES_NEW;
    vec![
        to_hex_body(rc_new),
        to_list_body(list_new, list_push),
        from_list_body(bytes_new),
        read_int_body(),
        write_int_body(),
    ]
}

/// `(h) -> str`: lowercase hex of every byte.
fn to_hex_body(rc_new: u32) -> WasmFunction {
    // locals: 1 = len, 2 = out, 3 = i, 4 = b, 5 = nibble.
    let mut f = WasmFunction::new([(5, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(4));
    // high nibble
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    hex_digit(&mut f, true);
    f.instruction(&Ins::I32Store8(m(0)));
    // low nibble
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    hex_digit(&mut f, false);
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// Leaves the hex ASCII code for the high or low nibble of `local4`.
fn hex_digit(f: &mut WasmFunction, high: bool) {
    if high {
        f.instruction(&Ins::LocalGet(4));
        f.instruction(&Ins::I32Const(4));
        f.instruction(&Ins::I32ShrU);
    } else {
        f.instruction(&Ins::LocalGet(4));
        f.instruction(&Ins::I32Const(15));
        f.instruction(&Ins::I32And);
    }
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(10));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(87));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::End);
}

/// `(h) -> list`: each byte as a `u8` element.
fn to_list_body(list_new: u32, list_push: u32) -> WasmFunction {
    // locals: 1 = len, 2 = out, 3 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(list_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::Call(list_push));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// `(list) -> bytes`: low byte of every element.
fn from_list_body(bytes_new: u32) -> WasmFunction {
    // locals: 1 = len, 2 = out, 3 = i.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(bytes_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// `(h, offset, width, flags) -> i64`; flags bit0 = big-endian, bit1 = signed.
fn read_int_body() -> WasmFunction {
    // locals: 4 = value(i64), 5 = k, 6 = big, 7 = bits.
    let mut f = WasmFunction::new([(1, ValType::I64), (3, ValType::I32)]);
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::If(BlockType::Result(I64)));
    // value = (value << 8) | byte[offset + k]
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64Const(8));
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64Or);
    f.instruction(&Ins::Else);
    // value |= byte[offset + k] << (8 * k)
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64Or);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    // sign extend when requested and width < 8
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64Const(64));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::I64Const(64));
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::I64ShrS);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::End);
    f
}

/// `(h, offset, width, value, flags) -> ()`; flags bit0 = big-endian.
fn write_int_body() -> WasmFunction {
    // locals: 5 = k, 6 = v(i64), 7 = big.
    let mut f = WasmFunction::new([(1, ValType::I32), (1, ValType::I64), (1, ValType::I32)]);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    // address
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I64Const(8));
    f.instruction(&Ins::I64ShrU);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(a: i32, b: i32, n: i32) -> i32`: first differing byte as -1/1, else 0.
fn mem_cmp_body() -> WasmFunction {
    // locals: 3 = i, 4 = x, 5 = y.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(-1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f
}

/// `(h, needed) -> ()`: grows the data buffer so it holds at least `needed`.
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
    // new_cap = cap == 0 ? 4 : cap * 2, then double until >= needed
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
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(5));
    // copy old data
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
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

fn clear_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0)));
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

fn reserve_body(ensure: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(ensure));
    f.instruction(&Ins::End);
    f
}

fn push_body(ensure: u32) -> WasmFunction {
    // locals: 2 = len.
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ensure));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

fn extend_body(ensure: u32) -> WasmFunction {
    // locals: 2 = len, 3 = olen.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
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
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
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

fn resize_body(ensure: u32) -> WasmFunction {
    // params: h=0, n=1, v=2; locals: 3 = len, 4 = i.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(ensure));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
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
    f.instruction(&Ins::I32Load8U(m(0)));
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
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::End);
    f
}

/// `(h, start, end) -> bytes`: byte-range slice, clamped.
fn slice_body(rc_new: u32, alloc: u32) -> WasmFunction {
    // locals: 3 = len, 4 = lo, 5 = hi, 6 = out.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    clamp(&mut f, 1, 3, 4);
    clamp(&mut f, 2, 3, 5);
    // ensure lo <= hi
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
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::End);
    f
}

/// `(h, needle) -> i64`: first byte index where `needle` occurs, or -1.
fn find_body() -> WasmFunction {
    // locals: 2 = hl, 3 = nl, 4 = i, 5 = j.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
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
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    byte_at(&mut f, 0, 12, 4, 5);
    byte_at(&mut f, 1, 12, 5, 0);
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eq);
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

fn starts_body(mem_cmp: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = pl.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(mem_cmp));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::End);
    f
}

fn ends_body(mem_cmp: u32) -> WasmFunction {
    // locals: 2 = hl, 3 = sl.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(mem_cmp));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::End);
    f
}

fn cmp_body(mem_cmp: u32) -> WasmFunction {
    // locals: 2 = la, 3 = lb, 4 = n, 5 = c.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32LtU);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::Call(mem_cmp));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::I64Const(-1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Else);
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
    f.instruction(&Ins::End);
    f
}

/// Loads byte `index` of buffer `base` (data at `base + 12`) into `dest_local`.
fn byte_at(f: &mut WasmFunction, base: u32, data_off: i64, index: u32, _scratch: u32) {
    f.instruction(&Ins::LocalGet(base));
    f.instruction(&Ins::I32Load(m(data_off)));
    f.instruction(&Ins::LocalGet(index));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
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
