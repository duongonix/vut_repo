//! Resolve nominal display parameter names through ordinary module visibility.
use vut_resolver::{Module, Resolution, SymbolId};
use vut_types::{SemanticResult, Type};

pub(super) fn visible(
    symbol: SymbolId,
    module: &Module,
    semantics: &SemanticResult,
    resolution: &Resolution,
) -> String {
    let matches = |candidate: SymbolId| {
        candidate == symbol || semantics.type_aliases.get(&candidate).is_some_and(|ty| {
            matches!(semantics.types[ty.0], Type::Data(owner) | Type::Enum(owner) if owner == symbol)
        })
    };
    let mut local: Vec<_> = module
        .symbols
        .iter()
        .filter(|(_, candidate)| matches(**candidate))
        .collect();
    local.sort_by_key(|(name, _)| *name);
    if let Some((name, _)) = local.first() {
        return (*name).clone();
    }
    let mut imports: Vec<_> = module.imports.iter().collect();
    imports.sort_by_key(|(name, _)| *name);
    for (alias, imported) in imports {
        if matches(*imported) {
            return alias.clone();
        }
        if let Some(target) = resolution.symbols[imported.0].target_module {
            let mut names: Vec<_> = resolution.modules[target.0]
                .symbols
                .iter()
                .filter(|(_, candidate)| {
                    resolution.symbols[candidate.0].public && matches(**candidate)
                })
                .collect();
            names.sort_by_key(|(name, _)| *name);
            if let Some((name, _)) = names.first() {
                return format!("{alias}.{name}");
            }
        }
    }
    resolution.symbols[symbol.0].name.clone()
}
