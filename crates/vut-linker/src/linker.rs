use std::{
    fmt,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
static LINK_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// Serializes native link invocations within a process. Linkers are external,
/// resource-heavy tools that share platform temporary files; running several
/// concurrently can race on those shared paths and yield a freshly linked
/// executable that is not immediately runnable.
static LINK_LOCK: Mutex<()> = Mutex::new(());
#[derive(Debug)]
pub struct LinkError(pub String);
impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LinkError {}

/// Typed description of a native link invocation.
///
/// Collects compiler objects, static libraries, system libraries and
/// frameworks so callers never pass ad-hoc string lists through the pipeline.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LinkPlan {
    pub objects: Vec<PathBuf>,
    pub static_libraries: Vec<PathBuf>,
    pub system_libraries: Vec<String>,
    pub frameworks: Vec<String>,
    pub runtime: Option<PathBuf>,
    pub output: PathBuf,
    pub entry: String,
    pub target: String,
}

impl LinkPlan {
    /// Returns static libraries in declaration order with duplicates removed.
    #[must_use]
    pub fn unique_static_libraries(&self) -> Vec<&Path> {
        let mut seen = std::collections::HashSet::new();
        self.static_libraries
            .iter()
            .filter(|path| seen.insert(path.as_os_str().to_owned()))
            .map(PathBuf::as_path)
            .collect()
    }
    #[must_use]
    pub fn unique_system_libraries(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        self.system_libraries
            .iter()
            .filter(|name| seen.insert(name.as_str()))
            .map(String::as_str)
            .collect()
    }
}

pub trait NativeLinker {
    /// Links a compiler object with any native libraries through the platform
    /// toolchain.
    /// # Errors
    /// Returns tool discovery and linker diagnostics.
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError>;
}
pub struct SystemLinker;
impl NativeLinker for SystemLinker {
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError> {
        let (program, args) = command(plan);
        let _guard = LINK_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = Command::new(&program)
            .args(args)
            .output()
            .map_err(|e| LinkError(format!("failed to start linker {}: {e}", program.display())))?;
        if result.status.success() {
            flush_artifact(&plan.output)?;
            Ok(())
        } else {
            Err(LinkError(format!(
                "native linker failed: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            )))
        }
    }
}

/// Ensures a just-written native artifact is fully materialized on the target
/// filesystem before a caller attempts to execute it. On Windows the linker
/// writes the executable through memory-mapped sections, so an immediate
/// `CreateProcess` can observe zero pages until the file is flushed.
fn flush_artifact(output: &Path) -> Result<(), LinkError> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(output)
        .map_err(|e| {
            LinkError(format!(
                "failed to open linked artifact {}: {e}",
                output.display()
            ))
        })?;
    file.sync_all().map_err(|e| {
        LinkError(format!(
            "failed to flush linked artifact {}: {e}",
            output.display()
        ))
    })
}
fn command(plan: &LinkPlan) -> (PathBuf, Vec<String>) {
    let shim = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/link_shim.rs");
    let crate_name = format!(
        "vut_link_{}_{}",
        std::process::id(),
        LINK_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let mut arguments = vec![
        shim.display().to_string(),
        "--crate-name".into(),
        crate_name,
        "-o".into(),
        plan.output.display().to_string(),
    ];
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
            // randomness) but whose `#[link]` metadata is lost when the
            // archive is passed directly to the linker.
            "-C".into(),
            "link-arg=bcrypt.lib".into(),
            "-C".into(),
            "link-arg=advapi32.lib".into(),
        ]);
    }
    (PathBuf::from("rustc"), arguments)
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
            output: PathBuf::from("app"),
            entry: "vut_main".into(),
            target: if cfg!(windows) {
                "x86_64-pc-windows-msvc".into()
            } else {
                "x86_64-unknown-linux-gnu".into()
            },
        }
    }
    #[test]
    fn command_keeps_entry_and_artifacts_explicit() {
        let plan = plan();
        let (program, arguments) = command(&plan);
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
            arguments.iter().any(|value| value.contains("link_shim.rs")),
            "the portable startup shim supplies the entry point"
        );
        assert_eq!(program, PathBuf::from("rustc"));
    }
}
