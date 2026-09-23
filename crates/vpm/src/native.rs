//! Schema and validation for a registry package's `native/build.toml`.
//!
//! Registry packages publish native *source*, never prebuilt binaries. This
//! module defines the structured build description that trusted CI consumes to
//! produce per-target artifacts. `custom` is an escape hatch; the first-class
//! backends are `cmake`, `cargo`, `make`, and `cc`.
//!
//! The Vut↔native boundary is always the stable C ABI; this file only
//! describes how the native library itself is built.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Component, path::Path};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Cmake,
    Cargo,
    Make,
    Cc,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cmake {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cargo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Make {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub makefile: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cc {
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Custom {
    /// Escape hatch: a single shell command run from the package root.
    pub command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmake: Option<Cmake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<Cargo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub make: Option<Make>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc: Option<Cc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<Custom>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBuild {
    pub backend: Backend,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Output>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmake: Option<Cmake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<Cargo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub make: Option<Make>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc: Option<Cc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<Custom>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub target: BTreeMap<String, TargetOverride>,
}

impl NativeBuild {
    pub fn parse(text: &str) -> Result<Self, String> {
        let build: Self =
            toml::from_str(text).map_err(|error| format!("invalid native build: {error}"))?;
        build.validate()?;
        Ok(build)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_backend(self.backend, None)?;
        for (target, over) in &self.target {
            if target.trim().is_empty() {
                return Err("native build target name is empty".into());
            }
            if let Some(artifact) = &over.artifact {
                validate_relative("native artifact", artifact)?;
            }
            if let Some(backend) = over.backend {
                self.validate_backend(backend, Some(over))?;
            } else {
                self.validate_options(self.backend, Some(over))?;
            }
        }
        Ok(())
    }

    fn validate_backend(
        &self,
        backend: Backend,
        over: Option<&TargetOverride>,
    ) -> Result<(), String> {
        if !self.options_present(backend, over) {
            let name = backend_name(backend);
            return Err(format!(
                "native build backend `{name}` requires a matching `[{name}]` options table"
            ));
        }
        self.validate_options(backend, over)
    }

    fn options_present(&self, backend: Backend, over: Option<&TargetOverride>) -> bool {
        let in_override = over.is_some_and(|over| match backend {
            Backend::Cmake => over.cmake.is_some(),
            Backend::Cargo => over.cargo.is_some(),
            Backend::Make => over.make.is_some(),
            Backend::Cc => over.cc.is_some(),
            Backend::Custom => over.custom.is_some(),
        });
        in_override
            || match backend {
                Backend::Cmake => self.cmake.is_some(),
                Backend::Cargo => self.cargo.is_some(),
                Backend::Make => self.make.is_some(),
                Backend::Cc => self.cc.is_some(),
                Backend::Custom => self.custom.is_some(),
            }
    }

    fn validate_options(
        &self,
        backend: Backend,
        over: Option<&TargetOverride>,
    ) -> Result<(), String> {
        match backend {
            Backend::Cmake => {
                if let Some(cmake) = over
                    .and_then(|over| over.cmake.as_ref())
                    .or(self.cmake.as_ref())
                {
                    validate_relative("cmake source", &cmake.source)?;
                    if let Some(build_dir) = &cmake.build_dir {
                        validate_relative("cmake build dir", build_dir)?;
                    }
                }
            }
            Backend::Cargo => {
                if let Some(cargo) = over
                    .and_then(|over| over.cargo.as_ref())
                    .or(self.cargo.as_ref())
                    && let Some(manifest) = &cargo.manifest
                {
                    validate_relative("cargo manifest", manifest)?;
                }
            }
            Backend::Make => {
                if let Some(make) = over
                    .and_then(|over| over.make.as_ref())
                    .or(self.make.as_ref())
                {
                    validate_relative("make source", &make.source)?;
                    if let Some(makefile) = &make.makefile {
                        validate_relative("makefile", makefile)?;
                    }
                }
            }
            Backend::Cc => {
                if let Some(cc) = over.and_then(|over| over.cc.as_ref()).or(self.cc.as_ref()) {
                    if cc.sources.is_empty() {
                        return Err("native `cc` backend requires at least one source".into());
                    }
                    for source in cc.sources.iter().chain(&cc.include) {
                        validate_relative("cc path", source)?;
                    }
                }
            }
            Backend::Custom => {
                if let Some(custom) = over
                    .and_then(|over| over.custom.as_ref())
                    .or(self.custom.as_ref())
                    && custom.command.trim().is_empty()
                {
                    return Err("native `custom` backend requires a command".into());
                }
            }
        }
        Ok(())
    }
}

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Cmake => "cmake",
        Backend::Cargo => "cargo",
        Backend::Make => "make",
        Backend::Cc => "cc",
        Backend::Custom => "custom",
    }
}

fn validate_relative(what: &str, value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "invalid {what} `{value}`; expected a relative path"
        ));
    }
    Ok(())
}

/// Compiled-binary extensions that a source package must never ship.
pub(crate) fn is_prebuilt_artifact(path: &str) -> bool {
    let normal = path.replace('\\', "/");
    if !normal.starts_with("native/") {
        return false;
    }
    Path::new(&normal).extension().is_some_and(|extension| {
        ["lib", "a", "so", "dylib", "dll", "obj", "o", "pdb", "exp"]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_backend_and_target_overrides() {
        let cmake = NativeBuild::parse(
            "backend = 'cmake'\n[output]\nname = 'math'\n[cmake]\nsource = 'native/src'\nbuild_dir = 'native/.build'\n[target.x86_64-pc-windows-msvc]\nartifact = 'native/.build/math.lib'\nsystem_libraries = ['user32']\n",
        )
        .unwrap();
        assert_eq!(cmake.backend, Backend::Cmake);
        assert_eq!(cmake.target.len(), 1);

        let cargo =
            NativeBuild::parse("backend = 'cargo'\n[cargo]\nmanifest = 'native/Cargo.toml'\n")
                .unwrap();
        assert_eq!(cargo.backend, Backend::Cargo);

        let custom =
            NativeBuild::parse("backend = 'custom'\n[custom]\ncommand = 'ninja -C native'\n")
                .unwrap();
        assert_eq!(custom.backend, Backend::Custom);
    }

    #[test]
    fn rejects_missing_options_and_unsafe_paths() {
        assert!(NativeBuild::parse("backend = 'cmake'\n").is_err());
        assert!(NativeBuild::parse("backend = 'cmake'\n[cmake]\nsource = '../escape'\n").is_err());
        assert!(NativeBuild::parse("backend = 'cc'\n[cc]\nsources = []\n").is_err());
        assert!(
            NativeBuild::parse(
                "backend = 'cmake'\n[cmake]\nsource = 'native/src'\n[target.a]\nartifact = '/abs.lib'\n"
            )
            .is_err()
        );
    }

    #[test]
    fn detects_prebuilt_artifacts() {
        assert!(is_prebuilt_artifact("native/build/math.lib"));
        assert!(is_prebuilt_artifact("native\\math.a"));
        assert!(is_prebuilt_artifact("native/libmath.so"));
        assert!(!is_prebuilt_artifact("native/src/math.c"));
        assert!(!is_prebuilt_artifact("src/mod.vut"));
    }
}
