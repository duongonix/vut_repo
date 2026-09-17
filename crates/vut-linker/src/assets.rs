//! Runtime archive resolution for standalone backends.
//!
//! The compiler exposes the core runtime as an `.rlib` on some platforms
//! because the rustc backend consumes Rust rlibs. Standalone backends
//! (`link.exe`, `cc`, `lld`) need a C-linkable static archive instead, so an
//! `.rlib` input is mapped to its staticlib sibling produced by the same crate
//! (`vut_runtime.lib` / `libvut_runtime.a` and likewise for the stdlib).
use std::path::{Path, PathBuf};

use crate::error::{LinkError, LinkFailure};

/// Returns a linkable static archive for `path`.
///
/// * `.a` / `.lib` are returned unchanged.
/// * `.rlib` is mapped to its staticlib sibling when one exists.
/// * Any other extension is returned unchanged and left to the linker.
///
/// # Errors
/// Returns [`LinkFailure::MissingLibrary`] when an `.rlib` has no staticlib
/// sibling, because the standalone backend cannot link Rust rlibs directly.
pub fn resolve_static_archive(path: &Path) -> Result<PathBuf, LinkError> {
    if is_static_archive(path) {
        return Ok(path.to_path_buf());
    }
    if is_rlib(path) {
        if let Some(sibling) = static_sibling(path) {
            return Ok(sibling);
        }
        return Err(LinkError::classified(
            format!(
                "no static runtime archive for `{}`; build or install the Vut runtime \
                 archives (vut-core / vut-stdlib)",
                path.display()
            ),
            LinkFailure::MissingLibrary,
        ));
    }
    Ok(path.to_path_buf())
}

fn is_static_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("a") || extension.eq_ignore_ascii_case("lib")
        })
}

fn is_rlib(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rlib"))
}

/// Maps `lib<name>.rlib` to `<name>.lib` (MSVC) or `lib<name>.a` (Unix).
fn static_sibling(rlib: &Path) -> Option<PathBuf> {
    let directory = rlib.parent()?;
    let stem = rlib.file_stem()?.to_str()?;
    let base = stem.strip_prefix("lib").unwrap_or(stem);
    [format!("{base}.lib"), format!("lib{base}.a")]
        .into_iter()
        .map(|name| directory.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vut-assets-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch");
        root
    }

    #[test]
    fn static_archives_pass_through() {
        let root = scratch("static");
        let archive = root.join("libvut-core.a");
        std::fs::write(&archive, b"x").unwrap();
        assert_eq!(resolve_static_archive(&archive).unwrap(), archive);
    }

    #[test]
    fn rlib_maps_to_msvc_staticlib_sibling() {
        let root = scratch("msvc");
        let rlib = root.join("libvut_runtime.rlib");
        std::fs::write(&rlib, b"x").unwrap();
        std::fs::write(root.join("vut_runtime.lib"), b"y").unwrap();
        assert_eq!(
            resolve_static_archive(&rlib).unwrap(),
            root.join("vut_runtime.lib")
        );
    }

    #[test]
    fn rlib_maps_to_unix_staticlib_sibling() {
        let root = scratch("unix");
        let rlib = root.join("libvut_stdlib_native.rlib");
        std::fs::write(&rlib, b"x").unwrap();
        std::fs::write(root.join("libvut_stdlib_native.a"), b"y").unwrap();
        assert_eq!(
            resolve_static_archive(&rlib).unwrap(),
            root.join("libvut_stdlib_native.a")
        );
    }

    #[test]
    fn rlib_without_static_sibling_is_an_actionable_error() {
        let root = scratch("missing");
        let rlib = root.join("libvut_runtime.rlib");
        std::fs::write(&rlib, b"x").unwrap();
        let error = resolve_static_archive(&rlib).expect_err("must fail");
        assert_eq!(error.failure(), LinkFailure::MissingLibrary);
        assert!(error.message().contains("vut-core"));
    }
}
