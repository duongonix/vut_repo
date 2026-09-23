//! Range-based `for` loop regression tests.
//!
//! `for i in start..end:` / `start..=end` must lower to a counter loop in MIR,
//! evaluate both bounds exactly once, and handle empty ranges, nesting,
//! expression bounds, and `break`/`continue`.
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
    let root = std::env::temp_dir().join(format!("vut-range-for-{nonce}-{sequence}"));
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

fn assert_runs(source: &str, expected: &str) {
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, expected);
}

#[test]
fn literal_exclusive_range() {
    assert_runs(
        "fn main():\n  total = 0\n  for i in 0..10:\n    total = total + i\n  out(\"sum=$total\")\n",
        "sum=45\n",
    );
}

#[test]
fn variable_end_range() {
    assert_runs(
        "fn main():\n  n = 4\n  count = 0\n  for i in 0..n:\n    count = count + 1\n  out(\"count=$count\")\n",
        "count=4\n",
    );
}

#[test]
fn variable_start_and_end_range() {
    assert_runs(
        "fn main():\n  start = 2\n  end = 5\n  acc = \"\"\n  for i in start..end:\n    acc = acc + \"$i\"\n  out(\"acc=$acc\")\n",
        "acc=234\n",
    );
}

#[test]
fn empty_range_runs_zero_times() {
    assert_runs(
        "fn main():\n  empty = 0\n  for i in 5..5:\n    empty = empty + 1\n  out(\"empty=$empty\")\n",
        "empty=0\n",
    );
    assert_runs(
        "fn main():\n  empty = 0\n  for i in 9..2:\n    empty = empty + 1\n  out(\"empty=$empty\")\n",
        "empty=0\n",
    );
}

#[test]
fn inclusive_range_includes_the_end() {
    assert_runs(
        "fn main():\n  inc = \"\"\n  for i in 0..=3:\n    inc = inc + \"$i\"\n  out(\"inc=$inc\")\n",
        "inc=0123\n",
    );
}

#[test]
fn nested_range_loops() {
    assert_runs(
        "fn main():\n  s = \"\"\n  for i in 0..2:\n    for j in 0..2:\n      s = s + \"$i$j\"\n  out(\"nested=$s\")\n",
        "nested=00011011\n",
    );
}

#[test]
fn range_bounds_may_be_expressions() {
    assert_runs(
        "fn main():\n  n = 3\n  b = \"\"\n  for i in 0..(n * 2):\n    b = b + \"$i\"\n  out(\"expr=$b\")\n",
        "expr=012345\n",
    );
}

#[test]
fn range_supports_break_and_continue() {
    assert_runs(
        "fn main():\n  c = 0\n  for i in 0..10:\n    if i == 3:\n      continue\n    if i == 6:\n      break\n    c = c + i\n  out(\"cf=$c\")\n",
        "cf=12\n",
    );
}

#[test]
fn range_binds_index() {
    assert_runs(
        "fn main():\n  idx = \"\"\n  for i, j in 0..3:\n    idx = idx + \"$i:$j \"\n  out(\"idx=$idx\")\n",
        "idx=0:0 1:1 2:2 \n",
    );
}

#[test]
fn range_bounds_are_evaluated_once() {
    assert_runs(
        "fn limit() -> int:\n  out(\"eval-end\")\n  3\nfn start() -> int:\n  out(\"eval-start\")\n  1\nfn main():\n  for i in start()..limit():\n    out(\"i=$i\")\n",
        "eval-start\neval-end\ni=1\ni=2\n",
    );
}
