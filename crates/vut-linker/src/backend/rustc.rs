//! `rustc`-driven backend (development/migration fallback).
//!
//! `rustc` supplies each platform's native system libraries and compiles a tiny
//! startup shim that defines `main`. This backend must never be required by a
//! release installation; see the module docs in [`super`].
use std::path::PathBuf;

use crate::backend::LinkerBackend;
use crate::error::LinkError;
use crate::plan::LinkPlan;
use crate::process;
use crate::resolver::ResolvedLinker;

/// Portable startup shim compiled by `rustc` at link time.
const SHIM_SOURCE: &str = include_str!("../link_shim.rs");

/// Links through `rustc`.
pub struct RustcBackend {
    program: PathBuf,
    search_paths: Vec<PathBuf>,
    environment: Vec<(String, String)>,
}

impl RustcBackend {
    #[must_use]
    pub fn new(resolved: ResolvedLinker) -> Self {
        Self {
            program: resolved.program,
            search_paths: resolved.search_paths,
            environment: resolved.environment,
        }
    }
}

impl LinkerBackend for RustcBackend {
    fn name(&self) -> &'static str {
        "rustc"
    }
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError> {
        let arguments = command(plan, &self.search_paths)?;
        process::run(&self.program, &arguments, &plan.output, &self.environment)
    }
}

/// Builds the `rustc` argument vector for a plan.
///
/// # Errors
/// Returns an error when the temporary shim cannot be written.
pub(crate) fn command(plan: &LinkPlan, search_paths: &[PathBuf]) -> Result<Vec<String>, LinkError> {
    let shim = write_shim()?;
    let crate_name = format!(
        "vut_link_{}_{}",
        std::process::id(),
        process::next_sequence()
    );
    let mut arguments = vec![
        shim.display().to_string(),
        "--crate-name".into(),
        crate_name,
        "-o".into(),
        plan.output.display().to_string(),
    ];
    for path in search_paths.iter().chain(plan.search_paths.iter()) {
        arguments.extend(["-C".into(), format!("link-arg=-L{}", path.display())]);
    }
    for object in &plan.objects {
        arguments.extend(["-C".into(), format!("link-arg={}", object.display())]);
    }
    if let Some(runtime) = &plan.runtime {
        arguments.extend(["-C".into(), format!("link-arg={}", runtime.display())]);
    }
    for library in plan.unique_static_libraries() {
        arguments.extend(["-C".into(), format!("link-arg={}", library.display())]);
    }
    let windows = plan.target.contains("windows") || cfg!(windows);
    for library in plan.unique_system_libraries() {
        let flag = if windows {
            if library.to_ascii_lowercase().ends_with(".lib") {
                library.to_owned()
            } else {
                format!("{library}.lib")
            }
        } else {
            format!("-l{library}")
        };
        arguments.extend(["-C".into(), format!("link-arg={flag}")]);
    }
    for framework in &plan.frameworks {
        arguments.extend([
            "-C".into(),
            "link-arg=-framework".into(),
            "-C".into(),
            format!("link-arg={framework}"),
        ]);
    }
    if windows {
        arguments.extend([
            "-C".into(),
            "link-arg=/subsystem:console".into(),
            // OS services used by the bundled native stdlib (cryptographic
            // randomness) whose `#[link]` metadata is lost when the archive is
            // passed directly to the linker.
            "-C".into(),
            "link-arg=bcrypt.lib".into(),
            "-C".into(),
            "link-arg=advapi32.lib".into(),
        ]);
    }
    Ok(arguments)
}

/// Writes the embedded startup shim to a unique temporary file.
fn write_shim() -> Result<PathBuf, LinkError> {
    let directory = std::env::temp_dir().join("vut-link-shim");
    std::fs::create_dir_all(&directory).map_err(|error| {
        LinkError::new(format!(
            "failed to create linker shim directory `{}`: {error}",
            directory.display()
        ))
    })?;
    let path = directory.join(format!(
        "link_shim_{}_{}.rs",
        std::process::id(),
        process::next_sequence()
    ));
    std::fs::write(&path, SHIM_SOURCE).map_err(|error| {
        LinkError::new(format!(
            "failed to write linker shim `{}`: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> LinkPlan {
        LinkPlan {
            objects: vec![PathBuf::from("main.o")],
            static_libraries: vec![PathBuf::from("native.lib"), PathBuf::from("native.lib")],
            system_libraries: vec!["user32".into(), "user32".into()],
            frameworks: Vec::new(),
            runtime: Some(PathBuf::from("runtime.a")),
            startup: None,
            output: PathBuf::from("app"),
            entry: "vut_entry".into(),
            target: if cfg!(windows) {
                "x86_64-pc-windows-msvc".into()
            } else {
                "x86_64-unknown-linux-gnu".into()
            },
            ..LinkPlan::default()
        }
    }

    #[test]
    fn command_keeps_entry_and_artifacts_explicit() {
        let plan = plan();
        let arguments = command(&plan, &[]).expect("command");
        assert!(arguments.iter().any(|value| value.contains("main.o")));
        assert!(arguments.iter().any(|value| value.contains("runtime.a")));
        assert_eq!(
            arguments
                .iter()
                .filter(|value| value.contains("native.lib"))
                .count(),
            1,
            "duplicate static libraries are deduplicated"
        );
        assert!(
            arguments.iter().any(|value| std::path::Path::new(value)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))),
            "the embedded startup shim supplies the entry point"
        );
    }
}
