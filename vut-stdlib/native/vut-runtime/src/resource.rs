//! Canonical allocation and exactly-once destruction of native-owned resources.
use std::ffi::c_void;

unsafe extern "C" {
    fn vut_rt_resource_new_v1(
        pointer: *mut c_void,
        destructor: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> *mut c_void;
}

unsafe extern "C" fn destroy<T>(pointer: *mut c_void) {
    if !pointer.is_null() {
        // SAFETY: registered only for a Box<T> allocated in owned().
        drop(unsafe { Box::from_raw(pointer.cast::<T>()) });
    }
}

pub(crate) fn owned<T>(value: T) -> *mut c_void {
    let pointer = Box::into_raw(Box::new(value)).cast();
    // SAFETY: the resource takes sole ownership and registers its matching drop.
    unsafe { vut_rt_resource_new_v1(pointer, Some(destroy::<T>)) }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Probe(Arc<AtomicUsize>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn native_box_destructor_runs_exactly_once_per_resource() {
        let count = Arc::new(AtomicUsize::new(0));
        for expected in 1..=1024 {
            let handle = super::owned(Probe(count.clone()));
            // SAFETY: every live resource is released once, consuming its owner.
            unsafe { vut_runtime::abi::vut_rt_resource_release_v1(handle) };
            assert_eq!(count.load(Ordering::SeqCst), expected);
            assert_eq!(Arc::strong_count(&count), 1);
        }
    }
}
