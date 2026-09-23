//! Shared harness for stdlib integration tests.
//!
//! Compiles a single-module Vut program with the standard library available,
//! links it against the native runtime archives, runs it, and returns the
//! result. A clean exit code also means the runtime leak detector found no live
//! managed allocation.
#![allow(dead_code)]

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Path of the core runtime archive produced by `cargo build -p vut-runtime`.
#[must_use]
pub fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn unique_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vut-stdlib-{name}-{nonce}-{sequence}"))
}

pub fn scratch(name: &str) -> Scratch {
    let root = unique_root(name);
    fs::create_dir_all(&root).expect("create scratch root");
    Scratch { root }
}

/// Compiles `source` (with the stdlib available) into an executable.
///
/// Returns the scratch project root (kept alive by the caller, removed on drop)
/// and the executable path.
pub fn build(name: &str, source: &str) -> (Scratch, PathBuf) {
    let scratch = scratch(name);
    let root = &scratch.root;
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
    let mut session = CompilerSession::new(config);
    let checked = session
        .check_source_file(&root.join("main.vut"))
        .expect("check source");
    let diagnostics = session.render_diagnostics(&checked);
    assert!(
        !checked.resolution.diagnostics.has_errors() && !checked.semantics.diagnostics.has_errors(),
        "{diagnostics}"
    );
    session
        .emit_executable(root, &[], &executable)
        .expect("emit executable");
    (scratch, executable)
}

/// Compiles and runs `source`, returning `(exit code, stdout, stderr)`.
#[must_use]
pub fn run(name: &str, source: &str) -> (Option<i32>, String, String) {
    let (_scratch, executable) = build(name, source);
    run_executable(&executable, &[], None)
}

/// Compiles and runs `source` feeding `stdin` to the program.
#[must_use]
pub fn run_with_stdin(name: &str, source: &str, stdin: &str) -> (Option<i32>, String, String) {
    let (_scratch, executable) = build(name, source);
    run_executable(&executable, &[], Some(stdin))
}

/// Runs a prebuilt executable with optional args and stdin.
#[must_use]
pub fn run_executable(
    executable: &Path,
    args: &[&str],
    stdin: Option<&str>,
) -> (Option<i32>, String, String) {
    let mut command = Command::new(executable);
    command.args(args);
    let output = if let Some(input) = stdin {
        command.stdin(Stdio::piped()).stdout(Stdio::piped());
        let mut child = command.spawn().expect("spawn program");
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
        child.wait_with_output().expect("run program")
    } else {
        command.output().expect("run program")
    };
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A scratch project directory removed when dropped.
pub struct Scratch {
    pub root: PathBuf,
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
