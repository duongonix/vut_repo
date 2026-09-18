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
    if out.is_null() {
        return 0;
    }
    let length = unsafe { vut_rt_list_len_v1(list) };
    if index >= length {
        super::vut_rt_bounds_panic_v1(
            super::bounds_op::LIST_AT,
            super::signed_index(index),
            i64::try_from(length).unwrap_or(i64::MAX),
        );
    }
    let list_ref = unsafe { &*list };
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
    if value.is_null() {
        return 0;
    }
    let length = unsafe { vut_rt_list_len_v1(list) };
    if index >= length {
        super::vut_rt_bounds_panic_v1(
            super::bounds_op::LIST_SET,
            super::signed_index(index),
            i64::try_from(length).unwrap_or(i64::MAX),
        );
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
    if list.is_null() || value.is_null() {
        return 0;
    }
    let list_ref = unsafe { &mut *list };
    if index > list_ref.len {
        super::vut_rt_bounds_panic_v1(
            super::bounds_op::LIST_INSERT,
            super::signed_index(index),
            i64::try_from(list_ref.len).unwrap_or(i64::MAX),
        );
    }
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
        super::vut_rt_bounds_panic_v1(
            super::bounds_op::LIST_REMOVE,
            super::signed_index(index),
            i64::try_from(list_ref.len).unwrap_or(i64::MAX),
        );
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

/// Element comparison kind passed to [`vut_rt_list_sort_v1`].
pub const LIST_SORT_INT: i64 = 0;
pub const LIST_SORT_FLOAT: i64 = 1;
pub const LIST_SORT_STRING: i64 = 2;
pub const LIST_SORT_BOOL: i64 = 3;

fn compare_float(left: f64, right: f64) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    }
}

/// Vut element ordering over raw element storage. `kind` selects the
/// comparison; `stride` is the element storage size. Managed elements are only
/// read, never retained/released.
unsafe fn compare_at(
    data: *const u8,
    stride: usize,
    kind: i64,
    left: usize,
    right: usize,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let slot = |index: usize| data.wrapping_add(index.saturating_mul(stride));
    unsafe {
        match kind {
            LIST_SORT_INT => {
                let mut a = [0_u8; 8];
                let mut b = [0_u8; 8];
                std::ptr::copy_nonoverlapping(slot(left), a.as_mut_ptr(), 8);
                std::ptr::copy_nonoverlapping(slot(right), b.as_mut_ptr(), 8);
                i64::from_ne_bytes(a).cmp(&i64::from_ne_bytes(b))
            }
            LIST_SORT_FLOAT => {
                let mut a = [0_u8; 8];
                let mut b = [0_u8; 8];
                std::ptr::copy_nonoverlapping(slot(left), a.as_mut_ptr(), 8);
                std::ptr::copy_nonoverlapping(slot(right), b.as_mut_ptr(), 8);
                compare_float(f64::from_ne_bytes(a), f64::from_ne_bytes(b))
            }
            LIST_SORT_STRING => {
                let a = std::ptr::read_unaligned(
                    slot(left).cast::<*const super::string::ManagedString>(),
                );
                let b = std::ptr::read_unaligned(
                    slot(right).cast::<*const super::string::ManagedString>(),
                );
                let a = super::string::string_value(a).map_or("", |value| value.as_str());
                let b = super::string::string_value(b).map_or("", |value| value.as_str());
                a.cmp(b)
            }
            LIST_SORT_BOOL => (*slot(left)).cmp(&(*slot(right))),
            _ => Ordering::Equal,
        }
    }
}

/// Sorts raw element storage in place. `stride` is the element storage size.
pub(crate) fn sort_raw(data: *mut u8, length: usize, stride: usize, kind: i64) {
    if data.is_null() || length < 2 || stride == 0 {
        return;
    }
    let mut order: Vec<usize> = (0..length).collect();
    order.sort_by(|&left, &right| unsafe { compare_at(data, stride, kind, left, right) });
    apply_permutation_raw(data, stride, &order);
}

/// Reverses raw element storage in place.
pub(crate) fn reverse_raw(data: *mut u8, length: usize, stride: usize) {
    if data.is_null() || length < 2 || stride == 0 {
        return;
    }
    let mut left = 0_usize;
    let mut right = length - 1;
    while left < right {
        swap_slots_raw(data, stride, left, right);
        left += 1;
        right -= 1;
    }
}

fn swap_slots_raw(data: *mut u8, stride: usize, left: usize, right: usize) {
    let base_left = data.wrapping_add(left.saturating_mul(stride));
    let base_right = data.wrapping_add(right.saturating_mul(stride));
    for offset in 0..stride {
        unsafe { std::ptr::swap(base_left.add(offset), base_right.add(offset)) };
    }
}

fn apply_permutation_raw(data: *mut u8, stride: usize, order: &[usize]) {
    if data.is_null() || stride == 0 {
        return;
    }
    let slot = |index: usize| data.wrapping_add(index.saturating_mul(stride));
    let mut visited = vec![false; order.len()];
    let mut temp = vec![0_u8; stride];
    for start in 0..order.len() {
        if visited[start] || order[start] == start {
            visited[start] = true;
            continue;
        }
        unsafe { std::ptr::copy_nonoverlapping(slot(start), temp.as_mut_ptr(), stride) };
        let mut current = start;
        loop {
            visited[current] = true;
            let next = order[current];
            if next == start {
                break;
            }
            unsafe { std::ptr::copy_nonoverlapping(slot(next), slot(current), stride) };
            current = next;
        }
        unsafe { std::ptr::copy_nonoverlapping(temp.as_ptr(), slot(current), stride) };
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle; `out` must be writable
/// for one element. Removes and returns the last element; `out` is zeroed and
/// the return is `0` when the list is empty.
pub unsafe extern "C" fn vut_rt_list_pop_v1(list: *mut ManagedList, out: *mut u8) -> u8 {
    if list.is_null() || out.is_null() {
        return 0;
    }
    let list_ref = unsafe { &mut *list };
    if list_ref.len == 0 {
        unsafe { std::ptr::write_bytes(out, 0, list_ref.element_size) };
        return 0;
    }
    let index = list_ref.len - 1;
    unsafe {
        std::ptr::copy_nonoverlapping(list_ref.slot(index), out, list_ref.element_size);
    }
    list_ref.len = index;
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be live and `out` writable for one element. Bounds are checked;
/// `out` is zeroed and the return is `0` when the list is empty.
pub unsafe extern "C" fn vut_rt_list_first_v1(list: *const ManagedList, out: *mut u8) -> u8 {
    unsafe { copy_first_or_last(list, out, false) }
}

#[unsafe(no_mangle)]
/// # Safety
/// See [`vut_rt_list_first_v1`].
pub unsafe extern "C" fn vut_rt_list_last_v1(list: *const ManagedList, out: *mut u8) -> u8 {
    unsafe { copy_first_or_last(list, out, true) }
}

unsafe fn copy_first_or_last(list: *const ManagedList, out: *mut u8, from_end: bool) -> u8 {
    if list.is_null() || out.is_null() {
        return 0;
    }
    let list_ref = unsafe { &*list };
    if list_ref.len == 0 {
        unsafe { std::ptr::write_bytes(out, 0, list_ref.element_size) };
        return 0;
    }
    let index = if from_end { list_ref.len - 1 } else { 0 };
    unsafe {
        std::ptr::copy_nonoverlapping(list_ref.slot(index), out, list_ref.element_size);
        list_ref.retain_element(out);
    }
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle; `value` points to one
/// initialized element. Returns the first matching index or `-1`.
pub unsafe extern "C" fn vut_rt_list_find_index_v1(
    list: *const ManagedList,
    value: *const u8,
) -> i64 {
    if list.is_null() || value.is_null() {
        return -1;
    }
    let list_ref = unsafe { &*list };
    for index in 0..list_ref.len {
        let equal = unsafe {
            std::slice::from_raw_parts(list_ref.slot(index), list_ref.element_size)
                == std::slice::from_raw_parts(value, list_ref.element_size)
        };
        if equal {
            return i64::try_from(index).unwrap_or(i64::MAX);
        }
    }
    -1
}

#[unsafe(no_mangle)]
/// # Safety
/// `destination` and `source` must be null or live managed-list handles with
/// the same element layout. Appends a copy of every source element.
pub unsafe extern "C" fn vut_rt_list_extend_v1(
    destination: *mut ManagedList,
    source: *const ManagedList,
) -> u8 {
    if destination.is_null() || source.is_null() {
        return 0;
    }
    let source_ref = unsafe { &*source };
    if source_ref.len == 0 {
        return 1;
    }
    // Snapshot the source elements so a self-extend cannot be invalidated by
    // reallocation while pushing.
    let byte_len = source_ref.len.saturating_mul(source_ref.element_size);
    let mut snapshot = vec![0_u8; byte_len];
    unsafe {
        std::ptr::copy_nonoverlapping(source_ref.slot(0), snapshot.as_mut_ptr(), byte_len);
    }
    for index in 0..source_ref.len {
        let offset = index.saturating_mul(source_ref.element_size);
        let slot = unsafe { snapshot.as_ptr().add(offset) };
        unsafe { vut_rt_list_push_v1(destination, slot) };
    }
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_reverse_v1(list: *mut ManagedList) {
    if list.is_null() {
        return;
    }
    let list_ref = unsafe { &mut *list };
    reverse_raw(list_ref.data, list_ref.len, list_ref.element_size);
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle. Swaps two elements by
/// raw move (refcount-neutral); out-of-range indices are ignored.
pub unsafe extern "C" fn vut_rt_list_swap_v1(list: *mut ManagedList, left: usize, right: usize) {
    if list.is_null() {
        return;
    }
    let list_ref = unsafe { &mut *list };
    if left >= list_ref.len || right >= list_ref.len || left == right {
        return;
    }
    swap_slots_raw(list_ref.data, list_ref.element_size, left, right);
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle whose element type matches
/// `kind` (see the `LIST_SORT_*` constants).
pub unsafe extern "C" fn vut_rt_list_sort_v1(list: *mut ManagedList, kind: i64) {
    if list.is_null() {
        return;
    }
    let list_ref = unsafe { &mut *list };
    sort_raw(list_ref.data, list_ref.len, list_ref.element_size, kind);
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must point to `length` initialized elements of `element_size` bytes.
/// Appends a copy of every element into a newly allocated managed list. The
/// optional retain/release callbacks manage managed elements.
pub unsafe extern "C" fn vut_rt_array_to_list_v1(
    data: *const u8,
    length: usize,
    element_size: usize,
    element_align: usize,
    element_retain: *const (),
    element_release: *const (),
) -> *mut ManagedList {
    let list = unsafe {
        vut_rt_list_new_v1(
            element_size,
            element_align,
            element_retain,
            element_release,
            length,
        )
    };
    if list.is_null() {
        return std::ptr::null_mut();
    }
    let stride = element_size.max(1);
    for index in 0..length {
        let slot = data.wrapping_add(index.saturating_mul(stride));
        unsafe { vut_rt_list_push_v1(list, slot) };
    }
    list
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must point to `length` initialized elements of `element_size` bytes.
pub unsafe extern "C" fn vut_rt_array_reverse_v1(
    data: *mut u8,
    length: usize,
    element_size: usize,
) {
    reverse_raw(data, length, element_size);
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must point to `length` initialized elements of `element_size` bytes
/// matching `kind` (see the `LIST_SORT_*` constants).
pub unsafe extern "C" fn vut_rt_array_sort_v1(
    data: *mut u8,
    length: usize,
    element_size: usize,
    kind: i64,
) {
    sort_raw(data, length, element_size, kind);
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must point to `length` initialized elements of `element_size` bytes;
/// `value` must point to one initialized element of the same layout.
pub unsafe extern "C" fn vut_rt_array_contains_v1(
    data: *const u8,
    length: usize,
    element_size: usize,
    value: *const u8,
) -> u8 {
    if data.is_null() || value.is_null() {
        return 0;
    }
    let stride = element_size.max(1);
    for index in 0..length {
        let slot = data.wrapping_add(index.saturating_mul(stride));
        let equal = unsafe {
            std::slice::from_raw_parts(slot, stride) == std::slice::from_raw_parts(value, stride)
        };
        if equal {
            return 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_truncate_v1(list: *mut ManagedList, len: usize) {
    if list.is_null() {
        return;
    }
    let list_ref = unsafe { &mut *list };
    if len >= list_ref.len {
        return;
    }
    for index in len..list_ref.len {
        unsafe { list_ref.release_element(list_ref.slot(index)) };
    }
    list_ref.len = len;
}

#[unsafe(no_mangle)]
/// # Safety
/// `list` must be null or a live managed-list handle.
pub unsafe extern "C" fn vut_rt_list_shrink_to_fit_v1(list: *mut ManagedList) {
    if list.is_null() {
        return;
    }
    let list_ref = unsafe { &mut *list };
    if list_ref.capacity <= list_ref.len {
        return;
    }
    if list_ref.len == 0 {
        if !list_ref.data.is_null() && list_ref.capacity != 0 {
            let old_size = list_ref.element_size.saturating_mul(list_ref.capacity);
            if let Ok(layout) =
                std::alloc::Layout::from_size_align(old_size, list_ref.element_align)
            {
                unsafe { std::alloc::dealloc(list_ref.data, layout) };
            }
        }
        list_ref.data = std::ptr::null_mut();
        list_ref.capacity = 0;
        return;
    }
    let new_size = list_ref.element_size.saturating_mul(list_ref.len);
    let old_size = list_ref.element_size.saturating_mul(list_ref.capacity);
    let Ok(new_layout) = std::alloc::Layout::from_size_align(new_size, list_ref.element_align)
    else {
        return;
    };
    let Ok(old_layout) = std::alloc::Layout::from_size_align(old_size, list_ref.element_align)
    else {
        return;
    };
    let new_data = unsafe { std::alloc::realloc(list_ref.data, old_layout, new_size) };
    if !new_data.is_null() {
        list_ref.data = new_data;
        list_ref.capacity = list_ref.len;
    }
    let _ = new_layout;
}
