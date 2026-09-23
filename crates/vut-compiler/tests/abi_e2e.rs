//! Runtime ABI compatibility: a mismatched installed runtime must fail early.
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession, RUNTIME_ABI_VERSION, runtime_abi_mismatch};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-abi-{label}-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    root
}

#[test]
fn matching_installed_runtime_abi_is_accepted() {
    assert!(runtime_abi_mismatch(Some(RUNTIME_ABI_VERSION)).is_none());
    assert!(runtime_abi_mismatch(None).is_none());
}

#[test]
fn mismatched_installed_runtime_abi_is_rejected() {
    let error = runtime_abi_mismatch(Some(RUNTIME_ABI_VERSION + 1))
        .expect("a mismatched runtime ABI must produce an error");
    assert!(
        error.to_string().contains("runtime ABI mismatch"),
        "{error}"
    );
}

#[test]
fn development_tree_without_manifest_compiles() {
    let project = scratch("dev");
    fs::write(project.join("main.vut"), "fn main() -> int:\n  0\n").expect("write source");
    let executable = project.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        ..CompilerConfig::default()
    };
    // The development tree has no install manifest, so the ABI check passes.
    let result = CompilerSession::new(config).emit_executable(&project, &[], &executable);
    assert!(result.is_ok(), "{result:?}");
    fs::remove_dir_all(&project).ok();
}
