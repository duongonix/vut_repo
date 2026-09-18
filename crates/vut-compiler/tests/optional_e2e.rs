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
    let root = std::env::temp_dir().join(format!("vut-optional-e2e-{nonce}-{sequence}"));
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
fn optional_parameter_if_narrowing_does_not_leak() {
    let source = "fn describe(value: str?) -> str:\n  if value != null:\n    return value\n  \"none\"\n\nfn main():\n  out(describe(\"hi\"))\n  out(describe(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\nnone\n");
}

#[test]
fn optional_else_branch_narrows_to_present_type() {
    let source = "fn describe(value: str?) -> str:\n  text: str = if value == null:\n    \"none\"\n  else:\n    value\n  text\n\nfn main():\n  out(describe(\"hi\"))\n  out(describe(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\nnone\n");
}

#[test]
fn optional_guard_clause_narrows_after_terminating_if() {
    let source = "fn first(values: list(str?)) -> str:\n  for value in values:\n    if value == null:\n      continue\n    return value\n  \"\"\n\nfn main():\n  values: list(str?) = @()\n  values.push(\"a\")\n  values.push(null)\n  values.push(\"b\")\n  out(first(values))\n  empty: list(str?) = @()\n  empty.push(null)\n  out(first(empty))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a\n\n");
}

#[test]
fn optional_match_accepts_null_pattern() {
    let source = "fn is_none(value: str?) -> bool:\n  match value:\n    null: true\n    _: false\n\nfn main():\n  out(is_none(\"x\"))\n  out(is_none(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "false\ntrue\n");
}

#[test]
fn optional_repeated_narrowed_reads_balance_ownership() {
    let source = "fn size(value: str?) -> int:\n  if value != null:\n    return value.byte_len() + value.byte_len()\n  0\n\nfn main():\n  out(size(\"hello\"))\n  out(size(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "10\n0\n");
}

#[test]
fn optional_nested_control_flow_balances_ownership() {
    let source = "fn pick(value: str?, flag: bool) -> str:\n  if flag:\n    if value != null:\n      return value\n    else:\n      return \"absent\"\n  \"skipped\"\n\nfn main():\n  out(pick(\"a\", true))\n  out(pick(null, true))\n  out(pick(\"a\", false))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a\nabsent\nskipped\n");
}

#[test]
fn optional_bytes_narrowing_balances_ownership() {
    let source = "fn size(value: bytes?) -> int:\n  if value != null:\n    return value.len()\n  0\n\nfn main():\n  raw: list(u8) = @(1, 2, 3)\n  out(size(bytes.from_list(raw)))\n  out(size(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n0\n");
}

#[test]
fn scalar_optional_distinguishes_zero_value_from_absent() {
    let source = "fn main():\n  a: int? = 5\n  b: int? = 0\n  c: int? = null\n  if a != null:\n    out(\"a=$(a)\")\n  if b != null:\n    out(\"b=$(b)\")\n  if c == null:\n    out(\"c=absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=5\nb=0\nc=absent\n");
}

#[test]
fn scalar_optional_float_and_bool() {
    let source = "fn main():\n  f: float? = 1.5\n  g: bool? = true\n  h: bool? = null\n  if f != null:\n    out(f)\n  if g != null:\n    out(g)\n  if h == null:\n    out(\"absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1.5\ntrue\nabsent\n");
}

#[test]
fn scalar_optional_argument_and_narrowing() {
    let source = "fn plus(value: int?) -> int:\n  if value != null:\n    return value + 1\n  -1\n\nfn main():\n  x: int? = 4\n  out(plus(x))\n  out(plus(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\n-1\n");
}

#[test]
fn scalar_optional_return_is_rejected_with_e1007() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-optional-check-{nonce}"));
    fs::create_dir(&root).expect("create root");
    fs::write(
        root.join("main.vut"),
        "fn make() -> int?:\n  5\n\nfn main():\n  out(make())\n",
    )
    .expect("write source");
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        ..CompilerConfig::default()
    };
    let mut session = CompilerSession::new(config);
    let checked = session.check_source_root(&root, &[]).expect("check");
    let codes: Vec<&str> = checked
        .semantics
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref())
        .collect();
    fs::remove_dir_all(&root).ok();
    assert!(
        codes.contains(&"E1007"),
        "scalar optional return must be rejected: {codes:?}"
    );
}
