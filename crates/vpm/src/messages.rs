//! CLI messages, colour policy, and output formatting.
//!
//! The library performs work and returns plain data; this module owns how that
//! data and the resulting status is presented. Success goes to stdout, while
//! diagnostics (errors, warnings, failed test reports) go to stderr.
use std::{fmt::Write as _, io::IsTerminal};

use crate::testing::TestReport;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, clap::ValueEnum)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

/// Resolved colour policy for a single CLI invocation.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    color: bool,
}

impl Style {
    #[must_use]
    pub fn new(mode: ColorMode) -> Self {
        let color = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                std::env::var_os("NO_COLOR").is_none()
                    && std::io::stdout().is_terminal()
                    && std::env::var("TERM").map_or(true, |term| term != "dumb")
            }
        };
        Self { color }
    }

    #[cfg(test)]
    #[must_use]
    pub fn plain() -> Self {
        Self { color: false }
    }

    fn paint(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_owned()
        }
    }

    #[must_use]
    pub fn success(self, text: &str) -> String {
        self.paint("32", text)
    }

    #[must_use]
    pub fn warning(self, text: &str) -> String {
        self.paint("33", text)
    }

    #[must_use]
    pub fn error(self, text: &str) -> String {
        self.paint("31", text)
    }

    #[must_use]
    pub fn bold(self, text: &str) -> String {
        self.paint("1", text)
    }
}

pub fn created(style: Style, name: &str, path: &str) -> String {
    format!("{} project `{name}` at {path}", style.success("created"))
}

pub fn initialized(style: Style, name: &str, path: &str) -> String {
    format!(
        "{} project `{name}` at {path}",
        style.success("initialized")
    )
}

pub fn added(style: Style, import: &str, version: &str, source: &str) -> String {
    format!("{} `{import}` {version} ({source})", style.success("added"))
}

pub fn removed(style: Style, name: &str) -> String {
    format!("{} `{name}`", style.success("removed"))
}

pub fn installed(style: Style, count: usize) -> String {
    format!("{} {count} package(s)", style.success("installed"))
}

pub fn already_up_to_date(style: Style, total: usize) -> String {
    format!(
        "{} ({total} package(s))",
        style.success("already up to date")
    )
}

pub fn updated(style: Style, count: usize) -> String {
    format!("{} {count} package(s)", style.success("updated"))
}

pub fn built(style: Style, path: &str, release: bool) -> String {
    let mode = if release { "release" } else { "debug" };
    format!("{} {path} ({mode})", style.success("built"))
}

pub fn running(style: Style, path: &str) -> String {
    format!("{} {path}", style.success("running"))
}

pub fn checked(style: Style, name: &str) -> String {
    format!("{} project `{name}`", style.success("checked"))
}

pub fn cleaned(style: Style) -> String {
    format!("{} build artifacts", style.success("cleaned"))
}

pub fn generated(style: Style, path: &str) -> String {
    format!("{} {path}", style.success("generated"))
}

pub fn no_lint_warnings(style: Style) -> String {
    style.success("no lint warnings")
}

pub fn formatted(style: Style, count: usize) -> String {
    format!("{} {count} file(s)", style.success("formatted"))
}

pub fn all_formatted(style: Style) -> String {
    style.success("all files formatted")
}

/// A `vpm fmt --check` failure (files need formatting); printed to stderr.
pub fn require_formatting(style: Style, count: usize) -> String {
    format!(
        "{} {count} file(s) require formatting",
        style.warning("warning:")
    )
}

/// One lint finding; printed to stderr as a warning.
pub fn lint_warning(style: Style, path: &str, line: usize, code: &str, message: &str) -> String {
    format!(
        "{path}:{line}: {} {message}",
        style.warning(&format!("warning[{code}]"))
    )
}

pub fn lint_summary(style: Style, count: usize) -> String {
    format!("{} {count} lint warning(s)", style.error("error:"))
}

pub fn error_line(style: Style, message: &str) -> String {
    format!("{} {message}", style.error("error:"))
}

/// Highlights the first line of multi-line data output (headers), leaving the
/// rest untouched so piped data stays valid when colour is disabled.
#[must_use]
pub fn highlight_header(style: Style, output: &str) -> String {
    if !style.color {
        return output.to_owned();
    }
    match output.split_once('\n') {
        Some((header, rest)) => format!("{}\n{rest}", style.bold(header)),
        None => style.bold(output),
    }
}

/// Formats a full test report with coloured pass/fail tokens.
#[must_use]
pub fn test_report(style: Style, report: &TestReport) -> String {
    let mut output = String::new();
    for outcome in &report.outcomes {
        let status = if outcome.success {
            style.success("ok")
        } else {
            style.error("FAILED")
        };
        let _ = writeln!(output, "test {} ... {status}", outcome.name);
        if !outcome.success {
            if let Some(error) = &outcome.error {
                let _ = writeln!(output, "  error: {error}");
            }
            if !outcome.stdout.is_empty() {
                let _ = writeln!(output, "  stdout:\n{}", indent(&outcome.stdout));
            }
            if !outcome.stderr.is_empty() {
                let _ = writeln!(output, "  stderr:\n{}", indent(&outcome.stderr));
            }
        }
    }
    let result = if report.failed == 0 {
        style.success("ok")
    } else {
        style.error("FAILED")
    };
    let _ = write!(
        output,
        "test result: {result}. {} passed; {} failed",
        report.passed, report.failed
    );
    output
}

fn indent(value: &str) -> String {
    value
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestOutcome;

    #[test]
    fn plain_messages_match_the_documented_wording() {
        let style = Style::plain();
        assert_eq!(
            created(style, "app", "/tmp/app"),
            "created project `app` at /tmp/app"
        );
        assert_eq!(
            added(style, "math", "1.2.0", "registry:math"),
            "added `math` 1.2.0 (registry:math)"
        );
        assert_eq!(installed(style, 3), "installed 3 package(s)");
        assert_eq!(
            already_up_to_date(style, 2),
            "already up to date (2 package(s))"
        );
        assert_eq!(
            built(style, "/tmp/app.exe", true),
            "built /tmp/app.exe (release)"
        );
        assert_eq!(
            require_formatting(style, 2),
            "warning: 2 file(s) require formatting"
        );
        assert_eq!(
            lint_warning(style, "src/a.vut", 4, "W2001", "unused binding"),
            "src/a.vut:4: warning[W2001] unused binding"
        );
        assert_eq!(error_line(style, "boom"), "error: boom");
    }

    #[test]
    fn forced_colour_wraps_only_tokens() {
        let style = Style::new(ColorMode::Always);
        let message = installed(style, 1);
        assert!(
            message.contains("\u{1b}[32minstalled\u{1b}[0m"),
            "{message:?}"
        );
        assert!(message.ends_with("1 package(s)"), "{message:?}");
        assert!(highlight_header(style, "Head\nbody").contains("\u{1b}[1mHead\u{1b}[0m"));
    }

    #[test]
    fn test_report_colours_status_tokens() {
        let style = Style::new(ColorMode::Always);
        let report = TestReport {
            outcomes: vec![
                TestOutcome {
                    name: "tests/pass.vut".into(),
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: None,
                },
                TestOutcome {
                    name: "tests/fail.vut".into(),
                    success: false,
                    stdout: "details".into(),
                    stderr: String::new(),
                    error: None,
                },
            ],
            passed: 1,
            failed: 1,
        };
        let text = test_report(style, &report);
        assert!(
            text.contains("test tests/pass.vut ... \u{1b}[32mok\u{1b}[0m"),
            "{text}"
        );
        assert!(
            text.contains("test tests/fail.vut ... \u{1b}[31mFAILED\u{1b}[0m"),
            "{text}"
        );
        assert!(text.contains("stdout:\n    details"), "{text}");
    }
}
