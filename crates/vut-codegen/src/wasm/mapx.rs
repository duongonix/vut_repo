//! Extra `map` runtime helpers for the wasm backend.
//!
//! A map handle points at `[len: i32][cap: i32][unused: i32][data: i32]`
//! (16-byte payload) after the eight-byte reference-count prefix; entries are
//! 16 bytes (key `i64`, value `i64`).
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

/// Signatures.
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32], vec![I32]),   // MAP_CAPACITY
        (vec![I32, I32], vec![]), // MAP_RESERVE
    ]
}

/// Bodies.
#[must_use]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let alloc = base + idx::ALLOC;
    vec![capacity_body(), reserve_body(alloc)]
}

fn capacity_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::End);
    f
}

fn reserve_body(alloc: u32) -> WasmFunction {
    // locals: 2 = cap, 3 = len, 4 = data.
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::End);
    f
}
