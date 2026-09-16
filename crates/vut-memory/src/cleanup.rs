use vut_hir::TypeId;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitKind {
    Scope,
    Return,
    Break,
    Continue,
    Error,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupPlan {
    pub exit: ExitKind,
    pub locals: Vec<usize>,
}
impl CleanupPlan {
    #[must_use]
    pub fn for_scope(exit: ExitKind, locals: &[(TypeId, bool, bool)], start: usize) -> Self {
        let locals = locals
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .filter_map(|(index, (_, needs_drop, moved))| (*needs_drop && !*moved).then_some(index))
            .collect();
        Self { exit, locals }
    }
}
