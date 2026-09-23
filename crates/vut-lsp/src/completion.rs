//! Scope- and receiver-aware completion backed by compiler semantics.

use std::collections::HashSet;

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};
use vut_resolver::{SymbolId, SymbolKind};
use vut_types::{Type, builtin_methods};

use super::{
    Analysis, completion_kind,
    members::{member_context, receiver_kind_at, receiver_type_at},
    presentation::{render_type, symbol_signature},
    query::SemanticQueries,
};

pub(super) fn items(a: &Analysis, text: &str, at: usize) -> Vec<CompletionItem> {
    let mut items = if let Some((dot, partial)) = member_context(text, at) {
        member_items(a, dot, &partial)
    } else {
        scope_items(a, at)
    };
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(item.label.clone()));
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

fn scope_items(a: &Analysis, at: usize) -> Vec<CompletionItem> {
    let Some(module) = a
        .resolution
        .modules
        .iter()
        .find(|module| module.source == a.current_source)
    else {
        return Vec::new();
    };
    module
        .symbols
        .values()
        .chain(module.imports.values())
        .copied()
        .chain(
            a.resolution
                .symbols
                .iter()
                .filter(|symbol| {
                    symbol.module == module.id
                        && matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::Local)
                        && symbol.span.start() <= at
                })
                .map(|symbol| symbol.id),
        )
        .map(|id| symbol_item(a, id))
        .collect()
}

fn member_items(a: &Analysis, dot: usize, partial: &str) -> Vec<CompletionItem> {
    if let Some(module) = module_receiver(a, dot) {
        return a.resolution.modules[module.0]
            .symbols
            .values()
            .map(|id| &a.resolution.symbols[id.0])
            .filter(|symbol| symbol.public && symbol.name.starts_with(partial))
            .map(|symbol| symbol_item(a, symbol.id))
            .collect();
    }
    if let Some(owner) = type_receiver(a, dot) {
        return a
            .semantics
            .method_symbols
            .iter()
            .filter(|((receiver, name), id)| {
                *receiver == owner
                    && name.starts_with(partial)
                    && a.semantics.static_methods.contains(id)
            })
            .map(|(_, id)| symbol_item(a, *id))
            .collect();
    }

    let mut result = Vec::new();
    if let Some(kind) = receiver_kind_at(a, dot) {
        result.extend(
            builtin_methods(kind)
                .iter()
                .filter(|(name, _)| name.starts_with(partial))
                .map(|(name, signature)| CompletionItem {
                    label: (*name).to_owned(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some((*signature).to_owned()),
                    ..CompletionItem::default()
                }),
        );
    }
    let Some(receiver) = receiver_type_at(a, dot).and_then(|ty| nominal_symbol(a, ty)) else {
        return result;
    };
    if let Some(fields) = a.semantics.data_fields.get(&receiver) {
        result.extend(
            fields
                .iter()
                .filter(|field| field.public && field.name.starts_with(partial))
                .map(|field| CompletionItem {
                    label: field.name.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(render_type(a, field.ty)),
                    ..CompletionItem::default()
                }),
        );
    }
    result.extend(
        a.semantics
            .method_symbols
            .iter()
            .filter(|((owner, name), _)| *owner == receiver && name.starts_with(partial))
            .map(|(_, id)| symbol_item(a, *id)),
    );
    result
}

fn module_receiver(a: &Analysis, dot: usize) -> Option<vut_resolver::ModuleId> {
    let id = receiver_symbol(a, dot)?;
    let symbol = &a.resolution.symbols[id.0];
    (symbol.kind == SymbolKind::Module)
        .then_some(symbol.target_module)
        .flatten()
}

fn type_receiver(a: &Analysis, dot: usize) -> Option<SymbolId> {
    let id = receiver_symbol(a, dot)?;
    matches!(
        a.resolution.symbols[id.0].kind,
        SymbolKind::Data | SymbolKind::Enum | SymbolKind::Interface
    )
    .then_some(id)
}

fn receiver_symbol(a: &Analysis, dot: usize) -> Option<SymbolId> {
    SemanticQueries::new(&a.resolution).symbol_at(a.current_source, dot.saturating_sub(1))
}

fn nominal_symbol(a: &Analysis, ty: vut_hir::TypeId) -> Option<SymbolId> {
    match &a.semantics.types[ty.0] {
        Type::Data(symbol)
        | Type::Enum(symbol)
        | Type::Interface(symbol)
        | Type::Applied(symbol, _) => Some(*symbol),
        _ => None,
    }
}

fn symbol_item(a: &Analysis, id: SymbolId) -> CompletionItem {
    let symbol = &a.resolution.symbols[id.0];
    CompletionItem {
        label: symbol.name.clone(),
        kind: Some(completion_kind(symbol.kind)),
        detail: Some(symbol_signature(a, id)),
        ..CompletionItem::default()
    }
}
