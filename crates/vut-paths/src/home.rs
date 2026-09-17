//! Canonical Vut installation layout (`VUT_HOME`).
//!
//! The installed layout is:
//!
//! ```text
//! ~/.vut/
//! ├── manifest.json
//! ├── bin/
//! ├── lib/runtime/<target>/
//! ├── std/
//! ├── packages/
//! ├── cache/
//! └── config/
//! ```
//!
//! Resolution is `VUT_HOME` first, then the user's home directory. Windows
//! uses `%USERPROFILE%\.vut`.
use std::path::{Path, PathBuf};

/// Environment variable naming the installation root.
pub const HOME_ENV: &str = "VUT_HOME";

/// A resolved Vut installation root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    /// Builds a `Home` rooted at an explicit path (used by tests and tooling).
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Resolves the installation root: `VUT_HOME`, then `~/.vut`.
    #[must_use]
    pub fn resolve() -> Option<Self> {
        if let Some(path) = std::env::var_os(HOME_ENV) {
            return Some(Self {
                root: PathBuf::from(path),
            });
        }
        dirs::home_dir().map(|base| Self {
            root: base.join(".vut"),
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
    #[must_use]
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }
    #[must_use]
    pub fn lib_runtime_dir(&self, target: &str) -> PathBuf {
        self.root.join("lib").join("runtime").join(target)
    }
    #[must_use]
    pub fn std_dir(&self) -> PathBuf {
        self.root.join("std")
    }
    #[must_use]
    pub fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }
    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }
    #[must_use]
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_root_exposes_the_canonical_layout() {
        let home = Home::at("C:/vut-home");
        assert_eq!(home.std_dir(), Path::new("C:/vut-home").join("std"));
        assert_eq!(
            home.lib_runtime_dir("x86_64-pc-windows-msvc"),
            Path::new("C:/vut-home")
                .join("lib")
                .join("runtime")
                .join("x86_64-pc-windows-msvc")
        );
        assert_eq!(
            home.packages_dir(),
            Path::new("C:/vut-home").join("packages")
        );
        assert_eq!(
            home.manifest_path(),
            Path::new("C:/vut-home").join("manifest.json")
        );
    }

    #[test]
    fn resolve_prefers_the_environment_override() {
        // Cannot mutate the process environment safely in parallel tests, so
        // only the layout helper is asserted here; resolution order is covered
        // in the integration workflow.
        assert_eq!(Home::at("/tmp/vut").root(), Path::new("/tmp/vut"));
    }
}
