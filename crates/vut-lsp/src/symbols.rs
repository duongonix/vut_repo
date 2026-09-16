//! Document outline derived from declarations in the compiler AST.

use tower_lsp::lsp_types::DocumentSymbol;
use vut_ast::{File, Item};
use vut_resolver::SymbolKind as ResolverSymbolKind;
use vut_source::Span;

use super::range;

pub(super) fn symbols(file: &File, text: &str) -> Vec<DocumentSymbol> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Function(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Function,
                x.span,
                x.name.span,
                text,
                Vec::new(),
            )),
            Item::Method(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Method,
                x.span,
                x.name.span,
                text,
                Vec::new(),
            )),
            Item::Data(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Data,
                x.span,
                x.name.span,
                text,
                x.fields
                    .iter()
                    .map(|f| {
                        doc(
                            &f.name.text,
                            ResolverSymbolKind::Binding,
                            f.span,
                            f.name.span,
                            text,
                            Vec::new(),
                        )
                    })
                    .collect(),
            )),
            Item::Interface(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Interface,
                x.span,
                x.name.span,
                text,
                Vec::new(),
            )),
            Item::Enum(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Enum,
                x.span,
                x.name.span,
                text,
                Vec::new(),
            )),
            Item::TypeAlias(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::TypeAlias,
                x.span,
                x.name.span,
                text,
                Vec::new(),
            )),
            _ => None,
        })
        .collect()
}
fn doc(
    name: &str,
    kind: ResolverSymbolKind,
    span: Span,
    selection: Span,
    text: &str,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    DocumentSymbol {
        name: name.into(),
        detail: None,
        kind: match kind {
            ResolverSymbolKind::Function => tower_lsp::lsp_types::SymbolKind::FUNCTION,
            ResolverSymbolKind::Method => tower_lsp::lsp_types::SymbolKind::METHOD,
            ResolverSymbolKind::Data => tower_lsp::lsp_types::SymbolKind::STRUCT,
            ResolverSymbolKind::Interface => tower_lsp::lsp_types::SymbolKind::INTERFACE,
            ResolverSymbolKind::Enum => tower_lsp::lsp_types::SymbolKind::ENUM,
            _ => tower_lsp::lsp_types::SymbolKind::VARIABLE,
        },
        tags: None,
        deprecated: None,
        range: range(text, span),
        selection_range: range(text, selection),
        children: (!children.is_empty()).then_some(children),
    }
}
