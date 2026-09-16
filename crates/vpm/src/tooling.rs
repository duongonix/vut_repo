use crate::{Dependency, Manifest, PackageProvider, PackageSource};
use semver::Version;
use serde::Deserialize;
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

pub fn format_project(root: &Path, check: bool) -> Result<usize, Box<dyn std::error::Error>> {
    Manifest::read(root)?;
    let mut changed = 0;
    for path in vut_tooling::discover(root)? {
        let source = std::fs::read_to_string(&path)?;
        let formatted = vut_tooling::format_source(&source)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if source != formatted {
            changed += 1;
            if !check {
                super::project::atomic_write(&path, &formatted)?;
            }
        }
    }
    Ok(changed)
}

pub fn lint_project(root: &Path) -> Result<Vec<LintWarning>, Box<dyn std::error::Error>> {
    super::project::check(root)?;
    let mut warnings = Vec::new();
    for path in vut_tooling::discover(root)? {
        let source = std::fs::read_to_string(&path)?;
        for lint in vut_tooling::lint_source(&source)? {
            warnings.push(LintWarning {
                path: path.to_string_lossy().into_owned(),
                line: lint.line,
                code: lint.code.to_owned(),
                message: lint.message.clone(),
            });
        }
    }
    Ok(warnings)
}

/// One lint finding, formatted by the CLI into a warning line.
pub struct LintWarning {
    pub path: String,
    pub line: usize,
    pub code: String,
    pub message: String,
}

pub fn generate_docs(root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = Manifest::read(root)?;
    let mut output = format!(
        "# {} {}\n\n",
        manifest.package.name, manifest.package.version
    );
    for path in vut_tooling::discover(root)?
        .into_iter()
        .filter(|path| path.starts_with(root.join("src")))
    {
        let source = std::fs::read_to_string(&path)?;
        let mut docs = Vec::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(doc) = trimmed.strip_prefix("###") {
                docs.push(doc.trim());
            } else if !docs.is_empty() && is_declaration(trimmed) {
                writeln!(
                    output,
                    "## `{}`\n\n{}\n",
                    trimmed.trim_end_matches(':'),
                    docs.join("\n")
                )?;
                docs.clear();
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                docs.clear();
            }
        }
    }
    let path = root.join("build/docs/index.md");
    std::fs::create_dir_all(path.parent().ok_or("invalid documentation path")?)?;
    std::fs::write(&path, output)?;
    Ok(path)
}
fn is_declaration(line: &str) -> bool {
    ["fn ", "data ", "interface ", "enum ", "type "]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

pub fn clean(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Manifest::read(root)?;
    for path in [
        root.join("build"),
        root.join(".vut-cache"),
        root.join(".vpm/build"),
    ] {
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    package: Vec<Locked>,
}
#[derive(Deserialize)]
struct Locked {
    name: String,
    version: String,
    source: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

pub fn dependency_tree(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest = Manifest::read(root)?;
    let lock: Lock = toml::from_str(&std::fs::read_to_string(root.join("vpm.lock"))?)?;
    let mut output = format!("{} {}\n", manifest.package.name, manifest.package.version);
    let mut packages = lock.package;
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    for (index, package) in packages.iter().enumerate() {
        let branch = if index + 1 == packages.len() {
            "└──"
        } else {
            "├──"
        };
        writeln!(
            output,
            "{branch} {} {} ({})",
            package.name, package.version, package.source
        )?;
        for dependency in &package.dependencies {
            writeln!(output, "    └── {dependency}")?;
        }
    }
    Ok(output)
}

fn versions(source: &PackageSource) -> Result<Vec<Version>, Box<dyn std::error::Error>> {
    let provider = crate::resolver::Providers::new()?;
    let mut values = provider
        .list_versions(source)?
        .into_iter()
        .filter_map(|value| Version::parse(value.trim_start_matches('v')).ok())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    Ok(values)
}
pub fn package_info(spec: &str) -> Result<String, Box<dyn std::error::Error>> {
    let source = PackageSource::parse(spec)?;
    let available = versions(&source)?;
    let latest = available
        .last()
        .map_or_else(|| "none".into(), ToString::to_string);
    Ok(format!(
        "name: {}\nsource: {}\nlatest: {}\nversions: {}\n",
        source.name(),
        source,
        latest,
        available
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
pub fn search(query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let source = PackageSource::parse(query)?;
    let latest = versions(&source)?
        .last()
        .map_or_else(|| "none".into(), ToString::to_string);
    Ok(format!(
        "Package\tLatest\tSource\n{}\t{}\t{}\n",
        source.name(),
        latest,
        source
    ))
}
pub fn outdated(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest = Manifest::read(root)?;
    let mut output = String::from("Package\tCurrent\tLatest\n");
    for (name, dependency) in manifest.dependencies {
        let (source, current) = match dependency {
            Dependency::Registry(version) => (PackageSource::parse(&name)?, version),
            Dependency::Hosted { source, version } => (PackageSource::parse(&source)?, version),
            Dependency::Detailed {
                package,
                source,
                version,
            } => {
                let source = source.map_or_else(
                    || PackageSource::parse(package.as_deref().unwrap_or(&name)),
                    |source| PackageSource::parse(&source),
                )?;
                (source, version)
            }
        };
        let latest = versions(&source)?
            .last()
            .map_or_else(|| current.clone(), ToString::to_string);
        writeln!(output, "{name}\t{current}\t{latest}")?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vpm_tooling_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        super::super::project::init_named(&root, name).unwrap();
        root
    }
    #[test]
    fn fmt_lint_docs_tree_and_clean_are_deterministic_and_safe() {
        let root = project("workflow");
        std::fs::write(
            root.join("src/main.vut"),
            "### Entry\nfn main():\n    unused=1\n",
        )
        .unwrap();
        assert_eq!(format_project(&root, false).unwrap(), 1);
        assert_eq!(format_project(&root, true).unwrap(), 0);
        assert!(
            lint_project(&root)
                .unwrap()
                .iter()
                .any(|warning| warning.code == "W2001")
        );
        assert!(generate_docs(&root).unwrap().is_file());
        assert!(
            dependency_tree(&root)
                .unwrap()
                .starts_with("workflow 0.1.0")
        );
        clean(&root).unwrap();
        assert!(root.join("src/main.vut").is_file());
        assert!(root.join("vpm.toml").is_file());
        assert!(root.join("vpm.lock").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }
}
