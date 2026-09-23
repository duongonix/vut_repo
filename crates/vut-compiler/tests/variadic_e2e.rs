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
    let root = std::env::temp_dir().join(format!("vut-variadic-e2e-{nonce}-{sequence}"));
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

fn diagnostic_codes(source: &str) -> Vec<String> {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-variadic-diag-{nonce}"));
    fs::create_dir(&root).expect("create root");
    let path = root.join("main.vut");
    fs::write(&path, source).expect("write source");
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
fn variadic_int_accepts_zero_or_more_arguments() {
    let source = "fn sum(...values: int) -> int:\n  total = 0\n  for value in values:\n    total = total + value\n  total\n\nfn main():\n  out(\"zero=$(sum())\")\n  out(\"four=$(sum(1, 2, 3, 4))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "zero=0\nfour=10\n");
}

#[test]
fn variadic_supports_len_at_iteration_and_spread() {
    let source = "fn sum(...values: int) -> int:\n  total = 0\n  for value in values:\n    total = total + value\n  total\n\nfn first(...values: int) -> int:\n  if values.len() == 0:\n    return 0\n  values.at(0)\n\nfn main():\n  items = @[10, 20, 30]\n  spread = sum(...items)\n  out(\"spread=$spread\")\n  head = first(7, 8)\n  out(\"head=$head\")\n  empty = first()\n  out(\"empty=$empty\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "spread=60\nhead=7\nempty=0\n");
}

#[test]
fn variadic_str_elements_and_forwarding_do_not_leak() {
    let source = "fn join(...parts: str) -> str:\n  out = \"\"\n  for part in parts:\n    out = out + part\n  out\n\nfn count(...parts: str) -> int:\n  total = 0\n  for part in parts:\n    total = total + part.byte_len()\n  total\n\nfn forward(...parts: str) -> int:\n  count(...parts)\n\nfn main():\n  joined = join(\"a\", \"b\", \"c\")\n  out(\"join=$joined\")\n  items = @[\"x\", \"yy\", \"zzz\"]\n  total = count(...items)\n  out(\"count=$total\")\n  forwarded = forward(\"aa\", \"bb\")\n  out(\"forward=$forwarded\")\n  empty = join()\n  out(\"empty=$empty\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "join=abc\ncount=6\nforward=4\nempty=\n");
}

#[test]
fn variadic_of_arrays_and_data_aggregates() {
    let source = "data Point:\n  x: int\n  y: int\n\nfn sum_x(...points: Point) -> int:\n  total = 0\n  for point in points:\n    total = total + point.x\n  total\n\nfn rows_sum(...rows: array[int, 2]) -> int:\n  total = 0\n  for row in rows:\n    total = total + row.at(0) + row.at(1)\n  total\n\nfn main():\n  points = sum_x(Point(x: 1, y: 2), Point(x: 3, y: 4))\n  out(\"points=$points\")\n  rows = rows_sum([1, 2], [3, 4])\n  out(\"rows=$rows\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "points=4\nrows=10\n");
}

#[test]
fn variadic_spread_of_array() {
    let source = "fn sum(...values: int) -> int:\n  total = 0\n  for value in values:\n    total = total + value\n  total\n\nfn main():\n  items: array[int, 3] = [1, 2, 3]\n  result = sum(...items)\n  out(\"arr=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "arr=6\n");
}

#[test]
fn variadic_rejects_wrong_element_type() {
    let codes = diagnostic_codes(
        "fn sum(...values: int) -> int:\n  values.len()\nfn main():\n  result = sum(\"bad\")\n  out(\"$result\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1003"), "{codes:?}");
}

#[test]
fn variadic_parameter_must_be_last() {
    let codes = diagnostic_codes(
        "fn bad(...values: int, tail: int) -> int:\n  tail\nfn main():\n  out(\"x\")\n",
    );
    assert!(codes.iter().any(|code| code == "E7013"), "{codes:?}");
}

#[test]
fn spread_requires_a_variadic_parameter() {
    let codes = diagnostic_codes(
        "fn plain(a: int) -> int:\n  a\nfn main():\n  xs = @[1, 2]\n  result = plain(...xs)\n  out(\"$result\")\n",
    );
    assert!(codes.iter().any(|code| code == "E7013"), "{codes:?}");
}
