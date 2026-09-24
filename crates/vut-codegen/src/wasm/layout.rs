//! Target-layout helpers for the wasm backend.
use target_lexicon::{Architecture, Triple};
use vut_hir::TypeId;
use vut_mir::{Function, LayoutTable, ValueRepr};
use wasm_encoder::ValType;

/// Whether `triple` is a 32/64-bit WebAssembly target.
#[must_use]
pub fn is_wasm_target(triple: &str) -> bool {
    triple.parse::<Triple>().is_ok_and(|triple| {
        matches!(
            triple.architecture,
            Architecture::Wasm32 | Architecture::Wasm64
        )
    })
}

/// Maps a semantic type to its wasm value type via the MIR layout.
///
/// `int` is 64-bit (`i64`); pointers and `usize`/`isize` follow the target
/// pointer width (`i32` on wasm32); `float` is `f64`.
#[must_use]
pub fn valtype(layouts: &LayoutTable, ty: TypeId) -> ValType {
    let info = layouts.types[ty.0];
    match info.repr {
        ValueRepr::Float if info.size == 4 => ValType::F32,
        ValueRepr::Float => ValType::F64,
        ValueRepr::Void => ValType::I32,
        // Pointers/handles follow the target pointer width (authoritative from
        // the layout table); integers are 64-bit `int`.
        ValueRepr::Pointer => {
            if layouts.pointer_size <= 4 {
                ValType::I32
            } else {
                ValType::I64
            }
        }
        ValueRepr::Integer => {
            if info.size <= 4 {
                ValType::I32
            } else {
                ValType::I64
            }
        }
    }
}

/// The wasm signature `(params, result)` of a MIR function.
///
/// Async bodies take a single frame pointer (or `(frame, out)` for a poll
/// state-machine entry); declared parameters are read from frame slots.
#[must_use]
pub fn wasm_signature(
    layouts: &LayoutTable,
    function: &Function,
) -> (Vec<ValType>, Option<ValType>) {
    if function.is_poll {
        // State-machine poll entry: `(frame, out) -> i32`.
        return (vec![ValType::I32, ValType::I32], Some(ValType::I32));
    }
    let params = if function.frame_param.is_some() {
        // Async body: a single frame-pointer parameter.
        vec![ValType::I32]
    } else {
        function
            .locals
            .iter()
            .take(function.parameter_count)
            .map(|local| local.ty.map_or(ValType::I32, |ty| valtype(layouts, ty)))
            .collect()
    };
    let result = function.return_type.and_then(|ty| {
        if matches!(layouts.types[ty.0].repr, ValueRepr::Void) {
            None
        } else {
            Some(valtype(layouts, ty))
        }
    });
    (params, result)
}
