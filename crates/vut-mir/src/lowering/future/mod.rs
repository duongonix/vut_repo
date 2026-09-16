//! Async lowering: future frame layout, suspension liveness, and the
//! frame-based async body convention.
//!
//! This pass runs after ordinary MIR lowering. For every `async fn` it:
//!
//! 1. computes the layout of its future frame (B2.2),
//! 2. records the locals that must survive each `await` (B2.3),
//! 3. rewrites the body so declared parameters are read from the frame instead
//!    of being passed as registers (B2.6a),
//! 4. spills values and iterator state that must survive a suspension into the
//!    frame, then turns bodies into a suspendable poll state machine (B2.6b).
//!
//! The frame-based convention is the prerequisite for real suspension: on
//! resume the body is re-entered with only the frame pointer, so every value it
//! still needs must live in the frame.
use std::collections::{HashMap, HashSet};

use vut_hir::TypeId;
use vut_source::{SourceId, Span};

use super::ValueRepr;
use super::{
    AbiClass, Function, Instruction, Local, LocalId, LocalStorage, OwnershipKind, Program,
    TypeInfo, ValueId,
};
pub use frame::{FRAME_CHILD_OFFSET, FRAME_HEADER_SIZE, FRAME_STATE_OFFSET, FrameLayout};
pub use liveness::AwaitLive;

pub mod frame;
pub mod liveness;
pub mod machine;
pub mod spill;
pub mod values;

/// Size of the compiler-generated iterator state block (`data`, `current`,
/// `len`, `stride`).
const ITERATOR_STATE_SIZE: usize = 32;
/// Bytes reserved per spilled value (a machine word, plus slack).
const SPILL_SLOT_SIZE: usize = 16;

/// Computes frame layouts, per-await liveness, and the frame-based body layout
/// for all async functions.
#[expect(
    clippy::too_many_lines,
    reason = "frame planning and body rewriting are one coherent async pass"
)]
pub fn lower(program: &mut Program) {
    // A raw pointer type is used for the frame-pointer local and for untyped
    // (raw 64-bit) locals that must survive a suspension.
    let pointer = pointer_type(program);
    let mut frames = HashMap::new();
    let mut awaits = HashMap::new();
    let mut iterator_slots: HashMap<vut_resolver::SymbolId, Vec<usize>> = HashMap::new();
    let mut spill_plans: HashMap<vut_resolver::SymbolId, HashMap<ValueId, usize>> = HashMap::new();
    for function in &program.functions {
        if !function.is_async {
            continue;
        }
        let mut live = liveness::analyze(function);
        let value_live = values::live_across(function);
        // Values living across a suspension are counted for diagnostics, then
        // spilled below so the state-machine pass sees none.
        let mut crossing: Vec<ValueId> = value_live.values().flatten().copied().collect();
        crossing.sort_by_key(|value| value.0);
        crossing.dedup();
        for site in &mut live {
            site.crossing_values = value_live
                .get(&(site.block, site.index))
                .map_or(0, Vec::len);
        }

        // Every local live across at least one await needs a frame slot.
        // Declared parameters are already frame-resident, so they are excluded
        // here to avoid a second, conflicting slot. Untyped locals (loop
        // iterators) are treated as pointer-sized words.
        let mut persisted: Vec<(usize, TypeId)> = Vec::new();
        let mut seen: HashSet<usize> = HashSet::new();
        for site in &live {
            for &local in &site.locals {
                if local >= function.parameter_count && seen.insert(local) {
                    let ty = function
                        .locals
                        .get(local)
                        .and_then(|local| local.ty)
                        .unwrap_or(pointer);
                    persisted.push((local, ty));
                }
            }
        }

        // Every iterator state must live in the frame so it survives a
        // suspension; every spilled value needs its own frame region.
        let iterator_count = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|instruction| matches!(instruction, Instruction::IteratorInit { .. }))
            .count();
        let mut regions: Vec<(usize, usize)> = vec![(SPILL_SLOT_SIZE, 8); crossing.len()];
        regions.extend((0..iterator_count).map(|_| (ITERATOR_STATE_SIZE, 8)));

        let parameters: Vec<TypeId> = function
            .locals
            .iter()
            .take(function.parameter_count)
            .filter_map(|local| local.ty)
            .collect();
        let completion = function
            .return_type
            .filter(|ty| program.layouts.types[ty.0].repr != ValueRepr::Void);
        let layout = frame::build(
            &program.layouts,
            &parameters,
            completion,
            &persisted,
            &regions,
        );
        let spills = crossing
            .iter()
            .enumerate()
            .map(|(index, &value)| (value, layout.regions[index].offset))
            .collect();
        iterator_slots.insert(
            function.symbol,
            layout.regions[crossing.len()..]
                .iter()
                .map(|region| region.offset)
                .collect(),
        );
        spill_plans.insert(function.symbol, spills);
        frames.insert(function.symbol, layout);
        awaits.insert(function.symbol, live);
    }

    for function in &mut program.functions {
        if !function.is_async {
            continue;
        }
        let Some(layout) = frames.get(&function.symbol) else {
            continue;
        };
        make_frame_based(function, layout, pointer);
        machine::apply_layout(function, layout);
        if let Some(offsets) = iterator_slots.get(&function.symbol) {
            let mut next = offsets.iter();
            for block in &mut function.blocks {
                for instruction in &mut block.instructions {
                    if let Instruction::IteratorInit { slot, .. } = instruction {
                        *slot = next.next().copied();
                    }
                }
            }
        }
        if let Some(spills) = spill_plans.get(&function.symbol) {
            spill::spill_values(function, spills);
        }
        if let Some(sites) = awaits.get(&function.symbol) {
            machine::make_poll_body(function, sites, pointer, true);
        }
    }

    program.frames = frames;
    program.awaits = awaits;
}

/// Appends a raw pointer type used for the frame-pointer local.
fn pointer_type(program: &mut Program) -> TypeId {
    let size = program.layouts.pointer_size;
    let id = TypeId(program.layouts.types.len());
    program.layouts.types.push(TypeInfo {
        size,
        alignment: size,
        is_copy: true,
        needs_drop: false,
        contains_managed: false,
        contains_linear: false,
        abi: AbiClass::Scalar,
        repr: ValueRepr::Pointer,
        ownership: OwnershipKind::None,
    });
    id
}

/// Rewrites an async body to take only a frame pointer, making declared
/// parameters frame-resident.
fn make_frame_based(function: &mut Function, layout: &FrameLayout, frame_ty: TypeId) {
    let parameter_count = function.parameter_count;
    let span = function.locals.first().map_or_else(
        || Span::new(SourceId::from_index(0), 0, 0),
        |local| local.span,
    );
    let frame_local = LocalId(function.locals.len());
    function.locals.push(Local {
        name: "$frame".to_string(),
        ty: Some(frame_ty),
        span,
        storage: LocalStorage::Stack,
    });
    function.frame_param = Some(frame_local.0);
    function.parameter_count = 0;

    for (index, slot) in layout.parameters.iter().enumerate() {
        if index >= parameter_count {
            break;
        }
        function.locals[index].storage = LocalStorage::Frame(slot.offset);
    }
}
