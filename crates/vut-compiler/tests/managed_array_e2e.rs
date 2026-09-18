//! Managed array ownership regression tests. A clean exit also means the leak
//! detector found no live managed allocation.
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

fn run(source: &str) -> (bool, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-managed-array-{nonce}-{sequence}"));
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
    (output.status.success(), format!("{stdout}{stderr}"))
}

fn expect_clean(source: &str, expected: &str) {
    let (success, output) = run(source);
    assert!(
        success,
        "managed array must be leak-free and crash-free: {output:?}"
    );
    assert_eq!(output, expected);
}

#[test]
fn array_of_strings_set_at_and_display_are_leak_free() {
    expect_clean(
        "fn main():\n  a: array(str, 2) = array(\"x\", \"y\")\n  a.set(0, \"longer\")\n  out(a)\n  out(a.at(0))\n  out(a.at(1))\n  out(a.first())\n  out(a.last())\n",
        "[\"longer\", \"y\"]\nlonger\ny\nlonger\ny\n",
    );
}

#[test]
fn nested_array_and_data_with_array_field_are_leak_free() {
    expect_clean(
        "data Holder:\n  items: array(str, 2)\n\nfn main():\n  h = Holder(items = array(\"a\", \"b\"))\n  h.items.set(0, \"changed\")\n  out(h.items.at(0))\n  out(h)\n  grid: array(array(str, 2), 2) = array(array(\"p\", \"q\"), array(\"r\", \"s\"))\n  out(grid.at(1).at(0))\n",
        "changed\nHolder(items = [\"changed\", \"b\"])\nr\n",
    );
}

#[test]
fn arrays_in_lists_maps_and_results_are_leak_free() {
    expect_clean(
        "fn first(a: array(str, 1)) -> str:\n  a.at(0)\n\nfn main():\n  rows: list(array(str, 1)) = @(array(\"x\"), array(\"y\"))\n  out(rows.at(0).at(0))\n  out(rows.map(first))\n  table: map(str, array(str, 1)) = map((\"k\", array(\"v\")))\n  out(table.get(\"k\").at(0))\n  out(first(array(\"z\")))\n",
        "x\n[\"x\", \"y\"]\nv\nz\n",
    );
}

#[test]
fn array_of_managed_data_is_leak_free() {
    expect_clean(
        "data Inner:\n  s: str\n\ndata Outer:\n  items: array(Inner, 2)\n\nfn main():\n  o = Outer(items = array(Inner(s = \"a\"), Inner(s = \"b\")))\n  out(o.items.at(1).s)\n  inner = o.items.at(0)\n  out(inner.s)\n  out(o)\n",
        "b\na\nOuter(items = [Inner(s = \"a\"), Inner(s = \"b\")])\n",
    );
}

#[test]
fn empty_array_of_strings_via_slice_roundtrip_is_leak_free() {
    expect_clean(
        "fn main():\n  a: array(str, 2) = array(\"a\", \"b\")\n  b: list(str) = a.to_list()\n  out(b)\n  out(b.len())\n",
        "[\"a\", \"b\"]\n2\n",
    );
}

#[test]
fn iteration_over_array_fields_and_temporaries_is_leak_free() {
    expect_clean(
        "fn make() -> array(str, 2):\n  array(\"p\", \"q\")\n\ndata Holder:\n  items: array(str, 2)\n  names: list(str)\n\nfn main():\n  h = Holder(items = array(\"x\", \"y\"), names = @(\"a\", \"b\"))\n  for item in h.items:\n    out(item)\n  for name in h.names:\n    out(name)\n  for value in make():\n    out(value)\n  local: array(str, 1) = array(\"m\")\n  for value in local:\n    out(value)\n",
        "x\ny\na\nb\np\nq\nm\n",
    );
}
