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
    let root = std::env::temp_dir().join(format!("vut-phase-a-e2e-{nonce}-{sequence}"));
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
fn list_sort_reverse_first_last_index() {
    let source = "fn main():\n  items = @[3, 1, 2]\n  items.sort()\n  out(items)\n  items.reverse()\n  out(items)\n  out(\"first=$(items.first()) last=$(items.last())\")\n  out(\"index=$(items.index_of(2)) missing=$(items.index_of(99))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "[1, 2, 3]\n[3, 2, 1]\nfirst=3 last=1\nindex=1 missing=-1\n"
    );
}

#[test]
fn list_pop_extend_truncate_swap_shrink() {
    let source = "fn main():\n  items = @[1, 2, 3]\n  items.push(4)\n  out(\"popped=$(items.pop())\")\n  out(items)\n  items.extend(@[7, 8])\n  out(items)\n  items.swap(0, 2)\n  out(items)\n  items.truncate(2)\n  out(items)\n  items.shrink_to_fit()\n  out(\"capacity=$(items.capacity())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "popped=4\n[1, 2, 3]\n[1, 2, 3, 7, 8]\n[3, 2, 1, 7, 8]\n[3, 2]\ncapacity=2\n"
    );
}

#[test]
fn list_sort_strings_and_floats() {
    let source = "fn main():\n  words = @[\"banana\", \"apple\", \"cherry\", \"apple\"]\n  words.sort()\n  out(words)\n  floats = @[3.5, 1.25, 2.0]\n  floats.sort()\n  out(floats)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "[\"apple\", \"apple\", \"banana\", \"cherry\"]\n[1.25, 2, 3.5]\n"
    );
}

#[test]
fn collections_higher_order_functions() {
    let source = "fn double(x: int) -> int:\n  x * 2\n\nfn is_even(x: int) -> bool:\n  x % 2 == 0\n\nfn add(acc: int, x: int) -> int:\n  acc + x\n\nfn main():\n  items = @[3, 1, 2]\n  out(items.map(double))\n  out(items.filter(is_even))\n  out(\"total=$(items.fold(0, add))\")\n  out(\"any=$(items.any(is_even))\")\n  out(\"all=$(items.all(is_even))\")\n  out(\"idx=$(items.find_index(is_even))\")\n  out(@[\"a\", \"b\", \"c\"].join(\"-\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "[6, 2, 4]\n[2]\ntotal=6\nany=1\nall=0\nidx=2\na-b-c\n"
    );
}

#[test]
fn collections_sort_by_is_usable_and_leak_free() {
    let source = "fn compare_int(a: int, b: int) -> int:\n  a - b\n\nfn main():\n  items = @[5, 3, 9, 1, 3]\n  items.sort_by(compare_int)\n  out(items)\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "[1, 3, 3, 5, 9]\n");
}

#[test]
fn large_list_sort_reverse_extend_does_not_leak() {
    let source = "fn compare_int(a: int, b: int) -> int:\n  a - b\n\nfn main():\n  items: list[int] = @[]\n  for index in @[0, 1]:\n    count = 0\n    for count < 20000:\n      items.push(count)\n      count = count + 1\n  items.sort()\n  items.reverse()\n  items.extend(@[1, 2, 3])\n  items.sort_by(compare_int)\n  out(\"len=$(items.len()) first=$(items.first()) last=$(items.last())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "len=40003 first=0 last=19999\n");
}
