//! Deterministic scope cleanup and terminated-path bookkeeping.
use super::{Builder, Instruction, LocalId, ValueId};

impl Builder<'_> {
    pub(in crate::lowering) fn cleanup_except(&mut self, returned: Option<ValueId>) {
        let _ = returned;
        self.cleanup_from(0);
    }
    pub(super) fn cleanup_from(&mut self, start: usize) {
        let drops: Vec<_> = self
            .local_data
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .filter_map(|(index, local)| {
                let id = LocalId(index);
                // A receiver is borrowed by the caller and must never be
                // released by the callee's cleanup.
                if self.receiver_local == Some(id) {
                    return None;
                }
                // A closure capture aliases a value owned by the environment.
                if self.borrowed_locals.contains(&id) {
                    return None;
                }
                (!self.moved.contains(&id)
                    && local
                        .ty
                        .is_some_and(|ty| self.layouts.types[ty.0].needs_drop))
                .then_some(id)
            })
            .collect();
        for local in drops {
            self.emit(Instruction::Drop(local));
        }
    }
    /// Drops the locals declared at or after `start` on the current path and
    /// marks them consumed. Used to close a scoped arm/branch so its bindings
    /// cannot be dropped again by an outer cleanup (path-insensitive move
    /// tracking otherwise treats them as still live).
    pub(super) fn close_scope(&mut self, start: usize) {
        self.cleanup_from(start);
        self.forget_scope(start);
    }
    /// A terminating branch has already emitted cleanup. Its scoped locals
    /// must not participate in cleanup on a sibling or the continuing path.
    pub(super) fn forget_scope(&mut self, start: usize) {
        for index in start..self.local_data.len() {
            self.moved.insert(LocalId(index));
        }
    }
}
