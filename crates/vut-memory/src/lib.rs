//! Ownership and deterministic-cleanup analysis for MIR lowering.
mod cleanup;
mod escape;
mod last_use;
mod ownership;
pub use cleanup::{CleanupPlan, ExitKind};
pub use escape::{EscapeClass, EscapeFacts};
pub use last_use::LastUse;
pub use ownership::{Duplication, MoveError, Transfer, transfer_for_use};

#[cfg(test)]
mod tests {
    use super::*;
    use vut_hir::TypeId;

    #[test]
    fn ownership_transfer_distinguishes_copy_retain_move_and_linear() {
        assert_eq!(
            transfer_for_use(Duplication::Copy, false),
            Ok(Transfer::Copy)
        );
        assert_eq!(
            transfer_for_use(Duplication::Retain, false),
            Ok(Transfer::RetainedCopy)
        );
        assert_eq!(
            transfer_for_use(Duplication::Retain, true),
            Ok(Transfer::Move)
        );
        assert_eq!(
            transfer_for_use(Duplication::Structural, false),
            Ok(Transfer::RetainedCopy)
        );
        assert_eq!(
            transfer_for_use(Duplication::Linear, true),
            Ok(Transfer::Move)
        );
        assert_eq!(
            transfer_for_use(Duplication::Linear, false),
            Err(MoveError::NotDuplicable)
        );
    }

    #[test]
    fn cleanup_is_reverse_order_and_skips_moved_values() {
        let locals = [
            (TypeId(0), true, false),
            (TypeId(1), false, false),
            (TypeId(2), true, true),
            (TypeId(3), true, false),
        ];
        assert_eq!(
            CleanupPlan::for_scope(ExitKind::Return, &locals, 0).locals,
            vec![3, 0]
        );
    }

    #[test]
    fn escape_facts_default_local_and_mark_stored_values() {
        let mut facts = EscapeFacts::default();
        assert_eq!(facts.class(4), EscapeClass::Local);
        facts.mark(4);
        assert_eq!(facts.class(4), EscapeClass::Stored);
    }
}
