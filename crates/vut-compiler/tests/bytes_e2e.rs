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
    let root = std::env::temp_dir().join(format!("vut-bytes-e2e-{nonce}-{sequence}"));
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

const DECODE: &str = "fn text_of(blob: bytes) -> str:\n  match blob.to_str():\n    ok(value): value\n    err(error): \"invalid:$(error.valid_up_to):$(error.error_len)\"\n";

#[test]
fn bytes_conversions_preserve_exact_contents() {
    let source = format!(
        "{DECODE}fn main():\n  text = \"Xin chào Vut\"\n  blob = text.to_bytes()\n  out(\"$(blob.len())\")\n  out(text_of(blob))\n  values: list(u8) = blob.to_list()\n  again = bytes.from_list(values)\n  out(text_of(again))\n  out(\"$(values.len())\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "13\nXin chào Vut\nXin chào Vut\n13\n");
}

#[test]
fn bytes_buffer_methods_are_bounds_safe_and_complete() {
    let source = format!(
        "{DECODE}fn main():\n  empty = bytes()\n  out(\"$(empty.is_empty()) $(empty.len())\")\n  blob = \"abc\".to_bytes()\n  out(\"$(blob.len()) $(blob.at(1)) $(blob.first()) $(blob.last())\")\n  blob.set(1, 90)\n  out(\"$(blob.at(1))\")\n  sliced = blob.slice(1, 3)\n  out(\"$(sliced.len())\")\n  out(text_of(sliced))\n  blob.clear()\n  out(\"$(blob.is_empty()) $(blob.len())\")\n  out(\"$(blob.first()) $(blob.last())\")\n  out(\"$(blob.slice(5, 9).len())\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1 0\n3 98 97 99\n90\n2\nZc\n1 0\n0 0\n0\n");
}

#[test]
fn bytes_utf8_validation_handles_multibyte_and_invalid_input() {
    let source = format!(
        "{DECODE}fn main():\n  out(text_of(\"中文\".to_bytes()))\n  out(text_of(\"😀\".to_bytes()))\n  out(text_of(\"abc🇻🇳\".to_bytes()))\n  first: list(u8) = @(255, 255)\n  out(text_of(bytes.from_list(first)))\n  second: list(u8) = @(65, 200, 66)\n  out(text_of(bytes.from_list(second)))\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "中文\n😀\nabc🇻🇳\ninvalid:0:1\ninvalid:1:1\n");
}

#[test]
fn bytes_value_semantics_isolate_copies_across_fields_and_collections() {
    let source = format!(
        "{DECODE}data Packet:\n  payload: bytes\nfn touch(blob: bytes) -> int:\n  blob.len()\nfn main():\n  blob = \"Xin chào Vut\".to_bytes()\n  copied = blob\n  copied.set(0, 86)\n  out(text_of(blob))\n  out(text_of(copied))\n  packet = Packet(payload = blob)\n  out(\"$(packet.payload.len())\")\n  values: list(bytes) = @(blob, copied)\n  out(\"$(values.at(0).at(0)) $(values.at(1).at(0))\")\n  table: map(str, bytes) = map((\"a\", blob))\n  entry: bytes? = table.get(\"a\")\n  entry_text = 0\n  if entry != null:\n    entry_text = entry.len()\n  out(\"$(entry_text)\")\n  items: array(bytes, 2) = array(blob, copied)\n  out(\"$(items.at(0).at(0)) $(items.at(1).at(0))\")\n  out(\"$(touch(blob))\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "leak or crash: {stdout}");
    assert_eq!(
        stdout,
        "Xin chào Vut\nVin chào Vut\n13\n88 86\n13\n88 86\n13\n"
    );
}

#[test]
fn bytes_result_propagation_uses_typed_error() {
    let source = "fn read_text(blob: bytes) -> result(str, Utf8Error):\n  value = blob.to_str()?\n  ok(value)\nfn main():\n  match read_text(\"hi\".to_bytes()):\n    ok(value): out(value)\n    err(error): out(\"bad $(error.valid_up_to)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\n");
}

#[test]
fn bytes_large_buffers_grow_without_corruption() {
    let values: Vec<i64> = (0..1000).map(|index| index % 256).collect();
    let literal = values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!(
        "fn main():\n  values: list(u8) = @({literal})\n  blob = bytes.from_list(values)\n  out(\"$(blob.len()) $(blob.at(0)) $(blob.at(255)) $(blob.at(999))\")\n  blob.reserve(8192)\n  out(\"$(blob.capacity() >= 8192)\")\n  out(\"$(blob.at(999))\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1000 0 255 231\n1\n231\n");
}
