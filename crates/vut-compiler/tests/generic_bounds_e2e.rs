use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn write_source(source: &str, prefix: &str) -> (PathBuf, PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("{prefix}-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    let path = root.join("main.vut");
    fs::write(&path, source).expect("write source");
    (root, path)
}

fn run(source: &str) -> (Option<i32>, String) {
    let (root, _path) = write_source(source, "vut-generic-bounds-e2e");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
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

fn diagnostic_codes(source: &str) -> Vec<String> {
    let (root, path) = write_source(source, "vut-generic-bounds-diag");
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session.check_source_file(Path::new(&path)).expect("check");
    let codes = checked
        .semantics
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    fs::remove_dir_all(&root).ok();
    codes
}

#[test]
fn generic_bound_method_dispatches_to_the_concrete_method() {
    let source = "interface Encodable:\n  to_json() -> int\ndata User:\n  age: int\nfn User.to_json() -> int:\n  self.age\nfn encode(T: Encodable)(value: T) -> int:\n  value.to_json()\nfn main():\n  out(\"$(encode(User(age = 7)))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn generic_bound_dispatch_specializes_per_instantiation() {
    let source = "interface Encodable:\n  to_json() -> int\ndata A:\n  n: int\nfn A.to_json() -> int:\n  self.n\ndata B:\n  n: int\nfn B.to_json() -> int:\n  self.n * 100\nfn encode(T: Encodable)(value: T) -> int:\n  value.to_json()\nfn main():\n  out(\"$(encode(A(n = 3))) $(encode(B(n = 4)))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3 400\n");
}

#[test]
fn multiple_bounds_and_nested_container_receivers_work() {
    let source = "interface Ping:\n  ping() -> int\ninterface Pong:\n  pong() -> int\ndata D:\n  n: int\nfn D.ping() -> int:\n  self.n\nfn D.pong() -> int:\n  self.n + 10\nfn use(T: Ping + Pong)(items: list(T)) -> int:\n  items.at(0).ping() + items.at(0).pong()\nfn main():\n  items: list(D) = @()\n  items.push(D(n = 5))\n  out(\"$(use(items))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "20\n");
}

#[test]
fn bound_call_lowers_to_an_ordinary_concrete_call() {
    let source = "interface Encodable:\n  to_json() -> int\ndata User:\n  age: int\nfn User.to_json() -> int:\n  self.age\nfn encode(T: Encodable)(value: T) -> int:\n  value.to_json()\nfn main():\n  out(\"$(encode(User(age = 7)))\")\n";
    let (root, path) = write_source(source, "vut-generic-bounds-mir");
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session.check_source_file(Path::new(&path)).expect("check");
    let mut calls = 0_usize;
    let mut interface_calls = 0_usize;
    for function in &checked.mir.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                match instruction {
                    vut_mir::Instruction::Call { .. } => calls += 1,
                    vut_mir::Instruction::InterfaceCall { .. } => interface_calls += 1,
                    _ => {}
                }
            }
        }
    }
    fs::remove_dir_all(&root).ok();
    assert!(calls > 0, "expected an ordinary call for the bound method");
    assert_eq!(
        interface_calls, 0,
        "bound dispatch must not introduce an interface/vtable call"
    );
}

#[test]
fn unsatisfied_bound_reports_missing_method() {
    let codes = diagnostic_codes(
        "interface Ping:\n  ping() -> int\ndata D:\n  n: int\nfn other(T: Ping)(value: T) -> int:\n  value.ping()\nfn main():\n  out(\"$(other(42))\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1017"), "{codes:?}");
}

#[test]
fn method_not_declared_by_any_bound_reports_e1018() {
    let codes = diagnostic_codes(
        "interface Encodable:\n  to_json() -> int\ndata User:\n  age: int\nfn User.to_json() -> int:\n  self.age\nfn bad(T: Encodable)(value: T) -> int:\n  value.missing()\nfn main():\n  out(\"$(bad(User(age = 1)))\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1018"), "{codes:?}");
    assert!(!codes.iter().any(|code| code == "E2005"), "{codes:?}");
}

#[test]
fn conflicting_bounds_report_e1019() {
    let codes = diagnostic_codes(
        "interface A:\n  m() -> int\ninterface B:\n  m() -> str\ndata D:\n  n: int\nfn D.m() -> int:\n  self.n\nfn use(T: A + B)(value: T) -> int:\n  value.m()\n",
    );
    assert!(codes.iter().any(|code| code == "E1019"), "{codes:?}");
}
