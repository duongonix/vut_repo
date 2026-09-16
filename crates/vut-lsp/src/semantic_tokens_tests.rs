use super::{legend, tokens};
use tower_lsp::lsp_types::SemanticTokenType;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_resolver::{ModuleInput, ModulePath, Resolution, Resolver};
use vut_source::SourceId;

fn resolve(text: &str) -> Resolution {
    let source = SourceId::from_index(0);
    let (lexed, _) = Lexer::new(source, text).lex();
    let (file, _) = Parser::new(source, text, lexed).parse();
    Resolver::new(vec![ModuleInput {
        logical_path: ModulePath(vec!["main".into()]),
        filesystem_path: None,
        file,
    }])
    .resolve()
}

fn decoded(text: &str) -> Vec<(String, SemanticTokenType, bool)> {
    let resolution = resolve(text);
    let mut line = 0;
    let mut column = 0;
    let lines: Vec<_> = text.lines().collect();
    tokens(text, &resolution)
        .iter()
        .map(|token| {
            line += token.delta_line;
            column = if token.delta_line == 0 {
                column + token.delta_start
            } else {
                token.delta_start
            };
            let utf16: Vec<_> = lines[line as usize].encode_utf16().collect();
            let name =
                String::from_utf16(&utf16[column as usize..(column + token.length) as usize])
                    .unwrap();
            (
                name,
                legend().token_types[token.token_type as usize].clone(),
                token.token_modifiers_bitset == 1,
            )
        })
        .collect()
}

#[test]
fn functions_methods_parameters_and_types_keep_their_semantic_kinds() {
    let text = "data counter:\n  value: int\nfn counter.add(amount: int) -> int:\n  self.value + amount\nfn twice(value: int) -> int:\n  value + value\nx = counter(value = twice(2))\n";
    let highlighted = decoded(text);
    for (name, kind, declaration) in [
        ("counter", SemanticTokenType::STRUCT, true),
        ("counter", SemanticTokenType::STRUCT, false),
        ("add", SemanticTokenType::METHOD, true),
        ("amount", SemanticTokenType::PARAMETER, true),
        ("amount", SemanticTokenType::PARAMETER, false),
        ("twice", SemanticTokenType::FUNCTION, true),
        ("twice", SemanticTokenType::FUNCTION, false),
    ] {
        assert!(
            highlighted.contains(&(name.into(), kind, declaration)),
            "{highlighted:?}"
        );
    }
    assert!(!highlighted.iter().any(|(name, _, _)| name == "self"));
    assert!(!highlighted.contains(&("add".into(), SemanticTokenType::PARAMETER, true)));
}

#[test]
fn lexical_scopes_are_not_overwritten_by_semantic_fallbacks() {
    let text = "### documentation\n##\nmultiline comment 😀\n##\nMAX_SIZE = 10\nvalue = \"hello\\n\"\nitems = @(1, 2)\nitems.at(0)\nif true:\n  value\n";
    assert!(decoded(text).is_empty());
}

#[test]
fn interpolation_and_unicode_have_exact_identifier_ranges() {
    let text = "fn greet(name: str) -> str:\r\n  \"😀 tiếng Việt $name $(greet(name)) \\n\"\r\n";
    let highlighted = decoded(text);
    assert_eq!(
        highlighted
            .iter()
            .filter(|(name, _, _)| name == "name")
            .count(),
        3
    );
    assert_eq!(
        highlighted
            .iter()
            .filter(|(name, _, _)| name == "greet")
            .count(),
        2
    );
    assert!(highlighted.iter().all(|(name, _, _)| !name.contains('$')));
}

#[test]
fn aliases_interfaces_and_enums_are_not_generic_properties() {
    let highlighted = decoded(
        "type count = int\ninterface Reader:\n  read() -> int\nenum Status:\n  done\nfn read(value: count) -> count:\n  value\n",
    );
    for (name, kind) in [
        ("count", SemanticTokenType::TYPE),
        ("Reader", SemanticTokenType::INTERFACE),
        ("Status", SemanticTokenType::ENUM),
    ] {
        assert!(
            highlighted.contains(&(name.into(), kind, true)),
            "{highlighted:?}"
        );
    }
}

#[test]
fn incomplete_documents_produce_sorted_single_line_tokens_without_panicking() {
    for text in [
        "",
        "fn",
        "fn broken(x: int):\n  \"$(x",
        "type X =\nfn good(x: int):\n  x\n",
    ] {
        let _ = decoded(text);
    }
}
