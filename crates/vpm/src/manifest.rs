use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    /// Local native static libraries linked into the project build.
    #[serde(default)]
    pub native: Native,
}
/// Local native artifacts declared under `[native]`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    /// Static library paths (`.lib`/`.a`) relative to the project root.
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
