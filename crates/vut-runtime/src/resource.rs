//! Owned opaque native resource handles.
//!
//! A `resource[T]` value is move-only with a single owner. The runtime stores a
//! pointer plus the native destructor registered by the producing library; when
//! the owning Vut value is dropped the runtime invokes the destructor exactly
//! once. There is no reference counting: duplication is rejected by the
//! compiler, so a handle never has more than one owner.
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Registry of live native resources, used by leak tests.
static LIVE_RESOURCES: AtomicUsize = AtomicUsize::new(0);

/// Returns the number of live native resource handles.
#[must_use]
pub fn live_resources() -> usize {
    LIVE_RESOURCES.load(Ordering::Relaxed)
}

/// Creates an owned resource handle wrapping `ptr`.
///
/// # Safety
/// `ptr` must remain valid until `drop_fn` runs; `drop_fn` must be `None` or a
/// valid destructor for `ptr`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_resource_new_v1(
    ptr: *mut c_void,
    drop_fn: Option<unsafe extern "C" fn(*mut c_void)>,
) -> *mut c_void {
    let handle = Box::into_raw(Box::new(Resource { drop: drop_fn, ptr })).cast::<c_void>();
    LIVE_RESOURCES.fetch_add(1, Ordering::Relaxed);
    handle
}

/// Returns the native pointer wrapped by a live resource handle without
/// consuming it. Used when a borrowed `resource[T]` crosses the ABI as `ptr[T]`.
///
/// # Safety
/// `resource` must be null or a live handle returned by
/// [`vut_rt_resource_new_v1`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_resource_ptr_v1(resource: *mut c_void) -> *mut c_void {
    if resource.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: the handle is live and owned by the caller.
    let handle = unsafe { &*resource.cast::<Resource>() };
    handle.ptr
}

/// Releases a resource handle, running its destructor exactly once.
///
/// # Safety
/// `resource` must be null or a live handle returned by
/// [`vut_rt_resource_new_v1`] that has not been released yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_resource_release_v1(resource: *mut c_void) {
    if resource.is_null() {
        return;
    }
    // SAFETY: the handle was produced by `vut_rt_resource_new_v1` and is
    // released exactly once by the compiler-inserted drop.
    let owned = unsafe { Box::from_raw(resource.cast::<Resource>()) };
    if let Some(drop_fn) = owned.drop
        && !owned.ptr.is_null()
    {
        // SAFETY: the native library registered `drop_fn` for `ptr`.
        unsafe { drop_fn(owned.ptr) };
    }
    LIVE_RESOURCES.fetch_sub(1, Ordering::Relaxed);
}

struct Resource {
    drop: Option<unsafe extern "C" fn(*mut c_void)>,
    ptr: *mut c_void,
}

/// Returns the number of live native resource handles.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_resource_live_count_v1() -> usize {
    live_resources()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn count_drop(token: *mut c_void) {
        DROPS.fetch_add(1, Ordering::Relaxed);
        if !token.is_null() {
            // SAFETY: the token was allocated by the test with `Box::into_raw`.
            drop(unsafe { Box::from_raw(token.cast::<u8>()) });
        }
    }

    #[test]
    fn release_runs_the_destructor_exactly_once_and_is_null_safe() {
        let before = live_resources();
        let token = Box::into_raw(Box::new(0_u8)).cast::<c_void>();
        // SAFETY: the handle is created by this ABI and released exactly once;
        // the destructor frees the token allocated above.
        unsafe {
            vut_rt_resource_release_v1(std::ptr::null_mut());
            let resource = vut_rt_resource_new_v1(token, Some(count_drop));
            assert_eq!(live_resources(), before + 1);
            vut_rt_resource_release_v1(resource);
        }
        assert_eq!(live_resources(), before);
        assert_eq!(DROPS.load(Ordering::Relaxed), 1);
    }
}
