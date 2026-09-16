//! Managed list storage and its C ABI operations.
use std::sync::atomic::{AtomicUsize, Ordering};

use super::LIVE_LISTS;
use crate::bytes::{ManagedBytes, bytes_ref};

type RetainFn = unsafe extern "C" fn(*const u8);
type ReleaseFn = unsafe extern "C" fn(*mut u8);

#[repr(C)]
pub struct ManagedList {
    references: AtomicUsize,
    len: usize,
    capacity: usize,
    element_size: usize,
    element_align: usize,
    element_retain: Option<RetainFn>,
    element_release: Option<ReleaseFn>,
    data: *mut u8,
}

unsafe impl Send for ManagedList {}
unsafe impl Sync for ManagedList {}

impl ManagedList {
    unsafe fn allocate(
        element_size: usize,
        element_align: usize,
        element_retain: *const (),
        element_release: *const (),
        capacity: usize,
    ) -> *mut Self {
        let element_size = element_size.max(1);
        let element_align = element_align.max(1);
        let mut list = Box::new(Self {
            references: AtomicUsize::new(1),
            len: 0,
            capacity: 0,
            element_size,
            element_align,
            element_retain: unsafe { retain_fn(element_retain) },
            element_release: unsafe { release_fn(element_release) },
            data: std::ptr::null_mut(),
        });
        if capacity != 0 && !unsafe { list.grow_to(capacity) } {
            return std::ptr::null_mut();
        }
        LIVE_LISTS.fetch_add(1, Ordering::Relaxed);
        Box::into_raw(list)
    }

    unsafe fn grow_to(&mut self, capacity: usize) -> bool {
        if capacity <= self.capacity {
            return true;
        }
        let Some(size) = self.element_size.checked_mul(capacity) else {
            return false;
        };
        let Ok(layout) = std::alloc::Layout::from_size_align(size, self.element_align) else {
            return false;
        };
        let new_data = if self.data.is_null() {
            unsafe { std::alloc::alloc_zeroed(layout) }
        } else {
            let old_size = self.element_size.saturating_mul(self.capacity);
            let Ok(old_layout) = std::alloc::Layout::from_size_align(old_size, self.element_align)
            else {
                return false;
            };
            unsafe { std::alloc::realloc(self.data, old_layout, size) }
        };
        if new_data.is_null() {
            return false;
        }
        self.data = new_data;
        self.capacity = capacity;
        true
    }

    fn slot(&self, index: usize) -> *mut u8 {
        self.data
            .wrapping_add(index.saturating_mul(self.element_size))
    }

    unsafe fn retain_element(&self, slot: *const u8) {
        if let Some(retain) = self.element_retain {
            unsafe { retain(slot) };
        }
    }

    unsafe fn release_element(&self, slot: *mut u8) {
        if let Some(release) = self.element_release {
            unsafe { release(slot) };
        }
    }
}

unsafe fn retain_fn(pointer: *const ()) -> Option<RetainFn> {
    if pointer.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*const (), RetainFn>(pointer) })
    }
}

unsafe fn release_fn(pointer: *const ()) -> Option<ReleaseFn> {
    if pointer.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*const (), ReleaseFn>(pointer) })
    }
}

pub(crate) unsafe fn build_string_list<I: IntoIterator<Item = String>>(
    keys: I,
) -> *mut ManagedList {
    let list = unsafe {
        ManagedList::allocate(
            8,
            8,
            super::ownership::vut_rt_slot_retain_string_v1 as *const (),
            super::ownership::vut_rt_slot_release_string_v1 as *const (),
            0,
        )
    };
    for key in keys {
        let handle = super::string::managed_string(&key);
        // The list stores the handle itself, so pass the address of the local
        // handle slot (not the handle value, which would store the refcount).
        let slot = std::ptr::addr_of!(handle).cast::<u8>();
        unsafe { vut_rt_list_push_v1(list, slot) };
        unsafe { super::string::vut_rt_release_string_v1(handle) };
    }
    list
}

impl Drop for ManagedList {
    fn drop(&mut self) {
        for index in 0..self.len {
            unsafe { self.release_element(self.slot(index)) };
        }
        if !self.data.is_null() && self.capacity != 0 {
            let size = self.element_size.saturating_mul(self.capacity);
            if let Ok(layout) = std::alloc::Layout::from_size_align(size, self.element_align) {
                unsafe { std::alloc::dealloc(self.data, layout) };
            }
        }
        LIVE_LISTS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `element_size` and `element_align` must describe the concrete list element
/// type. `element_retain` and `element_release` are optional compiler-generated
/// callbacks for managed elements and must be null for trivial elements.
pub unsafe extern "C" fn vut_rt_list_new_v1(
    element_size: usize,
    element_align: usize,
    element_retain: *const (),
    element_release: *const (),
    capacity: usize,
) -> *mut ManagedList {
    unsafe {
        ManagedList::allocate(
            element_size,
            element_align,
            element_retain,
            element_release,
            capacity,
        )
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_retain_v1(list: *mut ManagedList) {
    if !list.is_null() {
        unsafe { &*list }.references.fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or own one live managed-list reference.
pub unsafe extern "C" fn vut_rt_list_release_v1(list: *mut ManagedList) {
    if !list.is_null() && unsafe { &*list }.references.fetch_sub(1, Ordering::AcqRel) == 1 {
        drop(unsafe { Box::from_raw(list) });
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_len_v1(list: *const ManagedList) -> usize {
    if list.is_null() {
        0
    } else {
        unsafe { (*list).len }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_is_empty_v1(list: *const ManagedList) -> u8 {
    u8::from(unsafe { vut_rt_list_len_v1(list) } == 0)
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_capacity_v1(list: *const ManagedList) -> usize {
    if list.is_null() {
        0
    } else {
        unsafe { (*list).capacity }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_data_v1(list: *const ManagedList) -> *const u8 {
    if list.is_null() {
        std::ptr::null()
    } else {
        unsafe { (*list).data }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_reserve_v1(list: *mut ManagedList, capacity: usize) {
    if !list.is_null() {
        unsafe { (*list).grow_to(capacity) };
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live list handle and `value` to one initialized element.
pub unsafe extern "C" fn vut_rt_list_push_v1(list: *mut ManagedList, value: *const u8) {
    if list.is_null() || value.is_null() {
        return;
    }
    let list_ref = unsafe { &mut *list };
    let next_capacity = if list_ref.capacity == 0 {
        4
    } else {
        list_ref.capacity.saturating_mul(2)
    };
    if list_ref.len == list_ref.capacity && !unsafe { list_ref.grow_to(next_capacity) } {
        return;
    }
    unsafe {
        let slot = list_ref.slot(list_ref.len);
        std::ptr::copy_nonoverlapping(value, slot, list_ref.element_size);
        list_ref.retain_element(slot);
    }
    list_ref.len += 1;
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be live and `out` must point to writable storage for one element.
pub unsafe extern "C" fn vut_rt_list_at_v1(
    list: *const ManagedList,
    index: usize,
    out: *mut u8,
) -> u8 {
    if list.is_null() || out.is_null() {
        return 0;
    }
    let list_ref = unsafe { &*list };
    if index >= list_ref.len {
        unsafe { std::ptr::write_bytes(out, 0, list_ref.element_size) };
        return 0;
    }
    unsafe { std::ptr::copy_nonoverlapping(list_ref.slot(index), out, list_ref.element_size) };
    unsafe { list_ref.retain_element(out) };
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live list handle and `value` to one initialized element.
pub unsafe extern "C" fn vut_rt_list_set_v1(
    list: *mut ManagedList,
    index: usize,
    value: *const u8,
) -> u8 {
    if list.is_null() || value.is_null() || index >= unsafe { (*list).len } {
        return 0;
    }
    let list_ref = unsafe { &mut *list };
    unsafe {
        let slot = list_ref.slot(index);
        list_ref.release_element(slot);
        std::ptr::copy_nonoverlapping(value, slot, list_ref.element_size);
        list_ref.retain_element(slot);
    }
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live list handle and `value` to one initialized element.
pub unsafe extern "C" fn vut_rt_list_insert_v1(
    list: *mut ManagedList,
    index: usize,
    value: *const u8,
) -> u8 {
    if list.is_null() || value.is_null() || index > unsafe { (*list).len } {
        return 0;
    }
    let list_ref = unsafe { &mut *list };
    let next_capacity = if list_ref.capacity == 0 {
        4
    } else {
        list_ref.capacity.saturating_mul(2)
    };
    if list_ref.len == list_ref.capacity && !unsafe { list_ref.grow_to(next_capacity) } {
        return 0;
    }
    unsafe {
        let slot = list_ref.slot(index);
        std::ptr::copy(
            slot,
            slot.add(list_ref.element_size),
            (list_ref.len - index) * list_ref.element_size,
        );
        std::ptr::copy_nonoverlapping(value, slot, list_ref.element_size);
        list_ref.retain_element(slot);
    }
    list_ref.len += 1;
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live list handle and `out` to writable element storage.
pub unsafe extern "C" fn vut_rt_list_remove_v1(
    list: *mut ManagedList,
    index: usize,
    out: *mut u8,
) -> u8 {
    if list.is_null() || out.is_null() {
        return 0;
    }
    let list_ref = unsafe { &mut *list };
    if index >= list_ref.len {
        unsafe { std::ptr::write_bytes(out, 0, list_ref.element_size) };
        return 0;
    }
    unsafe {
        let slot = list_ref.slot(index);
        std::ptr::copy_nonoverlapping(slot, out, list_ref.element_size);
        std::ptr::copy(
            slot.add(list_ref.element_size),
            slot,
            (list_ref.len - index - 1) * list_ref.element_size,
        );
    }
    list_ref.len -= 1;
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_clear_v1(list: *mut ManagedList) {
    if !list.is_null() {
        let list_ref = unsafe { &mut *list };
        for index in 0..list_ref.len {
            unsafe { list_ref.release_element(list_ref.slot(index)) };
        }
        list_ref.len = 0;
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be live or null. Bounds are checked by the runtime.
pub unsafe extern "C" fn vut_rt_list_slice_v1(
    list: *const ManagedList,
    start: usize,
    end: usize,
) -> *mut ManagedList {
    if list.is_null() || start > end || end > unsafe { (*list).len } {
        return std::ptr::null_mut();
    }
    let source = unsafe { &*list };
    let copy = unsafe {
        ManagedList::allocate(
            source.element_size,
            source.element_align,
            source
                .element_retain
                .map_or(std::ptr::null(), |retain| retain as *const ()),
            source
                .element_release
                .map_or(std::ptr::null(), |release| release as *const ()),
            end - start,
        )
    };
    if copy.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(
            source.slot(start),
            (*copy).data,
            (end - start).saturating_mul(source.element_size),
        );
        (*copy).len = end - start;
        for index in 0..(*copy).len {
            (*copy).retain_element((*copy).slot(index));
        }
    }
    copy
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be live and `value` must point to one initialized element.
pub unsafe extern "C" fn vut_rt_list_contains_v1(list: *const ManagedList, value: *const u8) -> u8 {
    if list.is_null() || value.is_null() {
        return 0;
    }
    let list_ref = unsafe { &*list };
    for index in 0..list_ref.len {
        let equal = unsafe {
            std::slice::from_raw_parts(list_ref.slot(index), list_ref.element_size)
                == std::slice::from_raw_parts(value, list_ref.element_size)
        };
        if equal {
            return 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_to_list_v1(value: *const ManagedBytes) -> *mut ManagedList {
    let bytes = unsafe { bytes_ref(value) }.cloned().unwrap_or_default();
    let list =
        unsafe { ManagedList::allocate(1, 1, std::ptr::null(), std::ptr::null(), bytes.len()) };
    if list.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), (*list).data, bytes.len());
        (*list).len = bytes.len();
    }
    list
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live `list(u8)` handle.
pub unsafe extern "C" fn vut_rt_bytes_from_list_v1(list: *const ManagedList) -> *mut ManagedBytes {
    if list.is_null() {
        return ManagedBytes::allocate(Vec::new());
    }
    let list = unsafe { &*list };
    if list.element_size != 1 {
        return std::ptr::null_mut();
    }
    ManagedBytes::allocate(unsafe { std::slice::from_raw_parts(list.data, list.len) }.to_vec())
}
