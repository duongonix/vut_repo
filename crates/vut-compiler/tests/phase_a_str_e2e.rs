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
    let root = std::env::temp_dir().join(format!("vut-phase-a-str-{nonce}-{sequence}"));
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
fn lines_and_split_whitespace() {
    let source = "fn main():\n  text = \"Hello\\nWorld\\r\\nVut\"\n  out(text.lines())\n  out(\"a\\n\".lines())\n  out(\"a\\n\\n\".lines())\n  out(\"\".lines())\n  out(\" a  b\\tc \".split_whitespace())\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "[\"Hello\", \"World\", \"Vut\"]\n[\"a\"]\n[\"a\", \"\"]\n[]\n[\"a\", \"b\", \"c\"]\n"
    );
}

#[test]
fn chars_and_char_at_use_unicode_scalars() {
    let source = "fn main():\n  out(\"aé中\".chars())\n  out(\"aé中\".char_at(1))\n  out(\"aé中\".char_at(2))\n  out(\"aé中\".char_at(-1))\n  out(\"aé中\".char_at(99))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "[\"a\", \"é\", \"中\"]\né\n中\n\n\n");
}

#[test]
fn repeat_and_pad() {
    let source = "fn main():\n  out(\"ab\".repeat(3))\n  out(\"ab\".repeat(0))\n  out(\"ab\".repeat(-2))\n  out(\"7\".pad_left(3, \"0\"))\n  out(\"7\".pad_right(3, \".\"))\n  out(\"hello\".pad_left(3, \"0\"))\n  out(\"7\".pad_left(3, \"ab\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ababab\n\n\n007\n7..\nhello\n7\n");
}

#[test]
fn strip_prefix_and_suffix_return_original_when_absent() {
    let source = "fn main():\n  out(\"prefix-body\".strip_prefix(\"prefix-\"))\n  out(\"body\".strip_prefix(\"nope\"))\n  out(\"body-suffix\".strip_suffix(\"-suffix\"))\n  out(\"body\".strip_suffix(\"nope\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "body\nbody\nbody\nbody\n");
}

#[test]
fn rfind_and_compare() {
    let source = "fn main():\n  out(\"banana\".rfind(\"an\"))\n  out(\"banana\".rfind(\"zz\"))\n  out(\"a\".compare(\"b\"))\n  out(\"b\".compare(\"a\"))\n  out(\"a\".compare(\"a\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n-1\n-1\n1\n0\n");
}

#[test]
fn to_int_and_to_float_aliases() {
    let source = "fn main():\n  out(\"5\".to_int())\n  out(\"2.5\".to_float())\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\n2.5\n");
}

#[test]
fn large_string_operations_do_not_leak() {
    let source = "fn main():\n  block = \"x\".repeat(50000)\n  pieces = block.split(\"x\")\n  out(\"pieces=$(pieces.len())\")\n  joined = \"ab\".repeat(10000)\n  out(\"bytes=$(joined.byte_len())\")\n  out(\"chars=$(joined.chars().len())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "pieces=50001\nbytes=20000\nchars=20000\n");
}
