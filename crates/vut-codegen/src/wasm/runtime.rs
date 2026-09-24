//! Minimal wasm runtime helpers (WASM.6): list, bytes and map.
//!
//! All handles are pointers into linear memory. Elements are fixed 8-byte
//! slots; map keys/values are 8-byte slots with identity keys. There is no
//! reclamation (bump allocator) and COW is a no-op — a later phase adds RC/COW.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

/// Runtime function offsets from the runtime base (after `alloc`).
pub(crate) mod idx {
    pub(crate) const ALLOC: u32 = 0;
    pub(crate) const PRINT: u32 = 1;
    pub(crate) const OUT: u32 = 2;
    pub(crate) const FORMAT: u32 = 3;
    pub(crate) const CONCAT: u32 = 4;
    pub(crate) const LIST_NEW: u32 = 5;
    pub(crate) const LIST_LEN: u32 = 6;
    pub(crate) const LIST_PUSH: u32 = 7;
    pub(crate) const LIST_AT: u32 = 8;
    pub(crate) const LIST_SET: u32 = 9;
    pub(crate) const LIST_POP: u32 = 10;
    pub(crate) const LIST_CLEAR: u32 = 11;
    pub(crate) const BYTES_NEW: u32 = 12;
    pub(crate) const BYTES_LEN: u32 = 13;
    pub(crate) const BYTES_AT: u32 = 14;
    pub(crate) const BYTES_SET: u32 = 15;
    pub(crate) const BYTES_FROM_STR: u32 = 16;
    pub(crate) const BYTES_TO_STR: u32 = 17;
    pub(crate) const MAP_NEW: u32 = 18;
    pub(crate) const MAP_LEN: u32 = 19;
    pub(crate) const MAP_SET: u32 = 20;
    pub(crate) const MAP_GET: u32 = 21;
    pub(crate) const MAP_CONTAINS: u32 = 22;
    pub(crate) const MAP_REMOVE: u32 = 23;
    pub(crate) const MAP_CLEAR: u32 = 24;
    pub(crate) const MAP_FIND: u32 = 25;
    pub(crate) const RC_NEW: u32 = 26;
    pub(crate) const STR_FROM_STATIC: u32 = 27;
    pub(crate) const STR_RETAIN: u32 = 28;
    pub(crate) const STR_RELEASE: u32 = 29;
    pub(crate) const BYTES_RETAIN: u32 = 30;
    pub(crate) const BYTES_RELEASE: u32 = 31;
    pub(crate) const BYTES_MAKE_UNIQUE: u32 = 32;
    pub(crate) const LIST_RETAIN: u32 = 33;
    pub(crate) const LIST_RELEASE: u32 = 34;
    pub(crate) const LIST_MAKE_UNIQUE: u32 = 35;
    pub(crate) const MAP_RETAIN: u32 = 36;
    pub(crate) const MAP_RELEASE: u32 = 37;
    pub(crate) const ELEM_RETAIN: u32 = 38;
    pub(crate) const ELEM_RELEASE: u32 = 39;
    pub(crate) const STR_COPY: u32 = 40;
    pub(crate) const STR_EQ_AT: u32 = 41;
    #[allow(dead_code)]
    pub(crate) const STR_IS_WS: u32 = 42;
    pub(crate) const STR_EQ: u32 = 43;
    pub(crate) const STR_CMP: u32 = 44;
    pub(crate) const STR_CONTAINS: u32 = 45;
    pub(crate) const STR_STARTS: u32 = 46;
    pub(crate) const STR_ENDS: u32 = 47;
    pub(crate) const STR_FIND: u32 = 48;
    pub(crate) const STR_RFIND: u32 = 49;
    pub(crate) const STR_SUBSTRING: u32 = 50;
    pub(crate) const STR_REPEAT: u32 = 51;
    pub(crate) const STR_STRIP_PREFIX: u32 = 52;
    pub(crate) const STR_STRIP_SUFFIX: u32 = 53;
    pub(crate) const STR_TRIM_START: u32 = 54;
    pub(crate) const STR_TRIM_END: u32 = 55;
    pub(crate) const STR_LOWER: u32 = 56;
    pub(crate) const STR_UPPER: u32 = 57;
    pub(crate) const BT_ENSURE: u32 = 58;
    pub(crate) const BT_CLEAR: u32 = 59;
    pub(crate) const BT_CAPACITY: u32 = 60;
    pub(crate) const BT_RESERVE: u32 = 61;
    pub(crate) const BT_PUSH: u32 = 62;
    pub(crate) const BT_EXTEND: u32 = 63;
    pub(crate) const BT_TRUNCATE: u32 = 64;
    pub(crate) const BT_RESIZE: u32 = 65;
    pub(crate) const BT_FIRST: u32 = 66;
    pub(crate) const BT_LAST: u32 = 67;
    pub(crate) const BT_SLICE: u32 = 68;
    pub(crate) const BT_FIND: u32 = 69;
    pub(crate) const BT_STARTS: u32 = 70;
    pub(crate) const BT_ENDS: u32 = 71;
    pub(crate) const BT_CMP: u32 = 72;
    pub(crate) const BT_MEMCMP: u32 = 73;
    pub(crate) const LS_ENSURE: u32 = 74;
    pub(crate) const LS_CAPACITY: u32 = 75;
    pub(crate) const LS_RESERVE: u32 = 76;
    pub(crate) const LS_TRUNCATE: u32 = 77;
    pub(crate) const LS_SWAP: u32 = 78;
    pub(crate) const LS_REVERSE: u32 = 79;
    pub(crate) const LS_SHRINK: u32 = 80;
    pub(crate) const LS_FIRST: u32 = 81;
    pub(crate) const LS_LAST: u32 = 82;
    pub(crate) const LS_INSERT: u32 = 83;
    pub(crate) const LS_REMOVE: u32 = 84;
    pub(crate) const LS_EXTEND: u32 = 85;
    pub(crate) const LS_SLICE: u32 = 86;
    pub(crate) const LS_CONTAINS: u32 = 87;
    pub(crate) const LS_FIND: u32 = 88;
    pub(crate) const LS_ELEM_EQ: u32 = 89;
    pub(crate) const LS_JOIN: u32 = 90;
    pub(crate) const STR_TO_I64: u32 = 91;
    pub(crate) const STR_TO_F64: u32 = 92;
    pub(crate) const SO_SPLIT: u32 = 93;
    pub(crate) const SO_LINES: u32 = 94;
    pub(crate) const SO_REPLACE: u32 = 95;
    pub(crate) const SO_CHAR_AT: u32 = 96;
    pub(crate) const SO_CHARS: u32 = 97;
    pub(crate) const SO_PAD_LEFT: u32 = 98;
    pub(crate) const SO_PAD_RIGHT: u32 = 99;
    pub(crate) const SO_SPLIT_WS: u32 = 100;
    pub(crate) const SO_UTF8_LEN: u32 = 101;
    pub(crate) const SO_SCALAR_COUNT: u32 = 102;
    pub(crate) const SO_IS_WS: u32 = 103;
    pub(crate) const BT_TO_HEX: u32 = 104;
    pub(crate) const BT_TO_LIST: u32 = 105;
    pub(crate) const BT_FROM_LIST: u32 = 106;
    pub(crate) const BT_READ_INT: u32 = 107;
    pub(crate) const BT_WRITE_INT: u32 = 108;
    pub(crate) const FMT_F64: u32 = 109;
    pub(crate) const MAP_CAPACITY: u32 = 110;
    pub(crate) const MAP_RESERVE: u32 = 111;
    /// Number of runtime functions.
    pub(crate) const COUNT: u32 = 112;
}

fn m(offset: i64) -> MemArg {
    MemArg {
        offset: u64::try_from(offset).unwrap_or(0),
        align: 0,
        memory_index: 0,
    }
}

const I32: ValType = ValType::I32;
const I64: ValType = ValType::I64;

/// Signatures of every runtime helper, in the same order as [`bodies`].
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    let mut sigs = vec![
        (vec![I32], vec![I32]),        // ALLOC
        (vec![I32], vec![]),           // PRINT
        (vec![I32], vec![]),           // OUT
        (vec![I64], vec![I32]),        // FORMAT
        (vec![I32, I32], vec![I32]),   // CONCAT
        (vec![I32, I32], vec![I32]),   // LIST_NEW
        (vec![I32], vec![I32]),        // LIST_LEN
        (vec![I32, I64], vec![]),      // LIST_PUSH
        (vec![I32, I32], vec![I64]),   // LIST_AT
        (vec![I32, I32, I64], vec![]), // LIST_SET
        (vec![I32], vec![I64]),        // LIST_POP
        (vec![I32], vec![]),           // LIST_CLEAR
        (vec![I32], vec![I32]),        // BYTES_NEW
        (vec![I32], vec![I32]),        // BYTES_LEN
        (vec![I32, I32], vec![I32]),   // BYTES_AT
        (vec![I32, I32, I32], vec![]), // BYTES_SET
        (vec![I32], vec![I32]),        // BYTES_FROM_STR
        (vec![I32], vec![I32]),        // BYTES_TO_STR
        (vec![], vec![I32]),           // MAP_NEW
        (vec![I32], vec![I32]),        // MAP_LEN
        (vec![I32, I64, I64], vec![]), // MAP_SET
        (vec![I32, I64], vec![I64]),   // MAP_GET
        (vec![I32, I64], vec![I32]),   // MAP_CONTAINS
        (vec![I32, I64], vec![I64]),   // MAP_REMOVE
        (vec![I32], vec![]),           // MAP_CLEAR
        (vec![I32, I64], vec![I32]),   // MAP_FIND
    ];
    sigs.extend(super::rc::signatures());
    sigs.extend(super::strings::signatures());
    sigs.extend(super::bytes_ext::signatures());
    sigs.extend(super::listx::signatures());
    sigs.extend(super::strings::extra_signatures());
    sigs.extend(super::str_ops::signatures());
    sigs.extend(super::bytes_ext::extra_signatures());
    sigs.push(super::fmt::signature());
    sigs.extend(super::mapx::signatures());
    sigs
}

/// Runtime helper bodies in index order.
#[must_use]
#[allow(clippy::too_many_lines)]
pub(crate) fn bodies(base: u32) -> Vec<WasmFunction> {
    let alloc = base + idx::ALLOC;
    let rc_new = base + idx::RC_NEW;
    let elem_release = base + idx::ELEM_RELEASE;
    let mut funcs = vec![
        alloc_body(),
        print_body(0),
        out_body(0, base + idx::PRINT),
        format_body(rc_new),
        concat_body(rc_new),
        list_new_body(rc_new, alloc),
        list_len_body(),
        list_push_body(alloc),
        list_at_body(),
        list_set_body(elem_release),
        list_pop_body(),
        list_clear_body(elem_release),
        bytes_new_body(rc_new, alloc),
        bytes_len_body(),
        bytes_at_body(),
        bytes_set_body(),
        bytes_from_str_body(rc_new, alloc),
        bytes_to_str_body(rc_new),
        map_new_body(rc_new),
        map_len_body(),
        map_set_body(alloc, base + idx::MAP_FIND),
        map_get_body(base + idx::MAP_FIND),
        map_contains_body(base + idx::MAP_FIND),
        map_remove_body(base + idx::MAP_FIND),
        map_clear_body(),
        map_find_body(),
    ];
    funcs.extend(super::rc::bodies(base));
    funcs.extend(super::strings::bodies(base));
    funcs.extend(super::bytes_ext::bodies(base));
    funcs.extend(super::listx::bodies(base));
    funcs.extend(super::strings::extra_bodies(base));
    funcs.extend(super::str_ops::bodies(base));
    funcs.extend(super::bytes_ext::extra_bodies(base));
    funcs.push(super::fmt::body(base));
    funcs.extend(super::mapx::bodies(base));
    funcs
}

fn alloc_body() -> WasmFunction {
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::GlobalGet(0));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::MemorySize(0));
    f.instruction(&Ins::I32Const(65536));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32GtU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::MemorySize(0));
    f.instruction(&Ins::I32Const(65536));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Const(65535));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(65536));
    f.instruction(&Ins::I32DivU);
    f.instruction(&Ins::MemoryGrow(0));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::GlobalSet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::End);
    f
}

fn print_body(fd: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    // iovec.buf = handle + 4
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(fd));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::End);
    f
}

fn out_body(fd: u32, print: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(print));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(12));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(fd));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::End);
    f
}

fn format_body(rc_new: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(1, ValType::I64), (5, ValType::I32)]);
    // neg = v < 0
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64LtS);
    f.instruction(&Ins::LocalSet(3));
    // u = neg ? -v : v
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::If(BlockType::Result(ValType::I64)));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(128));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalTee(2));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::End);
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Const(10));
    f.instruction(&Ins::I64RemU);
    f.instruction(&Ins::I32WrapI64);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Const(10));
    f.instruction(&Ins::I64DivU);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalTee(2));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalTee(2));
    f.instruction(&Ins::I32Const(45));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(128));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f
}

fn concat_body(rc_new: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::End);
    f
}

// ---- list: header [len][cap][0][data]; 8-byte elements ----

fn list_new_body(rc_new: u32, alloc: u32) -> WasmFunction {
    // (elem_kind: i32, capacity: i32) -> handle
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0))); // len = 0
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4))); // cap = capacity
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Store(m(8))); // elem_kind
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32GtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Store(m(12))); // data
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

fn list_len_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::End);
    f
}

fn list_push_body(alloc: u32) -> WasmFunction {
    // (h: i32, v: i64) -> (); locals 2..5 = len, cap, new_cap, new_data.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    // len=local2, cap=local3
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalSet(3));
    // if len == cap { grow }
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    // new_cap = cap == 0 ? 4 : cap * 2
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Result(ValType::I32)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(4));
    // new_data = alloc(new_cap * 8)
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(5));
    // memory.copy(new_data, old_data, len*8)
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    // store new data/cap
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::End);
    // data[len*8] = v
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Store(m(0)));
    // len++
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

fn list_at_body() -> WasmFunction {
    // (h: i32, i: i32) -> i64; traps when out of bounds (Vut semantics).
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
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
    f.instruction(&Ins::End);
    f
}

fn list_set_body(elem_release: u32) -> WasmFunction {
    // (h, i, v: i64); locals 3 = slot, 4 = kind.
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
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
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(elem_release));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::End);
    f
}

fn list_pop_body() -> WasmFunction {
    // (h) -> i64
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::End);
    f
}

fn list_clear_body(elem_release: u32) -> WasmFunction {
    // locals 1 = i, 2 = data, 3 = kind, 4 = len.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(elem_release));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

// ---- bytes: header [len][cap][0][data]; 1-byte elements ----

fn bytes_new_body(rc_new: u32, alloc: u32) -> WasmFunction {
    // (len: i32) -> handle
    let mut f = WasmFunction::new([(1, ValType::I32)]);
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Store(m(0))); // len
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Store(m(4))); // cap = len
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64Store(m(8)));
    // data = alloc(len)
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::End);
    f
}

fn bytes_len_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::End);
    f
}

fn bytes_at_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Unreachable);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::End);
    f
}

fn bytes_set_body() -> WasmFunction {
    // (h, i, b)
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Unreachable);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::End);
    f
}

fn bytes_from_str_body(rc_new: u32, alloc: u32) -> WasmFunction {
    // (str_handle) -> bytes_handle
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1)); // len
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::I32Store(m(12)));
    // copy
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(12)));
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

fn bytes_to_str_body(rc_new: u32) -> WasmFunction {
    // (bytes_handle) -> str_handle ([len][bytes])
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
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

// ---- map: header [len][cap][0][data]; entries are 16 bytes (key i64, value i64) ----

fn map_new_body(rc_new: u32) -> WasmFunction {
    // No parameters; locals 0 (unused) and 1 (handle).
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(rc_new));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::I64Store(m(8)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::End);
    f
}

fn map_len_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::End);
    f
}

/// Linear scan for `key`; returns the entry pointer or 0.
fn map_find(f: &mut WasmFunction) {
    // locals: 2 = i (index), 3 = data
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0))); // entry.key
    f.instruction(&Ins::LocalGet(1)); // key
    f.instruction(&Ins::I64Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Return);
}

fn map_set_body(alloc: u32, find: u32) -> WasmFunction {
    // (h, k: i64, v: i64); locals 3..6 = scratch, entry, new_cap, new_data.
    let mut f = WasmFunction::new([(4, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(find));
    f.instruction(&Ins::LocalSet(4)); // entry ptr (0 if absent)
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64Store(m(8)));
    f.instruction(&Ins::Else);
    // grow if needed
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Result(ValType::I32)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::End);
    // data[len] = k; data[len].value = v
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64Store(m(8)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn map_get_body(find: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(find));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::If(BlockType::Result(ValType::I64)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64Load(m(8)));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn map_contains_body(find: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(3, ValType::I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(find));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::End);
    f
}

fn map_remove_body(find: u32) -> WasmFunction {
    // (h, k: i64) -> i64; locals 2..7 i32 (entry = 4, last = 5), 8 i64 (old).
    let mut f = WasmFunction::new([(6, ValType::I32), (1, ValType::I64)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(find));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I64Const(0));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I64Load(m(8)));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Store(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Shl);
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64Load(m(8)));
    f.instruction(&Ins::I64Store(m(8)));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::End);
    f
}
fn map_clear_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::End);
    f
}

fn map_find_body() -> WasmFunction {
    let mut f = WasmFunction::new([(2, ValType::I32)]);
    map_find(&mut f);
    f.instruction(&Ins::End);
    f
}
