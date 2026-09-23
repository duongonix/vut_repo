//! Registry of globally installed CLI packages (`~/.vut/bin/installs.toml`).
//!
//! The registry is the source of truth for which package owns each globally
//! installed command name, enabling collision detection and clean uninstall.

use serde::{Deserialize, Serialize};
use std::path::Path;

const FILE: &str = "installs.toml";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package: Vec<Installed>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Installed {
    pub name: String,
    pub version: String,
    pub source: String,
    pub bins: Vec<String>,
}

impl Registry {
    pub fn load(bin_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = bin_dir.join(FILE);
        if !path.is_file() {
            return Ok(Self::default());
        }
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn save(&self, bin_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(bin_dir)?;
        let path = bin_dir.join(FILE);
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, toml::to_string_pretty(self)?)?;
        std::fs::rename(temp, path)?;
        Ok(())
    }

    /// The package that owns a globally installed command name, if any.
    #[must_use]
    pub fn owner_of(&self, bin: &str) -> Option<&str> {
        self.package
            .iter()
            .find(|entry| entry.bins.iter().any(|name| name == bin))
            .map(|entry| entry.name.as_str())
    }

    pub fn upsert(&mut self, entry: Installed) {
        if let Some(existing) = self
            .package
            .iter_mut()
            .find(|package| package.name == entry.name)
        {
            *existing = entry;
        } else {
            self.package.push(entry);
        }
        self.package
            .sort_by(|left, right| left.name.cmp(&right.name));
    }

    pub fn remove(&mut self, name: &str) -> Option<Installed> {
        let index = self.package.iter().position(|entry| entry.name == name)?;
        Some(self.package.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_owners_and_detects_collisions() {
        let root = std::env::temp_dir().join(format!("vpm-registry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut registry = Registry::load(&root).unwrap();
        assert!(registry.owner_of("foo").is_none());
        registry.upsert(Installed {
            name: "foo".into(),
            version: "1.0.0".into(),
            source: "registry:foo".into(),
            bins: vec!["foo".into(), "foo-tool".into()],
        });
        registry.save(&root).unwrap();
        let reloaded = Registry::load(&root).unwrap();
        assert_eq!(reloaded.owner_of("foo-tool"), Some("foo"));
        assert_eq!(reloaded.package[0].bins.len(), 2);

        let removed = {
            let mut registry = reloaded;
            registry.remove("foo")
        };
        assert_eq!(removed.unwrap().bins.len(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
}
