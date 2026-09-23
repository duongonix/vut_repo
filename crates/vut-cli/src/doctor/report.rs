//! Structured `vut doctor` report (human and JSON rendering).
use serde::Serialize;

/// Overall readiness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Issues,
}

/// One resolved artifact (runtime archive, startup object, ...).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Asset {
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Asset {
    pub fn present(path: impl Into<String>) -> Self {
        Self {
            present: true,
            path: Some(path.into()),
            detail: None,
        }
    }
    pub fn missing(detail: impl Into<String>) -> Self {
        Self {
            present: false,
            path: None,
            detail: Some(detail.into()),
        }
    }
}

/// Runtime ABI / manifest-format compatibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Abi {
    pub expected: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<u32>,
    pub compatible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_format: Option<u32>,
}

/// The selected native linker.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Linker {
    pub program: String,
    pub backend: String,
    pub source: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub search_paths: Vec<String>,
}

/// Platform SDK / toolchain availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Sdk {
    pub windows_build_tools: bool,
    pub xcode_clt: bool,
    pub cc: bool,
}

/// Whether the Vut `bin` directory is on `PATH`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PathInfo {
    pub bin_dir: String,
    pub on_path: bool,
}

/// Result of the compile → link → run smoke test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Smoke {
    pub ran: bool,
    pub detail: String,
}

/// The full diagnostic report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub status: Status,
    pub target: String,
    pub compiler: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vut_lsp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdlib_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdlib_version: Option<String>,
    pub runtime: Asset,
    pub native_runtime: Asset,
    pub startup: Asset,
    pub abi: Abi,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linker: Option<Linker>,
    pub sdk: Sdk,
    pub path: PathInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smoke: Option<Smoke>,
    pub warnings: Vec<String>,
    pub issues: Vec<String>,
}

impl Report {
    /// Whether the toolchain is ready (no gating issues).
    #[must_use]
    pub fn ready(&self) -> bool {
        self.issues.is_empty()
    }

    /// Renders the report as pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("doctor report always serializes")
    }
}
