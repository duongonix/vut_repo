//! End-to-end checks for the offline `vpm` command surface.
//!
//! These invoke the built `vpm` binary so the clap surface, validation, and
//! exit codes are exercised together. They never touch the network.

use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique(name: &str) -> PathBuf {
    let nonce = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vpm-cli-{name}-{}-{nonce}", std::process::id()))
}

fn package(name: &str, prebuilt: bool) -> PathBuf {
    let root = unique(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/bin")).unwrap();
    std::fs::write(
        root.join("vpm.toml"),
        format!(
            "[package]\nname = '{name}'\nversion = '1.0.0'\n\n[[bin]]\nname = '{name}'\npath = 'src/bin/{name}.vut'\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(format!("src/bin/{name}.vut")),
        "fn main():\n  out(\"hi\")\n",
    )
    .unwrap();
    if prebuilt {
        std::fs::create_dir_all(root.join("native")).unwrap();
        std::fs::write(root.join("native/math.lib"), "binary").unwrap();
    }
    root
}

fn vpm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vpm"))
}

#[test]
fn publish_dry_run_validates_without_network() {
    let root = package("math", false);
    let output = vpm()
        .current_dir(&root)
        .args(["publish", "--dry-run"])
        .output()
        .expect("run vpm");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("publish dry run"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publish_rejects_prebuilt_binaries() {
    let root = package("math", true);
    let output = vpm()
        .current_dir(&root)
        .args(["publish", "--dry-run"])
        .output()
        .expect("run vpm");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("prebuilt"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn install_without_lock_asks_for_update() {
    let root = unique("nolock");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("vpm.toml"),
        "[package]\nname = 'app'\nversion = '0.1.0'\n\n[dependencies]\nmath = '1.0.0'\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.vut"), "fn main():\n  0\n").unwrap();
    let output = vpm()
        .current_dir(&root)
        .arg("install")
        .output()
        .expect("run vpm");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("vpm update"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(root).unwrap();
}
