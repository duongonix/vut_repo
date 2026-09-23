//! Native linking end-to-end: the default resolver must link and run both Debug
//! and Release builds without a per-build backend switch.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{BuildMode, CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn run(source: &str, mode: BuildMode) -> (Option<i32>, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-link-e2e-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        build_mode: mode,
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
fn debug_and_release_link_and_run_identically() {
    // The reported bug: a Release `vut` selected the platform linker while Debug
    // used `rustc`. The resolver now makes the choice environment-driven, so
    // both build modes link and run.
    let source = "fn main():\n  out(\"hello, vut\")\n";
    let (debug_code, debug_out) = run(source, BuildMode::Debug);
    let (release_code, release_out) = run(source, BuildMode::Release);
    assert_eq!(debug_code, Some(0), "debug: {debug_out}");
    assert_eq!(release_code, Some(0), "release: {release_out}");
    assert_eq!(debug_out, "hello, vut\n");
    assert_eq!(release_out, "hello, vut\n");
}

#[test]
fn linker_resolver_finds_a_host_linker() {
    let host = vut_codegen::Target::host().triple;
    let profile = vut_linker::TargetProfile::parse(&host).expect("host triple");
    let resolved = vut_linker::resolve(&profile).expect("a host linker");
    assert!(!resolved.program.as_os_str().is_empty());
}
