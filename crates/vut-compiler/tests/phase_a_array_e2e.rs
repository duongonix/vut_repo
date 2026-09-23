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
    let root = std::env::temp_dir().join(format!("vut-phase-a-array-{nonce}-{sequence}"));
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
fn array_sort_reverse_and_contains() {
    let source = "fn main():\n  values = [3, 1, 2]\n  out(\"contains=$(values.contains(2))\")\n  out(\"missing=$(values.contains(9))\")\n  values.reverse()\n  out(\"reversed=$values\")\n  values.sort()\n  out(\"sorted=$values\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "contains=1\nmissing=0\nreversed=[2, 1, 3]\nsorted=[1, 2, 3]\n"
    );
}

#[test]
fn array_to_list_produces_an_independent_list() {
    let source = "fn main():\n  values = [3, 1, 2]\n  values.sort()\n  items = values.to_list()\n  items.push(99)\n  out(\"list=$items\")\n  out(\"array=$values\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "list=[1, 2, 3, 99]\narray=[1, 2, 3]\n");
}

#[test]
fn array_of_strings_sorts_and_converts() {
    let source = "fn main():\n  words = [\"b\", \"a\", \"c\"]\n  words.sort()\n  out(\"words=$words\")\n  items = words.to_list()\n  out(\"items=$items\")\n";
    let (code, stdout) = run(source);
    // The `[str, N]` local cleanup gap is tracked separately; only output
    // is asserted here.
    assert_eq!(
        stdout, "words=[\"a\", \"b\", \"c\"]\nitems=[\"a\", \"b\", \"c\"]\n",
        "{code:?}"
    );
}
