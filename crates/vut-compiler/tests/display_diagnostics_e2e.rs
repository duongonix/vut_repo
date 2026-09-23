//! The display pass must not hide type errors in `out`/`print` arguments or
//! template interpolations.
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn compile_error(source: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-display-diag-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    let result = CompilerSession::new(config).emit_executable(&root, &[], &executable);
    fs::remove_dir_all(&root).ok();
    match result {
        Ok(()) => panic!("expected a compile error"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn out_argument_type_error_is_reported() {
    let message = compile_error("fn main():\n  nums = @[1, 2, 3]\n  out(nums.join(\"-\"))\n");
    assert!(
        message.contains("E1028"),
        "join on list[int] must be reported, not hidden: {message}"
    );
}

#[test]
fn interpolation_type_error_is_reported() {
    let message =
        compile_error("fn main():\n  nums = @[1, 2, 3]\n  out(\"value=$(nums.nonexistent())\")\n");
    assert!(
        message.contains("E2005"),
        "an unknown method in interpolation must be reported: {message}"
    );
}
