//! WASI-backed implementations of the standard-library native ABI (`vut_rt_*`).
//!
//! The official stdlib calls `extern "C"` primitives (`@link_name("vut_rt_…")`).
//! On `wasm32-wasip1` these are provided here as wasm helper functions instead
//! of native imports. Two shapes are used:
//!
//! * **Result envelope** (`bytes`): `[status: i32 little-endian][payload]`.
//!   `status == 0` is success, `status == 1` is a null optional, anything else
//!   is an error whose payload is a UTF-8 message.
//! * **Counted resource** (`count.Handle`): a pointer to
//!   `[valid: i32][value: i64][error: bytes]`.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use std::collections::HashMap;

use wasm_encoder::{BlockType, Function as WasmFunction, Instruction as Ins, MemArg, ValType};

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

/// A WASI import the wasm stdlib runtime may call.
pub(crate) struct WasiImport {
    pub name: &'static str,
    /// Index into the shared WASI signature table.
    pub signature: u8,
}

/// The WASI preview1 imports used by the stdlib runtime, in import order.
pub(crate) const WASI_IMPORTS: &[WasiImport] = &[
    WasiImport {
        name: "fd_write",
        signature: 0,
    },
    WasiImport {
        name: "fd_read",
        signature: 0,
    },
    WasiImport {
        name: "environ_sizes_get",
        signature: 1,
    },
    WasiImport {
        name: "environ_get",
        signature: 1,
    },
    WasiImport {
        name: "args_sizes_get",
        signature: 1,
    },
    WasiImport {
        name: "args_get",
        signature: 1,
    },
    WasiImport {
        name: "clock_time_get",
        signature: 2,
    },
    WasiImport {
        name: "random_get",
        signature: 1,
    },
    WasiImport {
        name: "poll_oneoff",
        signature: 0,
    },
];

/// Internal helper names emitted before the public helpers.
const INTERNAL: &[&str] = &[
    "__make_bytes",
    "__load_environ",
    "__load_args",
    "__env_find",
    "__ldexp",
];

/// The `vut_rt_*` link names implemented for wasm, in helper-body order.
pub(crate) const HELPERS: &[&str] = &[
    "vut_rt_count_ok_v1",
    "vut_rt_count_value_v1",
    "vut_rt_count_error_v1",
    "vut_rt_env_get_v1",
    "vut_rt_env_has_v1",
    "vut_rt_env_set_v1",
    "vut_rt_env_remove_v1",
    "vut_rt_env_all_v1",
    "vut_rt_env_args_v1",
    "vut_rt_time_now_nanos_v1",
    "vut_rt_time_instant_nanos_v1",
    "vut_rt_time_sleep_nanos_v1",
    "vut_rt_io_stdin_read_v1",
    "vut_rt_io_stdout_write_v2",
    "vut_rt_io_stderr_write_v2",
    "vut_rt_io_flush_v1",
    "vut_rt_os_name_v1",
    "vut_rt_os_family_v1",
    "vut_rt_os_arch_v1",
    "vut_rt_os_cpu_count_v1",
    "vut_rt_os_home_dir_v1",
    "vut_rt_os_temp_dir_v1",
    "vut_rt_os_current_dir_v1",
    "vut_rt_os_current_exe_v1",
    "vut_rt_os_set_current_dir_v1",
    "vut_rt_math_random_float_v1",
    "vut_rt_math_random_int_v1",
    "vut_rt_math_random_bool_v1",
    "vut_rt_math_exp_v1",
    "vut_rt_math_exp2_v1",
    "vut_rt_math_log_v1",
    "vut_rt_math_log2_v1",
    "vut_rt_math_log10_v1",
    "vut_rt_math_sin_v1",
    "vut_rt_math_cos_v1",
    "vut_rt_math_tan_v1",
    "vut_rt_math_asin_v1",
    "vut_rt_math_acos_v1",
    "vut_rt_math_atan_v1",
    "vut_rt_math_atan2_v1",
    "vut_rt_math_sinh_v1",
    "vut_rt_math_cosh_v1",
    "vut_rt_math_tanh_v1",
    "vut_rt_math_cbrt_v1",
    "vut_rt_math_hypot_v1",
];

/// Total helper body count (internal + public).
#[must_use]
pub(crate) fn body_count() -> u32 {
    (INTERNAL.len() + HELPERS.len()) as u32
}

/// Helper signatures in body order (internal then public).
#[must_use]
pub(crate) fn signatures() -> Vec<(Vec<ValType>, Vec<ValType>)> {
    vec![
        (vec![I32, I32, I32], vec![I32]), // __make_bytes
        (vec![], vec![I32]),              // __load_environ
        (vec![], vec![I32]),              // __load_args
        (vec![I32, I32], vec![I32]),      // __env_find
        (vec![F64, I32], vec![F64]),      // __ldexp
        (vec![I32], vec![I32]),           // count_ok
        (vec![I32], vec![I64]),           // count_value
        (vec![I32], vec![I32]),           // count_error
        (vec![I32], vec![I32]),           // env_get
        (vec![I32], vec![I32]),           // env_has
        (vec![I32, I32], vec![I32]),      // env_set
        (vec![I32], vec![I32]),           // env_remove
        (vec![], vec![I32]),              // env_all
        (vec![], vec![I32]),              // env_args
        (vec![], vec![I64]),              // time_now
        (vec![], vec![I64]),              // time_instant
        (vec![I64], vec![]),              // time_sleep
        (vec![], vec![I32]),              // io_stdin_read
        (vec![I32], vec![I32]),           // io_stdout_write
        (vec![I32], vec![I32]),           // io_stderr_write
        (vec![], vec![I32]),              // io_flush
        (vec![], vec![I32]),              // os_name
        (vec![], vec![I32]),              // os_family
        (vec![], vec![I32]),              // os_arch
        (vec![], vec![I32]),              // os_cpu_count
        (vec![], vec![I32]),              // os_home_dir
        (vec![], vec![I32]),              // os_temp_dir
        (vec![], vec![I32]),              // os_current_dir
        (vec![], vec![I32]),              // os_current_exe
        (vec![I32], vec![I32]),           // os_set_current_dir
        (vec![], vec![F64]),              // random_float
        (vec![I64, I64], vec![I64]),      // random_int
        (vec![], vec![I32]),              // random_bool
        (vec![F64], vec![F64]),           // math exp
        (vec![F64], vec![F64]),           // math exp2
        (vec![F64], vec![F64]),           // math log
        (vec![F64], vec![F64]),           // math log2
        (vec![F64], vec![F64]),           // math log10
        (vec![F64], vec![F64]),           // math sin
        (vec![F64], vec![F64]),           // math cos
        (vec![F64], vec![F64]),           // math tan
        (vec![F64], vec![F64]),           // math asin
        (vec![F64], vec![F64]),           // math acos
        (vec![F64], vec![F64]),           // math atan
        (vec![F64, F64], vec![F64]),      // math atan2
        (vec![F64], vec![F64]),           // math sinh
        (vec![F64], vec![F64]),           // math cosh
        (vec![F64], vec![F64]),           // math tanh
        (vec![F64], vec![F64]),           // math cbrt
        (vec![F64, F64], vec![F64]),      // math hypot
    ]
}

/// Whether `link_name` is implemented for wasm.
#[must_use]
pub(crate) fn is_supported(link_name: &str) -> bool {
    HELPERS.contains(&link_name)
}

/// Absolute function indices available to helper bodies.
struct Ctx {
    wasi: HashMap<&'static str, u32>,
    alloc: u32,
    bytes_new: u32,
    make_bytes: u32,
    load_environ: u32,
    load_args: u32,
    env_find: u32,
    ldexp: u32,
    base: u32,
}

/// Helper body index for `link_name`, given `base`.
#[must_use]
pub(crate) fn helper_index(base: u32, link_name: &str) -> Option<u32> {
    HELPERS
        .iter()
        .position(|name| *name == link_name)
        .map(|position| base + INTERNAL.len() as u32 + position as u32)
}

/// Emits all helper bodies in [`INTERNAL`] then [`HELPERS`] order.
#[must_use]
pub(crate) fn bodies(
    base: u32,
    wasi: HashMap<&'static str, u32>,
    alloc: u32,
    bytes_new: u32,
) -> Vec<WasmFunction> {
    let ctx = Ctx {
        wasi,
        alloc,
        bytes_new,
        make_bytes: base,
        load_environ: base + 1,
        load_args: base + 2,
        env_find: base + 3,
        ldexp: base + 4,
        base,
    };
    let mut bodies = vec![
        make_bytes_body(&ctx),
        load_list_body(&ctx, "environ_sizes_get", "environ_get"),
        load_list_body(&ctx, "args_sizes_get", "args_get"),
        env_find_body(&ctx),
        ldexp_body(&ctx),
    ];
    bodies.extend(public_bodies(&ctx));
    bodies.extend(math_bodies(&ctx));
    bodies
}

fn public_bodies(ctx: &Ctx) -> Vec<WasmFunction> {
    vec![
        count_read(ctx, 0, I32),
        count_read(ctx, 4, I64),
        count_read(ctx, 12, I32),
        env_get(ctx),
        env_has(ctx),
        env_unsupported(
            ctx,
            "environment mutation is not supported on wasm32-wasip1",
        ),
        env_unsupported(
            ctx,
            "environment mutation is not supported on wasm32-wasip1",
        ),
        env_all(ctx),
        env_args(ctx),
        time_now(ctx),
        time_instant(ctx),
        time_sleep(ctx),
        io_stdin_read(ctx),
        io_write(ctx, 1),
        io_write(ctx, 2),
        io_flush(ctx),
        os_static(ctx, "wasm"),
        os_static(ctx, "wasip1"),
        os_static(ctx, "wasm32"),
        os_static(ctx, "1"),
        os_env_or(ctx, "HOME", "/"),
        os_env_or(ctx, "TMPDIR", "/tmp"),
        os_static(ctx, "/"),
        os_current_exe(ctx),
        env_unsupported(
            ctx,
            "changing the working directory is not supported on wasm32-wasip1",
        ),
        random_float(ctx),
        random_int(ctx),
        random_bool(ctx),
    ]
}

/// Math helper bodies, appended after the public helpers.
fn math_bodies(ctx: &Ctx) -> Vec<WasmFunction> {
    let math_base =
        ctx.base + INTERNAL.len() as u32 + (HELPERS.len() as u32 - super::math::COUNT as u32);
    super::math::bodies(math_base, ctx.ldexp)
}

// ---- internal helpers ----

/// `(handle: i32) -> value`: reads a field of a counted-resource handle.
fn count_read(_ctx: &Ctx, offset: i64, ty: ValType) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::LocalGet(0));
    match ty {
        ValType::I64 => f.instruction(&Ins::I64Load(m(offset))),
        _ => f.instruction(&Ins::I32Load(m(offset))),
    };
    f.instruction(&Ins::End);
    f
}

/// `(status: i32, ptr: i32, len: i32) -> bytes` envelope.
fn make_bytes_body(ctx: &Ctx) -> WasmFunction {
    // locals: 3 = handle, 4 = data.
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ctx.bytes_new));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

/// `(sizes_get, get) -> ptr` to `[count, len, buf, ptrs]` for environ/args.
fn load_list_body(ctx: &Ctx, sizes: &str, get: &str) -> WasmFunction {
    let sizes_get = ctx.wasi[sizes];
    let get = ctx.wasi[get];
    // locals: 1 = sizes, 2 = count, 3 = size, 4 = buf, 5 = ptrs, 6 = out.
    let mut f = WasmFunction::new([(7, I32)]);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(sizes_get));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::Call(get));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::I32Const(16));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(8)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::End);
    f
}

/// `(name_ptr, name_len) -> ptr` to `[found: i32, value_ptr: i32, value_len: i32]`.
#[expect(clippy::too_many_lines, reason = "linear environ scan")]
fn env_find_body(ctx: &Ctx) -> WasmFunction {
    // locals: 2 = list, 3 = buf, 4 = size, 5 = i, 6 = j, 7 = match, 8 = out,
    // 9 = value_ptr, 10 = value_end, 11 = done.
    let mut f = WasmFunction::new([(10, I32)]);
    f.instruction(&Ins::Call(ctx.load_environ));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(12));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(8));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store(m(8)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(11));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(11));
    f.instruction(&Ins::BrIf(1));
    // entry start: i == 0 || buf[i-1] == 0
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::End);
    f.instruction(&Ins::If(BlockType::Empty));
    // room check: name_len + 1 <= size - i, and buf[i+name_len] == '='
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32LeU);
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(61));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::If(BlockType::Empty));
    // compare name bytes
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Ne);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::Br(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::If(BlockType::Empty));
    // value_ptr = buf + i + name_len + 1
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(9));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Store(m(4)));
    // value_end = value_ptr; while buf[value_end] != 0: value_end++
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::LocalSet(10));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(10));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::LocalGet(10));
    f.instruction(&Ins::LocalGet(9));
    f.instruction(&Ins::I32Sub);
    f.instruction(&Ins::I32Store(m(8)));
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalSet(11));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(8));
    f.instruction(&Ins::End);
    f
}

// ---- env public helpers ----

fn env_get(ctx: &Ctx) -> WasmFunction {
    // (name: str) -> bytes; locals: 1 = name_len, 2 = found.
    let mut f = WasmFunction::new([(3, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(ctx.env_find));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn env_has(ctx: &Ctx) -> WasmFunction {
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(ctx.env_find));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::End);
    f
}

fn env_unsupported(ctx: &Ctx, message: &str) -> WasmFunction {
    // Returns an envelope with status 8 and `message` as the payload.
    let bytes = message.as_bytes();
    let mut f = WasmFunction::new([(2, I32)]);
    // static message is copied from a fixed scratch region we write inline.
    f.instruction(&Ins::I32Const(bytes.len() as i32));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    for (index, byte) in bytes.iter().enumerate() {
        f.instruction(&Ins::LocalGet(1));
        f.instruction(&Ins::I32Const(index as i32));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::I32Const(i32::from(*byte)));
        f.instruction(&Ins::I32Store8(m(0)));
    }
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(bytes.len() as i32));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

fn env_all(ctx: &Ctx) -> WasmFunction {
    // Envelope status 0 with payload = environ buffer, but `KEY=VALUE` must
    // become `KEY\0VALUE\0`. We rewrite `=` to `\0` and append a `\0` per entry.
    // locals: 1 = list, 2 = buf, 3 = size, 4 = i, 5 = out, 6 = out_len, 7 = b.
    let mut f = WasmFunction::new([(8, I32)]);
    f.instruction(&Ins::Call(ctx.load_environ));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalSet(3));
    // out = alloc(size + count + 1) — over-allocate size*2 to be safe.
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Mul);
    f.instruction(&Ins::I32Const(2));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(5));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(6));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32GeU);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::LocalSet(7));
    // '=' (61) -> '\0' (0)
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Const(61));
    f.instruction(&Ins::I32Eq);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(7));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalGet(7));
    f.instruction(&Ins::I32Store8(m(0)));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(6));
    // at end of entry (byte was '\0'), emit an extra '\0'? The value already
    // ended with '\0'; we replaced '=' with '\0', giving `KEY\0VALUE\0`.
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(5));
    f.instruction(&Ins::LocalGet(6));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

fn env_args(ctx: &Ctx) -> WasmFunction {
    // Envelope status 0 with payload = the raw argv buffer (`\0`-separated).
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::Call(ctx.load_args));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

// ---- time ----

fn time_now(ctx: &Ctx) -> WasmFunction {
    clock_time(ctx, 0)
}
fn time_instant(ctx: &Ctx) -> WasmFunction {
    clock_time(ctx, 1)
}
fn clock_time(ctx: &Ctx, clock_id: i32) -> WasmFunction {
    // () -> i64; locals: 1 = out.
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(clock_id));
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(ctx.wasi["clock_time_get"]));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::End);
    f
}

fn time_sleep(ctx: &Ctx) -> WasmFunction {
    // (nanos: i64) -> (); subscription: [userdata i64][tag i8][pad][clock i32]
    // [precision i64][flags i16][pad][timeout i64] = 48 bytes.
    // locals: 1 = sub, 2 = event.
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(2));
    // zero the subscription (alloc is not guaranteed zeroed)
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(48));
    f.instruction(&Ins::MemoryFill(0));
    // tag = 0 (clock) at offset 8
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Store8(m(8)));
    // clock id = 1 (monotonic) at offset 16
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Store(m(16)));
    // timeout = nanos at offset 32
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64Store(m(32)));
    // poll_oneoff(sub, event, 1, nevents)
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::Call(ctx.wasi["poll_oneoff"]));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::End);
    f
}

// ---- io ----

fn io_stdin_read(ctx: &Ctx) -> WasmFunction {
    // () -> bytes envelope; locals: 1 = buf, 2 = nread, 3 = iovec.
    let mut f = WasmFunction::new([(4, I32)]);
    f.instruction(&Ins::I32Const(4096));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(4096));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(ctx.wasi["fd_read"]));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

fn io_write(ctx: &Ctx, fd: i32) -> WasmFunction {
    // (blob: bytes) -> resource[Handle]; locals: 1 = iovec, 2 = nwritten,
    // 3 = handle, 4 = error bytes.
    let mut f = WasmFunction::new([(4, I32)]);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::I32Const(4));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(12)));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I32Store(m(4)));
    f.instruction(&Ins::I32Const(fd));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::Call(ctx.wasi["fd_write"]));
    f.instruction(&Ins::Drop);
    // error = empty bytes envelope (status 0)
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::LocalSet(4));
    // handle = alloc(12): [valid=1, value=nwritten, error]
    f.instruction(&Ins::I32Const(12));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Store(m(0)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::I64ExtendI32U);
    f.instruction(&Ins::I64Store(m(4)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::I32Store(m(12)));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::End);
    f
}

fn io_flush(ctx: &Ctx) -> WasmFunction {
    let mut f = WasmFunction::new([]);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

// ---- os ----

fn os_static(ctx: &Ctx, value: &str) -> WasmFunction {
    let bytes = value.as_bytes();
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::I32Const(bytes.len().max(1) as i32));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    for (index, byte) in bytes.iter().enumerate() {
        f.instruction(&Ins::LocalGet(1));
        f.instruction(&Ins::I32Const(index as i32));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::I32Const(i32::from(*byte)));
        f.instruction(&Ins::I32Store8(m(0)));
    }
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(bytes.len() as i32));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

fn os_env_or(ctx: &Ctx, name: &str, fallback: &str) -> WasmFunction {
    // Build the name string in scratch, look it up, else use the fallback.
    let name_bytes = name.as_bytes();
    let fallback_bytes = fallback.as_bytes();
    let mut f = WasmFunction::new([(3, I32)]);
    f.instruction(&Ins::I32Const(name_bytes.len().max(1) as i32));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    for (index, byte) in name_bytes.iter().enumerate() {
        f.instruction(&Ins::LocalGet(1));
        f.instruction(&Ins::I32Const(index as i32));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::I32Const(i32::from(*byte)));
        f.instruction(&Ins::I32Store8(m(0)));
    }
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(name_bytes.len() as i32));
    f.instruction(&Ins::Call(ctx.env_find));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(0)));
    f.instruction(&Ins::If(BlockType::Result(I32)));
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(4)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::Else);
    // fallback
    f.instruction(&Ins::I32Const(fallback_bytes.len().max(1) as i32));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    for (index, byte) in fallback_bytes.iter().enumerate() {
        f.instruction(&Ins::LocalGet(1));
        f.instruction(&Ins::I32Const(index as i32));
        f.instruction(&Ins::I32Add);
        f.instruction(&Ins::I32Const(i32::from(*byte)));
        f.instruction(&Ins::I32Store8(m(0)));
    }
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(fallback_bytes.len() as i32));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn os_current_exe(ctx: &Ctx) -> WasmFunction {
    // argv[0] from the args buffer.
    let mut f = WasmFunction::new([(4, I32)]);
    f.instruction(&Ins::Call(ctx.load_args));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load(m(8)));
    f.instruction(&Ins::LocalSet(2));
    // length = strlen(buf)
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Block(BlockType::Empty));
    f.instruction(&Ins::Loop(BlockType::Empty));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Eqz);
    f.instruction(&Ins::BrIf(1));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::Br(0));
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::I32Const(0));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::Call(ctx.make_bytes));
    f.instruction(&Ins::End);
    f
}

// ---- random ----

fn random_float(ctx: &Ctx) -> WasmFunction {
    // () -> f64; locals: 1 = buf.
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.wasi["random_get"]));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::I64Const(11));
    f.instruction(&Ins::I64ShrU);
    f.instruction(&Ins::F64ConvertI64U);
    f.instruction(&Ins::F64Const(1.110_223_024_625_156_5e-16.into()));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f
}

fn random_int(ctx: &Ctx) -> WasmFunction {
    // (min: i64, max: i64) -> i64; locals: 2 = range, 3 = buf.
    let mut f = WasmFunction::new([(1, I64), (1, I32)]);
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(8));
    f.instruction(&Ins::Call(ctx.wasi["random_get"]));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::I64Sub);
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64Eqz);
    f.instruction(&Ins::If(BlockType::Result(I64)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I64Load(m(0)));
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I64RemU);
    f.instruction(&Ins::I64Add);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}

fn random_bool(ctx: &Ctx) -> WasmFunction {
    // () -> i32 (0/1); locals: 1 = buf.
    let mut f = WasmFunction::new([(2, I32)]);
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Call(ctx.alloc));
    f.instruction(&Ins::LocalSet(1));
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::Call(ctx.wasi["random_get"]));
    f.instruction(&Ins::Drop);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::I32Load8U(m(0)));
    f.instruction(&Ins::I32Const(1));
    f.instruction(&Ins::I32And);
    f.instruction(&Ins::End);
    f
}

/// `(x: f64, k: i32) -> f64`: `x * 2^k` via exponent-field construction.
fn ldexp_body(_ctx: &Ctx) -> WasmFunction {
    // params: 0 = x(f64), 1 = k(i32); locals: 2,3 = i32, 4 = f64, 5 = i64.
    let mut f = WasmFunction::new([(2, I32), (1, F64), (1, I64)]);
    f.instruction(&Ins::LocalGet(1));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::F64Const(0.0.into()));
    f.instruction(&Ins::F64Eq);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1023));
    f.instruction(&Ins::I32GtS);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(1023));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(-1074));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Empty));
    f.instruction(&Ins::I32Const(-1074));
    f.instruction(&Ins::LocalSet(2));
    f.instruction(&Ins::End);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(-1022));
    f.instruction(&Ins::I32LtS);
    f.instruction(&Ins::If(BlockType::Result(F64)));
    // two-step for subnormal results
    f.instruction(&Ins::I64Const(1));
    f.instruction(&Ins::I64Const(52));
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::F64ReinterpretI64);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1022));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::LocalSet(3));
    f.instruction(&Ins::LocalGet(3));
    f.instruction(&Ins::I32Const(1023));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64ExtendI32S);
    f.instruction(&Ins::I64Const(52));
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::F64ReinterpretI64);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::Else);
    f.instruction(&Ins::LocalGet(2));
    f.instruction(&Ins::I32Const(1023));
    f.instruction(&Ins::I32Add);
    f.instruction(&Ins::I64ExtendI32S);
    f.instruction(&Ins::I64Const(52));
    f.instruction(&Ins::I64Shl);
    f.instruction(&Ins::F64ReinterpretI64);
    f.instruction(&Ins::LocalSet(4));
    f.instruction(&Ins::LocalGet(0));
    f.instruction(&Ins::LocalGet(4));
    f.instruction(&Ins::F64Mul);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f.instruction(&Ins::End);
    f
}
