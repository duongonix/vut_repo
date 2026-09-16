use crate::Satisfaction;
use std::collections::HashMap;
use vut_hir::TypeId;
use vut_resolver::SymbolId;

#[derive(Clone, Debug, Default)]
pub struct SatisfactionCache(HashMap<(TypeId, SymbolId), Satisfaction>);
impl SatisfactionCache {
    #[must_use]
    pub fn get(&self, ty: TypeId, interface: SymbolId) -> Option<&Satisfaction> {
        self.0.get(&(ty, interface))
    }
    pub fn insert(&mut self, ty: TypeId, interface: SymbolId, result: Satisfaction) {
        self.0.insert((ty, interface), result);
    }
    #[must_use]
    pub fn into_inner(self) -> HashMap<(TypeId, SymbolId), Satisfaction> {
        self.0
    }
}
