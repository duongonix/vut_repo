//! Future frame layout for compiler-generated async state machines.
//!
//! Each async function owns one frame. The frame begins with a fixed header:
//!
//! ```text
//! offset 0   state: u32        resume state (0 = not started)
//! offset 4   flags: u32        initialized-slot bitmask (for cleanup)
//! offset 8   child: *mut void  in-flight awaited sub-future
//! offset 16  parameters...     declared parameters, in order
//!            completion...     the logical result slot
//!            locals...         values live across an `await`
//! ```
//!
//! Parameter/result/local offsets are computed here so the state-machine pass
//! and codegen agree without duplicating size/alignment rules.
use std::collections::HashMap;

use vut_hir::TypeId;

use super::super::LayoutTable;

/// Size of the fixed frame header (state, flags, child pointer).
pub const FRAME_HEADER_SIZE: usize = 16;
/// Offset of the resume-state word.
pub const FRAME_STATE_OFFSET: usize = 0;
/// Offset of the initialized-slot flags word.
pub const FRAME_FLAGS_OFFSET: usize = 4;
/// Offset of the in-flight child future pointer.
pub const FRAME_CHILD_OFFSET: usize = 8;

/// A typed slot inside a frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameSlot {
    pub offset: usize,
    pub ty: TypeId,
}

/// An untyped byte region inside a frame (e.g. an iterator state block).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameRegion {
    pub offset: usize,
    pub size: usize,
}

/// Layout of one async function's frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrameLayout {
    pub size: usize,
    pub align: usize,
    /// Declared parameters, in order.
    pub parameters: Vec<FrameSlot>,
    /// Logical result slot, when the async function returns a value.
    pub completion: Option<FrameSlot>,
    /// Slots for locals that live across an `await`, keyed by `LocalId`.
    pub locals: HashMap<usize, FrameSlot>,
    /// Untyped regions (iterator states), in allocation order.
    pub regions: Vec<FrameRegion>,
}

fn align_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}

/// Builds a frame layout from parameter types, an optional logical result type,
/// the set of locals that must survive a suspension, and untyped byte regions
/// (size, alignment) such as iterator states.
#[must_use]
pub fn build(
    layouts: &LayoutTable,
    parameters: &[TypeId],
    completion: Option<TypeId>,
    persisted: &[(usize, TypeId)],
    regions: &[(usize, usize)],
) -> FrameLayout {
    let mut offset = FRAME_HEADER_SIZE;
    let mut align = 8_usize;

    let place = |ty: TypeId, offset: &mut usize, align: &mut usize| FrameSlot {
        offset: {
            let info = layouts.types[ty.0];
            *offset = align_up(*offset, info.alignment.max(1));
            let slot = *offset;
            *offset = slot.saturating_add(info.size.max(1));
            *align = (*align).max(info.alignment.max(1));
            slot
        },
        ty,
    };

    let parameters = parameters
        .iter()
        .map(|ty| place(*ty, &mut offset, &mut align))
        .collect();
    let completion = completion.map(|ty| place(ty, &mut offset, &mut align));
    let mut locals = HashMap::new();
    for (local, ty) in persisted {
        let slot = place(*ty, &mut offset, &mut align);
        locals.insert(*local, slot);
    }
    let mut placed = Vec::with_capacity(regions.len());
    for (size, alignment) in regions {
        let alignment = (*alignment).max(1);
        offset = align_up(offset, alignment);
        let region = offset;
        offset = offset.saturating_add((*size).max(1));
        align = align.max(alignment);
        placed.push(FrameRegion {
            offset: region,
            size: *size,
        });
    }

    let size = align_up(offset, align);
    FrameLayout {
        size,
        align,
        parameters,
        completion,
        locals,
        regions: placed,
    }
}
