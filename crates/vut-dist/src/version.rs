//! Build/version metadata (`version.json`) shipped inside every distribution.
//!
//! Records the exact inputs that produced the artifact so a release is
//! traceable to its commit/tag (see `specs/deploy/release.md`).
use serde::Serialize;

/// Provenance metadata written next to the distribution manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VersionInfo {
    pub vut: String,
    pub vpm: String,
    pub stdlib: String,
    pub runtime: String,
    pub abi_version: u32,
    pub manifest_format: u32,
    pub channel: String,
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cranelift: Option<String>,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_commit: Option<String>,
}

impl VersionInfo {
    /// Renders the metadata as pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("version metadata always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_absent_provenance() {
        let info = VersionInfo {
            vut: "0.1.0".into(),
            vpm: "0.1.0".into(),
            stdlib: "0.1.0".into(),
            runtime: "0.1.0".into(),
            abi_version: 13,
            manifest_format: 1,
            channel: "stable".into(),
            profile: "release".into(),
            cranelift: None,
            target: "x86_64-apple-darwin".into(),
            build_commit: None,
        };
        let json = info.to_json();
        assert!(json.contains("\"target\""));
        assert!(!json.contains("cranelift"));
        assert!(!json.contains("build_commit"));
    }
}
