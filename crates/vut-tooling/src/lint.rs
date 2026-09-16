use crate::validation::validate;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lint {
    pub code: &'static str,
    pub line: usize,
    pub message: String,
}

pub fn lint_source(source: &str) -> Result<Vec<Lint>, String> {
    validate(source)?;
    let mut findings = Vec::new();
    let lines = source.lines().collect::<Vec<_>>();
    for (index, raw) in lines.iter().enumerate() {
        let text = raw.trim();
        if matches!(text, "if true:" | "if false:") {
            findings.push(finding("W2004", index, "constant condition"));
        }
        if text.contains(" == true") || text.contains(" == false") {
            findings.push(finding("W2005", index, "redundant boolean comparison"));
        }
        if let Some(name) = binding_name(text)
            && !name.starts_with('_')
            && identifier_count(source, name) == 1
        {
            findings.push(Lint {
                code: "W2001",
                line: index + 1,
                message: format!("unused variable `{name}`"),
            });
        }
        if let Some(name) = import_name(text)
            && identifier_count(source, name) == 1
        {
            findings.push(Lint {
                code: "W2002",
                line: index + 1,
                message: format!("unused import `{name}`"),
            });
        }
        if index > 0 && is_terminator(lines[index - 1].trim()) {
            let previous_indent = lines[index - 1].len() - lines[index - 1].trim_start().len();
            let indent = raw.len() - raw.trim_start().len();
            if !text.is_empty() && indent == previous_indent {
                findings.push(finding("W2003", index, "unreachable code"));
            }
        }
    }
    Ok(findings)
}

fn finding(code: &'static str, index: usize, message: &str) -> Lint {
    Lint {
        code,
        line: index + 1,
        message: message.into(),
    }
}
fn binding_name(line: &str) -> Option<&str> {
    let (left, _) = line.split_once('=')?;
    let name = left.split(':').next()?.trim();
    (!name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()))
        .then_some(name)
}
fn import_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("import ")?;
    if let Some((_, alias)) = rest.split_once(" as ") {
        return Some(alias.trim());
    }
    rest.split(['.', ' ']).next()
}
fn identifier_count(source: &str, name: &str) -> usize {
    source
        .split(|c: char| !(c == '_' || c.is_ascii_alphanumeric()))
        .filter(|part| *part == name)
        .count()
}
fn is_terminator(line: &str) -> bool {
    line == "return" || line.starts_with("return ") || line == "break" || line == "continue"
}
