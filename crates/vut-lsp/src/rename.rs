//! Validation and cross-file edit planning for semantic symbol rename.

use std::collections::{BTreeSet, HashMap};

use tower_lsp::lsp_types::{PrepareRenameResponse, TextEdit, Url, WorkspaceEdit};
use vut_lexer::{Lexer, TokenKind};
use vut_resolver::{SymbolId, SymbolKind};
use vut_source::{SourceId, Span};

use super::{Analysis, location, query::SemanticQueries, range};

pub(super) struct RenameTarget {
    symbol: SymbolId,
    pub(super) response: PrepareRenameResponse,
}

pub(super) fn target(a: &Analysis, source: SourceId, at: usize) -> Option<RenameTarget> {
    let queries = SemanticQueries::new(&a.resolution);
    let symbol = queries.symbol_at(source, at)?;
    let declaration = queries.declaration(symbol);
    if declaration.span.is_empty()
        || !is_supported(declaration.kind)
        || !is_editable(a, declaration.span)
    {
        return None;
    }
    let occurrence = a
        .resolution
        .references
        .iter()
        .filter(|reference| {
            reference.symbol == symbol
                && reference.span.source() == source
                && reference.span.start() <= at
                && at <= reference.span.end()
        })
        .min_by_key(|reference| reference.span.end() - reference.span.start())
        .map_or(declaration.span, |reference| reference.span);
    let source = a.sources.get(&occurrence.source())?;
    Some(RenameTarget {
        symbol,
        response: PrepareRenameResponse::RangeWithPlaceholder {
            range: range(&source.text, occurrence),
            placeholder: declaration.name.clone(),
        },
    })
}

pub(super) fn edits(
    a: &Analysis,
    target: &RenameTarget,
    new_name: &str,
) -> Result<WorkspaceEdit, String> {
    if !valid_identifier(new_name) {
        return Err(format!("`{new_name}` is not a valid Vut identifier"));
    }
    let symbol = &a.resolution.symbols[target.symbol.0];
    if symbol.name == new_name {
        return Ok(WorkspaceEdit::default());
    }
    if conflicts(a, target.symbol, new_name) {
        return Err(format!(
            "renaming `{}` to `{new_name}` would conflict with an existing symbol",
            symbol.name
        ));
    }

    let queries = SemanticQueries::new(&a.resolution);
    let mut spans = BTreeSet::from([span_key(queries.declaration(target.symbol).span)]);
    spans.extend(
        queries
            .references(target.symbol)
            .map(|reference| span_key(reference.span)),
    );
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for (source_id, start, end) in spans {
        let span = Span::new(source_id, start, end);
        if !is_editable(a, span) {
            return Err(
                "rename would modify a dependency, standard library, or synthetic source".into(),
            );
        }
        let Some(source) = a.sources.get(&source_id) else {
            return Err("rename source is not present in the project source map".into());
        };
        let Some(location) = location(a, span) else {
            return Err("rename source does not have an editable file location".into());
        };
        changes.entry(location.uri).or_default().push(TextEdit {
            range: range(&source.text, span),
            new_text: new_name.to_owned(),
        });
    }
    for edits in changes.values_mut() {
        edits.sort_by(|left, right| {
            right
                .range
                .start
                .cmp(&left.range.start)
                .then_with(|| right.range.end.cmp(&left.range.end))
        });
    }
    Ok(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

fn conflicts(a: &Analysis, target: SymbolId, new_name: &str) -> bool {
    let symbol = &a.resolution.symbols[target.0];
    a.resolution.symbols.iter().any(|candidate| {
        candidate.id != target
            && candidate.module == symbol.module
            && candidate.name == new_name
            && match symbol.kind {
                SymbolKind::Method => {
                    candidate.kind == SymbolKind::Method
                        && candidate.receiver == symbol.receiver
                        && candidate.is_static == symbol.is_static
                }
                SymbolKind::Parameter | SymbolKind::Local => {
                    matches!(candidate.kind, SymbolKind::Parameter | SymbolKind::Local)
                }
                _ => !matches!(candidate.kind, SymbolKind::Parameter | SymbolKind::Local),
            }
    })
}

fn valid_identifier(name: &str) -> bool {
    let (tokens, diagnostics) = Lexer::new(SourceId::from_index(0), name).lex();
    !diagnostics.has_errors()
        && matches!(tokens.as_slice(), [token, eof] if token.kind == TokenKind::Identifier && eof.kind == TokenKind::Eof)
}

fn is_editable(a: &Analysis, span: Span) -> bool {
    a.sources
        .get(&span.source())
        .and_then(|source| source.path.as_ref())
        .is_some_and(|path| path.starts_with(&a.root))
}

const fn is_supported(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Module
            | SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Data
            | SymbolKind::Interface
            | SymbolKind::Enum
            | SymbolKind::TypeAlias
            | SymbolKind::TypeParameter
            | SymbolKind::Binding
            | SymbolKind::Parameter
            | SymbolKind::Local
    )
}

const fn span_key(span: Span) -> (SourceId, usize, usize) {
    (span.source(), span.start(), span.end())
}
