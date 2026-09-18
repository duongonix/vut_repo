//! `dyn` runtime value tests: boxing, transport, ownership, and display.
//!
//! A clean exit also means the leak detector found no live managed allocation.
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

fn run(source: &str) -> (Option<i32>, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-dyn-e2e-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
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

#[test]
fn dyn_scalar_assignment_and_reassignment() {
    let source = "fn main():\n  value: dyn = 10\n  out(value)\n  value = \"hello\"\n  out(value)\n  value = true\n  out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n<dyn>\n<dyn>\n");
}

#[test]
fn dyn_str_payload_is_leak_free() {
    let source = "fn main():\n  value: dyn = \"hello\"\n  out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n");
}

#[test]
fn dyn_data_payload_is_leak_free() {
    let source = "data Person:\n  name: str\n\nfn main():\n  person = Person(name = \"ann\")\n  value: dyn = person\n  out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n");
}

#[test]
fn dyn_enum_payload_is_leak_free() {
    let source = "enum Payload:\n  none\n  text(value: str)\n\nfn main():\n  value: dyn = Payload.text(value = \"hi\")\n  out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n");
}

#[test]
fn dyn_argument_and_return() {
    let source = "fn accept(value: dyn):\n  out(value)\n\nfn make() -> dyn:\n  42\n\nfn main():\n  accept(7)\n  accept(\"hi\")\n  out(make())\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n<dyn>\n<dyn>\n");
}

#[test]
fn dyn_copy_is_leak_free() {
    let source = "fn main():\n  original: dyn = \"shared\"\n  copy: dyn = original\n  out(original)\n  out(copy)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n<dyn>\n");
}

#[test]
fn dyn_null_is_absent_handle() {
    let source = "fn main():\n  value: dyn = null\n  out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n");
}

#[test]
fn dyn_heterogeneous_list() {
    let source = "fn main():\n  values: list(dyn) = @(1, \"hello\", true)\n  out(\"$(values.len())\")\n  for value in values:\n    out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n<dyn>\n<dyn>\n<dyn>\n");
}

#[test]
fn dyn_data_field_is_leak_free() {
    let source = "data Box:\n  value: dyn\n\nfn main():\n  boxed = Box(value = \"payload\")\n  out(boxed.value)\n  other = Box(value = 3)\n  out(other.value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n<dyn>\n");
}

#[test]
fn dyn_map_value_is_leak_free() {
    let source = "fn main():\n  table: map(str, dyn) = map((\"n\", 1), (\"s\", \"two\"))\n  entry: dyn? = table.get(\"s\")\n  if entry != null:\n    out(entry)\n  table.set(\"b\", true)\n  out(\"$(table.len())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n3\n");
}

#[test]
fn dyn_list_of_managed_payloads_is_leak_free() {
    let source = "fn main():\n  values: list(dyn) = @(\"alpha\", \"beta\")\n  for value in values:\n    out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n<dyn>\n");
}

#[test]
fn dyn_branch_return_and_scope_cleanup() {
    let source = "fn pick(flag: bool) -> dyn:\n  if flag:\n    return \"yes\"\n  return 1\n\nfn main():\n  out(pick(true))\n  out(pick(false))\n  if true:\n    scoped: dyn = \"scoped\"\n    out(scoped)\n  out(\"done\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n<dyn>\n<dyn>\ndone\n");
}

#[test]
fn dyn_reassignment_across_categories_is_leak_free() {
    let source = "fn main():\n  value: dyn = \"start\"\n  value = 10\n  value = true\n  value = \"end\"\n  value = 3.5\n  out(value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "<dyn>\n");
}
