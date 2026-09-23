//! Shared resolver-backed semantic queries used by LSP features.

use vut_resolver::{Resolution, ResolvedReference, Symbol, SymbolId, SymbolKind};
use vut_source::{SourceId, Span};

pub(super) struct SemanticQueries<'a> {
    resolution: &'a Resolution,
}

impl<'a> SemanticQueries<'a> {
    pub(super) const fn new(resolution: &'a Resolution) -> Self {
        Self { resolution }
    }

    pub(super) fn symbol_at(&self, source: SourceId, offset: usize) -> Option<SymbolId> {
        self.resolution
            .references
            .iter()
            .filter(|reference| contains(reference.span, source, offset))
            .min_by_key(|reference| reference.span.end() - reference.span.start())
            .map(|reference| reference.symbol)
            .or_else(|| {
                self.resolution
                    .symbols
                    .iter()
                    .filter(|symbol| contains(symbol.span, source, offset))
                    .min_by_key(|symbol| symbol.span.end() - symbol.span.start())
                    .map(|symbol| symbol.id)
            })
    }

    pub(super) fn declaration(&self, id: SymbolId) -> &Symbol {
        &self.resolution.symbols[id.0]
    }

    pub(super) fn definition_span(&self, id: SymbolId) -> Span {
        let symbol = self.declaration(id);
        symbol.target_module.map_or(symbol.span, |module| {
            Span::new(self.resolution.modules[module.0].source, 0, 0)
        })
    }

    pub(super) fn references(&self, id: SymbolId) -> impl Iterator<Item = &ResolvedReference> {
        self.resolution
            .references
            .iter()
            .filter(move |reference| reference.symbol == id)
    }

    pub(super) fn workspace_symbols(&self, query: &str) -> impl Iterator<Item = &Symbol> {
        let query = query.to_lowercase();
        self.resolution.symbols.iter().filter(move |symbol| {
            is_workspace_symbol(symbol.kind) && symbol.name.to_lowercase().contains(&query)
        })
    }
}

fn contains(span: Span, source: SourceId, offset: usize) -> bool {
    span.source() == source && span.start() <= offset && offset <= span.end()
}

const fn is_workspace_symbol(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Module
            | SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Data
            | SymbolKind::Interface
            | SymbolKind::Enum
            | SymbolKind::TypeAlias
            | SymbolKind::Binding
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vut_lexer::Lexer;
    use vut_parser::Parser;
    use vut_resolver::{ModuleInput, ModulePath, Resolver};

    fn module(index: usize, path: &[&str], text: &str) -> ModuleInput {
        let source = SourceId::from_index(index);
        let (tokens, _) = Lexer::new(source, text).lex();
        let (file, _) = Parser::new(source, text, tokens).parse();
        ModuleInput {
            logical_path: ModulePath(path.iter().map(|part| (*part).into()).collect()),
            filesystem_path: None,
            file,
        }
    }

    #[test]
    fn resolves_symbols_and_references_across_modules() {
        let resolution = Resolver::new(vec![
            module(0, &["helper"], "fn answer() -> int:\n  42\n"),
            module(
                1,
                &["main"],
                "import helper\nfn main() -> int:\n  helper.answer()\n",
            ),
        ])
        .resolve();
        let queries = SemanticQueries::new(&resolution);
        let call = resolution
            .references
            .iter()
            .find(|reference| resolution.symbols[reference.symbol.0].name == "answer")
            .unwrap();
        let id = queries
            .symbol_at(call.span.source(), call.span.start())
            .unwrap();
        assert_eq!(queries.declaration(id).name, "answer");
        assert_eq!(
            queries.declaration(id).span.source(),
            SourceId::from_index(0)
        );
        assert_eq!(queries.references(id).count(), 1);
        assert_eq!(queries.workspace_symbols("ans").count(), 1);
    }
}
