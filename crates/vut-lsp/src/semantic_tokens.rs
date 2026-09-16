//! Semantic highlighting only refines identifiers known to the resolver.
//!
//! `TextMate` owns lexical scopes (keywords, literals, escapes and interpolation).
//! Emitting a generic property for every identifier or a string over an escape
//! would override those more precise scopes in VS Code.

use std::collections::HashMap;
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend,
};
use vut_lexer::{Lexer, TokenKind};
use vut_resolver::{Resolution, Symbol, SymbolKind};
use vut_source::{SourceId, Span};

#[derive(Clone, Copy)]
#[repr(u32)]
enum Kind {
    Namespace,
    Type,
    Struct,
    Interface,
    Enum,
    Function,
    Method,
    Parameter,
    TypeParameter,
}

impl Kind {
    const ALL: [Self; 9] = [
        Self::Namespace,
        Self::Type,
        Self::Struct,
        Self::Interface,
        Self::Enum,
        Self::Function,
        Self::Method,
        Self::Parameter,
        Self::TypeParameter,
    ];

    fn token_type(self) -> SemanticTokenType {
        match self {
            Self::Namespace => SemanticTokenType::NAMESPACE,
            Self::Type => SemanticTokenType::TYPE,
            Self::Struct => SemanticTokenType::STRUCT,
            Self::Interface => SemanticTokenType::INTERFACE,
            Self::Enum => SemanticTokenType::ENUM,
            Self::Function => SemanticTokenType::FUNCTION,
            Self::Method => SemanticTokenType::METHOD,
            Self::Parameter => SemanticTokenType::PARAMETER,
            Self::TypeParameter => SemanticTokenType::TYPE_PARAMETER,
        }
    }
}

pub(super) fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: Kind::ALL.into_iter().map(Kind::token_type).collect(),
        token_modifiers: vec![SemanticTokenModifier::DECLARATION],
    }
}

fn classification(symbol: &Symbol) -> Option<Kind> {
    // The implicit receiver has a synthetic declaration span on the method name.
    if symbol.name == "self" {
        return None;
    }
    Some(match symbol.kind {
        SymbolKind::Module => Kind::Namespace,
        SymbolKind::Data => Kind::Struct,
        SymbolKind::Interface => Kind::Interface,
        SymbolKind::Enum => Kind::Enum,
        SymbolKind::TypeAlias => Kind::Type,
        SymbolKind::TypeParameter => Kind::TypeParameter,
        SymbolKind::Function => Kind::Function,
        SymbolKind::Method => Kind::Method,
        SymbolKind::Parameter => Kind::Parameter,
        // Keep syntax scopes for constants, callable bindings, and unknown names.
        SymbolKind::Binding | SymbolKind::Local | SymbolKind::AnonymousFunction => return None,
    })
}

pub(super) fn tokens(text: &str, resolution: &Resolution) -> Vec<SemanticToken> {
    let mut classified = HashMap::new();
    for reference in &resolution.references {
        if let Some(kind) = classification(&resolution.symbols[reference.symbol.0]) {
            classified.insert(reference.span, (kind, false));
        }
    }
    for symbol in &resolution.symbols {
        if let Some(kind) = classification(symbol)
            && text.get(symbol.span.start()..symbol.span.end()) == Some(symbol.name.as_str())
        {
            classified.insert(symbol.span, (kind, true));
        }
    }

    let source = SourceId::from_index(0);
    let (lexed, _) = Lexer::new(source, text).lex();
    let mut encoder = Encoder::default();
    lexed
        .into_iter()
        .filter_map(|token| {
            let span = match token.kind {
                TokenKind::Identifier => token.span,
                TokenKind::InterpolationIdentifier => {
                    Span::new(source, token.span.start() + 1, token.span.end())
                }
                _ => return None,
            };
            let &(kind, declaration) = classified.get(&span)?;
            Some(encoder.encode(text, span, kind, declaration))
        })
        .collect()
}

/// Encodes sorted identifier spans in one pass, using UTF-16 LSP columns.
#[derive(Default)]
struct Encoder {
    cursor: usize,
    line: u32,
    column: u32,
    last_line: u32,
    last_column: u32,
}

impl Encoder {
    fn encode(&mut self, text: &str, span: Span, kind: Kind, declaration: bool) -> SemanticToken {
        for character in text[self.cursor..span.start()].chars() {
            if character == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += character.len_utf16() as u32;
            }
        }
        self.cursor = span.start();
        let delta_line = self.line - self.last_line;
        let result = SemanticToken {
            delta_line,
            delta_start: if delta_line == 0 {
                self.column - self.last_column
            } else {
                self.column
            },
            length: text[span.start()..span.end()].encode_utf16().count() as u32,
            token_type: kind as u32,
            token_modifiers_bitset: u32::from(declaration),
        };
        self.last_line = self.line;
        self.last_column = self.column;
        result
    }
}

#[cfg(test)]
#[path = "semantic_tokens_tests.rs"]
mod tests;
