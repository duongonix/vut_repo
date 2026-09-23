//! Typed description of a native link invocation.
use std::path::{Path, PathBuf};

/// Collects compiler objects, static libraries, system libraries, frameworks
/// and the startup object so callers never pass ad-hoc string lists.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LinkPlan {
    pub objects: Vec<PathBuf>,
    pub static_libraries: Vec<PathBuf>,
    pub system_libraries: Vec<String>,
    pub frameworks: Vec<String>,
    /// Extra library search directories (`/LIBPATH:` on MSVC, `-L` elsewhere).
    /// The resolver contributes platform/SDK paths at link time; callers may add
    /// project-specific directories here.
    pub search_paths: Vec<PathBuf>,
    /// Core runtime archive. Required by the rustc backend as an input; the
    /// system backends treat it as one more archive.
    pub runtime: Option<PathBuf>,
    /// Prebuilt or resolved startup object (`vut-startup.o`/`.obj`). The rustc
    /// backend supplies its own entry shim and ignores this; the system
    /// backends require it.
    pub startup: Option<PathBuf>,
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
    /// Returns search paths in declaration order with duplicates removed.
    #[must_use]
    pub fn unique_search_paths(&self) -> Vec<&Path> {
        let mut seen = std::collections::HashSet::new();
        self.search_paths
            .iter()
            .filter(|path| seen.insert(path.as_os_str().to_owned()))
            .map(PathBuf::as_path)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_libraries_in_order() {
        let plan = LinkPlan {
            static_libraries: vec![
                PathBuf::from("a.lib"),
                PathBuf::from("b.lib"),
                PathBuf::from("a.lib"),
            ],
            system_libraries: vec!["user32".into(), "user32".into(), "kernel32".into()],
            search_paths: vec![
                PathBuf::from("/sdk/lib"),
                PathBuf::from("/sdk/lib"),
                PathBuf::from("/ucrt/lib"),
            ],
            ..LinkPlan::default()
        };
        assert_eq!(
            plan.unique_static_libraries(),
            vec![Path::new("a.lib"), Path::new("b.lib")]
        );
        assert_eq!(plan.unique_system_libraries(), vec!["user32", "kernel32"]);
        assert_eq!(
            plan.unique_search_paths(),
            vec![Path::new("/sdk/lib"), Path::new("/ucrt/lib")]
        );
    }
}
