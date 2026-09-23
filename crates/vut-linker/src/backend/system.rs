//! Standalone system linker backend.
//!
//! Drives the platform toolchain directly: MSVC `link.exe` or LLVM `lld-link`
//! on Windows MSVC, `cc` or `ld.lld` elsewhere. It resolves the prebuilt
//! `vut-startup` object, maps runtime `.rlib` inputs to their C-linkable
//! staticlib siblings, links the runtime archives, and adds the system
//! libraries and library search paths from the resolved linker. No `rustc` is
//! invoked for the final link.
use std::path::{Path, PathBuf};

use crate::assets;
use crate::backend::{BackendKind, LinkerBackend, toolchain_hint};
use crate::error::{LinkError, LinkFailure};
use crate::plan::LinkPlan;
use crate::process;
use crate::resolver::ResolvedLinker;
use crate::startup::StartupObject;
use crate::system_libs;
use crate::target::{Flavor, TargetProfile};

/// Links through the platform-native (or LLVM lld) driver.
pub struct SystemBackend {
    kind: BackendKind,
    profile: TargetProfile,
    program: PathBuf,
    search_paths: Vec<PathBuf>,
    environment: Vec<(String, String)>,
}

impl SystemBackend {
    #[must_use]
    pub fn new(resolved: ResolvedLinker, profile: TargetProfile) -> Self {
        Self {
            kind: resolved.kind,
            profile,
            program: resolved.program,
            search_paths: resolved.search_paths,
            environment: resolved.environment,
        }
    }

    /// Linker program for this backend and target.
    #[must_use]
    pub(crate) fn driver(&self) -> PathBuf {
        self.program.clone()
    }

    /// All library search paths: the plan's explicit ones plus the resolved
    /// platform/SDK ones, deduplicated.
    fn search_paths(&self, plan: &LinkPlan) -> Vec<PathBuf> {
        let mut seen = std::collections::HashSet::new();
        self.search_paths
            .iter()
            .chain(plan.search_paths.iter())
            .filter(|path| seen.insert(path.as_os_str().to_owned()))
            .cloned()
            .collect()
    }

    /// Full argument vector for a resolved startup object.
    ///
    /// # Errors
    /// Returns [`LinkFailure::MissingLibrary`] when a runtime `.rlib` has no
    /// staticlib sibling.
    pub(crate) fn argv(&self, plan: &LinkPlan, startup: &Path) -> Result<Vec<String>, LinkError> {
        match self.profile.flavor() {
            Flavor::Msvc => self.msvc_argv(plan, startup),
            _ => self.unix_argv(plan, startup),
        }
    }

    fn msvc_argv(&self, plan: &LinkPlan, startup: &Path) -> Result<Vec<String>, LinkError> {
        let mut arguments = vec![
            "/NOLOGO".into(),
            "/SUBSYSTEM:CONSOLE".into(),
            format!("/OUT:{}", plan.output.display()),
            startup.display().to_string(),
        ];
        for path in self.search_paths(plan) {
            arguments.push(format!("/LIBPATH:{}", path.display()));
        }
        append_archives(&mut arguments, plan)?;
        for library in system_libs::system_libraries(&self.profile) {
            arguments.push(with_lib_extension(library));
        }
        Ok(arguments)
    }

    fn unix_argv(&self, plan: &LinkPlan, startup: &Path) -> Result<Vec<String>, LinkError> {
        let mut arguments = vec![
            "-o".into(),
            plan.output.display().to_string(),
            startup.display().to_string(),
        ];
        for path in self.search_paths(plan) {
            arguments.push(format!("-L{}", path.display()));
        }
        append_archives(&mut arguments, plan)?;
        for library in system_libs::system_libraries(&self.profile) {
            arguments.push(format!("-l{library}"));
        }
        for framework in system_libs::frameworks(&self.profile) {
            arguments.push("-framework".into());
            arguments.push(framework.to_owned());
        }
        for framework in &plan.frameworks {
            arguments.push("-framework".into());
            arguments.push(framework.clone());
        }
        Ok(arguments)
    }

    fn resolve_startup(&self, plan: &LinkPlan) -> Result<PathBuf, LinkError> {
        let search_dir = plan
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.parent())
            .map(Path::to_path_buf);
        let cache_dir = std::env::temp_dir().join("vut-startup");
        StartupObject::resolve(
            &self.profile,
            plan.startup.as_deref(),
            search_dir.as_deref(),
            &cache_dir,
        )
    }

    /// Adds a platform toolchain hint to discovery/SDK failures.
    fn enrich(&self, error: LinkError) -> LinkError {
        match error.failure() {
            LinkFailure::ToolNotFound | LinkFailure::MissingSdk | LinkFailure::MissingCrt => {
                error.with_hint(toolchain_hint(&self.profile))
            }
            _ => error,
        }
    }
}

impl LinkerBackend for SystemBackend {
    fn name(&self) -> &'static str {
        match self.kind {
            BackendKind::Lld => "lld",
            _ => "system",
        }
    }
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError> {
        let startup = self.resolve_startup(plan)?;
        let program = self.driver();
        let arguments = self.argv(plan, &startup)?;
        process::run(&program, &arguments, &plan.output, &self.environment)
            .map_err(|error| self.enrich(error))
    }
}

/// Appends the runtime and static archives, mapping `.rlib` to staticlibs.
fn append_archives(arguments: &mut Vec<String>, plan: &LinkPlan) -> Result<(), LinkError> {
    for object in &plan.objects {
        arguments.push(object.display().to_string());
    }
    if let Some(runtime) = &plan.runtime {
        arguments.push(
            assets::resolve_static_archive(runtime)?
                .display()
                .to_string(),
        );
    }
    for library in plan.unique_static_libraries() {
        arguments.push(
            assets::resolve_static_archive(library)?
                .display()
                .to_string(),
        );
    }
    Ok(())
}

fn with_lib_extension(name: &str) -> String {
    if name.to_ascii_lowercase().ends_with(".lib") {
        name.to_owned()
    } else {
        format!("{name}.lib")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::LinkerSource;

    fn resolved(kind: BackendKind, profile: &TargetProfile) -> ResolvedLinker {
        ResolvedLinker {
            kind,
            program: PathBuf::from(crate::backend::driver_program(profile, kind)),
            search_paths: Vec::new(),
            environment: Vec::new(),
            source: LinkerSource::System,
        }
    }

    fn plan(target: &str) -> LinkPlan {
        LinkPlan {
            objects: vec![PathBuf::from("program.o")],
            static_libraries: vec![PathBuf::from("libvut-stdlib.a")],
            runtime: Some(PathBuf::from("libvut-core.a")),
            startup: None,
            output: PathBuf::from("app"),
            target: target.into(),
            ..LinkPlan::default()
        }
    }

    #[test]
    fn msvc_uses_link_exe_with_crt_libraries() {
        let profile = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        let backend = SystemBackend::new(resolved(BackendKind::System, &profile), profile);
        assert_eq!(backend.driver(), PathBuf::from("link.exe"));
        let arguments = backend
            .argv(
                &plan("x86_64-pc-windows-msvc"),
                Path::new("vut-startup.obj"),
            )
            .expect("argv");
        assert!(arguments.iter().any(|arg| arg == "/SUBSYSTEM:CONSOLE"));
        assert!(arguments.iter().any(|arg| arg == "vut-startup.obj"));
        assert!(arguments.iter().any(|arg| arg == "program.o"));
        assert!(arguments.iter().any(|arg| arg == "libvut-core.a"));
        assert!(arguments.iter().any(|arg| arg == "msvcrt.lib"));
        assert!(arguments.iter().any(|arg| arg == "kernel32.lib"));
        assert!(arguments.iter().any(|arg| arg == "bcrypt.lib"));
    }

    #[test]
    fn msvc_emits_libpath_search_paths() {
        let profile = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        let mut resolved = resolved(BackendKind::System, &profile);
        resolved.search_paths = vec![PathBuf::from("C:/SDK/lib")];
        let backend = SystemBackend::new(resolved, profile);
        let mut plan = plan("x86_64-pc-windows-msvc");
        plan.search_paths = vec![PathBuf::from("C:/project/lib")];
        let arguments = backend
            .argv(&plan, Path::new("vut-startup.obj"))
            .expect("argv");
        assert!(
            arguments.iter().any(|arg| arg == "/LIBPATH:C:/SDK/lib"),
            "{arguments:?}"
        );
        assert!(
            arguments.iter().any(|arg| arg == "/LIBPATH:C:/project/lib"),
            "{arguments:?}"
        );
    }

    #[test]
    fn lld_backend_uses_lld_link_on_windows() {
        let profile = TargetProfile::parse("aarch64-pc-windows-msvc").unwrap();
        let backend = SystemBackend::new(resolved(BackendKind::Lld, &profile), profile);
        assert_eq!(backend.driver(), PathBuf::from("lld-link"));
    }

    #[test]
    fn linux_uses_cc_with_gcc_runtime_libraries() {
        let profile = TargetProfile::parse("x86_64-unknown-linux-gnu").unwrap();
        let backend = SystemBackend::new(resolved(BackendKind::System, &profile), profile);
        assert_eq!(backend.driver(), PathBuf::from("cc"));
        let arguments = backend
            .argv(
                &plan("x86_64-unknown-linux-gnu"),
                Path::new("vut-startup.o"),
            )
            .expect("argv");
        assert_eq!(arguments.first().map(String::as_str), Some("-o"));
        assert!(arguments.iter().any(|arg| arg == "-lpthread"));
        assert!(arguments.iter().any(|arg| arg == "-lgcc_s"));
        assert!(arguments.iter().any(|arg| arg == "-ldl"));
    }

    #[test]
    fn unix_emits_l_search_paths() {
        let profile = TargetProfile::parse("x86_64-unknown-linux-gnu").unwrap();
        let backend = SystemBackend::new(resolved(BackendKind::System, &profile), profile);
        let mut plan = plan("x86_64-unknown-linux-gnu");
        plan.search_paths = vec![PathBuf::from("/usr/local/lib")];
        let arguments = backend
            .argv(&plan, Path::new("vut-startup.o"))
            .expect("argv");
        assert!(
            arguments.iter().any(|arg| arg == "-L/usr/local/lib"),
            "{arguments:?}"
        );
    }

    #[test]
    fn darwin_adds_frameworks() {
        let profile = TargetProfile::parse("aarch64-apple-darwin").unwrap();
        let backend = SystemBackend::new(resolved(BackendKind::System, &profile), profile);
        let arguments = backend
            .argv(&plan("aarch64-apple-darwin"), Path::new("vut-startup.o"))
            .expect("argv");
        assert!(arguments.iter().any(|arg| arg == "-lSystem"));
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["-framework", "Security"])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["-framework", "CoreFoundation"])
        );
    }

    #[test]
    fn rlib_runtime_without_static_sibling_fails_with_a_hint() {
        let profile = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        let backend = SystemBackend::new(resolved(BackendKind::System, &profile), profile);
        let mut plan = plan("x86_64-pc-windows-msvc");
        plan.runtime = Some(PathBuf::from("definitely-missing/libvut_runtime.rlib"));
        let error = backend
            .argv(&plan, Path::new("vut-startup.obj"))
            .expect_err("missing static archive");
        assert_eq!(error.failure(), LinkFailure::MissingLibrary);
    }
}
