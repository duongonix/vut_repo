//! Lossless mapping from compiler diagnostics to the LSP diagnostic model.

use std::collections::HashMap;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    NumberOrString, Url,
};
use vut_diagnostics::{Diagnostic, Severity};
use vut_source::SourceId;

use super::{project::ProjectSnapshot, range};

pub(super) fn collect(snapshot: &ProjectSnapshot) -> HashMap<SourceId, Vec<LspDiagnostic>> {
    let diagnostics = snapshot
        .checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .chain(snapshot.checked.semantics.diagnostics.as_slice());
    collect_from(diagnostics, &snapshot.sources)
}

fn collect_from<'a>(
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
    sources: &HashMap<SourceId, super::project::SourceSnapshot>,
) -> HashMap<SourceId, Vec<LspDiagnostic>> {
    let mut result: HashMap<SourceId, Vec<LspDiagnostic>> = HashMap::new();
    for diagnostic in diagnostics {
        let Some(primary) = &diagnostic.primary else {
            continue;
        };
        let Some(source) = sources.get(&primary.span.source()) else {
            continue;
        };
        let related_information: Vec<_> = diagnostic
            .labels
            .iter()
            .chain(&diagnostic.related)
            .filter_map(|label| {
                let related = sources.get(&label.span.source())?;
                let uri = Url::from_file_path(related.path.as_ref()?).ok()?;
                Some(DiagnosticRelatedInformation {
                    location: Location::new(uri, range(&related.text, label.span)),
                    message: label.message.clone(),
                })
            })
            .collect();
        result
            .entry(primary.span.source())
            .or_default()
            .push(LspDiagnostic {
                range: range(&source.text, primary.span),
                severity: Some(severity(diagnostic.severity)),
                code: diagnostic
                    .code
                    .map(|value| NumberOrString::String(value.as_str().into())),
                code_description: None,
                source: Some("vut".into()),
                message: message(diagnostic),
                related_information: (!related_information.is_empty())
                    .then_some(related_information),
                tags: None,
                data: Some(serde_json::json!({
                    "title": diagnostic.title,
                    "primary": primary.message,
                    "expected": diagnostic.expected_found.as_ref().map(|value| &value.expected),
                    "found": diagnostic.expected_found.as_ref().map(|value| &value.found),
                    "notes": diagnostic.notes,
                    "help": diagnostic.help,
                })),
            });
    }
    result
}

fn severity(value: Severity) -> DiagnosticSeverity {
    match value {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::INFORMATION,
        Severity::Help => DiagnosticSeverity::HINT,
    }
}

fn message(diagnostic: &Diagnostic) -> String {
    let mut lines = vec![format!(
        "{}: {}",
        diagnostic.title,
        diagnostic
            .primary
            .as_ref()
            .map_or("", |label| &label.message)
    )];
    if let Some(value) = &diagnostic.expected_found {
        lines.push(format!("expected: {}", value.expected));
        lines.push(format!("found: {}", value.found));
    }
    lines.extend(diagnostic.notes.iter().map(|note| format!("note: {note}")));
    if let Some(help) = &diagnostic.help {
        lines.push(format!("help: {help}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::SourceSnapshot;
    use std::path::PathBuf;
    use vut_diagnostics::codes;
    use vut_source::{SourceId, Span};

    #[test]
    fn preserves_structured_details_and_cross_file_locations() {
        let main = SourceId::from_index(0);
        let helper = SourceId::from_index(1);
        let diagnostic = Diagnostic::error(
            codes::E0101,
            "type mismatch",
            Span::new(main, 4, 8),
            "invalid value",
        )
        .with_related(Span::new(helper, 0, 2), "declared here")
        .with_expected_found("int", "str")
        .with_note("types are static")
        .with_help("use an integer");
        let sources = HashMap::from([
            (
                main,
                SourceSnapshot {
                    path: Some(PathBuf::from("C:/project/main.vut")),
                    text: "a = 😀\n".into(),
                },
            ),
            (
                helper,
                SourceSnapshot {
                    path: Some(PathBuf::from("C:/project/helper.vut")),
                    text: "fn helper():\n".into(),
                },
            ),
        ]);
        let collected = collect_from(std::iter::once(&diagnostic), &sources);
        let item = &collected[&main][0];
        assert_eq!(item.code, Some(NumberOrString::String("E0101".into())));
        assert_eq!(item.range.start.character, 4);
        assert_eq!(item.range.end.character, 6);
        assert!(item.message.contains("expected: int"));
        assert!(item.message.contains("help: use an integer"));
        assert_eq!(item.related_information.as_ref().unwrap().len(), 1);
    }
}
