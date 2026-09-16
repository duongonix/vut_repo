use std::collections::HashMap;

use vut_ast::{Attribute, AttributeArg, Item};
use vut_diagnostics::{Diagnostic, DiagnosticSink, codes};
use vut_resolver::{Module, Resolution, Symbol, SymbolId};
use vut_source::Span;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttributeTarget {
    Function,
    ExternFunction,
    Data,
    Interface,
    Enum,
    TypeAlias,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Repr {
    C,
    Transparent,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AttributeSemantics {
    pub data_repr: HashMap<SymbolId, Repr>,
    pub link_names: HashMap<SymbolId, String>,
}

#[derive(Clone, Copy)]
enum ArgSchema {
    Ident,
    String,
}

#[derive(Clone, Copy)]
struct Definition {
    name: &'static str,
    targets: &'static [AttributeTarget],
    args: ArgSchema,
    duplicates: bool,
}

const DEFINITIONS: &[Definition] = &[
    Definition {
        name: "repr",
        targets: &[AttributeTarget::Data],
        args: ArgSchema::Ident,
        duplicates: false,
    },
    Definition {
        name: "link_name",
        targets: &[AttributeTarget::ExternFunction],
        args: ArgSchema::String,
        duplicates: false,
    },
];

#[must_use]
pub fn validate(resolution: &Resolution) -> (AttributeSemantics, DiagnosticSink) {
    let mut diagnostics = DiagnosticSink::new();
    let mut semantics = AttributeSemantics::default();
    for module in &resolution.modules {
        for item in &module.file.items {
            let Some((target, symbol, attrs, target_span, field_count)) =
                item_parts(module, &resolution.symbols, item)
            else {
                continue;
            };
            let mut seen: HashMap<&str, Span> = HashMap::new();
            for attr in attrs {
                let Some(definition) = definition(&attr.name.text) else {
                    diagnostics.push(Diagnostic::error(
                        codes::E0111,
                        format!("unknown attribute `{}`", attr.name.text),
                        attr.name.span,
                        "attribute is not defined by this compiler",
                    ));
                    continue;
                };
                if !definition.targets.contains(&target) {
                    let code = if definition.name == "link_name" {
                        codes::E8006
                    } else {
                        codes::E0111
                    };
                    diagnostics.push(
                        Diagnostic::error(
                            code,
                            format!("invalid attribute target `{}`", attr.name.text),
                            attr.span,
                            format!(
                                "`{}` cannot be applied to {}",
                                attr.name.text,
                                target_name(target)
                            ),
                        )
                        .with_label(target_span, "attribute target"),
                    );
                    continue;
                }
                if !definition.duplicates
                    && let Some(previous) = seen.insert(definition.name, attr.span)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            codes::E0111,
                            format!("duplicate attribute `{}`", attr.name.text),
                            attr.span,
                            "attribute can only appear once on this declaration",
                        )
                        .with_related(previous, "previous attribute here"),
                    );
                    continue;
                }
                match (
                    definition.name,
                    check_args(definition, attr, &mut diagnostics),
                ) {
                    ("repr", Some(AttributeValue::Ident(value))) => match value.as_str() {
                        "C" => {
                            semantics.data_repr.insert(symbol, Repr::C);
                        }
                        "transparent" => {
                            if field_count != Some(1) {
                                diagnostics.push(Diagnostic::error(
                                    codes::E0111,
                                    "invalid repr value",
                                    attr.span,
                                    "`repr(transparent)` requires exactly one data field",
                                ));
                            }
                            semantics.data_repr.insert(symbol, Repr::Transparent);
                        }
                        _ => diagnostics.push(Diagnostic::error(
                            codes::E0111,
                            "invalid repr value",
                            attr.args.first().map_or(attr.span, AttributeArg::span),
                            "expected `C` or `transparent`",
                        )),
                    },
                    ("link_name", Some(AttributeValue::String(value))) => {
                        semantics.link_names.insert(symbol, value);
                    }
                    _ => {}
                }
            }
        }
    }
    (semantics, diagnostics)
}

enum AttributeValue {
    Ident(String),
    String(String),
}

fn check_args(
    definition: Definition,
    attr: &Attribute,
    diagnostics: &mut DiagnosticSink,
) -> Option<AttributeValue> {
    let code = if definition.name == "link_name" {
        codes::E8006
    } else {
        codes::E0111
    };
    match definition.args {
        ArgSchema::Ident if attr.args.len() == 1 => match &attr.args[0] {
            AttributeArg::Ident(name) => Some(AttributeValue::Ident(name.text.clone())),
            other => {
                diagnostics.push(Diagnostic::error(
                    code,
                    "invalid attribute argument type",
                    other.span(),
                    format!("`{}` requires an identifier argument", attr.name.text),
                ));
                None
            }
        },
        ArgSchema::String if attr.args.len() == 1 => match &attr.args[0] {
            AttributeArg::String { value, .. } => Some(AttributeValue::String(value.clone())),
            other => {
                diagnostics.push(Diagnostic::error(
                    code,
                    "invalid attribute argument type",
                    other.span(),
                    format!("`{}` requires a string literal", attr.name.text),
                ));
                None
            }
        },
        _ => {
            diagnostics.push(Diagnostic::error(
                code,
                "invalid number of attribute arguments",
                attr.span,
                format!("`{}` has the wrong number of arguments", attr.name.text),
            ));
            None
        }
    }
}

fn definition(name: &str) -> Option<Definition> {
    DEFINITIONS.iter().find(|item| item.name == name).copied()
}

type ItemAttributeParts<'a> = (
    AttributeTarget,
    SymbolId,
    &'a [Attribute],
    Span,
    Option<usize>,
);

fn item_parts<'a>(
    module: &'a Module,
    symbols: &'a [Symbol],
    item: &'a Item,
) -> Option<ItemAttributeParts<'a>> {
    Some(match item {
        Item::Function(value) => (
            AttributeTarget::Function,
            module.symbols.get(&value.name.text).copied()?,
            &value.attributes,
            value.span,
            None,
        ),
        Item::ExternFunction(value) => (
            AttributeTarget::ExternFunction,
            module.symbols.get(&value.name.text).copied()?,
            &value.attributes,
            value.span,
            None,
        ),
        Item::Data(value) => (
            AttributeTarget::Data,
            module.symbols.get(&value.name.text).copied()?,
            &value.attributes,
            value.span,
            Some(value.fields.len()),
        ),
        Item::Interface(value) => (
            AttributeTarget::Interface,
            module.symbols.get(&value.name.text).copied()?,
            &value.attributes,
            value.span,
            None,
        ),
        Item::Enum(value) => (
            AttributeTarget::Enum,
            module.symbols.get(&value.name.text).copied()?,
            &value.attributes,
            value.span,
            None,
        ),
        Item::TypeAlias(value) => (
            AttributeTarget::TypeAlias,
            module.symbols.get(&value.name.text).copied()?,
            &value.attributes,
            value.span,
            None,
        ),
        Item::Method(value) => (
            AttributeTarget::Function,
            module
                .methods
                .values()
                .find(|id| symbols[id.0].span == value.name.span)
                .copied()?,
            &value.attributes,
            value.span,
            None,
        ),
        Item::Import(_) | Item::Statement(_) => return None,
    })
}

fn target_name(target: AttributeTarget) -> &'static str {
    match target {
        AttributeTarget::Function => "a function",
        AttributeTarget::ExternFunction => "an extern function",
        AttributeTarget::Data => "a data declaration",
        AttributeTarget::Interface => "an interface",
        AttributeTarget::Enum => "an enum",
        AttributeTarget::TypeAlias => "a type alias",
    }
}
