//! Artifact discovery: resolves stdlib source, runtime archives and the startup
//! object for a target.
//!
//! Order (production first, development fallback last):
//!
//! ```text
//! stdlib source   VUT_STDLIB_PATH     -> $VUT_HOME/std            -> bundled dev tree
//! runtime archive VUT_RUNTIME_LIBRARY -> $VUT_HOME/lib/runtime/<target> -> dev target dirs
//! stdlib runtime  VUT_STDLIB_RUNTIME  -> next to the core runtime / runtime dirs
//! startup object  VUT_STARTUP_OBJECT  -> runtime dirs
//!   where runtime dirs = VUT_RUNTIME_DIR, $VUT_HOME/lib/runtime/<target>, dev dirs
//! ```
//!
//! Artifact names prefer the distribution names (`vut-core`, `vut-stdlib`,
//! `vut-startup`) and fall back to the internal development names
//! (`libvut_runtime`, `vut_stdlib_native`).
use std::path::{Path, PathBuf};

use crate::home::Home;

/// Environment override for the standard library source root.
pub const STDLIB_PATH_ENV: &str = "VUT_STDLIB_PATH";
/// Environment override for the runtime directory (contains the archives).
pub const RUNTIME_DIR_ENV: &str = "VUT_RUNTIME_DIR";
/// Environment override for the core runtime archive.
pub const RUNTIME_LIBRARY_ENV: &str = "VUT_RUNTIME_LIBRARY";
/// Environment override for the native stdlib archive.
pub const STDLIB_RUNTIME_ENV: &str = "VUT_STDLIB_RUNTIME";
/// Environment override for the startup object.
pub const STARTUP_OBJECT_ENV: &str = "VUT_STARTUP_OBJECT";

/// Resolves the standard library `.vut` source root.
#[must_use]
pub fn stdlib_root() -> Option<PathBuf> {
    if let Some(path) = env_dir(STDLIB_PATH_ENV) {
        return Some(path);
    }
    if let Some(home) = Home::resolve() {
        let directory = home.std_dir();
        if directory.is_dir() {
            return Some(directory);
        }
    }
    dev_stdlib_root()
}

/// Resolves the core runtime archive.
///
/// Candidate directories are searched in order and the first existing archive
/// wins, so an empty `$VUT_HOME/lib/runtime/<target>` does not hide a
/// development build.
#[must_use]
pub fn runtime_library(target: &str) -> Option<PathBuf> {
    if let Some(path) = env_file(RUNTIME_LIBRARY_ENV) {
        return Some(path);
    }
    let names = core_candidates(target);
    search_dirs(target)
        .iter()
        .find_map(|dir| first_in(dir, &names))
}

/// Resolves the native stdlib archive, given the core runtime archive.
#[must_use]
pub fn stdlib_runtime_library(core: Option<&Path>, target: &str) -> Option<PathBuf> {
    if let Some(path) = env_file(STDLIB_RUNTIME_ENV) {
        return Some(path);
    }
    let names = stdlib_candidates(target);
    if let Some(directory) = core.and_then(Path::parent)
        && let Some(found) = first_in(directory, &names)
    {
        return Some(found);
    }
    search_dirs(target)
        .iter()
        .find_map(|dir| first_in(dir, &names))
}

/// Resolves the prebuilt startup object for a target.
#[must_use]
pub fn startup_object(target: &str) -> Option<PathBuf> {
    if let Some(path) = env_file(STARTUP_OBJECT_ENV) {
        return Some(path);
    }
    let name = if is_windows_target(target) {
        "vut-startup.obj"
    } else {
        "vut-startup.o"
    };
    search_dirs(target)
        .iter()
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

/// Returns true when the target triple is a Windows target.
#[must_use]
pub fn is_windows_target(target: &str) -> bool {
    target.contains("windows")
}

/// Reads the installed distribution's runtime ABI version from
/// `$VUT_HOME/manifest.json`, when present.
#[must_use]
pub fn installed_abi_version() -> Option<u32> {
    let home = Home::resolve()?;
    manifest_abi_version(&home.manifest_path())
}

/// Reads a string field from the installed `$VUT_HOME/manifest.json`.
#[must_use]
pub fn installed_manifest_string(key: &str) -> Option<String> {
    let home = Home::resolve()?;
    let text = std::fs::read_to_string(home.manifest_path()).ok()?;
    parse_string_field(&text, key)
}

/// Reads an unsigned integer field from the installed `$VUT_HOME/manifest.json`.
#[must_use]
pub fn installed_manifest_u32(key: &str) -> Option<u32> {
    let home = Home::resolve()?;
    let text = std::fs::read_to_string(home.manifest_path()).ok()?;
    parse_u32_field(&text, key)
}

/// Parses `abi_version` from a distribution `manifest.json`.
#[must_use]
pub fn manifest_abi_version(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_u32_field(&text, "abi_version")
}

fn parse_string_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let after = &text[text.find(&needle)? + needle.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_owned())
}

fn parse_u32_field(text: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{key}\"");
    let after = &text[text.find(&needle)? + needle.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Returns `true` when the installed ABI (if any) matches the expected revision.
/// A missing manifest (development tree) is always compatible.
#[must_use]
pub fn abi_compatible(installed: Option<u32>, expected: u32) -> bool {
    installed.is_none_or(|installed| installed == expected)
}

/// Candidate directories in resolution order.
fn search_dirs(target: &str) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(directory) = env_dir(RUNTIME_DIR_ENV) {
        directories.push(directory);
    }
    if let Some(home) = Home::resolve() {
        let directory = home.lib_runtime_dir(target);
        if directory.is_dir() {
            directories.push(directory);
        }
    }
    directories.extend(dev_runtime_dirs());
    directories
}

fn core_candidates(target: &str) -> Vec<&'static str> {
    if is_windows_target(target) {
        // rlib first keeps the development rustc backend working; distribution
        // installs ship only the static archive.
        vec![
            "libvut_runtime.rlib",
            "vut-core.lib",
            "vut_runtime.lib",
            "libvut_runtime.a",
        ]
    } else {
        vec!["libvut_runtime.a", "libvut-core.a", "libvut_runtime.rlib"]
    }
}

fn stdlib_candidates(target: &str) -> Vec<&'static str> {
    if is_windows_target(target) {
        vec![
            "vut_stdlib_native.lib",
            "vut-stdlib.lib",
            "libvut_stdlib_native.a",
            "libvut_stdlib_native.rlib",
        ]
    } else {
        vec![
            "libvut_stdlib_native.a",
            "libvut-stdlib.a",
            "libvut_stdlib_native.rlib",
        ]
    }
}

fn first_in(directory: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn env_dir(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(name)?);
    path.is_dir().then_some(path)
}

fn env_file(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(name)?);
    path.is_file().then_some(path)
}

fn dev_runtime_dirs() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(executable) = std::env::current_exe().ok()
        && let Some(directory) = executable.parent()
    {
        // Test binaries live in `target/debug/deps`; the archives are one level
        // up. Installed binaries keep their archives next to themselves.
        directories.push(directory.to_path_buf());
        if let Some(parent) = directory.parent() {
            directories.push(parent.to_path_buf());
        }
    }
    directories.push(PathBuf::from("target/debug"));
    directories
}

/// Bundled development tree fallback (`<repo>/vut-stdlib/std`).
///
/// Only resolves on a checkout where that path exists; a production install
/// resolves `$VUT_HOME/std` first and never reaches this.
fn dev_stdlib_root() -> Option<PathBuf> {
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vut-stdlib/std");
    bundled.is_dir().then_some(bundled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vut-paths-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch");
        root
    }

    #[test]
    fn core_candidates_prefer_development_rlib_on_windows() {
        assert_eq!(
            core_candidates("x86_64-pc-windows-msvc").first(),
            Some(&"libvut_runtime.rlib")
        );
        assert_eq!(
            core_candidates("x86_64-unknown-linux-gnu").first(),
            Some(&"libvut_runtime.a")
        );
    }

    #[test]
    fn first_in_returns_the_first_existing_candidate() {
        let directory = scratch("first-in");
        std::fs::write(directory.join("vut-core.lib"), b"x").unwrap();
        std::fs::write(directory.join("libvut_runtime.a"), b"x").unwrap();
        let found = first_in(
            &directory,
            &["missing.a", "vut-core.lib", "libvut_runtime.a"],
        );
        assert_eq!(found, Some(directory.join("vut-core.lib")));
        assert_eq!(first_in(&directory, &["missing.a"]), None);
    }

    #[test]
    fn windows_target_detection_uses_the_triple() {
        assert!(is_windows_target("aarch64-pc-windows-msvc"));
        assert!(!is_windows_target("aarch64-apple-darwin"));
    }

    #[test]
    fn parses_abi_version_from_a_manifest() {
        assert_eq!(
            parse_u32_field("{\"abi_version\": 13}", "abi_version"),
            Some(13)
        );
        assert_eq!(
            parse_u32_field(
                "{\n  \"abi_version\": 7,\n  \"target\": \"x\"\n}",
                "abi_version"
            ),
            Some(7)
        );
        assert_eq!(parse_u32_field("{}", "abi_version"), None);
    }

    #[test]
    fn parses_string_fields_from_a_manifest() {
        let text = "{\n  \"stdlib\": \"0.1.0\",\n  \"target\": \"x\"\n}";
        assert_eq!(parse_string_field(text, "stdlib"), Some("0.1.0".to_owned()));
        assert_eq!(parse_string_field(text, "missing"), None);
    }

    #[test]
    fn abi_compatibility_matches_or_ignores_absence() {
        assert!(abi_compatible(Some(13), 13));
        assert!(!abi_compatible(Some(12), 13));
        assert!(abi_compatible(None, 13));
    }
}
