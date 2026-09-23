//! `vut doctor`: diagnose the local toolchain, runtime archives, SDK and
//! backend, then run a real compile → link → run smoke test.
//!
//! It reports what is present and turns missing prerequisites into actionable
//! messages instead of raw linker failures. `--json` emits a machine-readable
//! report for installers and CI.
mod report;

pub use report::{Asset, Report, Sdk, Status};

use std::path::PathBuf;

use report::{Abi, Linker, PathInfo, Smoke};
use vut_linker::{BackendKind, MsvcArch, TargetProfile, find_on_path, resolve};

/// Runs the doctor command for a target (defaults to the host triple).
///
/// Returns `true` when the toolchain is ready.
///
/// # Errors
/// Returns an error when the target triple is malformed.
pub fn run(target: Option<String>, json: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let target = target.unwrap_or_else(|| vut_compiler::CompilerConfig::default().target);
    let profile = TargetProfile::parse(&target)?;
    let report = diagnose(&target, &profile);
    if json {
        println!("{}", report.to_json());
    } else {
        render(&report);
    }
    Ok(report.ready())
}

#[expect(
    clippy::too_many_lines,
    reason = "the diagnostic checklist is intentionally reported in one place"
)]
fn diagnose(target: &str, profile: &TargetProfile) -> Report {
    let mut warnings = Vec::new();
    let mut issues = Vec::new();

    // Linker + library search paths.
    let linker = match resolve(profile) {
        Ok(resolved) => {
            let search_paths = resolved
                .search_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect();
            Some(Linker {
                program: resolved.program.display().to_string(),
                backend: backend_label(resolved.kind).to_owned(),
                source: resolved.source.as_str().to_owned(),
                search_paths,
            })
        }
        Err(error) => {
            issues.push(error.message().to_owned());
            None
        }
    };

    // Runtime + native runtime + startup artifacts.
    let runtime = if let Some(path) = vut_paths::runtime_library(target) {
        match vut_linker::resolve_static_archive(&path) {
            Ok(archive) if archive != path => Asset {
                present: true,
                path: Some(path.display().to_string()),
                detail: Some(archive.display().to_string()),
            },
            Ok(_) => Asset::present(path.display().to_string()),
            Err(error) => {
                issues.push(error.message().to_owned());
                Asset::missing("no linkable static archive")
            }
        }
    } else {
        issues.push(
            "Vut runtime archive not found; set VUT_RUNTIME_LIBRARY or install the runtime".into(),
        );
        Asset::missing("runtime archive not found")
    };
    let native_runtime = if let Some(path) = vut_paths::stdlib_runtime_library(
        runtime.path.as_ref().map(PathBuf::from).as_deref(),
        target,
    ) {
        Asset::present(path.display().to_string())
    } else {
        issues.push("native stdlib runtime archive not found".into());
        Asset::missing("native stdlib archive not found")
    };
    let startup = if let Some(path) = vut_paths::startup_object(target) {
        Asset::present(path.display().to_string())
    } else {
        warnings.push("startup object not found (shipped in a distribution)".into());
        Asset::missing("startup object not found")
    };

    // ABI + manifest format.
    let expected = vut_compiler::RUNTIME_ABI_VERSION;
    let installed = vut_paths::installed_abi_version();
    let compatible = vut_paths::abi_compatible(installed, expected);
    if !compatible {
        issues.push(format!(
            "runtime ABI mismatch: compiler expects {expected}, installed runtime is {}",
            installed.map_or_else(|| "none".into(), |value| value.to_string())
        ));
    }
    let abi = Abi {
        expected,
        installed,
        compatible,
        manifest_format: vut_paths::installed_manifest_u32("format_version"),
    };

    // Toolchain companions.
    let vpm = sibling_version("vpm");
    let vut_lsp = sibling_version("vut-lsp");
    if vpm.is_none() {
        warnings.push("`vpm` was not found next to `vut`".into());
    }
    if vut_lsp.is_none() {
        warnings.push("`vut-lsp` was not found next to `vut`".into());
    }

    // SDK / platform toolchain.
    let sdk = sdk_status(target);
    if target.contains("windows") && !sdk.windows_build_tools {
        warnings.push(
            "Windows SDK/Build Tools not detected; install the C++ workload and Windows SDK".into(),
        );
    }
    if target.contains("darwin") && !sdk.xcode_clt {
        warnings.push("Xcode Command Line Tools not detected; run `xcode-select --install`".into());
    }
    if target.contains("linux") && !sdk.cc {
        warnings.push("no C toolchain (`cc`/`clang`/`gcc`) found for the final link".into());
    }

    // PATH.
    let bin_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_default();
    let on_path = std::env::var_os("PATH")
        .is_some_and(|value| std::env::split_paths(&value).any(|entry| entry == bin_dir));
    if !on_path {
        warnings.push(format!(
            "{} is not on PATH; open a new shell or add it",
            bin_dir.display()
        ));
    }
    let path = PathInfo {
        bin_dir: bin_dir.display().to_string(),
        on_path,
    };

    let stdlib_root = vut_paths::stdlib_root().map(|path| path.display().to_string());
    if stdlib_root.is_none() {
        warnings.push("standard library source not found".into());
    }

    // Real smoke test: compile → link → run.
    let smoke = if issues.is_empty() {
        Some(smoke_test(target))
    } else {
        None
    };
    if let Some(smoke) = &smoke
        && !smoke.ran
    {
        issues.push(format!("smoke test failed: {}", smoke.detail));
    }

    let status = if issues.is_empty() {
        Status::Ok
    } else {
        Status::Issues
    };
    Report {
        status,
        target: target.to_owned(),
        compiler: format!("vut {}", env!("CARGO_PKG_VERSION")),
        vpm,
        vut_lsp,
        stdlib_root,
        stdlib_version: vut_paths::installed_manifest_string("stdlib"),
        runtime,
        native_runtime,
        startup,
        abi,
        linker,
        sdk,
        path,
        smoke,
        warnings,
        issues,
    }
}

/// Compiles, links, and runs a trivial program to prove the toolchain works.
fn smoke_test(target: &str) -> Smoke {
    use vut_compiler::{CompilerConfig, CompilerSession};
    let root = std::env::temp_dir().join(format!("vut-doctor-smoke-{}", std::process::id()));
    if std::fs::create_dir_all(&root).is_err() {
        return Smoke {
            ran: false,
            detail: "cannot create a scratch directory".into(),
        };
    }
    let _ = std::fs::write(root.join("main.vut"), "fn main() -> int:\n  0\n");
    let executable = root.join(if cfg!(windows) { "smoke.exe" } else { "smoke" });
    let config = CompilerConfig {
        runtime_library: vut_paths::runtime_library(target),
        startup_object: vut_paths::startup_object(target),
        ..CompilerConfig::default()
    };
    let outcome = CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .map_err(|error| error.to_string())
        .and_then(|()| {
            std::process::Command::new(&executable)
                .status()
                .map_err(|error| error.to_string())
        });
    let _ = std::fs::remove_dir_all(&root);
    match outcome {
        Ok(status) if status.success() => Smoke {
            ran: true,
            detail: "compiled, linked, ran".into(),
        },
        Ok(status) => Smoke {
            ran: false,
            detail: format!("the smoke-test program exited with {status:?}"),
        },
        Err(error) => Smoke {
            ran: false,
            detail: error,
        },
    }
}

fn sibling_version(name: &str) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let path = exe.parent()?.join(file);
    if !path.is_file() {
        return None;
    }
    let output = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !stdout.is_empty() {
        return Some(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    (!stderr.is_empty()).then_some(stderr)
}

fn sdk_status(target: &str) -> Sdk {
    let windows = target.contains("windows");
    let darwin = target.contains("darwin");
    let windows_build_tools = windows && msvc_present(target);
    let xcode_clt = darwin && xcode_clt_present();
    let cc = ["cc", "clang", "gcc", "cl.exe"]
        .iter()
        .any(|name| find_on_path(name).is_some());
    Sdk {
        windows_build_tools,
        xcode_clt,
        cc,
    }
}

fn msvc_present(target: &str) -> bool {
    let arch = if target.starts_with("aarch64") {
        MsvcArch::Aarch64
    } else {
        MsvcArch::X64
    };
    vut_linker::discover_msvc(arch).is_some()
}

fn xcode_clt_present() -> bool {
    std::process::Command::new("xcode-select")
        .arg("-p")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn backend_label(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::Rustc => "rustc",
        BackendKind::System => "system",
        BackendKind::Lld => "lld",
    }
}

fn render(report: &Report) {
    println!("Vut doctor");
    println!("  compiler: {}", report.compiler);
    println!(
        "  vpm:      {}",
        report.vpm.as_deref().unwrap_or("not found")
    );
    println!(
        "  vut-lsp:  {}",
        report.vut_lsp.as_deref().unwrap_or("not found")
    );
    println!("  target:   {}", report.target);
    println!(
        "  stdlib:   {}",
        report.stdlib_root.as_deref().unwrap_or("NOT FOUND")
    );
    if let Some(version) = &report.stdlib_version {
        println!("  stdlib v: {version}");
    }
    match &report.linker {
        Some(linker) => {
            println!(
                "  linker:   {} ({}, {})",
                linker.program, linker.backend, linker.source
            );
            for path in &linker.search_paths {
                println!("  libpath:  {path}");
            }
        }
        None => println!("  linker:   NOT FOUND"),
    }
    println!("  core:     {}", asset_line(&report.runtime));
    println!("  stdlib rt:{}", asset_line(&report.native_runtime));
    println!("  startup:  {}", asset_line(&report.startup));
    let abi = &report.abi;
    match abi.installed {
        Some(installed) if abi.compatible => {
            println!("  abi:      {installed} (compatible)");
        }
        Some(installed) => {
            println!(
                "  abi:      {installed} (compiler expects {})",
                abi.expected
            );
        }
        None => println!(
            "  abi:      {} (no install manifest; development tree)",
            abi.expected
        ),
    }
    if let Some(format) = abi.manifest_format {
        println!("  manifest: format {format}");
    }
    println!(
        "  sdk:      windows_build_tools={} xcode_clt={} cc={}",
        report.sdk.windows_build_tools, report.sdk.xcode_clt, report.sdk.cc
    );
    println!(
        "  path:     {} ({})",
        report.path.bin_dir,
        if report.path.on_path {
            "on PATH"
        } else {
            "NOT on PATH"
        }
    );
    if let Some(smoke) = &report.smoke {
        println!(
            "  smoke:    {} ({})",
            if smoke.ran { "ok" } else { "FAILED" },
            smoke.detail
        );
    }
    for warning in &report.warnings {
        println!("  warning:  {warning}");
    }
    match report.status {
        Status::Ok => println!("\nstatus: ok (READY)"),
        Status::Issues => {
            println!("\nstatus: issues");
            for issue in &report.issues {
                println!("  - {issue}");
            }
        }
    }
}

fn asset_line(asset: &Asset) -> String {
    if asset.present {
        asset.path.clone().unwrap_or_else(|| "present".into())
    } else {
        format!(
            "NOT FOUND ({})",
            asset.detail.as_deref().unwrap_or("missing")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_labels_are_stable() {
        assert_eq!(backend_label(BackendKind::Rustc), "rustc");
        assert_eq!(backend_label(BackendKind::System), "system");
        assert_eq!(backend_label(BackendKind::Lld), "lld");
    }

    #[test]
    fn json_report_round_trips_through_serde() {
        let report = Report {
            status: Status::Ok,
            target: "x86_64-unknown-linux-gnu".into(),
            compiler: "vut 0.1.0".into(),
            vpm: None,
            vut_lsp: None,
            stdlib_root: None,
            stdlib_version: None,
            runtime: Asset::missing("x"),
            native_runtime: Asset::missing("x"),
            startup: Asset::missing("x"),
            abi: Abi {
                expected: 13,
                installed: Some(13),
                compatible: true,
                manifest_format: Some(1),
            },
            linker: None,
            sdk: Sdk {
                windows_build_tools: false,
                xcode_clt: false,
                cc: true,
            },
            path: PathInfo {
                bin_dir: "/tmp".into(),
                on_path: false,
            },
            smoke: None,
            warnings: Vec::new(),
            issues: Vec::new(),
        };
        let json = report.to_json();
        assert!(json.contains("\"status\": \"ok\""));
        assert!(json.contains("\"expected\": 13"));
    }
}
