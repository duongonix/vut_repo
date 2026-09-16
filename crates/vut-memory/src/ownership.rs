//! Ownership transfer classification shared by MIR lowering.
//!
//! A value's ownership class determines how a *use* of it is lowered:
//!
//! ```text
//! Copy        cheap to duplicate (scalars, pointers, repr(C) data)
//! Retain      shared/reference-counted value; duplication retains
//! Structural  aggregate with drop glue; duplication is field-wise
//! Linear      move-only; duplication is rejected
//! ```
//!
//! Linear types (native resources, Vutcons) have exactly one owner. Reading
//! one that is not its last use is a compile error rather than a silent
//! reference-count operation.

/// How a value may be duplicated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Duplication {
    Copy,
    Retain,
    Structural,
    Linear,
    /// Untracked/placeholder types that never need duplication handling.
    Opaque,
}

/// The lowering action for reading a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transfer {
    Copy,
    RetainedCopy,
    Move,
}

/// Why a linear value could not be read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveError {
    /// The value was already moved (or awaited) earlier on this path.
    UseAfterMove,
    /// The value is still live and would need a second owner.
    NotDuplicable,
}

/// Selects the transfer for reading a value of the given duplication class.
///
/// A linear value is only transferable by its last use. Any other read is a
/// use-after-move/duplication error.
///
/// # Errors
/// Returns [`MoveError`] when a linear value would be duplicated or used after
/// it was already moved.
pub const fn transfer_for_use(
    duplication: Duplication,
    is_last_use: bool,
) -> Result<Transfer, MoveError> {
    match duplication {
        Duplication::Copy | Duplication::Opaque => Ok(Transfer::Copy),
        Duplication::Retain | Duplication::Structural => {
            if is_last_use {
                Ok(Transfer::Move)
            } else {
                Ok(Transfer::RetainedCopy)
            }
        }
        Duplication::Linear => {
            if is_last_use {
                Ok(Transfer::Move)
            } else {
                Err(MoveError::NotDuplicable)
            }
        }
    }
}
