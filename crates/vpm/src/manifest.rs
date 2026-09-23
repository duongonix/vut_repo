use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    /// Packaged command-line entry points.
    #[serde(default)]
    pub bin: Vec<Bin>,
    /// Native build metadata or local native artifacts.
    #[serde(default)]
    pub native: Native,
}
/// A packaged command-line entry point (`[[bin]]`).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bin {
    /// Installed command name; a Vut identifier.
    pub name: String,
    /// Vut source file relative to the package root, always below `src/`.
    pub path: String,
}
/// Native artifacts declared under `[native]`.
///
/// Registry packages publish native *source* through `native/build.toml`.
/// Local projects may instead link prebuilt `libraries` directly; the two
/// forms are mutually exclusive.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    /// Path (relative to the package root) to `native/build.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// Local static library paths (`.lib`/`.a`) relative to the project root.
    #[serde(default)]
    pub libraries: Vec<String>,
    /// Platform system libraries referenced by native dependencies.
    #[serde(default)]
    pub system_libraries: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Dependency {
    Registry(String),
    Hosted {
        source: String,
        version: String,
    },
    Detailed {
        package: Option<String>,
        source: Option<String>,
        version: String,
    },
}

impl Dependency {
    #[must_use]
    pub fn version(&self) -> &str {
        match self {
            Self::Registry(version)
            | Self::Hosted { version, .. }
            | Self::Detailed { version, .. } => version,
        }
    }

    #[must_use]
    pub fn source_name<'a>(&'a self, import_name: &'a str) -> &'a str {
        match self {
            Self::Detailed {
                package: Some(package),
                ..
            } => package,
            _ => import_name,
        }
    }
}

impl Manifest {
    pub fn read(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = root.join("vpm.toml");
        let value: Self = toml::from_str(&std::fs::read_to_string(&path)?)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        validate_name(&self.package.name)?;
        Version::parse(&self.package.version)
            .map_err(|e| format!("invalid package version: {e}"))?;
        self.validate_bins()?;
        self.validate_native()?;
        for (name, dependency) in &self.dependencies {
            validate_name(name)?;
            Version::parse(dependency.version())
                .map_err(|e| format!("invalid version for dependency `{name}`: {e}"))?;
            match dependency {
                Dependency::Hosted { source, .. } => validate_source(source)?,
                Dependency::Detailed {
                    package, source, ..
                } => {
                    if let Some(package) = package {
                        validate_name(package)?;
                    }
                    if let Some(source) = source {
                        validate_source(source)?;
                    }
                }
                Dependency::Registry(_) => {}
            }
        }
        Ok(())
    }

    fn validate_bins(&self) -> Result<(), String> {
        let mut names = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for bin in &self.bin {
            validate_name(&bin.name)?;
            validate_bin_path(&bin.path)?;
            if !names.insert(&bin.name) {
                return Err(format!("duplicate bin name `{}`", bin.name));
            }
            if !paths.insert(&bin.path) {
                return Err(format!("duplicate bin path `{}`", bin.path));
            }
        }
        Ok(())
    }

    fn validate_native(&self) -> Result<(), String> {
        if let Some(build) = &self.native.build {
            validate_relative_path("native build", build)?;
            if !Path::new(build)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
            {
                return Err(format!(
                    "invalid native build `{build}`; expected a `.toml` path"
                ));
            }
            if !self.native.libraries.is_empty() {
                return Err(
                    "`[native] build` and `[native] libraries` are mutually exclusive".to_owned(),
                );
            }
        }
        for library in &self.native.libraries {
            validate_relative_path("native library", library)?;
        }
        for library in &self.native.system_libraries {
            if library.trim().is_empty() {
                return Err("native system library name is empty".to_owned());
            }
        }
        Ok(())
    }
}

/// Validates a `[[bin]].path`: a relative Vut source file below `src/`.
fn validate_bin_path(path: &str) -> Result<(), String> {
    validate_relative_path("bin path", path)?;
    let normal = path.replace('\\', "/");
    let is_vut = Path::new(&normal)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("vut"));
    if !normal.starts_with("src/") || !is_vut {
        return Err(format!(
            "invalid bin path `{path}`; expected a `.vut` file below `src/`"
        ));
    }
    Ok(())
}

fn validate_relative_path(what: &str, value: &str) -> Result<(), String> {
    let path = std::path::Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "invalid {what} `{value}`; expected a relative path"
        ));
    }
    Ok(())
}

pub fn validate_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    if !matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "invalid package name `{name}`; expected a Vut identifier"
        ));
    }
    Ok(())
}
fn validate_source(source: &str) -> Result<(), String> {
    let source = source.strip_prefix("gitlab:").unwrap_or(source);
    if source.split('/').count() < 3 {
        return Err(format!(
            "invalid hosted source `{source}`; expected owner/repository/package"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_semver_names_and_hosted_paths() {
        let valid: Manifest = toml::from_str(
            "[package]\nname='demo'\nversion='1.0.0'\n[dependencies.math]\nsource='owner/repo/libs/math'\nversion='2.1.0'\n",
        )
        .unwrap();
        assert!(valid.validate().is_ok());
        let mut invalid = valid;
        invalid.package.version = "v1".into();
        assert!(invalid.validate().is_err());
        assert!(
            toml::from_str::<Manifest>("[package]\nname='demo'\nversion='1.0.0'\ncritical=true\n")
                .is_err()
        );
        assert!(toml::from_str::<Manifest>("[package]\nname='demo'\nversion='1.0.0'\n[dependencies.math]\nsource='a/b/math'\nversion='1.0.0'\ninstaller='custom'\n").is_err());
    }

    #[test]
    fn parses_local_native_artifacts() {
        let manifest: Manifest = toml::from_str(
            "[package]\nname='demo'\nversion='1.0.0'\n[native]\nlibraries=['native/a.lib']\nsystem_libraries=['user32']\n",
        )
        .unwrap();
        assert_eq!(manifest.native.libraries, vec!["native/a.lib"]);
        assert_eq!(manifest.native.system_libraries, vec!["user32"]);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn parses_and_validates_bin_entries() {
        let manifest: Manifest = toml::from_str(
            "[package]\nname='demo'\nversion='1.0.0'\n\n[[bin]]\nname='demo'\npath='src/bin/demo.vut'\n",
        )
        .unwrap();
        assert_eq!(manifest.bin.len(), 1);
        assert_eq!(manifest.bin[0].name, "demo");
        assert_eq!(manifest.bin[0].path, "src/bin/demo.vut");
        assert!(manifest.validate().is_ok());

        for path in ["demo.vut", "src/demo.txt", "src/../../escape.vut"] {
            let bad: Manifest = toml::from_str(&format!(
                "[package]\nname='demo'\nversion='1.0.0'\n\n[[bin]]\nname='demo'\npath='{path}'\n"
            ))
            .unwrap();
            assert!(bad.validate().is_err(), "path `{path}` should be rejected");
        }
    }

    #[test]
    fn native_build_and_libraries_are_mutually_exclusive() {
        let manifest: Manifest = toml::from_str(
            "[package]\nname='demo'\nversion='1.0.0'\n[native]\nbuild='native/build.toml'\n",
        )
        .unwrap();
        assert_eq!(manifest.native.build.as_deref(), Some("native/build.toml"));
        assert!(manifest.validate().is_ok());
        let conflict: Manifest = toml::from_str(
            "[package]\nname='demo'\nversion='1.0.0'\n[native]\nbuild='native/build.toml'\nlibraries=['native/a.lib']\n",
        )
        .unwrap();
        assert!(conflict.validate().is_err());
    }

    #[test]
    fn dependency_key_is_import_namespace_and_package_may_be_aliased() {
        let manifest: Manifest = toml::from_str(
            "[package]\nname='demo'\nversion='1.0.0'\n[dependencies.webhttp]\npackage='http'\nversion='1.2.0'\n",
        )
        .unwrap();
        let dependency = manifest.dependencies.get("webhttp").unwrap();
        assert_eq!(dependency.source_name("webhttp"), "http");
        assert_eq!(dependency.version(), "1.2.0");
        assert!(manifest.validate().is_ok());
    }
}
