//! Value semantics and copy-on-write regression tests.
//!
//! A managed collection copied into another binding must detach on mutation:
//! the original binding must never observe the mutation. These cover locals,
//! fields, subscript elements, function arguments, closures, and every mutating
//! collection API.
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
    let root = std::env::temp_dir().join(format!("vut-value-semantics-{nonce}-{sequence}"));
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
fn assign_then_mutate_list_detaches() {
    assert_runs(
        "fn main():\n  a = @[1, 2, 3]\n  b = a\n  b.push(4)\n  out(\"a=$a b=$b\")\n",
        "a=[1, 2, 3] b=[1, 2, 3, 4]\n",
    );
}

#[test]
fn every_list_mutation_detaches() {
    assert_runs(
        "fn main():\n  base = @[3, 1, 2]\n  copy = base\n  copy.push(4)\n  copy.set(0, 9)\n  copy.sort()\n  copy.reverse()\n  out(\"base=$base copy=$copy\")\n",
        "base=[3, 1, 2] copy=[9, 4, 2, 1]\n",
    );
    assert_runs(
        "fn main():\n  base = @[1, 2, 3]\n  copy = base\n  copy.insert(0, 9)\n  out(\"base=$base copy=$copy\")\n",
        "base=[1, 2, 3] copy=[9, 1, 2, 3]\n",
    );
    assert_runs(
        "fn main():\n  base = @[1, 2, 3]\n  copy = base\n  copy.remove(0)\n  out(\"base=$base copy=$copy\")\n",
        "base=[1, 2, 3] copy=[2, 3]\n",
    );
    assert_runs(
        "fn main():\n  base = @[1, 2]\n  copy = base\n  copy.clear()\n  out(\"base=$base copy=$copy\")\n",
        "base=[1, 2] copy=[]\n",
    );
    assert_runs(
        "fn main():\n  base = @[1]\n  copy = base\n  copy.extend(@[2, 3])\n  out(\"base=$base copy=$copy\")\n",
        "base=[1] copy=[1, 2, 3]\n",
    );
    assert_runs(
        "fn main():\n  base = @[1, 2, 3]\n  copy = base\n  copy.truncate(1)\n  out(\"base=$base copy=$copy\")\n",
        "base=[1, 2, 3] copy=[1]\n",
    );
}

#[test]
fn sort_by_detaches() {
    assert_runs(
        "fn cmp(x: int, y: int) -> int:\n  x - y\nfn main():\n  base = @[3, 1]\n  copy = base\n  copy.sort_by(cmp)\n  out(\"base=$base copy=$copy\")\n",
        "base=[3, 1] copy=[1, 3]\n",
    );
}

#[test]
fn multiple_aliases_detach() {
    assert_runs(
        "fn main():\n  c = @[1]\n  d = c\n  e = c\n  e.push(7)\n  out(\"c=$c d=$d e=$e\")\n",
        "c=[1] d=[1] e=[1, 7]\n",
    );
}

#[test]
fn nested_managed_values_detach() {
    assert_runs(
        "fn main():\n  inner = @[1]\n  outer = @[inner]\n  outer[0].push(9)\n  out(\"outer=$outer inner=$inner\")\n",
        "outer=[[1, 9]] inner=[1]\n",
    );
}

#[test]
fn function_argument_mutation_detaches() {
    assert_runs(
        "fn add(m: list[int]):\n  m.push(99)\nfn main():\n  a = @[1]\n  add(a)\n  out(\"a=$a\")\n",
        "a=[1]\n",
    );
}

#[test]
fn closure_capture_mutation_detaches() {
    assert_runs(
        "fn main():\n  c = @[1]\n  f = fn():\n    c.push(2)\n    out(\"inside=$c\")\n  f()\n  out(\"outside=$c\")\n",
        "inside=[1, 2]\noutside=[1]\n",
    );
}

#[test]
fn field_mutation_detaches() {
    assert_runs(
        "data Box:\n  items: list[int]\nfn Box.add(x: int):\n  self.items.push(x)\nfn main():\n  b = Box(items: @[1])\n  a = b.items\n  b.add(2)\n  out(\"b.items=$(b.items) a=$a\")\n",
        "b.items=[1, 2] a=[1]\n",
    );
}

#[test]
fn unique_owner_fast_path_still_mutates() {
    assert_runs(
        "fn main():\n  b = @[1]\n  b.push(2)\n  out(\"b=$b\")\n",
        "b=[1, 2]\n",
    );
}

#[test]
fn assign_then_mutate_map_detaches() {
    assert_runs(
        "fn main():\n  m: map[str, int] = (\"x\": 1)\n  n = m\n  n.set(\"y\", 2)\n  out(\"m.len=$(m.len()) n.len=$(n.len())\")\n",
        "m.len=1 n.len=2\n",
    );
}

#[test]
fn assign_then_mutate_bytes_detaches() {
    assert_runs(
        "fn main():\n  p = bytes()\n  p.push(1)\n  q = p\n  q.push(2)\n  out(\"p.len=$(p.len()) q.len=$(q.len())\")\n",
        "p.len=1 q.len=2\n",
    );
}
