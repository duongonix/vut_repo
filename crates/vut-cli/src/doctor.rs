//! `vut doctor`: diagnose the local toolchain, runtime archives and backend.
//!
//! It reports what is present and turns missing prerequisites into actionable
//! messages instead of raw linker failures.
use vut_linker::{
    BackendKind, TargetProfile, driver_program, requested_kind, resolve_static_archive,
    toolchain_hint,
};

use crate::discovery;

/// Runs the doctor command for a target (defaults to the host triple).
///
/// # Errors
/// Returns an error when the target triple is malformed.
pub fn run(target: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let target = target.unwrap_or_else(|| vut_compiler::CompilerConfig::default().target);
    let profile = TargetProfile::parse(&target)?;
    let kind = requested_kind();
    let mut issues: Vec<String> = Vec::new();

    println!("Vut doctor");
    println!("  target:  {}", profile.triple());
    println!(
        "  backend: {} (override with VUT_LINKER=rustc|system|lld)",
        backend_label(kind)
    );

    report_driver(&profile, kind, &mut issues);
    report_rustc();
    report_assets(&target, &mut issues);

    if issues.is_empty() {
        println!("\nstatus: ok");
    } else {
        println!("\nstatus: issues");
        for issue in &issues {
            println!("  - {issue}");
        }
    }
    Ok(())
}

fn backend_label(kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::Rustc => "rustc",
        BackendKind::System => "system",
        BackendKind::Lld => "lld",
    }
}

fn report_driver(profile: &TargetProfile, kind: BackendKind, issues: &mut Vec<String>) {
    let driver = driver_program(profile, kind);
    if let Some(path) = discovery::find_on_path(driver) {
        println!("  linker:  {driver} ({})", path.display());
        return;
    }
    println!("  linker:  {driver} NOT FOUND");
    let hint = if kind == BackendKind::Rustc {
        "install a Rust toolchain (development fallback only)"
    } else {
        toolchain_hint(profile)
    };
    issues.push(format!("`{driver}` not found; {hint}"));
}

fn report_rustc() {
    if let Some(path) = discovery::find_on_path("rustc") {
        println!(
            "  rustc:   {} (development startup build only)",
            path.display()
        );
    } else {
        println!("  rustc:   not found (only needed for the development startup build)");
    }
}

fn report_assets(target: &str, issues: &mut Vec<String>) {
    let Some(runtime) = vut_paths::runtime_library(target) else {
        println!("  core:    runtime archive NOT FOUND");
        issues.push(
            "Vut runtime archive not found; set VUT_RUNTIME_LIBRARY or build the runtime".into(),
        );
        return;
    };
    println!("  core:    {}", runtime.display());
    match resolve_static_archive(&runtime) {
        Ok(archive) if archive != runtime => {
            println!("  core:    static archive {}", archive.display());
        }
        Ok(_) => {}
        Err(error) => {
            println!("  core:    static archive MISSING");
            issues.push(error.message().to_owned());
        }
    }

    if let Some(stdlib) = vut_paths::stdlib_runtime_library(Some(&runtime), target) {
        println!("  stdlib:  {}", stdlib.display());
    } else {
        println!("  stdlib:  NOT FOUND");
        issues.push("native stdlib runtime archive not found".into());
    }

    if let Some(startup) = vut_paths::startup_object(target) {
        println!("  startup: {}", startup.display());
    } else {
        println!(
            "  startup: not found (built with rustc in development; shipped in a distribution)"
        );
    }
}
