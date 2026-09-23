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
    let root = std::env::temp_dir().join(format!("vut-channel-e2e-{nonce}-{sequence}"));
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
fn channel_int_buffered_fifo_and_close_absence() {
    let source = "async fn main():\n  ch = channel[int](capacity: 4)\n  ch.send(10)\n  ch.send(20)\n  ch.close()\n  a = ch.recv()\n  b = ch.recv()\n  c = ch.recv()\n  if a != null:\n    out(\"a=$a\")\n  if b != null:\n    out(\"b=$b\")\n  if c == null:\n    out(\"c=absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=10\nb=20\nc=absent\n");
}

#[test]
fn channel_str_transfers_managed_values() {
    let source = "async fn main():\n  ch = channel[str](capacity: 2)\n  ch.send(\"hello\")\n  ch.send(\"world\")\n  ch.close()\n  a = ch.recv()\n  b = ch.recv()\n  if a != null:\n    out(\"a=$a\")\n  if b != null:\n    out(\"b=$b\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=hello\nb=world\n");
}

#[test]
fn channel_list_of_int_transfers_managed_values() {
    let source = "async fn main():\n  ch = channel[list[int]](capacity: 2)\n  ch.send(@[1, 2, 3])\n  ch.close()\n  a = ch.recv()\n  if a != null:\n    out(\"len=$(a.len())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "len=3\n");
}

#[test]
fn recv_suspends_until_a_producer_sends() {
    let source = "async fn main():\n  ch = channel[int]()\n  producer = vut(fn():\n    ch.send(42)\n  )\n  a = ch.recv()\n  if a != null:\n    out(\"got=$a\")\n  await producer\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "got=42\n");
}

#[test]
fn recv_suspends_until_close_reports_absence() {
    let source = "async fn main():\n  ch = channel[int]()\n  closer = vut(fn():\n    ch.close()\n  )\n  a = ch.recv()\n  if a == null:\n    out(\"absent\")\n  await closer\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "absent\n");
}

#[test]
fn channel_operation_outside_async_is_rejected() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-channel-reject-{nonce}"));
    fs::create_dir(&root).expect("create root");
    fs::write(
        root.join("main.vut"),
        "fn main():\n  ch = channel[int](capacity: 1)\n  ch.send(1)\n",
    )
    .expect("write source");
    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root, &[])
        .expect("check source");
    fs::remove_dir_all(&root).ok();
    let codes: Vec<String> = checked
        .semantics
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    assert!(codes.iter().any(|code| code == "E6011"), "{codes:?}");
}
