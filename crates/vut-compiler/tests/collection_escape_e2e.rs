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
    let root = std::env::temp_dir().join(format!("vut-collection-escape-e2e-{nonce}-{sequence}"));
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
fn aggregate_enum_in_map_escapes_function_without_leaking() {
    let source = "enum Payload:\n  none\n  text(value: str)\n\ndata Envelope:\n  entries: map(str, Payload)\n\nfn build() -> Envelope:\n  entries: map(str, Payload) = map()\n  entries.set(\"greeting\", Payload.text(value = \"hi\"))\n  Envelope(entries = entries)\n\nfn main():\n  env = build()\n  match env.entries.get(\"greeting\"):\n    text(value): out(\"text=$value\")\n    none: out(\"none\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "text=hi\n");
}

#[test]
fn aggregate_enum_in_list_escapes_function_without_leaking() {
    let source = "enum Node:\n  leaf(value: str)\n  branch(children: list(Node))\n\nfn build() -> Node:\n  kids: list(Node) = @()\n  kids.push(Node.leaf(value = \"x\"))\n  Node.branch(children = kids)\n\nfn main():\n  node = build()\n  match node:\n    branch(children): out(\"branch=$(children.len())\")\n    leaf(value): out(\"leaf=$value\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "branch=1\n");
}

#[test]
fn nested_collection_of_aggregates_survives_return_and_iteration() {
    let source = "enum Cell:\n  empty\n  named(value: str)\n\ndata Grid:\n  rows: list(list(Cell))\n\nfn build() -> Grid:\n  row: list(Cell) = @()\n  row.push(Cell.named(value = \"a\"))\n  row.push(Cell.named(value = \"b\"))\n  rows: list(list(Cell)) = @()\n  rows.push(row)\n  Grid(rows = rows)\n\nfn main():\n  grid = build()\n  for row in grid.rows:\n    for cell in row:\n      match cell:\n        named(value): out(\"cell=$value\")\n        empty: out(\"empty\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "cell=a\ncell=b\n");
}
