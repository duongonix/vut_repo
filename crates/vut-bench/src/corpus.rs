//! Benchmark corpus discovery.
//!
//! Layout:
//!
//! ```text
//! benchmarks/corpus/<name>/
//!   main.vut       (required)
//!   expected.txt   (required: exact expected stdout)
//!   stdin.txt      (optional)
//! ```

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Workload {
    pub name: String,
    pub source: PathBuf,
    pub expected: String,
    pub stdin: Option<String>,
}

impl Workload {
    /// Expected stdout with trailing whitespace removed for comparison.
    #[must_use]
    pub fn expected_trimmed(&self) -> &str {
        self.expected.trim_end()
    }
}

/// Discovers and validates every workload under `corpus`, sorted by name.
pub fn discover(corpus: &Path) -> Result<Vec<Workload>, String> {
    if !corpus.is_dir() {
        return Err(format!(
            "corpus directory `{}` does not exist",
            corpus.display()
        ));
    }
    let mut workloads = Vec::new();
    let entries = std::fs::read_dir(corpus).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let source = entry.path().join("main.vut");
        if !source.is_file() {
            continue;
        }
        // A directory with `main.vut` but no `expected.txt` is an incomplete
        // workload (for example a work in progress); skip it rather than
        // failing the whole corpus run.
        let Ok(expected) = std::fs::read_to_string(entry.path().join("expected.txt")) else {
            continue;
        };
        let stdin = std::fs::read_to_string(entry.path().join("stdin.txt")).ok();
        workloads.push(Workload {
            name,
            source,
            expected,
            stdin,
        });
    }
    workloads.sort_by(|left, right| left.name.cmp(&right.name));
    if workloads.is_empty() {
        return Err(format!("no workloads found under `{}`", corpus.display()));
    }
    Ok(workloads)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_sorted_workloads_with_expected_output() {
        let root = std::env::temp_dir().join(format!("vut-bench-corpus-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (name, body) in [("b", "out(\"b\")"), ("a", "out(\"a\")")] {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("main.vut"), body).unwrap();
            std::fs::write(dir.join("expected.txt"), body).unwrap();
        }
        // A directory without main.vut is ignored.
        std::fs::create_dir_all(root.join("ignored")).unwrap();

        let workloads = discover(&root).unwrap();
        assert_eq!(
            workloads
                .iter()
                .map(|workload| workload.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incomplete_workloads_are_skipped() {
        let root = std::env::temp_dir().join(format!("vut-bench-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let complete = root.join("ok");
        std::fs::create_dir_all(&complete).unwrap();
        std::fs::write(complete.join("main.vut"), "fn main():\n  0\n").unwrap();
        std::fs::write(complete.join("expected.txt"), "ok").unwrap();
        let incomplete = root.join("wip");
        std::fs::create_dir_all(&incomplete).unwrap();
        std::fs::write(incomplete.join("main.vut"), "fn main():\n  0\n").unwrap();

        let workloads = discover(&root).unwrap();
        assert_eq!(workloads.len(), 1);
        assert_eq!(workloads[0].name, "ok");
        std::fs::remove_dir_all(root).unwrap();
    }
}
