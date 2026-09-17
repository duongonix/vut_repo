//! Linker error type and failure classification.
//!
//! Failures are classified so higher layers (for example a future `vut doctor`
//! command) can explain a missing SDK or CRT instead of surfacing a raw linker
//! diagnostic.
use std::fmt;

/// High-level cause of a native link failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkFailure {
    /// The platform linker or a required tool is not installed.
    ToolNotFound,
    /// The platform SDK (for example the Windows SDK) is missing.
    MissingSdk,
    /// The C runtime (for example the MSVC/Windows UCRT) is missing.
    MissingCrt,
    /// A system or static library could not be found.
    MissingLibrary,
    /// Two inputs define the same symbol.
    DuplicateSymbol,
    /// The requested target cannot be linked by this backend.
    UnsupportedTarget,
    /// Any other linker failure.
    Other,
}

impl LinkFailure {
    /// Short machine-stable identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolNotFound => "tool-not-found",
            Self::MissingSdk => "missing-sdk",
            Self::MissingCrt => "missing-crt",
            Self::MissingLibrary => "missing-library",
            Self::DuplicateSymbol => "duplicate-symbol",
            Self::UnsupportedTarget => "unsupported-target",
            Self::Other => "other",
        }
    }
}

/// A native link failure with a classified cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkError {
    message: String,
    failure: LinkFailure,
}

impl LinkError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            failure: LinkFailure::Other,
        }
    }
    #[must_use]
    pub fn classified(message: impl Into<String>, failure: LinkFailure) -> Self {
        Self {
            message: message.into(),
            failure,
        }
    }
    /// Builds an error from raw linker stderr, classifying the cause.
    #[must_use]
    pub fn from_linker_output(program: &std::path::Path, stderr: &[u8]) -> Self {
        let text = String::from_utf8_lossy(stderr).trim().to_owned();
        let message = if text.is_empty() {
            format!("{} failed without diagnostics", program.display())
        } else {
            format!("{} failed: {text}", program.display())
        };
        Self {
            failure: classify(&text),
            message,
        }
    }
    #[must_use]
    pub fn failure(&self) -> LinkFailure {
        self.failure
    }
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LinkError {}

/// Classifies linker diagnostics into a [`LinkFailure`].
///
/// The patterns cover the toolchains measured in M-LINK.0 (MSVC `link.exe`,
/// GNU `cc`/`ld`, and Apple `ld`). Unknown output falls back to
/// [`LinkFailure::Other`].
#[must_use]
pub fn classify(output: &str) -> LinkFailure {
    let lower = output.to_ascii_lowercase();
    if lower.contains("text-relocation") || lower.contains("illegal text-relocation") {
        return LinkFailure::UnsupportedTarget;
    }
    if lower.contains("lnk1104")
        || lower.contains("cannot open file 'libc")
        || lower.contains("cannot find -lcrt")
        || lower.contains("libcmt.lib")
    {
        return LinkFailure::MissingCrt;
    }
    if lower.contains("lnk2005")
        || lower.contains("multiple definition")
        || lower.contains("duplicate symbol")
    {
        return LinkFailure::DuplicateSymbol;
    }
    if lower.contains("cannot open input file")
        || lower.contains("cannot find -l")
        || lower.contains("no such file or directory")
        || lower.contains("undefined reference")
        || lower.contains("unresolved external symbol")
    {
        return LinkFailure::MissingLibrary;
    }
    if lower.contains("windows sdk") || lower.contains("sdk not found") {
        return LinkFailure::MissingSdk;
    }
    if lower.contains("not found") || lower.contains("no such file") {
        return LinkFailure::ToolNotFound;
    }
    LinkFailure::Other
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn classifies_measured_toolchain_diagnostics() {
        assert_eq!(
            classify("ld: Found illegal text-relocations"),
            LinkFailure::UnsupportedTarget
        );
        assert_eq!(
            classify("LINK : fatal error LNK2005: symbol already defined"),
            LinkFailure::DuplicateSymbol
        );
        assert_eq!(
            classify("ld: cannot find -lbcrypt: No such file or directory"),
            LinkFailure::MissingLibrary
        );
        assert_eq!(
            classify("LINK : fatal error LNK1104: cannot open file 'libcmt.lib'"),
            LinkFailure::MissingCrt
        );
    }

    #[test]
    fn tool_start_failure_is_classified() {
        let error = LinkError::classified("failed to start linker", LinkFailure::ToolNotFound);
        assert_eq!(error.failure(), LinkFailure::ToolNotFound);
        assert_eq!(error.message(), "failed to start linker");
    }

    #[test]
    fn empty_output_reports_program_name() {
        let error = LinkError::from_linker_output(Path::new("link.exe"), b"");
        assert!(error.message().contains("link.exe"));
        assert_eq!(error.failure(), LinkFailure::Other);
    }
}
