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
    let root = std::env::temp_dir().join(format!("vut-phase-a-bytes-{nonce}-{sequence}"));
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
fn bytes_buffer_operations() {
    let source = "fn main():\n  blob = bytes()\n  blob.push(104)\n  blob.push(105)\n  out(\"blob=$blob\")\n  raw: list(u8) = @(33, 33)\n  extra = bytes.from_list(raw)\n  blob.extend(extra)\n  out(\"extended=$blob\")\n  out(\"find=$(blob.find(extra))\")\n  out(\"starts=$(blob.starts_with(extra))\")\n  out(\"ends=$(blob.ends_with(extra))\")\n  blob.truncate(2)\n  out(\"truncated=$blob\")\n  blob.resize(4, 65)\n  out(\"resized=$blob\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "blob=[104, 105]\nextended=[104, 105, 33, 33]\nfind=2\nstarts=0\nends=1\ntruncated=[104, 105]\nresized=[104, 105, 65, 65]\n"
    );
}

#[test]
fn bytes_hex_roundtrip_and_hex_error() {
    let source = "fn main():\n  raw: list(u8) = @(104, 105, 65, 65)\n  blob = bytes.from_list(raw)\n  hex = blob.to_hex()\n  out(\"hex=$hex\")\n  match bytes.from_hex(hex):\n    ok(value): out(\"roundtrip=$value\")\n    err(error): out(\"error idx=$(error.index)\")\n  match bytes.from_hex(\"zz\"):\n    ok(value): out(\"bad=$value\")\n    err(error): out(\"bad idx=$(error.index)\")\n  match bytes.from_hex(\"abc\"):\n    ok(value): out(\"odd=$value\")\n    err(error): out(\"odd idx=$(error.index)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "hex=68694141\nroundtrip=[104, 105, 65, 65]\nbad idx=0\nodd idx=3\n"
    );
}

#[test]
fn bytes_explicit_endian_roundtrips() {
    let source = "fn main():\n  buf = bytes()\n  buf.resize(8, 0)\n  buf.write_u32_le(0, 305419896)\n  out(\"le_hex=$(buf.to_hex())\")\n  out(\"read_le=$(buf.read_u32_le(0))\")\n  out(\"read_be=$(buf.read_u32_be(0))\")\n  buf.write_i16_be(4, 258)\n  out(\"i16_be=$(buf.read_i16_be(4))\")\n  out(\"i16_le=$(buf.read_i16_le(4))\")\n  buf.write_u64_le(0, 11259375)\n  out(\"u64=$(buf.read_u64_le(0))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "le_hex=7856341200000000\nread_le=305419896\nread_be=2018915346\ni16_be=258\ni16_le=513\nu64=11259375\n"
    );
}

#[test]
fn large_bytes_do_not_leak() {
    let source = "fn main():\n  blob = bytes()\n  blob.resize(500000, 7)\n  out(\"len=$(blob.len())\")\n  blob.write_u32_le(499996, 4294967295)\n  out(\"tail=$(blob.byte_at(499996))\")\n  blob.truncate(1000)\n  out(\"short=$(blob.len())\")\n  hex = blob.to_hex()\n  out(\"hexlen=$(hex.byte_len())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "len=500000\ntail=255\nshort=1000\nhexlen=2000\n");
}
