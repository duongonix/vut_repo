//! Reference counting and copy-on-write helpers for the wasm managed runtime.
//!
//! Managed objects (string, bytes, list, map) are allocated with an eight-byte
//! prefix: the `i32` reference count at `handle - 8`, and the payload at
//! `handle`. `*_retain` increments the count, `*_release` decrements it and runs
//! element destruction at zero, and `make_unique` returns a clone when the
//! object is shared so an in-place mutation cannot alias another binding.
//!
//! Compound objects store an element *kind code* (see [`KIND_STRING`] etc.) so
//! element retain/release can dispatch without a per-type callback table.
//! Aggregates with field-wise ownership (data/enum/optional/result/array) are
//! extended in C4.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

use super::runtime::idx;

const I32: ValType = ValType::I32;

fn m(offset: i64) -> MemArg {
    MemArg {
        offset: u64::try_from(offset).unwrap_or(0),
        align: 0,
        memory_index: 0,
    }
}

/// Element ownership kind codes stored in collection headers.
pub(crate) const KIND_NONE: i32 = 0;
pub(crate) const KIND_STRING: i32 = 1;
pub(crate) const KIND_BYTES: i32 = 2;
pub(crate) const KIND_LIST: i32 = 3;
pub(crate) const KIND_MAP: i32 = 4;
pub(crate) const KIND_FLOAT: i32 = 5;

/// Signatures of the reference-counting helpers, in [`idx`] order.
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32], vec![I32]),   // RC_NEW
        (vec![I32], vec![I32]),   // STR_FROM_STATIC
        (vec![I32], vec![]),      // STR_RETAIN
        (vec![I32], vec![]),      // STR_RELEASE
        (vec![I32], vec![]),      // BYTES_RETAIN
        (vec![I32], vec![]),      // BYTES_RELEASE
        (vec![I32], vec![I32]),   // BYTES_MAKE_UNIQUE
        (vec![I32], vec![]),      // LIST_RETAIN
        (vec![I32], vec![]),      // LIST_RELEASE
        (vec![I32], vec![I32]),   // LIST_MAKE_UNIQUE
        (vec![I32], vec![]),      // MAP_RETAIN
        (vec![I32], vec![]),      // MAP_RELEASE
        (vec![I32, I32], vec![]), // ELEM_RETAIN
        (vec![I32, I32], vec![]), // ELEM_RELEASE
    ]
}

/// Helper bodies in [`idx`] order, referencing the shared runtime by `base`.
#[must_use]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let alloc = base + idx::ALLOC;
    let rc_new = base + idx::RC_NEW;
    let str_retain = base + idx::STR_RETAIN;
    let str_release = base + idx::STR_RELEASE;
    let bytes_retain = base + idx::BYTES_RETAIN;
    let bytes_release = base + idx::BYTES_RELEASE;
    let list_retain = base + idx::LIST_RETAIN;
    let list_release = base + idx::LIST_RELEASE;
    let map_retain = base + idx::MAP_RETAIN;
    let map_release = base + idx::MAP_RELEASE;
    let elem_retain = base + idx::ELEM_RETAIN;
    let elem_release = base + idx::ELEM_RELEASE;
    vec![
        rc_new_body(alloc),
        str_from_static_body(rc_new),
        inc_ref_body(),
        simple_release_body(),
        inc_ref_body(),
        simple_release_body(),
        bytes_make_unique_body(rc_new, alloc),
        inc_ref_body(),
        list_release_body(elem_release),
        list_make_unique_body(rc_new, alloc, elem_retain),
        inc_ref_body(),
        simple_release_body(),
        elem_dispatch_body(str_retain, bytes_retain, list_retain, map_retain),
        elem_dispatch_body(str_release, bytes_release, list_release, map_release),
    ]
}

/// `(size: i32) -> handle`: allocates `8 + size`, sets the refcount to one.
fn rc_new_body(alloc: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::End);
    f
}

/// `(static_addr: i32) -> handle`: copies a `[len][bytes]` literal to a new
/// reference-counted string.
fn str_from_static_body(rc_new: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// `(h: i32) -> ()`: increments the reference count at `h - 8`.
fn inc_ref_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    push_refcount_addr(&mut f);
    push_refcount(&mut f);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

/// `(h: i32) -> ()`: decrements the reference count at `h - 8`.
fn simple_release_body() -> WasmFunction {
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    push_dec_ref(&mut f, 1);
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::End);
    f
}

/// `(h: i32) -> ()`: decrements a list refcount and destroys its elements when
/// the count reaches zero. Locals: 1 = count, 2 = i, 3 = data, 4 = kind, 5 = len.
fn list_release_body(elem_release: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(5, ValType::I32)]);
    push_dec_ref(&mut f, 1);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(elem_release));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(kind: i32, slot: i32) -> ()`: dispatches element retain/release by kind.
fn elem_dispatch_body(string: u32, bytes: u32, list: u32, map: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(KIND_STRING));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    call_slot(&mut f, string);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(KIND_BYTES));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    call_slot(&mut f, bytes);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(KIND_LIST));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    call_slot(&mut f, list);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(KIND_MAP));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    call_slot(&mut f, map);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(h: i32) -> handle`: clones shared bytes, otherwise returns `h`.
fn bytes_make_unique_body(rc_new: u32, alloc: u32) -> WasmFunction {
    // locals: 1 = len, 2 = new, 3 = data, 4 = count.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    push_is_unique(&mut f);
    f.instruction(&Ins::If(BlockType::Result(ValType::I32)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    push_dec_ref(&mut f, 4);
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(h: i32) -> handle`: clones a shared list, retaining every element.
fn list_make_unique_body(rc_new: u32, alloc: u32, elem_retain: u32) -> WasmFunction {
    // locals: 1 = len, 2 = kind, 3 = new, 4 = data, 5 = i, 6 = count.
    let mut f = WasmFunction::new([(6, ValType::I32)]);
    push_is_unique(&mut f);
    f.instruction(&Ins::If(BlockType::Result(ValType::I32)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(8)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
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
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
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
    push_dec_ref(&mut f, 6);
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn call_slot(f: &mut WasmFunction, target: u32) {
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::Call(target));
}

/// Leaves `load(rc(h)-1)` on the stack as the new count.
fn push_is_unique(f: &mut WasmFunction) {
    push_refcount(f);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Eq);
}

/// Stores `rc(h) - 1` and leaves it on the stack.
fn push_dec_ref(f: &mut WasmFunction, count_local: u32) {
    push_refcount_addr(f);
    push_refcount(f);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalTee(count_local));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(count_local));
}

fn push_refcount_addr(f: &mut WasmFunction) {
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Sub);
}

fn push_refcount(f: &mut WasmFunction) {
    push_refcount_addr(f);
    f.instruction(&Ins::I32Load(m(0)));
}
