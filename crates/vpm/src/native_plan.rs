//! Deterministic assembly of resolved native dependencies for the linker.
//!
//! Native artifacts and system libraries are collected transitively across the
//! dependency graph, de-duplicated, and ordered deterministically before they
//! reach `CompilerConfig`/`LinkPlan`. Users never pass `--native-lib`.

use crate::artifact::NativeLibrary;
use std::path::PathBuf;

/// Native link inputs derived from the resolved dependency graph.
#[derive(Clone, Debug, Default)]
pub struct NativePlan {
    /// Local absolute paths to verified static libraries.
    pub libraries: Vec<PathBuf>,
    /// Local absolute directories added to the linker search path.
    pub search_paths: Vec<PathBuf>,
    /// Platform system library names.
    pub system_libraries: Vec<String>,
}

impl NativePlan {
    /// Builds a de-duplicated, deterministically ordered plan.
    #[must_use]
    pub fn from_resolved(resolved: Vec<NativeLibrary>) -> Self {
        let mut libraries = Vec::new();
        let mut search_paths = Vec::new();
        let mut system_libraries = Vec::new();
        for library in resolved {
            if !libraries.contains(&library.path) {
                libraries.push(library.path.clone());
            }
            if let Some(parent) = library.path.parent() {
                let parent = parent.to_path_buf();
                if !search_paths.contains(&parent) {
                    search_paths.push(parent);
                }
            }
            for system in library.system_libraries {
                if !system_libraries.contains(&system) {
                    system_libraries.push(system);
                }
            }
        }
        libraries.sort();
        search_paths.sort();
        system_libraries.sort();
        Self {
            libraries,
            search_paths,
            system_libraries,
        }
    }

    /// Appends the plan into a compiler configuration without duplicating
    /// entries already present from local `[native] libraries`.
    pub fn merge_into(&self, config: &mut vut_compiler::CompilerConfig) {
        for library in &self.libraries {
            if !config.native_libraries.contains(library) {
                config.native_libraries.push(library.clone());
            }
        }
        for path in &self.search_paths {
            if !config.library_search_paths.contains(path) {
                config.library_search_paths.push(path.clone());
            }
        }
        for library in &self.system_libraries {
            if !config.system_libraries.contains(library) {
                config.system_libraries.push(library.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_duplicates_a_diamond_deterministically() {
        let plan = NativePlan::from_resolved(vec![
            NativeLibrary {
                path: PathBuf::from("C:/cache/b.lib"),
                system_libraries: vec!["user32".into(), "ws2_32".into()],
            },
            NativeLibrary {
                path: PathBuf::from("C:/cache/a.lib"),
                system_libraries: vec!["ws2_32".into()],
            },
            NativeLibrary {
                path: PathBuf::from("C:/cache/b.lib"),
                system_libraries: vec!["user32".into()],
            },
        ]);
        assert_eq!(
            plan.libraries,
            vec![
                PathBuf::from("C:/cache/a.lib"),
                PathBuf::from("C:/cache/b.lib")
            ]
        );
        assert_eq!(plan.search_paths, vec![PathBuf::from("C:/cache")]);
        assert_eq!(plan.system_libraries, vec!["user32", "ws2_32"]);
    }
}
