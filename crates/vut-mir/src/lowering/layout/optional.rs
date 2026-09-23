//! Tagged optional storage, resolved together with enclosing aggregates.
use super::{AbiClass, OwnershipKind, TypeInfo, ValueRepr, align_up};

pub(super) fn tagged(inner: TypeInfo, pointer_size: usize) -> TypeInfo {
    let alignment = pointer_size.max(inner.alignment).max(1);
    let payload_offset = align_up(pointer_size, inner.alignment.max(1));
    TypeInfo {
        size: align_up(payload_offset + inner.size, alignment),
        alignment,
        is_copy: inner.is_copy,
        needs_drop: inner.needs_drop,
        contains_managed: inner.contains_managed,
        contains_linear: inner.contains_linear,
        abi: AbiClass::Aggregate,
        repr: ValueRepr::Pointer,
        ownership: if inner.needs_drop {
            OwnershipKind::Aggregate
        } else {
            OwnershipKind::None
        },
    }
}
