use crate::{LosslessSyntaxTree, SyntaxKind, SyntaxToken, validation::validate};

pub fn format_source(source: &str) -> Result<String, String> {
    validate(source)?;
    if source.is_empty() {
        return Ok(String::new());
    }
    let tree = LosslessSyntaxTree::parse(source);
    debug_assert_eq!(tree.covered_bytes(), tree.source_len());
    let mut output = String::new();
    let mut line = Vec::new();
    let mut levels = vec![0_usize];
    for token in tree.tokens() {
        if token.kind == SyntaxKind::Newline {
            format_line(&line, &mut levels, &mut output);
            output.push('\n');
            line.clear();
        } else {
            line.push(token);
        }
    }
    if !line.is_empty() {
        format_line(&line, &mut levels, &mut output);
    }
    Ok(output)
}

fn format_line(tokens: &[&SyntaxToken], levels: &mut Vec<usize>, output: &mut String) {
    if tokens.is_empty() {
        return;
    }
    if tokens.iter().any(|token| {
        token.kind == SyntaxKind::Comment
            && token.text.starts_with("##")
            && token.text.contains(['\r', '\n'])
    }) {
        let indent = original_indent(tokens);
        update_levels(levels, indent);
        output.push_str(&"  ".repeat(levels.len() - 1));
        output.push_str(
            tokens
                .iter()
                .skip_while(|t| t.kind == SyntaxKind::Whitespace)
                .map(|t| t.text.as_str())
                .collect::<String>()
                .trim_end(),
        );
        return;
    }
    let indent = original_indent(tokens);
    let significant = tokens
        .iter()
        .filter(|token| token.kind != SyntaxKind::Whitespace)
        .copied()
        .collect::<Vec<_>>();
    if significant.is_empty() {
        return;
    }
    update_levels(levels, indent);
    output.push_str(&"  ".repeat(levels.len() - 1));
    let mut previous: Option<&SyntaxToken> = None;
    for token in significant {
        if token.kind == SyntaxKind::Comment {
            if previous.is_some() && !output.ends_with(' ') {
                output.push(' ');
            }
            output.push_str(token.text.trim_end());
            break;
        }
        if needs_space(previous, token) {
            output.push(' ');
        }
        output.push_str(&token.text);
        previous = Some(token);
    }
}

fn original_indent(tokens: &[&SyntaxToken]) -> usize {
    tokens
        .first()
        .filter(|t| t.kind == SyntaxKind::Whitespace)
        .map_or(0, |token| {
            token
                .text
                .chars()
                .map(|c| if c == '\t' { 2 } else { 1 })
                .sum()
        })
}

fn update_levels(levels: &mut Vec<usize>, width: usize) {
    while levels.last().is_some_and(|level| width < *level) {
        levels.pop();
    }
    if width > *levels.last().unwrap_or(&0) {
        levels.push(width);
    }
}

fn needs_space(previous: Option<&SyntaxToken>, current: &SyntaxToken) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    let left = previous.text.as_str();
    let right = current.text.as_str();
    if matches!(right, ")" | "," | ":" | "." | "?") || matches!(left, "(" | "." | "@") {
        return false;
    }
    if right == "(" {
        return false;
    }
    if matches!(left, "," | ":") {
        return true;
    }
    if is_operator(left) || is_operator(right) {
        return true;
    }
    matches!(previous.kind, SyntaxKind::Word | SyntaxKind::String)
        && matches!(current.kind, SyntaxKind::Word | SyntaxKind::String)
}

fn is_operator(value: &str) -> bool {
    matches!(
        value,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "="
            | "=="
            | "!="
            | ">"
            | "<"
            | ">="
            | "<="
            | "->"
            | "and"
            | "or"
            | "in"
            | ".."
            | "..="
    )
}
