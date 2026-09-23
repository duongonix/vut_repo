//! Bounds-contract regression tests: explicit index requests trap, and
//! accessors with their own fallback contracts are preserved.
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

/// Compiles and runs `source`. Returns `(success, stdout, stderr)`. Aborts on
/// other platforms surface as `success == false` (Windows) or a signal
/// (no numeric code), so tests assert on success and the stderr message.
fn run(source: &str) -> (bool, String, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-bounds-e2e-{nonce}-{sequence}"));
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
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fs::remove_dir_all(&root).ok();
    (output.status.success(), stdout, stderr)
}

fn assert_bounds_trap(source: &str, op: &str) {
    let (success, stdout, stderr) = run(source);
    assert!(
        !success,
        "an out-of-range index must trap; stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        stderr.contains(op) && stderr.contains("out of range"),
        "panic message must name the operation and index: {stderr:?}"
    );
}

#[test]
fn list_at_out_of_range_traps() {
    assert_bounds_trap(
        "fn main():\n  items = @[10, 20, 30]\n  out(items.at(1))\n  out(items.at(3))\n",
        "list.at",
    );
    assert_bounds_trap(
        "fn main():\n  items = @[10, 20, 30]\n  out(items.at(99))\n",
        "list.at",
    );
    assert_bounds_trap(
        "fn main():\n  empty: list[int] = @[]\n  out(empty.at(0))\n",
        "list.at",
    );
    assert_bounds_trap(
        "fn main():\n  items = @[10, 20, 30]\n  i = 0 - 1\n  out(items.at(i))\n",
        "list.at",
    );
}

#[test]
fn list_set_insert_remove_out_of_range_trap() {
    assert_bounds_trap(
        "fn main():\n  items = @[1, 2, 3]\n  i = 5\n  items.set(i, 9)\n",
        "list.set",
    );
    assert_bounds_trap(
        "fn main():\n  items = @[1, 2, 3]\n  i = 9\n  items.insert(i, 9)\n",
        "list.insert",
    );
    assert_bounds_trap(
        "fn main():\n  items = @[1, 2, 3]\n  i = 9\n  out(items.remove(i))\n",
        "list.remove",
    );
}

#[test]
fn managed_element_out_of_range_traps() {
    assert_bounds_trap(
        "fn main():\n  names = @[\"a\", \"b\"]\n  i = 4\n  out(names.at(i))\n",
        "list.at",
    );
}

#[test]
fn bytes_at_and_set_out_of_range_trap() {
    assert_bounds_trap(
        "fn main():\n  buf = \"hi\".to_bytes()\n  i = 9\n  out(buf.at(i))\n",
        "bytes.at",
    );
    assert_bounds_trap(
        "fn main():\n  buf = \"hi\".to_bytes()\n  i = 9\n  buf.set(i, 65)\n",
        "bytes.set",
    );
}

#[test]
fn array_at_runtime_index_traps() {
    assert_bounds_trap(
        "fn main():\n  nums = [1, 2, 3]\n  i = 7\n  out(nums.at(i))\n",
        "array.at",
    );
}

#[test]
fn documented_fallbacks_are_preserved() {
    let (success, stdout, stderr) = run(
        "fn main():\n  empty: list[int] = @[]\n  out(empty.first())\n  out(empty.last())\n  out(empty.pop())\n  out(empty.len())\n  buf = bytes()\n  out(buf.first())\n  out(buf.last())\n  out(\"ok\")\n",
    );
    assert!(success, "fallback accessors must not trap: {stderr:?}");
    assert_eq!(stdout, "0\n0\n0\n0\n0\n0\nok\n");

    let (success, stdout, stderr) =
        run("fn main():\n  text = \"abc\"\n  out(\"[\")\n  out(text.char_at(9))\n  out(\"]\")\n");
    assert!(
        success,
        "char_at keeps its empty-string contract: {stderr:?}"
    );
    assert_eq!(stdout, "[\n\n]\n");
}

#[test]
fn valid_indexes_still_work() {
    let (success, stdout, stderr) = run(
        "fn main():\n  items = @[1, 2, 3]\n  items.set(1, 9)\n  items.insert(3, 4)\n  out(items)\n  out(items.at(0))\n  out(items.remove(0))\n  out(items)\n  buf = \"hi\".to_bytes()\n  out(buf.at(1))\n  nums = [1, 2, 3]\n  out(nums.at(2))\n  out(nums.first())\n  out(nums.last())\n",
    );
    assert!(success, "{stderr:?}");
    assert_eq!(stdout, "[1, 9, 3, 4]\n1\n1\n[9, 3, 4]\n105\n3\n1\n3\n");
}
