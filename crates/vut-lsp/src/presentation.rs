//! Canonical Vut-facing type and declaration presentation.

use vut_hir::TypeId;
use vut_resolver::{SymbolId, SymbolKind};
use vut_types::Type;

use super::Analysis;

pub(super) fn symbol_signature(a: &Analysis, id: SymbolId) -> String {
    let symbol = &a.resolution.symbols[id.0];
    if let Some(source) = a.sources.get(&symbol.span.source()) {
        let start = source.text[..symbol.span.start()]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let end = source.text[symbol.span.end()..]
            .find('\n')
            .map_or(source.text.len(), |index| symbol.span.end() + index);
        let line = source.text[start..end].trim();
        if !line.is_empty() {
            return line.trim_end_matches(':').to_owned();
        }
    }
    let keyword = match symbol.kind {
        SymbolKind::Function | SymbolKind::Method | SymbolKind::AnonymousFunction => "fn",
        SymbolKind::Data => "data",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::TypeAlias | SymbolKind::TypeParameter => "type",
        _ => "let",
    };
    if let Some(signature) = a.semantics.function_signatures.get(&id) {
        let parameters = signature
            .parameters
            .iter()
            .map(|ty| render_type(a, *ty))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{keyword} {}({parameters}) -> {}",
            symbol.name,
            render_type(a, signature.result)
        )
    } else {
        format!("{keyword} {}", symbol.name)
    }
}

pub(super) fn render_type(a: &Analysis, id: TypeId) -> String {
    match &a.semantics.types[id.0] {
        Type::Error => "<error>".into(),
        Type::Void => "void".into(),
        Type::Unit => "unit".into(),
        Type::Bool => "bool".into(),
        Type::Int => "int".into(),
        Type::Float => "float".into(),
        Type::Str => "str".into(),
        Type::Bytes => "bytes".into(),
        Type::Dyn => "dyn".into(),
        Type::Null => "null".into(),
        Type::SelfType => "Self".into(),
        Type::Numeric(name) => name.clone(),
        Type::Param(symbol)
        | Type::Data(symbol)
        | Type::Enum(symbol)
        | Type::Interface(symbol)
        | Type::Function(symbol) => a.resolution.symbols[symbol.0].name.clone(),
        Type::Optional(inner) => format!("{}?", render_type(a, *inner)),
        Type::Result(ok, error) => {
            format!(
                "result[{}, {}]",
                render_type(a, *ok),
                render_type(a, *error)
            )
        }
        Type::Vutcon(inner) => format!("vutcon[{}]", render_type(a, *inner)),
        Type::Channel(inner) => format!("channel[{}]", render_type(a, *inner)),
        Type::Future(inner) => format!("future[{}]", render_type(a, *inner)),
        Type::Resource(inner) => format!("resource[{}]", render_type(a, *inner)),
        Type::List(inner) => format!("list[{}]", render_type(a, *inner)),
        Type::Variadic(inner) => format!("...{}", render_type(a, *inner)),
        Type::Pointer(inner) => format!("ptr[{}]", render_type(a, *inner)),
        Type::Array(inner, length) => format!("array[{}, {length}]", render_type(a, *inner)),
        Type::Map(key, value) => {
            format!("map[{}, {}]", render_type(a, *key), render_type(a, *value))
        }
        Type::Applied(symbol, arguments) => format!(
            "{}[{}]",
            a.resolution.symbols[symbol.0].name,
            arguments
                .iter()
                .map(|argument| render_type(a, *argument))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Callable {
            receiver,
            parameters,
            result,
        } => {
            let receiver = receiver
                .map(|receiver| format!("({})", render_type(a, receiver)))
                .unwrap_or_default();
            let parameters = parameters
                .iter()
                .map(|parameter| render_type(a, *parameter))
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn{receiver}({parameters}) -> {}", render_type(a, *result))
        }
        Type::FunctionPointer {
            abi,
            parameters,
            result,
        } => format!(
            "extern \"{abi}\" fn({}) -> {}",
            parameters
                .iter()
                .map(|parameter| render_type(a, *parameter))
                .collect::<Vec<_>>()
                .join(", "),
            render_type(a, *result)
        ),
        Type::Range(inner) => format!("range({})", render_type(a, *inner)),
    }
}
