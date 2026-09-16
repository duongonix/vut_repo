//! Heap frames for compiler-generated async state machines.
//!
//! A frame is a zero-initialized, alignment-correct block owned by exactly one
//! future. The runtime only allocates and frees it; the compiler lays out its
//! contents (state, init flags, child slot, parameter/result/persisted slots).
//!
//! Allocation is zeroed so the initial poll state (`0`) and every init flag
//! start clean without the compiler emitting explicit initialization.
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE_FRAMES: AtomicUsize = AtomicUsize::new(0);

/// Returns the number of live frames.
#[must_use]
pub fn live_frames() -> usize {
    LIVE_FRAMES.load(Ordering::Relaxed)
}

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size.max(1), align.max(1)).expect("vut async frame layout")
}

/// Allocates a zero-initialized frame.
///
/// # Safety
/// `size`/`align` must describe a valid layout for the generated frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_frame_alloc_v1(size: usize, align: usize) -> *mut u8 {
    // SAFETY: the layout is valid because `Layout::from_size_align` succeeded.
    let pointer = unsafe { alloc_zeroed(layout(size, align)) };
    if pointer.is_null() {
        return pointer;
    }
    LIVE_FRAMES.fetch_add(1, Ordering::Relaxed);
    pointer
}

/// Frees a frame previously returned by [`vut_rt_frame_alloc_v1`].
///
/// # Safety
/// `pointer` must be null or a live frame with the same `size`/`align`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_frame_free_v1(pointer: *mut u8, size: usize, align: usize) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: the pointer came from `vut_rt_frame_alloc_v1` with this layout.
    unsafe { dealloc(pointer, layout(size, align)) };
    LIVE_FRAMES.fetch_sub(1, Ordering::Relaxed);
}

/// Returns the number of live frames.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_frame_live_count_v1() -> usize {
    live_frames()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_zeroed_aligned_and_released() {
        let before = live_frames();
        // SAFETY: the layout is valid and the frame is freed exactly once.
        unsafe {
            let frame = vut_rt_frame_alloc_v1(24, 8);
            assert!(!frame.is_null());
            assert_eq!(frame as usize % 8, 0);
            assert_eq!(live_frames(), before + 1);
            assert!((0..24).all(|offset| *frame.add(offset) == 0));
            vut_rt_frame_free_v1(frame, 24, 8);
        }
        assert_eq!(live_frames(), before);
    }
}
