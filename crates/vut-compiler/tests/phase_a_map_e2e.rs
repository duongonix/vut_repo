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
    let root = std::env::temp_dir().join(format!("vut-phase-a-map-{nonce}-{sequence}"));
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
fn map_keys_and_values_for_string_keys() {
    let source = "fn main():\n  scores: map[str, int] = (\"a\": 1, \"b\": 2, \"c\": 3)\n  keys = scores.keys()\n  keys.sort()\n  out(\"keys=$keys\")\n  values = scores.values()\n  values.sort()\n  out(\"values=$values\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "keys=[\"a\", \"b\", \"c\"]\nvalues=[1, 2, 3]\n");
}

#[test]
fn map_keys_and_values_for_int_keys() {
    let source = "fn main():\n  counts: map[int, str] = (2: \"two\", 1: \"one\")\n  keys = counts.keys()\n  keys.sort()\n  out(\"keys=$keys\")\n  values = counts.values()\n  values.sort()\n  out(\"values=$values\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "keys=[1, 2]\nvalues=[\"one\", \"two\"]\n");
}

#[test]
fn map_get_or_returns_value_or_default() {
    let source = "fn main():\n  scores: map[str, int] = (\"a\": 1)\n  present = scores.get_or(\"a\", 0)\n  out(\"present=$present\")\n  missing = scores.get_or(\"z\", 99)\n  out(\"missing=$missing\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "present=1\nmissing=99\n");
}

#[test]
fn map_values_with_managed_values_do_not_leak() {
    let source = "fn main():\n  labels: map[str, str] = (\"a\": \"apple\", \"b\": \"banana\")\n  values = labels.values()\n  values.sort()\n  out(\"values=$values\")\n  first = labels.get_or(\"a\", \"none\")\n  out(\"first=$first\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "values=[\"apple\", \"banana\"]\nfirst=apple\n");
}

#[test]
fn large_map_does_not_leak() {
    let source = "fn main():\n  counts: map[int, int] = (0: 0)\n  index = 1\n  for index < 5000:\n    counts.set(index, index)\n    index = index + 1\n  keys = counts.keys()\n  out(\"len=$(keys.len())\")\n  total = 0\n  for key in keys:\n    total = total + counts.get_or(key, 0)\n  out(\"total=$total\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "len=5000\ntotal=12497500\n");
}
