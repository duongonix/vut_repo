use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn scratch(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("{prefix}-{nonce}-{sequence}"));
    fs::create_dir_all(&root).expect("create scratch");
    root
}

fn run_with_native(source: &str, native: &[PathBuf], system: &[String]) -> (Option<i32>, String) {
    let root = scratch("vut-ffi-e2e");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        native_libraries: native.to_vec(),
        system_libraries: system.to_vec(),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .expect("emit executable");
    let output = Command::new(&executable).output().expect("run executable");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    fs::remove_dir_all(&root).ok();
    (output.status.code(), stdout)
}

fn check_codes(source: &str) -> Vec<String> {
    let root = scratch("vut-ffi-check");
    fs::write(root.join("main.vut"), source).expect("write source");
    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root, &[])
        .expect("check source");
    fs::remove_dir_all(&root).ok();
    let mut codes: Vec<String> = checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    codes.extend(
        checked
            .semantics
            .diagnostics
            .as_slice()
            .iter()
            .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned)),
    );
    codes
}

const RUST_FIXTURE: &str = r#"
#[no_mangle]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 { a + b }

#[no_mangle]
pub extern "C" fn rust_add_usize(a: usize, b: usize) -> usize { a + b }

#[no_mangle]
pub extern "C" fn native_mul(a: i32, b: i32) -> i32 { a * b }

type Callback = extern "C" fn(i32) -> i32;

#[no_mangle]
pub extern "C" fn call_callback(cb: Callback, value: i32) -> i32 { cb(value) }

pub struct Counter { value: i32 }

#[no_mangle]
pub extern "C" fn counter_create() -> *mut Counter {
    Box::into_raw(Box::new(Counter { value: 0 }))
}
#[no_mangle]
pub extern "C" fn counter_inc(counter: *mut Counter) { unsafe { (*counter).value += 1; } }
#[no_mangle]
pub extern "C" fn counter_get(counter: *mut Counter) -> i32 { unsafe { (*counter).value } }
#[no_mangle]
pub extern "C" fn counter_destroy(counter: *mut Counter) { unsafe { drop(Box::from_raw(counter)); } }

use std::sync::atomic::{AtomicUsize, Ordering};

static CREATED: AtomicUsize = AtomicUsize::new(0);
static DROPPED: AtomicUsize = AtomicUsize::new(0);

type DropFn = unsafe extern "C" fn(*mut std::ffi::c_void);

extern "C" {
    fn vut_rt_resource_new_v1(
        ptr: *mut std::ffi::c_void,
        drop_fn: Option<DropFn>,
    ) -> *mut std::ffi::c_void;
}

unsafe extern "C" fn drop_token(token: *mut std::ffi::c_void) {
    DROPPED.fetch_add(1, Ordering::Relaxed);
    if !token.is_null() {
        drop(Box::from_raw(token.cast::<u64>()));
    }
}

#[no_mangle]
pub extern "C" fn native_resource_create() -> *mut std::ffi::c_void {
    CREATED.fetch_add(1, Ordering::Relaxed);
    let token = Box::into_raw(Box::new(7_u64)).cast::<std::ffi::c_void>();
    unsafe { vut_rt_resource_new_v1(token, Some(drop_token)) }
}

#[no_mangle]
pub extern "C" fn native_resource_outstanding() -> usize {
    CREATED.load(Ordering::Relaxed) - DROPPED.load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn native_resource_value() -> u64 { 7 }

#[no_mangle]
pub extern "C" fn native_resource_use(token: *mut std::ffi::c_void) -> u64 {
    if token.is_null() {
        return 0;
    }
    unsafe { *(token.cast::<u64>()) }
}
"#;

fn rust_fixture() -> &'static Path {
    static LIB: OnceLock<PathBuf> = OnceLock::new();
    LIB.get_or_init(|| {
        let root = scratch("vut-ffi-rust");
        let source = root.join("native.rs");
        fs::write(&source, RUST_FIXTURE).expect("write rust fixture");
        let library = root.join(if cfg!(windows) {
            "ffi_native.lib"
        } else {
            "libffi_native.a"
        });
        let status = Command::new("rustc")
            .args(["--crate-type=staticlib", "--edition", "2021", "-O"])
            .arg("-o")
            .arg(&library)
            .arg(&source)
            .status()
            .expect("run rustc");
        assert!(status.success(), "rust fixture failed to compile");
        library
    })
    .as_path()
}

fn find_gnu_cc() -> Option<String> {
    for candidate in ["cc", "gcc", "clang"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return Some(candidate.to_owned());
        }
    }
    None
}

fn c_fixture() -> Option<PathBuf> {
    let compiler = find_gnu_cc()?;
    let root = scratch("vut-ffi-c");
    let source = root.join("native.c");
    fs::write(&source, "int c_add(int a, int b) { return a + b; }\n").ok()?;
    let object = root.join("native.o");
    let status = Command::new(&compiler)
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let library = root.join("libnative.a");
    let status = Command::new("ar")
        .args(["rcs"])
        .arg(&library)
        .arg(&object)
        .status()
        .ok()?;
    status.success().then_some(library)
}

#[test]
fn calls_rust_staticlib_scalar_function() {
    let source = "extern \"C\" fn rust_add(a: i32, b: i32) -> i32\nfn main():\n  unsafe:\n    out(\"$(rust_add(2, 3))\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\n");
}

#[test]
fn calls_rust_staticlib_with_usize_arguments() {
    let source = "extern \"C\" fn rust_add_usize(a: usize, b: usize) -> usize\nfn main():\n  unsafe:\n    out(\"$(rust_add_usize(40, 2))\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn link_name_overrides_the_native_symbol() {
    let source = "@link_name(\"native_mul\")\nextern \"C\" fn mul(a: i32, b: i32) -> i32\nfn main():\n  unsafe:\n    out(\"$(mul(4, 5))\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "20\n");
}

#[test]
fn passes_named_function_as_c_callback() {
    let source = "type Callback = extern \"C\" fn(i32) -> i32\nextern \"C\" fn call_callback(callback: Callback, value: i32) -> i32\nfn double(value: i32) -> i32:\n  value * 2\nfn main():\n  unsafe:\n    out(\"$(call_callback(double, 10))\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "20\n");
}

#[test]
fn passes_lambda_as_c_callback() {
    let source = "type Callback = extern \"C\" fn(i32) -> i32\nextern \"C\" fn call_callback(callback: Callback, value: i32) -> i32\nfn main():\n  unsafe:\n    out(\"$(call_callback(x => x + 1, 41))\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn opaque_handle_round_trip() {
    let source = "opaque data Counter\nextern \"C\" fn counter_create() -> ptr(Counter)\nextern \"C\" fn counter_inc(counter: ptr(Counter))\nextern \"C\" fn counter_get(counter: ptr(Counter)) -> i32\nextern \"C\" fn counter_destroy(counter: ptr(Counter))\nfn main():\n  unsafe:\n    counter = counter_create()\n    counter_inc(counter)\n    counter_inc(counter)\n    out(\"$(counter_get(counter))\")\n    counter_destroy(counter)\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "2\n");
}

#[test]
fn calls_c_staticlib_when_toolchain_available() {
    let Some(library) = c_fixture() else {
        eprintln!("skipping C FFI test: no C toolchain available");
        return;
    };
    let source = "extern \"C\" fn c_add(a: i32, b: i32) -> i32\nfn main():\n  unsafe:\n    out(\"$(c_add(20, 22))\")\n";
    let (code, stdout) = run_with_native(source, &[library], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn rejects_unknown_abi() {
    let codes = check_codes("extern \"Rust\" fn foo()\n");
    assert!(codes.contains(&"E8005".to_owned()), "{codes:?}");
}

#[test]
fn rejects_extern_function_body() {
    let codes = check_codes("extern \"C\" fn foo():\n  1\n");
    assert!(codes.contains(&"E8007".to_owned()), "{codes:?}");
}

#[test]
fn rejects_non_ffi_safe_signatures() {
    for source in [
        "extern \"C\" fn foo(value: bool)\n",
        "extern \"C\" fn foo(value: map(str, i32))\n",
        "data Point:\n  x: f32\nextern \"C\" fn foo(point: Point)\n",
        "@repr(C)\ndata Point:\n  x: f32\nextern \"C\" fn foo(point: Point)\n",
    ] {
        let codes = check_codes(source);
        assert!(
            codes.contains(&"E8004".to_owned()),
            "expected E8004 for {source:?}, got {codes:?}"
        );
    }
}

#[test]
fn accepts_official_runtime_managed_handles() {
    let codes = check_codes(
        "extern \"C\" fn a(value: str) -> bytes\nextern \"C\" fn b(value: bytes) -> str\nextern \"C\" fn c(values: list(str)) -> i32\n",
    );
    assert!(!codes.contains(&"E8004".to_owned()), "{codes:?}");
}

#[test]
fn accepts_pointer_to_repr_struct_and_opaque() {
    let codes = check_codes(
        "@repr(C)\ndata Point:\n  x: f32\nopaque data Engine\nextern \"C\" fn a(point: ptr(Point))\nextern \"C\" fn b(engine: ptr(Engine)) -> i32\n",
    );
    assert!(!codes.contains(&"E8004".to_owned()), "{codes:?}");
}

#[test]
fn extern_call_requires_unsafe() {
    let codes = check_codes(
        "extern \"C\" fn rust_add(a: i32, b: i32) -> i32\nfn main():\n  out(\"$(rust_add(1, 2))\")\n",
    );
    assert!(codes.contains(&"E8001".to_owned()), "{codes:?}");
}

#[test]
fn named_callback_parameters_parse() {
    let codes = check_codes(
        "type Callback = extern \"C\" fn(a: i32, b: ptr(void)) -> i32\nextern \"C\" fn use_it(callback: Callback)\n",
    );
    assert!(
        !codes.iter().any(|code| code.starts_with("E01")),
        "{codes:?}"
    );
}

#[test]
fn valid_extern_signatures_are_accepted() {
    let codes = check_codes(
        "extern \"C\" fn a(x: i8, y: i64, z: f32, p: ptr(u8), len: usize)\nextern \"C\" fn b() -> void\n",
    );
    assert!(!codes.contains(&"E8004".to_owned()), "{codes:?}");
}

#[test]
fn accepts_resource_handles_in_extern_signatures() {
    let codes = check_codes(
        "opaque data Token\nextern \"C\" fn make() -> resource(Token)\nextern \"C\" fn use_it(token: resource(Token))\n",
    );
    assert!(!codes.contains(&"E8004".to_owned()), "{codes:?}");
}

#[test]
fn rejects_invalid_resource_pointee() {
    let codes =
        check_codes("data NotOpaque:\n  x: f32\nextern \"C\" fn make() -> resource(NotOpaque)\n");
    assert!(codes.contains(&"E8004".to_owned()), "{codes:?}");
}

#[test]
fn resource_is_dropped_at_scope_exit() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nfn scoped():\n  unsafe:\n    token = native_resource_create()\n    out(\"in=$(native_resource_outstanding())\")\nfn main():\n  scoped()\n  unsafe:\n    out(\"out=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "in=1\nout=0\n");
}

#[test]
fn resource_moves_into_function_and_drops_once() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nfn consume(token: resource(Token)):\n  unsafe:\n    out(\"consume=$(native_resource_outstanding())\")\nfn main():\n  unsafe:\n    token = native_resource_create()\n    out(\"before=$(native_resource_outstanding())\")\n    consume(token)\n    out(\"after=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "before=1\nconsume=1\nafter=0\n");
}

#[test]
fn resource_data_field_drops_exactly_once() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\ndata Holder:\n  token: resource(Token)\nfn scoped():\n  unsafe:\n    holder = Holder(token = native_resource_create())\n    out(\"held=$(native_resource_outstanding())\")\nfn main():\n  scoped()\n  unsafe:\n    out(\"done=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "held=1\ndone=0\n");
}

#[test]
fn resource_early_return_drops_the_resource() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nfn scoped() -> int:\n  unsafe:\n    token = native_resource_create()\n    if true:\n      return 1\n  0\nfn main():\n  result = scoped()\n  unsafe:\n    out(\"out=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "out=0\n");
}

#[test]
fn resource_reassignment_drops_the_previous_value() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nfn scoped():\n  unsafe:\n    token = native_resource_create()\n    token = native_resource_create()\n    out(\"out=$(native_resource_outstanding())\")\nfn main():\n  scoped()\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "out=1\n");
}

#[test]
fn resource_local_can_be_borrowed_repeatedly() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_use(token: ptr(Token)) -> u64\nextern \"C\" fn native_resource_outstanding() -> usize\nfn borrow_twice() -> u64:\n  unsafe:\n    token = native_resource_create()\n    a = native_resource_use(token)\n    b = native_resource_use(token)\n    out(\"live=$(native_resource_outstanding())\")\n    a + b\nfn main():\n  sum = borrow_twice()\n  unsafe:\n    out(\"sum=$sum after=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "live=1\nsum=14 after=0\n");
}

#[test]
fn resource_field_can_be_borrowed() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_use(token: ptr(Token)) -> u64\nextern \"C\" fn native_resource_outstanding() -> usize\ndata Holder:\n  token: resource(Token)\nfn read_holder(holder: Holder) -> u64:\n  unsafe:\n    native_resource_use(holder.token)\nfn read_one() -> u64:\n  unsafe:\n    holder = Holder(token = native_resource_create())\n    read_holder(holder)\nfn main():\n  v = read_one()\n  unsafe:\n    out(\"v=$v after=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=7 after=0\n");
}

#[test]
fn resource_field_of_borrowed_receiver_can_be_borrowed() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_use(token: ptr(Token)) -> u64\nextern \"C\" fn native_resource_outstanding() -> usize\ndata Inner:\n  token: resource(Token)\ndata Outer:\n  inner: Inner\nfn Inner.read() -> u64:\n  unsafe:\n    native_resource_use(self.token)\nfn Outer.via() -> u64:\n  self.inner.read()\nfn run() -> u64:\n  unsafe:\n    outer = Outer(inner = Inner(token = native_resource_create()))\n    v = outer.via()\n    out(\"live=$(native_resource_outstanding())\")\n    v\nfn main():\n  v = run()\n  unsafe:\n    out(\"v=$v after=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "live=1\nv=7 after=0\n");
}

#[test]
fn resource_used_after_move_is_rejected() {
    let codes = check_codes(
        "opaque data Token\nextern \"C\" fn make() -> resource(Token)\nfn consume(token: resource(Token)):\n  0\nfn main():\n  unsafe:\n    token = make()\n  consume(token)\n  consume(token)\n",
    );
    assert!(
        codes.iter().any(|code| code == "E8009" || code == "E8010"),
        "{codes:?}"
    );
}

#[test]
fn resource_inside_branch_is_rejected() {
    let codes = check_codes(
        "opaque data Token\nextern \"C\" fn make() -> resource(Token)\nfn consume(token: resource(Token)):\n  0\nfn main():\n  unsafe:\n    token = make()\n    if true:\n      consume(token)\n",
    );
    assert!(codes.contains(&"E8011".to_owned()), "{codes:?}");
}

#[test]
fn resource_field_projection_is_rejected() {
    let codes = check_codes(
        "opaque data Token\nextern \"C\" fn make() -> resource(Token)\ndata Holder:\n  token: resource(Token)\nfn main():\n  unsafe:\n    holder = Holder(token = make())\n    value = holder.token\n",
    );
    assert!(codes.contains(&"E8012".to_owned()), "{codes:?}");
}

#[test]
fn resource_flows_through_result_and_question_operator() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nfn make_ok() -> result(resource(Token), int):\n  unsafe:\n    ok(native_resource_create())\nfn load(flag: bool) -> result(resource(Token), int):\n  if flag:\n    return make_ok()\n  err(5)\nfn run(flag: bool) -> result(int, int):\n  unsafe:\n    token = load(flag)?\n    out(\"live=$(native_resource_outstanding())\")\n  ok(1)\nfn main():\n  match run(true):\n    ok(value): out(\"ok\")\n    err(error): out(\"err=$error\")\n  unsafe:\n    out(\"after=$(native_resource_outstanding())\")\n  match run(false):\n    ok(value): out(\"ok2\")\n    err(error): out(\"err2=$error\")\n  unsafe:\n    out(\"after2=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "live=1\nok\nafter=0\nerr2=5\nafter2=0\n");
}

#[test]
fn resource_created_each_loop_iteration_drops_once() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nfn scoped():\n  unsafe:\n    for index in @(0, 1, 2):\n      token = native_resource_create()\n      out(\"iter=$(native_resource_outstanding())\")\nfn main():\n  scoped()\n  unsafe:\n    out(\"end=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "iter=1\niter=1\niter=1\nend=0\n");
}

#[test]
fn resource_crosses_await_and_drops_once() {
    let source = "opaque data Token\nextern \"C\" fn native_resource_create() -> resource(Token)\nextern \"C\" fn native_resource_outstanding() -> usize\nasync fn make_token() -> resource(Token):\n  unsafe:\n    native_resource_create()\nasync fn scoped():\n  unsafe:\n    token = await make_token()\n    out(\"live=$(native_resource_outstanding())\")\nasync fn main():\n  await scoped()\n  unsafe:\n    out(\"end=$(native_resource_outstanding())\")\n";
    let (code, stdout) = run_with_native(source, &[rust_fixture().to_owned()], &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "live=1\nend=0\n");
}
