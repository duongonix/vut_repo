//! End-to-end tests for the compiler-lowered higher-order `list[T]` builtins
//! and the native `list[str].join`.
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

/// Compiles and runs `source`, returning `(exit_code, stdout)`. A clean exit
/// also means the leak detector found no live managed allocation.
fn run(source: &str) -> (Option<i32>, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-collections-e2e-{nonce}-{sequence}"));
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

const HELPERS: &str = "fn double(x: int) -> int:\n  x * 2\n\nfn is_even(x: int) -> bool:\n  x % 2 == 0\n\nfn add(acc: int, x: int) -> int:\n  acc + x\n";

#[test]
fn canonical_indexing_and_indexed_assignment_execute() {
    let source = "fn main():\n  fixed: array[int, 3] = [1, 2, 3]\n  fixed[1] = 20\n  items: list[int] = @[4, 5, 6]\n  items[0] = 40\n  scores: map[str, int] = (\"a\": 7)\n  scores[\"a\"] = 70\n  matrix: array[array[int, 2], 2] = [[1, 2], [3, 4]]\n  matrix[0][1] = 9\n  out(fixed[1], items[0], scores[\"a\"], matrix[0][1])\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "20 40 70 9\n");
}

#[test]
fn higher_order_methods_on_named_and_lambda_callbacks() {
    let source = format!(
        "{HELPERS}fn main():\n  items = @[3, 1, 2, 4]\n  out(\"map:\", items.map(double))\n  out(\"filter:\", items.filter(is_even))\n  out(\"fold:\", items.fold(0, add))\n  out(\"any:\", items.any(is_even))\n  out(\"all:\", items.all(is_even))\n  out(\"find_index:\", items.find_index(is_even))\n  out(\"lambda_map:\", items.map(x => x + 1))\n  out(\"lambda_filter:\", items.filter(x => x > 2))\n  out(\"lambda_fold:\", items.fold(1, (a, x) => a * x))\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "map: [6, 2, 4, 8]\nfilter: [2, 4]\nfold: 10\nany: true\nall: false\nfind_index: 2\nlambda_map: [4, 2, 3, 5]\nlambda_filter: [3, 4]\nlambda_fold: 24\n"
    );
}

#[test]
fn map_can_change_the_element_type() {
    let source = "fn main():\n  numbers = @[1, 2, 3]\n  labels: list[str] = numbers.map(x => x.to_str())\n  out(labels)\n  lengths: list[int] = labels.map(s => s.byte_len())\n  out(lengths)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "[\"1\", \"2\", \"3\"]\n[1, 1, 1]\n");
}

#[test]
fn higher_order_methods_chain() {
    let source = format!(
        "{HELPERS}fn main():\n  items = @[3, 1, 2, 4, 6]\n  out(items.filter(is_even).map(double))\n  out(items.filter(x => x > 2).fold(0, add))\n  out(items.map(double).find_index(x => x > 5))\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "[4, 8, 12]\n13\n0\n");
}

#[test]
fn short_circuit_results_are_correct() {
    let source = "fn at_least_two(x: int) -> bool:\n  x >= 2\n\nfn main():\n  items = @[1, 2, 3]\n  out(\"any_first:\", items.any(at_least_two))\n  out(\"all_first:\", items.all(at_least_two))\n  out(\"find_first:\", items.find_index(at_least_two))\n  empty: list[int] = @[]\n  out(\"any_empty:\", empty.any(at_least_two))\n  out(\"all_empty:\", empty.all(at_least_two))\n  out(\"find_empty:\", empty.find_index(at_least_two))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "any_first: true\nall_first: false\nfind_first: 1\nany_empty: false\nall_empty: true\nfind_empty: -1\n"
    );
}

#[test]
fn fold_threads_a_managed_accumulator() {
    let source = "fn append(acc: str, part: str) -> str:\n  acc + part\n\nfn main():\n  words = @[\"a\", \"b\", \"c\"]\n  out(words.fold(\"\", append))\n  out(words.fold(\"\", (a, b) => a + b + \".\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "abc\na.b.c.\n");
}

#[test]
fn sort_by_matches_sort_and_is_stable() {
    let source = "data Pair:\n  key: int\n  id: int\n\nfn by_key(a: Pair, b: Pair) -> int:\n  a.key - b.key\n\nfn compare_int(a: int, b: int) -> int:\n  a - b\n\nfn main():\n  pairs = @[Pair(key: 2, id: 0), Pair(key: 1, id: 1), Pair(key: 2, id: 2), Pair(key: 1, id: 3), Pair(key: 2, id: 4)]\n  pairs.sort_by(by_key)\n  out(pairs.map(p => p.id))\n  numbers = @[5, 3, 9, 1, 3, 7]\n  numbers.sort_by(compare_int)\n  out(numbers)\n  words = @[\"pear\", \"apple\", \"fig\"]\n  words.sort_by((a, b) => a.compare(b))\n  out(words)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "[1, 3, 0, 2, 4]\n[1, 3, 3, 5, 7, 9]\n[\"apple\", \"fig\", \"pear\"]\n"
    );
}

#[test]
fn sort_by_matches_sort_on_a_large_pseudorandom_list() {
    let source = "fn compare_int(a: int, b: int) -> int:\n  a - b\n\nfn main():\n  n = 2000\n  a: list[int] = @[]\n  seed = 12345\n  i = 0\n  for i < n:\n    seed = (seed * 1103515245 + 12345) % 2147483648\n    a.push(seed % 1000)\n    i = i + 1\n  b = a.slice(0, a.len())\n  a.sort()\n  b.sort_by(compare_int)\n  ok = true\n  j = 0\n  for j < n:\n    if a.at(j) != b.at(j):\n      ok = false\n    j = j + 1\n  out(\"matches_sort:\", ok)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "matches_sort: true\n");
}

#[test]
fn higher_order_methods_on_managed_elements_are_leak_free() {
    let source = "data Person:\n  name: str\n  age: int\n\nfn by_age(a: Person, b: Person) -> int:\n  a.age - b.age\n\nfn shout(name: str) -> str:\n  name + \"!\"\n\nfn main():\n  words = @[\"a\", \"bb\", \"ccc\"]\n  out(words.map(shout))\n  out(words.filter(w => w.byte_len() > 1))\n  out(words.fold(\"\", (a, b) => a + b))\n  people = @[Person(name: \"ann\", age: 30), Person(name: \"bo\", age: 20), Person(name: \"cy\", age: 20)]\n  people.sort_by(by_age)\n  out(people.map(p => p.name))\n  out(people.filter(p => p.age == 20).map(p => p.name))\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(
        stdout,
        "[\"a!\", \"bb!\", \"ccc!\"]\n[\"bb\", \"ccc\"]\nabbccc\n[\"bo\", \"cy\", \"ann\"]\n[\"bo\", \"cy\"]\n"
    );
}

#[test]
fn join_handles_edges() {
    let source = "fn main():\n  out(@[\"a\", \"b\", \"c\"].join(\"-\"))\n  empty: list[str] = @[]\n  out(empty.join(\", \").byte_len())\n  out(@[\"solo\"].join(\"--\"))\n  out(@[\"x\", \"y\"].join(\"\"))\n  words: list[str] = @[\"one\", \"two\"]\n  out(words.join(\" and \"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a-b-c\n0\nsolo\nxy\none and two\n");
}

#[test]
fn large_collection_workload_is_leak_free() {
    let source = format!(
        "{HELPERS}fn compare_int(a: int, b: int) -> int:\n  a - b\n\nfn main():\n  items: list[int] = @[]\n  i = 0\n  for i < 50000:\n    items.push((i * 7919) % 50021)\n    i = i + 1\n  items.sort_by(compare_int)\n  evens = items.filter(is_even)\n  doubled = evens.map(double)\n  out(\"len:\", items.len(), \"first:\", items.first(), \"last:\", items.last())\n  out(\"evens:\", evens.len(), \"doubled:\", doubled.len(), \"total:\", items.fold(0, add))\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(
        stdout,
        "len: 50000 first: 0 last: 50020\nevens: 25001 doubled: 25001 total: 1250453491\n"
    );
}
