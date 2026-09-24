//! Async runtime helpers for the wasm backend (C7 / WASM.9).
//!
//! A single-threaded cooperative executor. Futures, tasks and vutcons are all
//! `AsyncHandle`s in linear memory. `drive` polls a handle; when a poll returns
//! pending the handle records the child it is waiting on, and `drive` recurses
//! into that child, then re-polls the parent. There is no run queue and no
//! threads: suspension is a pending poll, resumption is the recursive re-poll
//! once the awaited child has completed.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

const I32: ValType = ValType::I32;

/// Offsets from the runtime base, in `bodies` order.
pub(crate) mod idx {
    pub(crate) const FRAME_ALLOC: u32 = 0;
    #[allow(dead_code)]
    pub(crate) const FRAME_FREE: u32 = 1;
    pub(crate) const HANDLE_NEW: u32 = 2;
    pub(crate) const HANDLE_DROP: u32 = 3;
    pub(crate) const AWAIT_CHILD: u32 = 4;
    pub(crate) const FUTURE_AWAIT: u32 = 5;
    pub(crate) const EXECUTOR_RUN: u32 = 6;
    pub(crate) const POLL: u32 = 7;
    pub(crate) const DRIVE: u32 = 8;
    /// Number of async runtime helpers.
    pub(crate) const COUNT: u32 = 9;
}

/// `AsyncHandle` field offsets (all pointer-sized words; 32-byte record).
pub(crate) const H_POLL: i32 = 0;
pub(crate) const H_DROP: i32 = 4;
pub(crate) const H_FRAME: i32 = 8;
pub(crate) const H_STATE: i32 = 12;
pub(crate) const H_RESULT: i32 = 16;
pub(crate) const H_RSIZE: i32 = 20;
pub(crate) const H_WAITING: i32 = 24;
const H_SIZE: i32 = 32;

const STATE_READY: i32 = 1;

fn m(offset: i32) -> MemArg {
    MemArg {
        offset: offset as u64,
        align: 0,
        memory_index: 0,
    }
}

/// Signatures of every async helper, in the same order as [`bodies`].
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32], vec![I32]),                // FRAME_ALLOC
        (vec![I32], vec![]),                   // FRAME_FREE
        (vec![I32, I32, I32, I32], vec![I32]), // HANDLE_NEW
        (vec![I32], vec![]),                   // HANDLE_DROP
        (vec![I32, I32], vec![I32]),           // AWAIT_CHILD
        (vec![I32, I32], vec![I32]),           // FUTURE_AWAIT
        (vec![I32], vec![I32]),                // EXECUTOR_RUN
        (vec![I32], vec![I32]),                // POLL
        (vec![I32], vec![I32]),                // DRIVE
    ]
}

/// Absolute indices of the async helpers, given the helper block `base`.
pub(crate) struct AsyncRt {
    pub frame_alloc: u32,
    pub handle_new: u32,
    pub await_child: u32,
    pub future_await: u32,
    pub executor_run: u32,
}

impl AsyncRt {
    #[must_use]
    pub(crate) fn new(base: u32) -> Self {
        Self {
            frame_alloc: base + idx::FRAME_ALLOC,
            handle_new: base + idx::HANDLE_NEW,
            await_child: base + idx::AWAIT_CHILD,
            future_await: base + idx::FUTURE_AWAIT,
            executor_run: base + idx::EXECUTOR_RUN,
        }
    }
}

/// Emits the async helper bodies. `alloc` allocates linear memory; `poll_type`
/// and `drop_type` are the `call_indirect` type indices of `(frame,out)->i32`
/// and `(frame)->()`; `current` is a mutable global holding the active handle.
#[must_use]
pub(crate) fn bodies(
    base: u32,
    alloc: u32,
    poll_type: u32,
    drop_type: u32,
    current: u32,
) -> Vec<WasmFunction> {
    let poll = base + idx::POLL;
    let drive = base + idx::DRIVE;
    let handle_drop = base + idx::HANDLE_DROP;
    vec![
        frame_alloc_body(alloc),
        frame_free_body(),
        handle_new_body(alloc),
        handle_drop_body(handle_drop, drop_type),
        await_child_body(handle_drop, current),
        future_await_body(handle_drop, drive),
        executor_run_body(drive),
        poll_body(poll_type, current),
        drive_body(poll, drive),
    ]
}

/// `(frame, out) -> i32`: poll entry for an async body with no suspension
/// points, so it has no state machine of its own. Calls the body once and
/// reports completion.
#[must_use]
pub(crate) fn resume_thunk(body: u32, result: Option<ValType>) -> WasmFunction {
    let locals = result.map_or_else(Vec::new, |ty| vec![(1, ty)]);
    let mut f = WasmFunction::new(locals);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(body));
    if let Some(ty) = result {
        f.instruction(&Ins::LocalSet(2));
        f.instruction(&Ins::LocalGet(1));
        f.instruction(&Ins::LocalGet(2));
        let _ = match ty {
            ValType::I64 => f.instruction(&Ins::I64Store(m(0))),
            ValType::F32 => f.instruction(&Ins::F32Store(m(0))),
            ValType::F64 => f.instruction(&Ins::F64Store(m(0))),
            _ => f.instruction(&Ins::I32Store(m(0))),
        };
    }
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::End);
    f
}

fn frame_alloc_body(alloc: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(7));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(-8));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::LocalTee(2));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::MemoryFill(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::End);
    f
}

/// `(ptr) -> ()`: frames live in a bump arena, so freeing is a no-op.
fn frame_free_body() -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::End);
    f
}

/// `(poll_fn, drop_fn, frame, result_size) -> handle`.
fn handle_new_body(alloc: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::I32Const(H_SIZE));
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Store(m(H_POLL)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(H_DROP)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(H_FRAME)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(H_STATE)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(H_WAITING)));
    // result buffer: at least one word so a void result has a valid pointer.
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(-4));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::Call(alloc));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Store(m(H_RESULT)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(H_RSIZE)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::End);
    f
}

/// `(handle) -> ()`: run the frame's drop thunk (releases in-flight children).
fn handle_drop_body(handle_drop: u32, drop_type: u32) -> WasmFunction {
    let _ = handle_drop;
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_DROP)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_FRAME)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_DROP)));
    f.instruction(&Ins::CallIndirect {
        type_index: drop_type,
        table_index: 0,
    });
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

/// `(slot, out) -> i32`: `1` when the awaited child completed, `0` when the
/// current task must suspend. `slot` is the frame's child-handle slot.
fn await_child_body(handle_drop: u32, current: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(1, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    // no child: treat as already complete.
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    // child ready: move the result out, drop the child, clear the slot.
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(H_STATE)));
    f.instruction(&Ins::I32Const(STATE_READY));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(H_RESULT)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(H_RSIZE)));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(handle_drop));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    // pending: record the child on the active handle and suspend.
    f.instruction(&Ins::GlobalGet(current));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(H_WAITING)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f
}

/// `(handle, out) -> i32`: blocking await outside a poll body.
fn future_await_body(handle_drop: u32, drive: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(drive));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_RESULT)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_RSIZE)));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(handle_drop));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::End);
    f
}

/// `(root) -> i32`: drive the root task to completion.
fn executor_run_body(drive: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(drive));
    f.instruction(&Ins::End);
    f
}

/// `(handle) -> i32`: poll the handle's state machine once.
fn poll_body(poll_type: u32, current: u32) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::GlobalSet(current));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_FRAME)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_RESULT)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_POLL)));
    f.instruction(&Ins::CallIndirect {
        type_index: poll_type,
        table_index: 0,
    });
    f.instruction(&Ins::End);
    f
}

/// `(handle) -> i32`: poll, and on suspension recurse into the awaited child
/// before re-polling. `1` when the handle completed, `0` if it cannot progress.
fn drive_body(poll: u32, drive: u32) -> WasmFunction {
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Call(poll));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(STATE_READY));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(STATE_READY));
    f.instruction(&Ins::I32Store(m(H_STATE)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Return);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(H_WAITING)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(H_WAITING)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(drive));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::End);
    f
}
