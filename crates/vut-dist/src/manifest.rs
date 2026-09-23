//! Distribution manifest (`manifest.json`).
use serde::{Deserialize, Serialize};

/// Manifest schema version.
pub const FORMAT_VERSION: u32 = 1;

/// Version and integrity metadata shipped inside every distribution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub vut: String,
    pub vpm: String,
    pub stdlib: String,
    pub runtime: String,
    /// Internal runtime ABI version (integer contract with generated code).
    pub abi_version: u32,
    pub target: String,
    /// Distribution archive file name this manifest belongs to.
    pub archive: String,
    /// Release channel (`stable`, ...).
    pub channel: String,
    /// Build profile (`release`).
    pub profile: String,
    /// Cranelift version the backend was built against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cranelift: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_os: Option<String>,
}

impl Manifest {
    /// Renders the manifest as pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("manifest always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            vut: "0.1.0".into(),
            vpm: "0.1.0".into(),
            stdlib: "0.1.0".into(),
            runtime: "0.1.0".into(),
            abi_version: 13,
            target: "x86_64-unknown-linux-gnu".into(),
            archive: "vut-v0.1.0-x86_64-unknown-linux-gnu.tar.gz".into(),
            channel: "stable".into(),
            profile: "release".into(),
            cranelift: Some("0.135.2".into()),
            build_commit: Some("abc123".into()),
            minimum_os: None,
        }
    }

    #[test]
    fn serializes_required_fields() {
        let json = sample().to_json();
        for key in [
            "\"format_version\"",
            "\"vut\"",
            "\"vpm\"",
            "\"stdlib\"",
            "\"runtime\"",
            "\"abi_version\"",
            "\"target\"",
            "\"archive\"",
            "\"channel\"",
            "\"profile\"",
            "\"cranelift\"",
            "\"build_commit\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        assert!(!json.contains("minimum_os"), "None fields are omitted");
    }
}
