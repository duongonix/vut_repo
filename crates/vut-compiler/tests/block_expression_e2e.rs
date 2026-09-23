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
    let root = std::env::temp_dir().join(format!("vut-block-expr-e2e-{nonce}-{sequence}"));
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
fn multi_statement_arms_and_nested_match() {
    let source = "data Pair:\n  name: str\n  kind: int\n\nenum Kind:\n  first\n  other\n\nfn describe(pair: Pair) -> str:\n  match pair.kind:\n    0:\n      value = pair.name\n      \"first:$value\"\n    _:\n      count = pair.kind\n      \"other:$count\"\n\nfn nested(left: Kind, right: int) -> str:\n  match left:\n    first:\n      match right:\n        0: \"first-zero\"\n        _: \"first-other\"\n    other: \"other\"\n\nfn reduce(outcome: result[str, int]) -> str:\n  match outcome:\n    ok(text):\n      upper = text\n      upper\n    err(code): \"error\"\n\nfn main():\n  out(describe(Pair(name: \"a\", kind: 0)))\n  out(describe(Pair(name: \"b\", kind: 5)))\n  out(nested(Kind.first, 0))\n  out(nested(Kind.first, 9))\n  out(nested(Kind.other, 0))\n  value: result[str, int] = ok(\"done\")\n  out(reduce(value))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "first:a\nother:5\nfirst-zero\nfirst-other\nother\ndone\n"
    );
}

#[test]
fn assignment_value_can_be_an_indented_block() {
    let source = "fn total() -> int:\n  value: int =\n    a = 10\n    b = 20\n    a + b\n  value\n\nfn greeting(name: str) -> str:\n  message: str =\n    prefix = \"hello\"\n    \"$prefix $name\"\n  message\n\nfn main():\n  out(\"total=$(total())\")\n  out(greeting(\"vut\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "total=30\nhello vut\n");
}

#[test]
fn managed_values_in_block_arms_and_assignment_do_not_leak() {
    let source = "data Box:\n  name: str\n\nfn pick(kind: int) -> str:\n  match kind:\n    0:\n      item = Box(name: \"hello\")\n      item.name\n    _:\n      other = Box(name: \"world\")\n      text = other.name\n      text\n\nfn build() -> str:\n  value: str =\n    first = Box(name: \"a\")\n    second = Box(name: \"b\")\n    first.name + second.name\n  value\n\nfn main():\n  out(pick(0))\n  out(pick(1))\n  out(\"build=$(build())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report clean exit: {stdout}"
    );
    assert_eq!(stdout, "hello\nworld\nbuild=ab\n");
}

#[test]
fn block_guard_arm_still_works() {
    let source = "fn sign(value: int) -> str:\n  match value:\n    n if n > 0:\n      label = \"positive\"\n      label\n    _: \"non-positive\"\n\nfn main():\n  out(sign(3))\n  out(sign(-1))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "positive\nnon-positive\n");
}
