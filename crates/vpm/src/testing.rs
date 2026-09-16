use std::{
    path::{Path, PathBuf},
    process::Command,
};
use vut_compiler::{BuildMode, CompilerConfig, CompilerSession};

#[derive(Debug)]
pub struct TestOutcome {
    pub name: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct TestReport {
    pub outcomes: Vec<TestOutcome>,
    pub passed: usize,
    pub failed: usize,
}

pub fn test_project(
    root: &Path,
    filter: Option<&str>,
    release: bool,
) -> Result<TestReport, Box<dyn std::error::Error>> {
    super::project::install(root)?;
    let files = discover_tests(root, filter)?;
    let scratch = root.join("build/tests");
    let mut outcomes = Vec::with_capacity(files.len());
    for (index, file) in files.iter().enumerate() {
        outcomes.push(run_one(root, &scratch, file, index, release));
    }
    outcomes.sort_by(|left, right| left.name.cmp(&right.name));
    let passed = outcomes.iter().filter(|outcome| outcome.success).count();
    let failed = outcomes.len() - passed;
    Ok(TestReport {
        outcomes,
        passed,
        failed,
    })
}

fn discover_tests(root: &Path, filter: Option<&str>) -> Result<Vec<PathBuf>, std::io::Error> {
    Ok(vut_tooling::discover(root)?
        .into_iter()
        .filter(|path| path.starts_with(root.join("tests")))
        .filter(|path| filter.is_none_or(|value| path.to_string_lossy().contains(value)))
        .collect())
}

fn run_one(root: &Path, scratch: &Path, file: &Path, index: usize, release: bool) -> TestOutcome {
    let name = file
        .strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/");
    match prepare_and_run(root, scratch, file, index, release) {
        Ok(output) => TestOutcome {
            name,
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            error: None,
        },
        Err(error) => TestOutcome {
            name,
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(error.to_string()),
        },
    }
}

fn prepare_and_run(
    root: &Path,
    scratch: &Path,
    file: &Path,
    index: usize,
    release: bool,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let case = scratch.join(index.to_string());
    if case.exists() {
        std::fs::remove_dir_all(&case)?;
    }
    std::fs::create_dir_all(case.join("src"))?;
    std::fs::copy(file, case.join("src/main.vut"))?;
    std::fs::write(
        case.join("vpm.toml"),
        format!("[package]\nname = \"test_{index}\"\nversion = \"0.1.0\"\n\n[dependencies]\n"),
    )?;
    std::fs::write(case.join("vpm.lock"), "lock-version = 1\npackage = []\n")?;
    let mode = if release {
        BuildMode::Release
    } else {
        BuildMode::Debug
    };
    let mut config = CompilerConfig::for_target(CompilerConfig::default().target, mode);
    config.runtime_library = super::project::runtime_library(root);
    config.output_dir = Some(case.join(".vpm/build-cache"));
    let executable = case.join(if cfg!(windows) { "test.exe" } else { "test" });
    CompilerSession::new(config).emit_executable(&case.join("src"), &[], &executable)?;
    Ok(Command::new(executable).output()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runner_filters_captures_and_reports_deterministically() {
        let root = std::env::temp_dir().join(format!("vpm_testing_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        super::super::project::init_named(&root, "runner").unwrap();
        std::fs::write(root.join("tests/pass.vut"), "fn main() -> int:\n  0\n").unwrap();
        std::fs::write(
            root.join("tests/fail.vut"),
            "fn main() -> int:\n  out(\"details\")\n  3\n",
        )
        .unwrap();
        let passing = test_project(&root, Some("pass"), false).unwrap();
        assert_eq!((passing.passed, passing.failed), (1, 0));
        assert_eq!(passing.outcomes[0].name, "tests/pass.vut");
        let failing = test_project(&root, Some("fail"), false).unwrap();
        assert_eq!((failing.passed, failing.failed), (0, 1));
        assert_eq!(failing.outcomes[0].name, "tests/fail.vut");
        assert_eq!(failing.outcomes[0].stdout.trim(), "details");
        std::fs::remove_dir_all(root).unwrap();
    }
}
