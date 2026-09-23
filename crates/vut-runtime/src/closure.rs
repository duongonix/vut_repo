//! Reference-counted closure environments.
//!
//! A capturing closure value is a tagged pointer (`block | (1 << 63)`). The
//! block layout is:
//!
//! ```text
//! +0  refcount: u64
//! +8  size:     u64   (total block size in bytes)
//! +16 code:     u64   (body code pointer)
//! +24 drop:     u64   (capture drop thunk, or 0 for a copy-only environment)
//! +32 environment: capture bytes
//! ```
//!
//! A non-capturing function value is an untagged code pointer. Retain/release
//! are no-ops for untagged values, so ordinary function values stay zero-cost.
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::sync::atomic::{AtomicUsize, Ordering};

const HEADER: usize = 32;
/// High bit used to tag a capturing closure pointer. User-space code and heap
/// addresses never use it on supported targets.
const TAG: usize = 1 << (usize::BITS - 1);
/// A compiler-generated thunk that releases a closure's managed captures.
type ClosureDropFn = unsafe extern "C" fn(*mut u8);
static LIVE_CLOSURES: AtomicUsize = AtomicUsize::new(0);

/// Returns the number of live closure environments.
#[must_use]
pub fn live_closures() -> usize {
    LIVE_CLOSURES.load(Ordering::Relaxed)
}

/// ABI wrapper around [`live_closures`] for leak-checking tests.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_closure_live_count_v1() -> usize {
    LIVE_CLOSURES.load(Ordering::Relaxed)
}

fn layout(size: usize) -> Layout {
    Layout::from_size_align(size.max(1), 8).expect("vut closure layout")
}

unsafe fn block(tagged: *mut u8) -> *mut u8 {
    // SAFETY: the caller passed a tagged pointer; clearing the tag recovers the
    // allocation base.
    (tagged as usize & !TAG) as *mut u8
}

/// Allocates a zeroed closure block with an `env_size`-byte environment.
///
/// # Safety
/// `env_size` must match the environment layout the compiler stores.
#[unsafe(no_mangle)]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the allocation is 8-byte aligned and the header fields are u64"
)]
pub unsafe extern "C" fn vut_rt_closure_new_v1(env_size: usize) -> *mut u8 {
    let total = HEADER.saturating_add(env_size);
    // SAFETY: `layout` yields a valid layout (alignment 8).
    let pointer = unsafe { alloc_zeroed(layout(total)) };
    if pointer.is_null() {
        return pointer;
    }
    // SAFETY: the allocation is at least `HEADER` bytes.
    unsafe {
        pointer.cast::<u64>().write(1);
        pointer
            .add(8)
            .cast::<u64>()
            .write(u64::try_from(total).unwrap_or(u64::MAX));
    }
    LIVE_CLOSURES.fetch_add(1, Ordering::Relaxed);
    pointer
}

/// Increments the reference count of a tagged closure value.
///
/// # Safety
/// `tagged` must be null, an untagged code pointer, or a live tagged closure.
#[unsafe(no_mangle)]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the block is 8-byte aligned; the refcount is the first u64"
)]
pub unsafe extern "C" fn vut_rt_closure_retain_v1(tagged: *mut u8) {
    if tagged.is_null() || tagged as usize & TAG == 0 {
        return;
    }
    // SAFETY: a tagged pointer identifies a live closure block.
    let base = unsafe { block(tagged) };
    let counter = base.cast::<AtomicUsize>();
    // SAFETY: the block begins with the refcount word.
    unsafe { (*counter).fetch_add(1, Ordering::Relaxed) };
}

/// Decrements the reference count of a tagged closure value, freeing it at zero.
///
/// # Safety
/// `tagged` must be null, an untagged code pointer, or a live tagged closure
/// whose reference count is not already zero.
#[unsafe(no_mangle)]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the block is 8-byte aligned; the refcount and size are leading u64s"
)]
pub unsafe extern "C" fn vut_rt_closure_release_v1(tagged: *mut u8) {
    if tagged.is_null() || tagged as usize & TAG == 0 {
        return;
    }
    // SAFETY: a tagged pointer identifies a live closure block.
    let base = unsafe { block(tagged) };
    let counter = base.cast::<AtomicUsize>();
    // SAFETY: the block begins with the refcount word.
    let previous = unsafe { (*counter).fetch_sub(1, Ordering::AcqRel) };
    if previous != 1 {
        return;
    }
    // SAFETY: the block begins with `refcount` then `size`.
    let size = usize::try_from(unsafe { base.add(8).cast::<u64>().read() }).unwrap_or(HEADER);
    // Release the environment's managed captures before freeing the block.
    let drop = unsafe { base.add(24).cast::<u64>().read() };
    if drop != 0 {
        let drop_target = usize::try_from(drop).unwrap_or(0);
        // SAFETY: the compiler stored a valid `ClosureDropFn` for this block.
        let drop_fn: ClosureDropFn =
            unsafe { std::mem::transmute::<usize, ClosureDropFn>(drop_target) };
        // SAFETY: the thunk releases only captures owned by this environment.
        unsafe { drop_fn(base) };
    }
    // SAFETY: `base` came from `vut_rt_closure_new_v1` with this layout.
    unsafe { dealloc(base, layout(size)) };
    LIVE_CLOSURES.fetch_sub(1, Ordering::Relaxed);
}
