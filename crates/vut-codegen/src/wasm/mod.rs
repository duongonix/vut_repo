//! Minimal WebAssembly backend (WASM.3–WASM.6).
//!
//! Lowers the supported MIR subset into a final `.wasm` module, with a small
//! runtime (strings, bytes, list, map, print/out via direct WASI `fd_write`).
//! Unsupported MIR fails with a clear diagnostic rather than miscompiling.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::map_unwrap_or,
    clippy::manual_is_multiple_of
)]
mod asyncrt;
mod bindings;
mod bytes_ext;
mod emit;
mod fmt;
mod layout;
mod listx;
mod mapx;
mod math;
mod rc;
mod runtime;
mod stdlib;
mod str_ops;
mod strings;
use crate::{CodegenError, Target};
pub use bindings::generate as generate_bindings;
pub use layout::{is_wasm_target, valtype, wasm_signature};
use std::collections::HashMap;
use vut_mir::{ExternalFunction, Function, Instruction, Program, SymbolId};
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function as WasmFunction,
    FunctionSection, GlobalSection, GlobalType, ImportSection, Instruction as Ins, MemArg,
    MemorySection, MemoryType, Module, TypeSection, ValType,
};
const PAGE: i32 = 65536;
const NEWLINE_ADDR: i32 = 12;

/// WASI preview1 import names recognized on `extern "C"` functions.
const WASI_IMPORTS: &[&str] = &[
    "args_get",
    "args_sizes_get",
    "environ_get",
    "environ_sizes_get",
    "clock_res_get",
    "clock_time_get",
    "fd_close",
    "fd_fdstat_get",
    "fd_prestat_get",
    "fd_prestat_dir_name",
    "fd_read",
    "fd_seek",
    "fd_write",
    "path_open",
    "poll_oneoff",
    "proc_exit",
    "random_get",
    "sched_yield",
];

/// The set of program functions reachable from `entry` through calls, closures,
/// and spawns. Unreachable functions (and their externs) are dropped.
fn reachable_functions(program: &Program, entry: SymbolId) -> std::collections::HashSet<SymbolId> {
    let defined: std::collections::HashSet<SymbolId> = program
        .functions
        .iter()
        .map(|function| function.symbol)
        .collect();
    let mut reachable = std::collections::HashSet::new();
    let mut stack = vec![entry];
    while let Some(symbol) = stack.pop() {
        if !reachable.insert(symbol) {
            continue;
        }
        if let Some(function) = program
            .functions
            .iter()
            .find(|function| function.symbol == symbol)
        {
            for block in &function.blocks {
                for instruction in &block.instructions {
                    let target = match instruction {
                        Instruction::Call { target, .. }
                        | Instruction::StartFuture { target, .. } => Some(*target),
                        Instruction::MakeFunction { symbol, .. }
                        | Instruction::MakeClosure { symbol, .. }
                        | Instruction::Spawn {
                            callable: symbol, ..
                        } => Some(*symbol),
                        _ => None,
                    };
                    if let Some(target) = target
                        && defined.contains(&target)
                    {
                        stack.push(target);
                    }
                }
            }
        }
    }
    reachable
}
fn import_module(link_name: &str) -> &'static str {
    if WASI_IMPORTS.contains(&link_name) {
        "wasi_snapshot_preview1"
    } else {
        "env"
    }
}

/// Returns the standard-library module name when `link_name` is a native runtime
/// import (`vut_rt_<module>_...`) that the wasm runtime does not provide.
fn unsupported_runtime_module(link_name: &str) -> Option<&str> {
    let rest = link_name.strip_prefix("vut_rt_")?;
    Some(rest.split('_').next().unwrap_or(rest))
}

/// Absolute indices of the emitted runtime helpers.
pub(super) struct Runtime {
    pub alloc: u32,
    pub print: u32,
    pub out: u32,
    pub format: u32,
    pub concat: u32,
    pub list_new: u32,
    pub list_len: u32,
    pub list_push: u32,
    pub list_at: u32,
    pub list_set: u32,
    pub list_pop: u32,
    pub list_clear: u32,
    pub bytes_new: u32,
    pub bytes_len: u32,
    pub bytes_at: u32,
    pub bytes_set: u32,
    pub bytes_from_str: u32,
    pub bytes_to_str: u32,
    pub map_new: u32,
    pub map_len: u32,
    pub map_set: u32,
    pub map_get: u32,
    pub map_contains: u32,
    pub map_remove: u32,
    pub map_clear: u32,
    pub str_from_static: u32,
    pub str_retain: u32,
    pub str_release: u32,
    pub bytes_retain: u32,
    pub bytes_release: u32,
    pub bytes_make_unique: u32,
    pub list_retain: u32,
    pub list_release: u32,
    pub list_make_unique: u32,
    pub map_retain: u32,
    pub map_release: u32,
    pub str_eq: u32,
    pub str_cmp: u32,
    pub str_contains: u32,
    pub str_starts: u32,
    pub str_ends: u32,
    pub str_find: u32,
    pub str_rfind: u32,
    pub str_substring: u32,
    pub str_repeat: u32,
    pub str_strip_prefix: u32,
    pub str_strip_suffix: u32,
    pub str_trim_start: u32,
    pub str_trim_end: u32,
    pub str_lower: u32,
    pub str_upper: u32,
    pub bt_clear: u32,
    pub bt_capacity: u32,
    pub bt_reserve: u32,
    pub bt_push: u32,
    pub bt_extend: u32,
    pub bt_truncate: u32,
    pub bt_resize: u32,
    pub bt_first: u32,
    pub bt_last: u32,
    pub bt_slice: u32,
    pub bt_find: u32,
    pub bt_starts: u32,
    pub bt_ends: u32,
    pub bt_cmp: u32,
    pub ls_capacity: u32,
    pub ls_reserve: u32,
    pub ls_truncate: u32,
    pub ls_swap: u32,
    pub ls_reverse: u32,
    pub ls_shrink: u32,
    pub ls_first: u32,
    pub ls_last: u32,
    pub ls_insert: u32,
    pub ls_remove: u32,
    pub ls_extend: u32,
    pub ls_slice: u32,
    pub ls_contains: u32,
    pub ls_find: u32,
    pub ls_elem_eq: u32,
    pub ls_join: u32,
    pub str_to_i64: u32,
    pub str_to_f64: u32,
    pub so_split: u32,
    pub so_lines: u32,
    pub so_replace: u32,
    pub so_char_at: u32,
    pub so_chars: u32,
    pub so_pad_left: u32,
    pub so_pad_right: u32,
    pub so_split_ws: u32,
    pub bt_to_hex: u32,
    pub bt_to_list: u32,
    pub bt_from_list: u32,
    pub bt_read_int: u32,
    pub bt_write_int: u32,
    pub fmt_f64: u32,
    pub map_capacity: u32,
    pub map_reserve: u32,
}
impl Runtime {
    #[expect(clippy::too_many_lines, reason = "one line per runtime helper index")]
    fn new(base: u32) -> Self {
        use runtime::idx;
        Self {
            alloc: base + idx::ALLOC,
            print: base + idx::PRINT,
            out: base + idx::OUT,
            format: base + idx::FORMAT,
            concat: base + idx::CONCAT,
            list_new: base + idx::LIST_NEW,
            list_len: base + idx::LIST_LEN,
            list_push: base + idx::LIST_PUSH,
            list_at: base + idx::LIST_AT,
            list_set: base + idx::LIST_SET,
            list_pop: base + idx::LIST_POP,
            list_clear: base + idx::LIST_CLEAR,
            bytes_new: base + idx::BYTES_NEW,
            bytes_len: base + idx::BYTES_LEN,
            bytes_at: base + idx::BYTES_AT,
            bytes_set: base + idx::BYTES_SET,
            bytes_from_str: base + idx::BYTES_FROM_STR,
            bytes_to_str: base + idx::BYTES_TO_STR,
            map_new: base + idx::MAP_NEW,
            map_len: base + idx::MAP_LEN,
            map_set: base + idx::MAP_SET,
            map_get: base + idx::MAP_GET,
            map_contains: base + idx::MAP_CONTAINS,
            map_remove: base + idx::MAP_REMOVE,
            map_clear: base + idx::MAP_CLEAR,
            str_from_static: base + idx::STR_FROM_STATIC,
            str_retain: base + idx::STR_RETAIN,
            str_release: base + idx::STR_RELEASE,
            bytes_retain: base + idx::BYTES_RETAIN,
            bytes_release: base + idx::BYTES_RELEASE,
            bytes_make_unique: base + idx::BYTES_MAKE_UNIQUE,
            list_retain: base + idx::LIST_RETAIN,
            list_release: base + idx::LIST_RELEASE,
            list_make_unique: base + idx::LIST_MAKE_UNIQUE,
            map_retain: base + idx::MAP_RETAIN,
            map_release: base + idx::MAP_RELEASE,
            str_eq: base + idx::STR_EQ,
            str_cmp: base + idx::STR_CMP,
            str_contains: base + idx::STR_CONTAINS,
            str_starts: base + idx::STR_STARTS,
            str_ends: base + idx::STR_ENDS,
            str_find: base + idx::STR_FIND,
            str_rfind: base + idx::STR_RFIND,
            str_substring: base + idx::STR_SUBSTRING,
            str_repeat: base + idx::STR_REPEAT,
            str_strip_prefix: base + idx::STR_STRIP_PREFIX,
            str_strip_suffix: base + idx::STR_STRIP_SUFFIX,
            str_trim_start: base + idx::STR_TRIM_START,
            str_trim_end: base + idx::STR_TRIM_END,
            str_lower: base + idx::STR_LOWER,
            str_upper: base + idx::STR_UPPER,
            bt_clear: base + idx::BT_CLEAR,
            bt_capacity: base + idx::BT_CAPACITY,
            bt_reserve: base + idx::BT_RESERVE,
            bt_push: base + idx::BT_PUSH,
            bt_extend: base + idx::BT_EXTEND,
            bt_truncate: base + idx::BT_TRUNCATE,
            bt_resize: base + idx::BT_RESIZE,
            bt_first: base + idx::BT_FIRST,
            bt_last: base + idx::BT_LAST,
            bt_slice: base + idx::BT_SLICE,
            bt_find: base + idx::BT_FIND,
            bt_starts: base + idx::BT_STARTS,
            bt_ends: base + idx::BT_ENDS,
            bt_cmp: base + idx::BT_CMP,
            ls_capacity: base + idx::LS_CAPACITY,
            ls_reserve: base + idx::LS_RESERVE,
            ls_truncate: base + idx::LS_TRUNCATE,
            ls_swap: base + idx::LS_SWAP,
            ls_reverse: base + idx::LS_REVERSE,
            ls_shrink: base + idx::LS_SHRINK,
            ls_first: base + idx::LS_FIRST,
            ls_last: base + idx::LS_LAST,
            ls_insert: base + idx::LS_INSERT,
            ls_remove: base + idx::LS_REMOVE,
            ls_extend: base + idx::LS_EXTEND,
            ls_slice: base + idx::LS_SLICE,
            ls_contains: base + idx::LS_CONTAINS,
            ls_find: base + idx::LS_FIND,
            ls_elem_eq: base + idx::LS_ELEM_EQ,
            ls_join: base + idx::LS_JOIN,
            str_to_i64: base + idx::STR_TO_I64,
            str_to_f64: base + idx::STR_TO_F64,
            so_split: base + idx::SO_SPLIT,
            so_lines: base + idx::SO_LINES,
            so_replace: base + idx::SO_REPLACE,
            so_char_at: base + idx::SO_CHAR_AT,
            so_chars: base + idx::SO_CHARS,
            so_pad_left: base + idx::SO_PAD_LEFT,
            so_pad_right: base + idx::SO_PAD_RIGHT,
            so_split_ws: base + idx::SO_SPLIT_WS,
            bt_to_hex: base + idx::BT_TO_HEX,
            bt_to_list: base + idx::BT_TO_LIST,
            bt_from_list: base + idx::BT_FROM_LIST,
            bt_read_int: base + idx::BT_READ_INT,
            bt_write_int: base + idx::BT_WRITE_INT,
            fmt_f64: base + idx::FMT_F64,
            map_capacity: base + idx::MAP_CAPACITY,
            map_reserve: base + idx::MAP_RESERVE,
        }
    }
}

/// The WebAssembly code generator.
pub struct WasmBackend {
    target: Target,
}
impl WasmBackend {
    /// Creates a wasm backend for `target`.
    #[must_use]
    pub fn new(target: Target) -> Self {
        Self { target }
    }

    /// Compiles `program` into a final wasm module exporting every function as
    /// `vut_fn_<symbol>`, the entry as `main`, `memory`, and `_start`.
    ///
    /// # Errors
    /// Returns an error for a non-wasm target, or for unsupported MIR.
    #[expect(
        clippy::too_many_lines,
        reason = "wasm module assembly is intentionally linear and self-contained"
    )]
    pub fn compile_executable_module(
        &self,
        program: &Program,
        entry: SymbolId,
    ) -> Result<Vec<u8>, CodegenError> {
        if !layout::is_wasm_target(&self.target.triple) {
            return Err(CodegenError::InvalidTarget(format!(
                "wasm backend cannot target `{}`",
                self.target.triple
            )));
        }
        let layouts = &program.layouts;
        let externals = &program.external_functions;
        // Only functions reachable from the entry (and their externs) are needed.
        let reachable = reachable_functions(program, entry);
        let functions: Vec<&Function> = program
            .functions
            .iter()
            .filter(|function| reachable.contains(&function.symbol))
            .collect();
        // Only externs that are actually referenced need an import/implementation.
        let mut called: std::collections::HashSet<SymbolId> = std::collections::HashSet::new();
        for function in &functions {
            for block in &function.blocks {
                for instruction in &block.instructions {
                    match instruction {
                        Instruction::Call { target, .. }
                        | Instruction::StartFuture { target, .. } => {
                            called.insert(*target);
                        }
                        Instruction::MakeFunction { symbol, .. }
                        | Instruction::MakeClosure { symbol, .. }
                        | Instruction::Spawn {
                            callable: symbol, ..
                        } => {
                            called.insert(*symbol);
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut supported: Vec<&ExternalFunction> = Vec::new();
        let mut remaining: Vec<&ExternalFunction> = Vec::new();
        for external in externals {
            if !called.contains(&external.symbol) {
                continue;
            }
            if stdlib::is_supported(&external.link_name) {
                supported.push(external);
            } else if let Some(module) = unsupported_runtime_module(&external.link_name) {
                return Err(CodegenError::Backend(format!(
                    "this program uses a standard-library capability (`{module}`) that is not \
                     available on wasm32-wasip1: native runtime import `{}` has no wasm mapping",
                    external.link_name
                )));
            } else {
                remaining.push(external);
            }
        }
        let function_count = functions.len() as u32;
        let wasi_count = stdlib::WASI_IMPORTS.len() as u32;
        let import_count = wasi_count + remaining.len() as u32;
        let runtime_base = import_count + function_count;
        let runtime = Runtime::new(runtime_base);
        let stdlib_base = runtime_base + runtime::idx::COUNT;
        let start_index = stdlib_base + stdlib::body_count();
        let async_base = start_index + 1;
        // Resume thunks for async bodies without a state machine (no awaits):
        // a `(frame, out) -> i32` poll entry that calls the body once.
        let thunk_base = async_base + asyncrt::idx::COUNT;
        let async_thunks: Vec<(SymbolId, u32, Option<ValType>)> = functions
            .iter()
            .enumerate()
            .filter(|(_, function)| {
                function.frame_param.is_some()
                    && !function.is_poll
                    && program.frames.contains_key(&function.symbol)
            })
            .map(|(index, function)| {
                (
                    function.symbol,
                    import_count + index as u32,
                    wasm_signature(layouts, function).1,
                )
            })
            .collect();
        let (data, strings) = collect_strings(program);
        let pages = u32::try_from(data.len().div_ceil(PAGE as usize).max(1)).unwrap_or(1);
        let heap_init = align16(data.len().max(1024) as i32);
        let fn_sigs = runtime::signatures();
        let mut types = TypeSection::new();
        let mut signatures: HashMap<SymbolId, u32> = HashMap::new();
        for (index, function) in functions.iter().enumerate() {
            let (params, result) = wasm_signature(layouts, function);
            types
                .ty()
                .function(params.iter().copied(), result.iter().copied());
            signatures.insert(function.symbol, index as u32);
        }
        let external_type_base = function_count;
        let mut external_types: HashMap<SymbolId, u32> = HashMap::new();
        for (index, external) in externals.iter().enumerate() {
            let params: Vec<ValType> = external
                .parameters
                .iter()
                .map(|ty| valtype(layouts, *ty))
                .collect();
            let results: Vec<ValType> = external
                .return_type
                .filter(|ty| !matches!(layouts.types[ty.0].repr, vut_mir::ValueRepr::Void))
                .map(|ty| vec![valtype(layouts, ty)])
                .unwrap_or_default();
            types.ty().function(params, results);
            external_types.insert(external.symbol, external_type_base + index as u32);
        }
        let wasi_type0 = external_type_base + externals.len() as u32;
        types.ty().function(
            [ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            [ValType::I32],
        );
        let wasi_type1 = wasi_type0 + 1;
        types
            .ty()
            .function([ValType::I32, ValType::I32], [ValType::I32]);
        let wasi_type2 = wasi_type1 + 1;
        types
            .ty()
            .function([ValType::I32, ValType::I64, ValType::I32], [ValType::I32]);
        let runtime_type_base = wasi_type2 + 1;
        for (params, results) in &fn_sigs {
            types
                .ty()
                .function(params.iter().copied(), results.iter().copied());
        }
        let stdlib_type_base = runtime_type_base + fn_sigs.len() as u32;
        for (params, results) in stdlib::signatures() {
            types.ty().function(params, results);
        }
        let start_type = stdlib_type_base + stdlib::body_count();
        types.ty().function([], []);
        // Indirect-callable signatures.
        let mut callable_types: HashMap<vut_hir::TypeId, u32> = HashMap::new();
        for (index, (ty, (params, result))) in layouts.callables.iter().enumerate() {
            let params: Vec<ValType> = params.iter().map(|ty| valtype(layouts, *ty)).collect();
            let results: Vec<ValType> =
                if matches!(layouts.types[result.0].repr, vut_mir::ValueRepr::Void) {
                    Vec::new()
                } else {
                    vec![valtype(layouts, *result)]
                };
            types.ty().function(params, results);
            callable_types.insert(*ty, start_type + 1 + index as u32);
        }
        // Async helper types: the `call_indirect` signatures for a poll body
        // `(frame, out) -> i32` and a drop thunk `(frame) -> ()`, followed by
        // the async helper signatures. Emitted after the callable types so the
        // indices line up with `async_type_base`.
        let async_poll_type = start_type + 1 + layouts.callables.len() as u32;
        types
            .ty()
            .function([ValType::I32, ValType::I32], [ValType::I32]);
        let async_drop_type = async_poll_type + 1;
        types.ty().function([ValType::I32], []);
        let async_type_base = async_drop_type + 1;
        for (params, results) in asyncrt::signatures() {
            types.ty().function(params, results);
        }
        let mut module = Module::new();
        module.section(&types);
        let mut imports = ImportSection::new();
        let mut wasi_map: HashMap<&'static str, u32> = HashMap::new();
        for (index, import) in stdlib::WASI_IMPORTS.iter().enumerate() {
            wasi_map.insert(import.name, index as u32);
            let type_index = match import.signature {
                0 => wasi_type0,
                1 => wasi_type1,
                _ => wasi_type2,
            };
            imports.import(
                "wasi_snapshot_preview1",
                import.name,
                wasm_encoder::EntityType::Function(type_index),
            );
        }
        for external in &remaining {
            imports.import(
                import_module(&external.link_name),
                &external.link_name,
                wasm_encoder::EntityType::Function(external_types[&external.symbol]),
            );
        }
        module.section(&imports);
        let mut function_section = FunctionSection::new();
        for function in &functions {
            function_section.function(signatures[&function.symbol]);
        }
        for index in 0..fn_sigs.len() as u32 {
            function_section.function(runtime_type_base + index);
        }
        for index in 0..stdlib::body_count() {
            function_section.function(stdlib_type_base + index);
        }
        function_section.function(start_type);
        for index in 0..asyncrt::idx::COUNT {
            function_section.function(async_type_base + index);
        }
        for _ in 0..async_thunks.len() {
            function_section.function(async_poll_type);
        }
        module.section(&function_section);
        // A funcref table whose entry `i` is function index `i`, so a closure's
        // code pointer can simply be its absolute function index.
        let total_functions = thunk_base + async_thunks.len() as u32;
        let mut table = wasm_encoder::TableSection::new();
        table.table(wasm_encoder::TableType {
            element_type: wasm_encoder::RefType::FUNCREF,
            minimum: u64::from(total_functions),
            maximum: Some(u64::from(total_functions)),
            table64: false,
            shared: false,
        });
        module.section(&table);
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: u64::from(pages),
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);
        let mut globals = GlobalSection::new();
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(heap_init),
        );
        // Global 1: the async handle currently being polled (await target).
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(0),
        );
        module.section(&globals);
        let mut exports = ExportSection::new();
        for (index, function) in functions.iter().enumerate() {
            exports.export(
                &format!("vut_fn_{}", function.symbol.0),
                ExportKind::Func,
                import_count + index as u32,
            );
        }
        if let Some(index) = functions.iter().position(|f| f.symbol == entry) {
            exports.export("main", ExportKind::Func, import_count + index as u32);
        }
        exports.export("vut_alloc", ExportKind::Func, runtime.alloc);
        exports.export("memory", ExportKind::Memory, 0);
        exports.export("_start", ExportKind::Func, start_index);
        module.section(&exports);
        let indices: Vec<u32> = (0..total_functions).collect();
        let mut elements = wasm_encoder::ElementSection::new();
        elements.active(
            None,
            &ConstExpr::i32_const(0),
            wasm_encoder::Elements::Functions(indices.into()),
        );
        module.section(&elements);
        let mut code = CodeSection::new();
        let mut symbol_index: HashMap<SymbolId, u32> = functions
            .iter()
            .enumerate()
            .map(|(index, function)| (function.symbol, import_count + index as u32))
            .collect();
        for (index, external) in remaining.iter().enumerate() {
            symbol_index.insert(external.symbol, wasi_count + index as u32);
        }
        for external in &supported {
            if let Some(index) = stdlib::helper_index(stdlib_base, &external.link_name) {
                symbol_index.insert(external.symbol, index);
            }
        }
        let mut callee_params: HashMap<SymbolId, Vec<ValType>> = HashMap::new();
        for function in &functions {
            callee_params.insert(function.symbol, wasm_signature(layouts, function).0);
        }
        for external in externals {
            callee_params.insert(
                external.symbol,
                external
                    .parameters
                    .iter()
                    .map(|ty| valtype(layouts, *ty))
                    .collect(),
            );
        }
        let mut poll_thunks: HashMap<SymbolId, u32> = HashMap::new();
        for (index, function) in functions.iter().enumerate() {
            if function.is_poll {
                poll_thunks.insert(function.symbol, import_count + index as u32);
            }
        }
        for (offset, (symbol, _, _)) in async_thunks.iter().enumerate() {
            poll_thunks.insert(*symbol, thunk_base + offset as u32);
        }
        let async_rt = asyncrt::AsyncRt::new(async_base);
        let ctx = emit::LowerCtx {
            layouts,
            symbols: &symbol_index,
            strings: &strings,
            callee_params: &callee_params,
            callable_types: &callable_types,
            rt: &runtime,
            frames: &program.frames,
            poll_thunks: &poll_thunks,
            async_rt: &async_rt,
        };
        for function in &functions {
            code.function(&emit::function_body(&ctx, function)?);
        }
        for body in runtime::bodies(runtime_base) {
            code.function(&body);
        }
        for body in stdlib::bodies(
            stdlib_base,
            wasi_map,
            runtime.alloc,
            runtime_base + runtime::idx::BYTES_NEW,
        ) {
            code.function(&body);
        }
        let mut start = WasmFunction::new([(2, ValType::I32)]);
        if let Some(index) = functions.iter().position(|f| f.symbol == entry) {
            let entry_fn = functions[index];
            let entry_index = import_count + index as u32;
            let frame_size = program.frames.get(&entry).map_or(0, |layout| layout.size);
            if entry_fn.is_poll {
                // Async `main`: allocate a root frame and handle, then drive the
                // single-threaded executor to completion.
                start.instruction(&Ins::I32Const(frame_size as i32));
                start.instruction(&Ins::Call(async_rt.frame_alloc));
                start.instruction(&Ins::LocalSet(0));
                start.instruction(&Ins::I32Const(entry_index as i32));
                start.instruction(&Ins::I32Const(0));
                start.instruction(&Ins::LocalGet(0));
                start.instruction(&Ins::I32Const(0));
                start.instruction(&Ins::Call(async_rt.handle_new));
                start.instruction(&Ins::Call(async_rt.executor_run));
                start.instruction(&Ins::Drop);
            } else if entry_fn.frame_param.is_some() {
                // Async `main` with no suspension: a single frame-based call.
                start.instruction(&Ins::I32Const(frame_size as i32));
                start.instruction(&Ins::Call(async_rt.frame_alloc));
                start.instruction(&Ins::Call(entry_index));
                let (_, result) = wasm_signature(layouts, entry_fn);
                if result.is_some() {
                    start.instruction(&Ins::Drop);
                }
            } else {
                start.instruction(&Ins::Call(entry_index));
                let (_, result) = wasm_signature(layouts, entry_fn);
                if result.is_some() {
                    start.instruction(&Ins::Drop);
                }
            }
        }
        start.instruction(&Ins::End);
        code.function(&start);
        for body in asyncrt::bodies(
            async_base,
            runtime.alloc,
            async_poll_type,
            async_drop_type,
            1,
        ) {
            code.function(&body);
        }
        for (_, body, result) in &async_thunks {
            code.function(&asyncrt::resume_thunk(*body, *result));
        }
        module.section(&code);
        let mut data_section = DataSection::new();
        data_section.active(0, &ConstExpr::i32_const(0), data);
        module.section(&data_section);
        // Host-neutral InterfaceMetadata (custom section `vut.interface`).
        let metadata = interface_metadata(program, &functions, entry);
        module.section(&wasm_encoder::CustomSection {
            name: "vut.interface".into(),
            data: metadata.as_bytes().into(),
        });
        Ok(module.finish())
    }
}
fn align16(value: i32) -> i32 {
    (value + 15) & !15
}

/// Host-neutral `InterfaceMetadata` describing exports and imports. Kept
/// host-agnostic so it can later map to JS, TypeScript, or WIT without a
/// compiler ABI redesign.
fn interface_metadata(program: &Program, functions: &[&Function], entry: SymbolId) -> String {
    let layouts = &program.layouts;
    let mut exports = Vec::new();
    for function in functions {
        let (params, result) = wasm_signature(layouts, function);
        let params = params
            .iter()
            .map(|param| format!("\"{}\"", valtype_name(*param)))
            .collect::<Vec<_>>()
            .join(",");
        let result = result.map_or_else(
            || "null".to_owned(),
            |result| format!("\"{}\"", valtype_name(result)),
        );
        exports.push(format!(
            "{{\"name\":\"vut_fn_{}\",\"params\":[{params}],\"result\":{result}}}",
            function.symbol.0
        ));
    }
    let mut imports =
        vec!["{\"module\":\"wasi_snapshot_preview1\",\"name\":\"fd_write\"}".to_owned()];
    for external in &program.external_functions {
        imports.push(format!(
            "{{\"module\":\"{}\",\"name\":\"{}\"}}",
            import_module(&external.link_name),
            external.link_name
        ));
    }
    format!(
        "{{\"schema\":1,\"entry\":\"vut_fn_{}\",\"exports\":[{}],\"imports\":[{}]}}",
        entry.0,
        exports.join(","),
        imports.join(",")
    )
}
fn valtype_name(ty: ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        _ => "other",
    }
}

/// Scratch `0..128`, then each string literal as `[len: u32][bytes]` padded.
fn collect_strings(program: &Program) -> (Vec<u8>, HashMap<String, u32>) {
    let mut data = vec![0_u8; 128];
    data[NEWLINE_ADDR as usize] = b'\n';
    let mut map = HashMap::new();
    for function in &program.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Instruction::ConstString { literal, .. } = instruction
                    && !map.contains_key(literal)
                {
                    let address = data.len() as u32;
                    data.extend_from_slice(&(literal.len() as u32).to_le_bytes());
                    data.extend_from_slice(literal.as_bytes());
                    while data.len() % 4 != 0 {
                        data.push(0);
                    }
                    map.insert(literal.clone(), address);
                }
            }
        }
    }
    // `bool.to_str()` needs these regardless of source literals.
    for literal in ["true", "false"] {
        if !map.contains_key(literal) {
            let address = data.len() as u32;
            data.extend_from_slice(&(literal.len() as u32).to_le_bytes());
            data.extend_from_slice(literal.as_bytes());
            while data.len() % 4 != 0 {
                data.push(0);
            }
            map.insert(literal.to_owned(), address);
        }
    }
    (data, map)
}

/// Emits a plain object (same as a final module here) for `program`.
///
/// # Errors
/// Returns an error when lowering fails.
pub fn compile_object(program: &Program) -> Result<Vec<u8>, CodegenError> {
    let entry = program
        .functions
        .first()
        .map_or_else(|| SymbolId(0), |function| function.symbol);
    WasmBackend::new(Target::portable("wasm32-wasip1")).compile_executable_module(program, entry)
}

/// Returns the wasm signature list for all functions (used by tests).
#[must_use]
pub fn function_types(program: &Program) -> Vec<(Vec<ValType>, Option<ValType>)> {
    program
        .functions
        .iter()
        .map(|function: &Function| wasm_signature(&program.layouts, function))
        .collect()
}

/// A `MemArg` for `offset` with no alignment assumption.
fn memarg(offset: i64) -> MemArg {
    MemArg {
        offset: u64::try_from(offset).unwrap_or(0),
        align: 0,
        memory_index: 0,
    }
}
