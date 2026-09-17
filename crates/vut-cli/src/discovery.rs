//! Local toolchain and runtime artifact discovery.
//!
//! This is the pre-`VUT_HOME` discovery used by the CLI: environment overrides
//! first, then the development `target/debug` layout next to the running binary.
use std::path::{Path, PathBuf};

/// Locates the core runtime archive used as a link input.
///
/// Order: `VUT_RUNTIME_LIBRARY`, then `target/debug/<name>`, then the directory
/// of the running `vut` executable.
#[must_use]
pub fn locate_runtime() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VUT_RUNTIME_LIBRARY").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let name = if cfg!(windows) {
        "libvut_runtime.rlib"
    } else {
        "libvut_runtime.a"
    };
    let candidates = [
        PathBuf::from("target/debug").join(name),
        std::env::current_exe().ok()?.parent()?.join(name),
    ];
    candidates.into_iter().find(|path| path.is_file())
}

/// Locates the native stdlib runtime archive next to the core runtime.
#[must_use]
pub fn locate_stdlib_runtime(runtime: &Path) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VUT_STDLIB_RUNTIME").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let directory = runtime.parent()?;
    [
        "vut_stdlib_native.lib",
        "libvut_stdlib_native.a",
        "libvut_stdlib_native.rlib",
    ]
    .into_iter()
    .map(|name| directory.join(name))
    .find(|candidate| candidate.is_file())
}

/// Locates the prebuilt startup object next to the core runtime.
#[must_use]
pub fn locate_startup(runtime: &Path, target: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VUT_STARTUP_OBJECT").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let directory = runtime.parent()?;
    let name = vut_linker::TargetProfile::parse(target)
        .ok()?
        .startup_object_name();
    let candidate = directory.join(name);
    candidate.is_file().then_some(candidate)
}

/// Finds a program on `PATH`, honoring `PATHEXT` on Windows.
#[must_use]
pub fn find_on_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .map(str::to_ascii_lowercase)
            .collect()
    } else {
        vec![String::new()]
    };
    for directory in std::env::split_paths(&path) {
        for extension in &extensions {
            let candidate = if extension.is_empty() {
                directory.join(program)
            } else {
                directory.join(format!("{program}{extension}"))
            };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
