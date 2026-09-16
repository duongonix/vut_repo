use crate::{Diagnostic, Label, Severity};
use std::{fmt::Write, io::IsTerminal};
use vut_source::{SourceManager, Span};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}
#[derive(Clone, Copy, Debug)]
pub struct Renderer {
    color: ColorChoice,
}
impl Renderer {
    #[must_use]
    pub const fn new(color: ColorChoice) -> Self {
        Self { color }
    }
    #[must_use]
    pub fn render(&self, diagnostic: &Diagnostic, sources: &SourceManager) -> String {
        self.render_with_terminal(diagnostic, sources, std::io::stderr().is_terminal())
    }
    #[must_use]
    pub fn render_with_terminal(
        &self,
        diagnostic: &Diagnostic,
        sources: &SourceManager,
        terminal: bool,
    ) -> String {
        let color =
            self.color == ColorChoice::Always || (self.color == ColorChoice::Auto && terminal);
        let mut output = String::new();
        let severity = diagnostic.severity.as_str();
        let label = if color {
            format!(
                "\x1b[1;{}m{severity}\x1b[0m",
                severity_color(diagnostic.severity)
            )
        } else {
            severity.to_owned()
        };
        if let Some(code) = diagnostic.code {
            let _ = writeln!(output, "{label}[{}]: {}", code.as_str(), diagnostic.title);
        } else {
            let _ = writeln!(output, "{label}: {}", diagnostic.title);
        }
        if let Some(primary) = &diagnostic.primary {
            render_label(&mut output, primary, sources, color, "-->");
        }
        for related in &diagnostic.related {
            render_label(&mut output, related, sources, color, ":::");
        }
        if let Some(values) = &diagnostic.expected_found {
            let _ = writeln!(
                output,
                "   = expected: {}\n   = found:    {}",
                values.expected, values.found
            );
        }
        for note in &diagnostic.notes {
            let _ = writeln!(output, "   = note: {note}");
        }
        if let Some(help) = &diagnostic.help {
            let _ = writeln!(output, "   = help: {help}");
        }
        output
    }
    #[must_use]
    pub fn render_json(&self, diagnostic: &Diagnostic, sources: &SourceManager) -> String {
        let (file, line, column, start, end) = diagnostic
            .primary
            .as_ref()
            .and_then(|label| {
                sources.get(label.span.source()).ok().and_then(|source| {
                    source.location(label.span.start()).ok().map(|location| {
                        (
                            source.name(),
                            location.line,
                            location.column,
                            label.span.start(),
                            label.span.end(),
                        )
                    })
                })
            })
            .unwrap_or(("", 0, 0, 0, 0));
        format!(
            "{{\"schema\":1,\"severity\":\"{}\",\"code\":{},\"message\":\"{}\",\"file\":\"{}\",\"start\":{},\"end\":{},\"line\":{},\"column\":{}}}",
            diagnostic.severity.as_str(),
            diagnostic.code.map_or_else(
                || "null".to_owned(),
                |code| format!("\"{}\"", code.as_str())
            ),
            escape(&diagnostic.title),
            escape(file),
            start,
            end,
            line,
            column
        )
    }
}

fn render_label(
    output: &mut String,
    label: &Label,
    sources: &SourceManager,
    color: bool,
    arrow: &str,
) {
    let Ok(source) = sources.get(label.span.source()) else {
        return;
    };
    let Ok(location) = source.location(label.span.start()) else {
        return;
    };
    let line = source.line(location.line).unwrap_or("");
    let width = location.line.to_string().len();
    let _ = writeln!(
        output,
        "  {arrow} {}:{}:{}\n{:width$} |\n{} | {line}",
        source.name(),
        location.line,
        location.column,
        "",
        location.line,
        width = width
    );
    let chars = source
        .slice(same_line_span(label.span, line.len()))
        .map_or(1, |text| text.chars().count().max(1));
    let underline = "^".repeat(chars);
    let underline = if color {
        format!("\x1b[31m{underline}\x1b[0m")
    } else {
        underline
    };
    let _ = writeln!(
        output,
        "{:width$} | {}{underline} {}",
        "",
        " ".repeat(location.column.saturating_sub(1)),
        label.message,
        width = width
    );
}
fn same_line_span(span: Span, line_len: usize) -> Span {
    Span::new(
        span.source(),
        span.start(),
        span.end().min(span.start() + line_len),
    )
}
const fn severity_color(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 31,
        Severity::Warning => 33,
        Severity::Note => 36,
        Severity::Help => 32,
    }
}
fn escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}
