//! Core managed `bytes` buffer: a contiguous, growable, reference-counted
//! sequence of `u8` values.
//!
//! `bytes` is a first-class core type distinct from `str` and `list(u8)`. The
//! buffer is always contiguous and never boxes individual bytes. Storage is
//! shared through an atomic reference count; the language layer copies on
//! assignment so observable values remain independent.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::utf8;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Number of live managed `bytes` allocations.
#[must_use]
pub fn live_bytes_allocations() -> usize {
    LIVE_BYTES.load(Ordering::Relaxed)
}

#[repr(C)]
pub struct ManagedBytes {
    references: AtomicUsize,
    value: Vec<u8>,
}

impl ManagedBytes {
    pub(crate) fn allocate(value: Vec<u8>) -> *mut Self {
        LIVE_BYTES.fetch_add(1, Ordering::Relaxed);
        Box::into_raw(Box::new(Self {
            references: AtomicUsize::new(1),
            value,
        }))
    }

    /// Views the contents of a live managed bytes buffer.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.value
    }
}

/// Creates a new managed bytes buffer from owned bytes.
#[must_use]
pub fn managed_bytes(value: Vec<u8>) -> *mut ManagedBytes {
    ManagedBytes::allocate(value)
}

pub(crate) unsafe fn bytes_ref<'a>(value: *const ManagedBytes) -> Option<&'a Vec<u8>> {
    (!value.is_null()).then(|| unsafe { &(*value).value })
}

pub(crate) unsafe fn bytes_mut<'a>(value: *mut ManagedBytes) -> Option<&'a mut Vec<u8>> {
    (!value.is_null()).then(|| unsafe { &mut (*value).value })
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_bytes_new_v1() -> *mut ManagedBytes {
    ManagedBytes::allocate(Vec::new())
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_clone_v1(value: *const ManagedBytes) -> *mut ManagedBytes {
    ManagedBytes::allocate(unsafe { bytes_ref(value) }.cloned().unwrap_or_default())
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_retain_v1(value: *mut ManagedBytes) {
    if !value.is_null() {
        unsafe { &*value }
            .references
            .fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or own one live managed-bytes reference.
pub unsafe extern "C" fn vut_rt_bytes_release_v1(value: *mut ManagedBytes) {
    if !value.is_null() && unsafe { &*value }.references.fetch_sub(1, Ordering::AcqRel) == 1 {
        LIVE_BYTES.fetch_sub(1, Ordering::Relaxed);
        drop(unsafe { Box::from_raw(value) });
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_len_v1(value: *const ManagedBytes) -> usize {
    unsafe { bytes_ref(value) }.map_or(0, Vec::len)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_is_empty_v1(value: *const ManagedBytes) -> u8 {
    u8::from(unsafe { vut_rt_bytes_len_v1(value) } == 0)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_capacity_v1(value: *const ManagedBytes) -> usize {
    unsafe { bytes_ref(value) }.map_or(0, Vec::capacity)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_read_i32_v1(value: *const ManagedBytes, offset: i64) -> i64 {
    let Some(data) = (unsafe { bytes_ref(value) }) else {
        return 0;
    };
    let Ok(offset) = usize::try_from(offset) else {
        return 0;
    };
    let mut raw = [0_u8; 4];
    if let Some(slice) = data.get(offset..offset.saturating_add(4)) {
        raw.copy_from_slice(slice);
    }
    i64::from(i32::from_le_bytes(raw))
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_read_i64_v1(value: *const ManagedBytes, offset: i64) -> i64 {
    let Some(data) = (unsafe { bytes_ref(value) }) else {
        return 0;
    };
    let Ok(offset) = usize::try_from(offset) else {
        return 0;
    };
    let mut raw = [0_u8; 8];
    if let Some(slice) = data.get(offset..offset.saturating_add(8)) {
        raw.copy_from_slice(slice);
    }
    i64::from_le_bytes(raw)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_reserve_v1(value: *mut ManagedBytes, capacity: usize) {
    if let Some(value) = unsafe { bytes_mut(value) }
        && capacity > value.capacity()
    {
        value.reserve(capacity - value.len());
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle and `out` writable.
pub unsafe extern "C" fn vut_rt_bytes_at_v1(
    value: *const ManagedBytes,
    index: usize,
    out: *mut u8,
) -> u8 {
    let reference = unsafe { bytes_ref(value) };
    let byte = reference
        .and_then(|value| value.get(index))
        .copied()
        .unwrap_or(0);
    if !out.is_null() {
        unsafe { *out = byte };
    }
    u8::from(reference.is_some_and(|value| index < value.len()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_set_v1(
    value: *mut ManagedBytes,
    index: usize,
    byte: u8,
) -> u8 {
    let Some(value) = (unsafe { bytes_mut(value) }) else {
        return 0;
    };
    let Some(slot) = value.get_mut(index) else {
        return 0;
    };
    *slot = byte;
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle and `out` writable.
pub unsafe extern "C" fn vut_rt_bytes_first_v1(value: *const ManagedBytes, out: *mut u8) -> u8 {
    unsafe { vut_rt_bytes_at_v1(value, 0, out) }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle and `out` writable.
pub unsafe extern "C" fn vut_rt_bytes_last_v1(value: *const ManagedBytes, out: *mut u8) -> u8 {
    let len = unsafe { vut_rt_bytes_len_v1(value) };
    if len == 0 {
        if !out.is_null() {
            unsafe { *out = 0 };
        }
        0
    } else {
        unsafe { vut_rt_bytes_at_v1(value, len - 1, out) }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_slice_v1(
    value: *const ManagedBytes,
    start: usize,
    end: usize,
) -> *mut ManagedBytes {
    let Some(value) = (unsafe { bytes_ref(value) }) else {
        return std::ptr::null_mut();
    };
    if start > end || end > value.len() {
        return std::ptr::null_mut();
    }
    ManagedBytes::allocate(value[start..end].to_vec())
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_clear_v1(value: *mut ManagedBytes) {
    if let Some(value) = unsafe { bytes_mut(value) } {
        value.clear();
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle. Invalid sequences
/// report the byte offset at which UTF-8 validation failed.
pub unsafe extern "C" fn vut_rt_bytes_utf8_valid_up_to_v1(value: *const ManagedBytes) -> usize {
    let Some(bytes) = (unsafe { bytes_ref(value) }) else {
        return 0;
    };
    match utf8::validate(bytes) {
        Ok(()) => bytes.len(),
        Err(issue) => issue.valid_up_to,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle. Returns the length of
/// the invalid UTF-8 sequence, or `0` when the input ends prematurely.
pub unsafe extern "C" fn vut_rt_bytes_utf8_error_len_v1(value: *const ManagedBytes) -> usize {
    let Some(bytes) = (unsafe { bytes_ref(value) }) else {
        return 0;
    };
    utf8::validate(bytes)
        .err()
        .and_then(|issue| issue.error_len)
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-bytes handles.
pub unsafe extern "C" fn vut_rt_bytes_eq_v1(
    left: *const ManagedBytes,
    right: *const ManagedBytes,
) -> u8 {
    let left = unsafe { bytes_ref(left) };
    let right = unsafe { bytes_ref(right) };
    u8::from(match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left == right,
        _ => false,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_live_bytes_count_v1() -> usize {
    live_bytes_allocations()
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle. Returns -1 out of range.
pub unsafe extern "C" fn vut_rt_bytes_byte_at_v1(value: *const ManagedBytes, index: i64) -> i64 {
    let Some(data) = (unsafe { bytes_ref(value) }) else {
        return -1;
    };
    let Ok(index) = usize::try_from(index) else {
        return -1;
    };
    data.get(index).map_or(-1, |byte| i64::from(*byte))
}
