use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxKind {
    Whitespace,
    Newline,
    Comment,
    String,
    Word,
    Symbol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxToken {
    pub kind: SyntaxKind,
    pub text: String,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LosslessSyntaxTree {
    tokens: Vec<SyntaxToken>,
    source_len: usize,
}

impl LosslessSyntaxTree {
    #[must_use]
    pub fn parse(source: &str) -> Self {
        let mut tokens = Vec::new();
        let mut cursor = 0;
        let bytes = source.as_bytes();
        while cursor < source.len() {
            let start = cursor;
            let rest = &source[cursor..];
            let (kind, end) = if rest.starts_with("##") && !rest.starts_with("###") {
                let offset = rest[2..].find("##").map_or(rest.len(), |index| index + 4);
                (SyntaxKind::Comment, cursor + offset)
            } else if rest.starts_with('#') {
                (
                    SyntaxKind::Comment,
                    cursor + rest.find(['\r', '\n']).unwrap_or(rest.len()),
                )
            } else if rest.starts_with("\r\n") {
                (SyntaxKind::Newline, cursor + 2)
            } else if matches!(bytes[cursor], b'\r' | b'\n') {
                (SyntaxKind::Newline, cursor + 1)
            } else if bytes[cursor].is_ascii_whitespace() {
                let count = rest.bytes().take_while(u8::is_ascii_whitespace).count();
                (SyntaxKind::Whitespace, cursor + count)
            } else if let Some(string_body) = rest.strip_prefix('"') {
                (SyntaxKind::String, string_end(string_body, cursor + 1))
            } else if rest
                .chars()
                .next()
                .is_some_and(|c| c == '_' || c.is_alphanumeric())
            {
                let count = rest
                    .char_indices()
                    .take_while(|(_, c)| *c == '_' || c.is_alphanumeric())
                    .map(|(offset, c)| offset + c.len_utf8())
                    .last()
                    .unwrap_or(1);
                (SyntaxKind::Word, cursor + count)
            } else {
                let width = ["..=", "->", "==", "!=", ">=", "<=", ".."]
                    .iter()
                    .find(|symbol| rest.starts_with(**symbol))
                    .map_or_else(
                        || rest.chars().next().map_or(1, char::len_utf8),
                        |s| s.len(),
                    );
                (SyntaxKind::Symbol, cursor + width)
            };
            tokens.push(SyntaxToken {
                kind,
                text: source[start..end].to_owned(),
                range: start..end,
            });
            cursor = end;
        }
        Self {
            tokens,
            source_len: source.len(),
        }
    }

    #[must_use]
    pub fn tokens(&self) -> &[SyntaxToken] {
        &self.tokens
    }
    #[must_use]
    pub fn reconstruct(&self) -> String {
        self.tokens.iter().map(|t| t.text.as_str()).collect()
    }
    #[must_use]
    pub fn covered_bytes(&self) -> usize {
        self.tokens.iter().map(|t| t.range.len()).sum()
    }
    #[must_use]
    pub fn source_len(&self) -> usize {
        self.source_len
    }
}

fn string_end(body: &str, absolute_start: usize) -> usize {
    let mut escaped = false;
    let mut interpolation_depth = 0_usize;
    let mut nested_quote = false;
    let mut previous = '\0';
    let mut end = absolute_start;
    for (offset, character) in body.char_indices() {
        end = absolute_start + offset + character.len_utf8();
        if escaped {
            escaped = false;
            previous = character;
            continue;
        }
        if character == '\\' {
            escaped = true;
            previous = character;
            continue;
        }
        if interpolation_depth > 0 {
            if character == '"' {
                nested_quote = !nested_quote;
            } else if !nested_quote && character == '(' && previous == '$' {
                interpolation_depth += 1;
            } else if !nested_quote && character == ')' {
                interpolation_depth -= 1;
            }
        } else if character == '"' {
            break;
        } else if character == '(' && previous == '$' {
            interpolation_depth = 1;
        }
        previous = character;
    }
    end
}
