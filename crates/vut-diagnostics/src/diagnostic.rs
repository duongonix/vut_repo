//! Structured diagnostics shared by every Vut compiler stage.
use std::collections::HashSet;
use vut_source::Span;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagnosticCode(&'static str);
impl DiagnosticCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}
impl std::ops::Deref for DiagnosticCode {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub mod codes {
    use super::DiagnosticCode;
    pub const E0001: DiagnosticCode = DiagnosticCode("E0001");
    pub const E0002: DiagnosticCode = DiagnosticCode("E0002");
    pub const E0003: DiagnosticCode = DiagnosticCode("E0003");
    pub const E0004: DiagnosticCode = DiagnosticCode("E0004");
    pub const E0005: DiagnosticCode = DiagnosticCode("E0005");
    pub const E0006: DiagnosticCode = DiagnosticCode("E0006");
    pub const E0007: DiagnosticCode = DiagnosticCode("E0007");
    pub const E0101: DiagnosticCode = DiagnosticCode("E0101");
    pub const E0102: DiagnosticCode = DiagnosticCode("E0102");
    pub const E0103: DiagnosticCode = DiagnosticCode("E0103");
    pub const E0104: DiagnosticCode = DiagnosticCode("E0104");
    pub const E0105: DiagnosticCode = DiagnosticCode("E0105");
    pub const E0106: DiagnosticCode = DiagnosticCode("E0106");
    pub const E0107: DiagnosticCode = DiagnosticCode("E0107");
    pub const E0108: DiagnosticCode = DiagnosticCode("E0108");
    pub const E0109: DiagnosticCode = DiagnosticCode("E0109");
    pub const E0110: DiagnosticCode = DiagnosticCode("E0110");
    pub const E0111: DiagnosticCode = DiagnosticCode("E0111");
    pub const E0112: DiagnosticCode = DiagnosticCode("E0112");
    pub const E1001: DiagnosticCode = DiagnosticCode("E1001");
    pub const E1003: DiagnosticCode = DiagnosticCode("E1003");
    pub const E1004: DiagnosticCode = DiagnosticCode("E1004");
    pub const E1005: DiagnosticCode = DiagnosticCode("E1005");
    pub const E1006: DiagnosticCode = DiagnosticCode("E1006");
    pub const E1007: DiagnosticCode = DiagnosticCode("E1007");
    pub const E1008: DiagnosticCode = DiagnosticCode("E1008");
    pub const E1009: DiagnosticCode = DiagnosticCode("E1009");
    pub const E1010: DiagnosticCode = DiagnosticCode("E1010");
    pub const E1012: DiagnosticCode = DiagnosticCode("E1012");
    pub const E1013: DiagnosticCode = DiagnosticCode("E1013");
    pub const E1014: DiagnosticCode = DiagnosticCode("E1014");
    pub const E1015: DiagnosticCode = DiagnosticCode("E1015");
    pub const E1016: DiagnosticCode = DiagnosticCode("E1016");
    pub const E1017: DiagnosticCode = DiagnosticCode("E1017");
    pub const E1018: DiagnosticCode = DiagnosticCode("E1018");
    pub const E1019: DiagnosticCode = DiagnosticCode("E1019");
    pub const E1020: DiagnosticCode = DiagnosticCode("E1020");
    pub const E1021: DiagnosticCode = DiagnosticCode("E1021");
    pub const E1023: DiagnosticCode = DiagnosticCode("E1023");
    pub const E1024: DiagnosticCode = DiagnosticCode("E1024");
    pub const E1025: DiagnosticCode = DiagnosticCode("E1025");
    pub const E1026: DiagnosticCode = DiagnosticCode("E1026");
    pub const E1028: DiagnosticCode = DiagnosticCode("E1028");
    pub const E1104: DiagnosticCode = DiagnosticCode("E1104");
    pub const E2004: DiagnosticCode = DiagnosticCode("E2004");
    pub const E2005: DiagnosticCode = DiagnosticCode("E2005");
    pub const E2001: DiagnosticCode = DiagnosticCode("E2001");
    pub const E2002: DiagnosticCode = DiagnosticCode("E2002");
    pub const E2003: DiagnosticCode = DiagnosticCode("E2003");
    pub const E2006: DiagnosticCode = DiagnosticCode("E2006");
    pub const E2007: DiagnosticCode = DiagnosticCode("E2007");
    pub const E2008: DiagnosticCode = DiagnosticCode("E2008");
    pub const E3001: DiagnosticCode = DiagnosticCode("E3001");
    pub const E3002: DiagnosticCode = DiagnosticCode("E3002");
    pub const E3003: DiagnosticCode = DiagnosticCode("E3003");
    pub const E3004: DiagnosticCode = DiagnosticCode("E3004");
    pub const E3005: DiagnosticCode = DiagnosticCode("E3005");
    pub const E3006: DiagnosticCode = DiagnosticCode("E3006");
    pub const E3007: DiagnosticCode = DiagnosticCode("E3007");
    pub const E3008: DiagnosticCode = DiagnosticCode("E3008");
    pub const E3010: DiagnosticCode = DiagnosticCode("E3010");
    pub const E4010: DiagnosticCode = DiagnosticCode("E4010");
    pub const E4011: DiagnosticCode = DiagnosticCode("E4011");
    pub const E4001: DiagnosticCode = DiagnosticCode("E4001");
    pub const E4004: DiagnosticCode = DiagnosticCode("E4004");
    pub const E4102: DiagnosticCode = DiagnosticCode("E4102");
    pub const E4103: DiagnosticCode = DiagnosticCode("E4103");
    pub const E4104: DiagnosticCode = DiagnosticCode("E4104");
    pub const E5001: DiagnosticCode = DiagnosticCode("E5001");
    pub const E5003: DiagnosticCode = DiagnosticCode("E5003");
    pub const E5004: DiagnosticCode = DiagnosticCode("E5004");
    pub const E5005: DiagnosticCode = DiagnosticCode("E5005");
    pub const E5006: DiagnosticCode = DiagnosticCode("E5006");
    pub const E5007: DiagnosticCode = DiagnosticCode("E5007");
    pub const E5009: DiagnosticCode = DiagnosticCode("E5009");
    pub const E6002: DiagnosticCode = DiagnosticCode("E6002");
    pub const E6003: DiagnosticCode = DiagnosticCode("E6003");
    pub const E6004: DiagnosticCode = DiagnosticCode("E6004");
    pub const E6005: DiagnosticCode = DiagnosticCode("E6005");
    pub const E6007: DiagnosticCode = DiagnosticCode("E6007");
    pub const E6008: DiagnosticCode = DiagnosticCode("E6008");
    pub const E6011: DiagnosticCode = DiagnosticCode("E6011");
    pub const E6012: DiagnosticCode = DiagnosticCode("E6012");
    pub const E6013: DiagnosticCode = DiagnosticCode("E6013");
    pub const E6014: DiagnosticCode = DiagnosticCode("E6014");
    pub const E6020: DiagnosticCode = DiagnosticCode("E6020");
    pub const E6021: DiagnosticCode = DiagnosticCode("E6021");
    pub const E6022: DiagnosticCode = DiagnosticCode("E6022");
    pub const E6023: DiagnosticCode = DiagnosticCode("E6023");
    pub const E6024: DiagnosticCode = DiagnosticCode("E6024");
    pub const E7101: DiagnosticCode = DiagnosticCode("E7101");
    pub const E7102: DiagnosticCode = DiagnosticCode("E7102");
    pub const E7103: DiagnosticCode = DiagnosticCode("E7103");
    pub const E7104: DiagnosticCode = DiagnosticCode("E7104");
    pub const E7105: DiagnosticCode = DiagnosticCode("E7105");
    pub const E7106: DiagnosticCode = DiagnosticCode("E7106");
    pub const E7107: DiagnosticCode = DiagnosticCode("E7107");
    pub const E7108: DiagnosticCode = DiagnosticCode("E7108");
    pub const E7109: DiagnosticCode = DiagnosticCode("E7109");
    pub const E7013: DiagnosticCode = DiagnosticCode("E7013");
    pub const E7110: DiagnosticCode = DiagnosticCode("E7110");
    pub const E7111: DiagnosticCode = DiagnosticCode("E7111");
    pub const E7112: DiagnosticCode = DiagnosticCode("E7112");
    pub const E8001: DiagnosticCode = DiagnosticCode("E8001");
    pub const E8002: DiagnosticCode = DiagnosticCode("E8002");
    pub const E8003: DiagnosticCode = DiagnosticCode("E8003");
    pub const E8004: DiagnosticCode = DiagnosticCode("E8004");
    pub const E8005: DiagnosticCode = DiagnosticCode("E8005");
    pub const E8006: DiagnosticCode = DiagnosticCode("E8006");
    pub const E8007: DiagnosticCode = DiagnosticCode("E8007");
    pub const E8008: DiagnosticCode = DiagnosticCode("E8008");
    pub const E8009: DiagnosticCode = DiagnosticCode("E8009");
    pub const E8010: DiagnosticCode = DiagnosticCode("E8010");
    pub const E8011: DiagnosticCode = DiagnosticCode("E8011");
    pub const E8012: DiagnosticCode = DiagnosticCode("E8012");
    pub const W2001: DiagnosticCode = DiagnosticCode("W2001");
    pub const W5004: DiagnosticCode = DiagnosticCode("W5004");
    pub const W6002: DiagnosticCode = DiagnosticCode("W6002");
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}
impl Severity {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
            Self::Help => "help",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedFound {
    pub expected: String,
    pub found: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<DiagnosticCode>,
    pub title: String,
    pub primary: Option<Label>,
    pub labels: Vec<Label>,
    pub related: Vec<Label>,
    pub expected_found: Option<ExpectedFound>,
    pub notes: Vec<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(
        code: DiagnosticCode,
        title: impl Into<String>,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: Some(code),
            title: title.into(),
            primary: Some(Label {
                span,
                message: message.into(),
                style: LabelStyle::Primary,
            }),
            labels: Vec::new(),
            related: Vec::new(),
            expected_found: None,
            notes: Vec::new(),
            help: None,
        }
    }
    #[must_use]
    pub fn warning(
        code: DiagnosticCode,
        title: impl Into<String>,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        let mut value = Self::error(code, title, span, message);
        value.severity = Severity::Warning;
        value
    }
    #[must_use]
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        });
        self
    }
    #[must_use]
    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related.push(Label {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        });
        self
    }
    #[must_use]
    pub fn with_expected_found(
        mut self,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        self.expected_found = Some(ExpectedFound {
            expected: expected.into(),
            found: found.into(),
        });
        self
    }
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
    invalid_roots: HashSet<Span>,
}
impl DiagnosticSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
    pub fn extend(&mut self, other: Self) {
        self.diagnostics.extend(other.diagnostics);
        self.invalid_roots.extend(other.invalid_roots);
    }
    pub fn push_root(&mut self, diagnostic: Diagnostic) {
        if let Some(primary) = &diagnostic.primary {
            self.invalid_roots.insert(primary.span);
        }
        self.diagnostics.push(diagnostic);
    }
    pub fn push_dependent(&mut self, cause: Span, diagnostic: Diagnostic) {
        if !self.invalid_roots.contains(&cause) {
            self.diagnostics.push(diagnostic);
        }
    }
    pub fn sort_deterministically(&mut self) {
        self.diagnostics.sort_by_key(|item| {
            let span = item.primary.as_ref().map(|label| label.span);
            (
                span.map(vut_source::Span::source),
                span.map_or(usize::MAX, vut_source::Span::start),
                item.severity,
                item.code,
            )
        });
    }
    #[must_use]
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|item| item.severity == Severity::Error)
    }
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|item| item.severity == Severity::Error)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorChoice, Renderer};
    use vut_source::SourceId;
    #[test]
    fn retains_all_structured_fields() {
        let source = SourceId::from_index(0);
        let diagnostic = Diagnostic::error(
            codes::E0001,
            "invalid",
            Span::new(source, 4, 5),
            "bad token",
        )
        .with_label(Span::new(source, 0, 1), "context")
        .with_related(Span::new(source, 8, 9), "declared here")
        .with_expected_found("identifier", "[")
        .with_note("note")
        .with_help("remove it");
        assert_eq!(diagnostic.code.map(DiagnosticCode::as_str), Some("E0001"));
        assert_eq!(diagnostic.labels.len(), 1);
        assert!(diagnostic.expected_found.is_some());
    }
    #[test]
    fn suppresses_dependents_of_known_root_errors() {
        let source = SourceId::from_index(0);
        let span = Span::new(source, 1, 2);
        let mut sink = DiagnosticSink::new();
        sink.push_root(Diagnostic::error(codes::E0101, "root", span, "root"));
        sink.push_dependent(
            span,
            Diagnostic::error(codes::E2001, "dependent", span, "dependent"),
        );
        assert_eq!(sink.error_count(), 1);
    }
    #[test]
    fn renders_plain_color_and_machine_output_from_one_model() {
        let mut sources = vut_source::SourceManager::new();
        let source = sources.add_text("src/main.vut", "age: int = \"20\"\n".into());
        let diagnostic = Diagnostic::error(
            codes::E0101,
            "type mismatch",
            Span::new(source, 11, 15),
            "expected `int`",
        )
        .with_expected_found("int", "str")
        .with_help("use an integer");
        let plain =
            Renderer::new(ColorChoice::Never).render_with_terminal(&diagnostic, &sources, true);
        assert!(plain.contains("error[E0101]: type mismatch"));
        assert!(plain.contains("src/main.vut:1:12"));
        assert!(!plain.contains("\x1b["));
        let color =
            Renderer::new(ColorChoice::Always).render_with_terminal(&diagnostic, &sources, false);
        assert!(color.contains("\x1b[31m"));
        let json = Renderer::new(ColorChoice::Never).render_json(&diagnostic, &sources);
        assert!(json.contains("\"schema\":1"));
        assert!(json.contains("\"code\":\"E0101\""));
    }
    #[test]
    fn deterministic_sort_orders_by_source_and_offset() {
        let source = SourceId::from_index(0);
        let mut sink = DiagnosticSink::new();
        sink.push(Diagnostic::error(
            codes::E2001,
            "later",
            Span::new(source, 8, 9),
            "later",
        ));
        sink.push(Diagnostic::error(
            codes::E0101,
            "earlier",
            Span::new(source, 1, 2),
            "earlier",
        ));
        sink.sort_deterministically();
        assert_eq!(sink.as_slice()[0].title, "earlier");
    }
}
