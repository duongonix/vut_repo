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
            Item::ExternFunction(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Function,
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
                interface_children(x, text),
            )),
            Item::Enum(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::Enum,
                x.span,
                x.name.span,
                text,
                enum_children(x, text),
            )),
            Item::TypeAlias(x) => Some(doc(
                &x.name.text,
                ResolverSymbolKind::TypeAlias,
                x.span,
                x.name.span,
                text,
                Vec::new(),
            )),
            Item::Import(_) | Item::Statement(_) => None,
        })
        .collect()
}

fn interface_children(value: &vut_ast::Interface, text: &str) -> Vec<DocumentSymbol> {
    value
        .methods
        .iter()
        .map(|method| {
            doc(
                &method.name.text,
                ResolverSymbolKind::Method,
                method.span,
                method.name.span,
                text,
                Vec::new(),
            )
        })
        .collect()
}

fn enum_children(value: &vut_ast::Enum, text: &str) -> Vec<DocumentSymbol> {
    value
        .variants
        .iter()
        .map(|variant| DocumentSymbol {
            name: variant.name.text.clone(),
            detail: None,
            kind: tower_lsp::lsp_types::SymbolKind::ENUM_MEMBER,
            tags: None,
            deprecated: None,
            range: range(text, variant.span),
            selection_range: range(text, variant.name.span),
            children: (!variant.fields.is_empty()).then(|| {
                variant
                    .fields
                    .iter()
                    .map(|field| DocumentSymbol {
                        name: field.name.text.clone(),
                        detail: None,
                        kind: tower_lsp::lsp_types::SymbolKind::FIELD,
                        tags: None,
                        deprecated: None,
                        range: range(text, field.span),
                        selection_range: range(text, field.name.span),
                        children: None,
                    })
                    .collect()
            }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use vut_lexer::Lexer;
    use vut_parser::Parser;
    use vut_source::SourceId;

    #[test]
    fn includes_extern_interface_methods_and_enum_payloads() {
        let text = "extern \"C\" fn native() -> int\ninterface Reader:\n  read() -> int\nenum Shape:\n  circle(radius: float)\n";
        let source = SourceId::from_index(0);
        let (tokens, lexical) = Lexer::new(source, text).lex();
        assert!(!lexical.has_errors());
        let (file, syntax) = Parser::new(source, text, tokens).parse();
        assert!(!syntax.has_errors());
        let result = symbols(&file, text);
        assert!(result.iter().any(|symbol| symbol.name == "native"));
        assert_eq!(
            result
                .iter()
                .find(|symbol| symbol.name == "Reader")
                .and_then(|symbol| symbol.children.as_ref())
                .unwrap()[0]
                .name,
            "read"
        );
        let variant = &result
            .iter()
            .find(|symbol| symbol.name == "Shape")
            .and_then(|symbol| symbol.children.as_ref())
            .unwrap()[0];
        assert_eq!(variant.name, "circle");
        assert_eq!(variant.children.as_ref().unwrap()[0].name, "radius");
    }
}
