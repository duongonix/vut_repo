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
    let length = unsafe { bytes_ref(value) }.map_or(0, Vec::len);
    if index >= length {
        crate::abi::vut_rt_bounds_panic_v1(
            crate::abi::bounds_op::BYTES_AT,
            crate::abi::signed_index(index),
            i64::try_from(length).unwrap_or(i64::MAX),
        );
    }
    let reference = unsafe { bytes_ref(value) };
    let byte = reference
        .and_then(|value| value.get(index))
        .copied()
        .unwrap_or(0);
    if !out.is_null() {
        unsafe { *out = byte };
    }
    1
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
    if index >= value.len() {
        crate::abi::vut_rt_bounds_panic_v1(
            crate::abi::bounds_op::BYTES_SET,
            crate::abi::signed_index(index),
            i64::try_from(value.len()).unwrap_or(i64::MAX),
        );
    }
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
    let len = unsafe { vut_rt_bytes_len_v1(value) };
    if len == 0 {
        if !out.is_null() {
            unsafe { *out = 0 };
        }
        0
    } else {
        unsafe { vut_rt_bytes_at_v1(value, 0, out) }
    }
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

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_push_v1(value: *mut ManagedBytes, byte: usize) {
    if let Some(value) = unsafe { bytes_mut(value) } {
        value.push(u8::try_from(byte).unwrap_or(0));
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-bytes handles.
pub unsafe extern "C" fn vut_rt_bytes_extend_v1(
    value: *mut ManagedBytes,
    source: *const ManagedBytes,
) {
    let Some(source) = (unsafe { bytes_ref(source) }) else {
        return;
    };
    let extension = source.clone();
    if let Some(value) = unsafe { bytes_mut(value) } {
        value.extend_from_slice(&extension);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_truncate_v1(value: *mut ManagedBytes, len: usize) {
    if let Some(value) = unsafe { bytes_mut(value) } {
        value.truncate(len);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_resize_v1(value: *mut ManagedBytes, len: usize, byte: usize) {
    if let Some(value) = unsafe { bytes_mut(value) } {
        value.resize(len, u8::try_from(byte).unwrap_or(0));
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-bytes handles. Returns the first
/// occurrence of `needle` or `-1`.
pub unsafe extern "C" fn vut_rt_bytes_find_v1(
    value: *const ManagedBytes,
    needle: *const ManagedBytes,
) -> i64 {
    let Some((value, needle)) = (unsafe { bytes_ref(value) }).zip(unsafe { bytes_ref(needle) })
    else {
        return -1;
    };
    if needle.is_empty() {
        return 0;
    }
    value
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .map_or(-1, |index| i64::try_from(index).unwrap_or(i64::MAX))
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-bytes handles.
pub unsafe extern "C" fn vut_rt_bytes_starts_with_v1(
    value: *const ManagedBytes,
    prefix: *const ManagedBytes,
) -> u8 {
    u8::from(
        (unsafe { bytes_ref(value) })
            .zip(unsafe { bytes_ref(prefix) })
            .is_some_and(|(value, prefix)| value.starts_with(prefix.as_slice())),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-bytes handles.
pub unsafe extern "C" fn vut_rt_bytes_ends_with_v1(
    value: *const ManagedBytes,
    suffix: *const ManagedBytes,
) -> u8 {
    u8::from(
        (unsafe { bytes_ref(value) })
            .zip(unsafe { bytes_ref(suffix) })
            .is_some_and(|(value, suffix)| value.ends_with(suffix.as_slice())),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-bytes handles. Returns `-1`,
/// `0`, or `1` by lexicographic byte order.
pub unsafe extern "C" fn vut_rt_bytes_compare_v1(
    left: *const ManagedBytes,
    right: *const ManagedBytes,
) -> i64 {
    match (unsafe { bytes_ref(left) }, unsafe { bytes_ref(right) }) {
        (Some(left), Some(right)) => match left.cmp(right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        _ => 0,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_to_hex_v1(
    value: *const ManagedBytes,
) -> *mut crate::abi::ManagedString {
    let text: String = unsafe { bytes_ref(value) }.map_or_else(String::new, |bytes| {
        let mut text = String::with_capacity(bytes.len().saturating_mul(2));
        for byte in bytes {
            text.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
            text.push(char::from_digit(u32::from(byte & 0x0F), 16).unwrap_or('0'));
        }
        text
    });
    crate::abi::managed_string(&text)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `text` must be null or a live managed-string handle. Returns null when the
/// text is not valid even-length hexadecimal.
pub unsafe extern "C" fn vut_rt_bytes_from_hex_v1(
    text: *const crate::abi::ManagedString,
) -> *mut ManagedBytes {
    let Some(text) = (unsafe { crate::abi::string_value(text) }) else {
        return std::ptr::null_mut();
    };
    let bytes = text.as_str().as_bytes();
    if bytes.len() % 2 != 0 {
        return std::ptr::null_mut();
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let (Some(high), Some(low)) = (hex_value(pair[0]), hex_value(pair[1])) else {
            return std::ptr::null_mut();
        };
        out.push((high << 4) | low);
    }
    ManagedBytes::allocate(out)
}

#[unsafe(no_mangle)]
/// # Safety
/// `text` must be null or a live managed-string handle. Returns the byte index
/// of the first invalid hex digit, or `text.byte_len()` when the digit count is
/// odd.
pub unsafe extern "C" fn vut_rt_bytes_from_hex_error_index_v1(
    text: *const crate::abi::ManagedString,
) -> i64 {
    let Some(text) = (unsafe { crate::abi::string_value(text) }) else {
        return 0;
    };
    let bytes = text.as_str().as_bytes();
    if bytes.len() % 2 != 0 {
        return i64::try_from(bytes.len()).unwrap_or(i64::MAX);
    }
    for (index, byte) in bytes.iter().enumerate() {
        if hex_value(*byte).is_none() {
            return i64::try_from(index).unwrap_or(i64::MAX);
        }
    }
    0
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle. Reads `width` bytes
/// (`1..=8`) at `offset` in the requested byte order, zero-extended to `i64`.
pub unsafe extern "C" fn vut_rt_bytes_read_int_v1(
    value: *const ManagedBytes,
    offset: i64,
    width: usize,
    big_endian: u8,
    signed: u8,
) -> i64 {
    let Some(data) = (unsafe { bytes_ref(value) }) else {
        return 0;
    };
    let Ok(offset) = usize::try_from(offset) else {
        return 0;
    };
    let width = width.clamp(1, 8);
    let mut accumulator: i64 = 0;
    for index in 0..width {
        let byte = i64::from(data.get(offset.saturating_add(index)).copied().unwrap_or(0));
        if big_endian != 0 {
            accumulator = (accumulator << 8) | byte;
        } else {
            accumulator |= byte << (8 * index);
        }
    }
    if signed != 0 && width < 8 {
        let shift = 64 - 8 * width;
        accumulator = (accumulator << shift) >> shift;
    }
    accumulator
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle. Writes `width` bytes
/// (`1..=8`) at `offset` in the requested byte order. Returns `0` when the
/// target range is out of bounds.
pub unsafe extern "C" fn vut_rt_bytes_write_int_v1(
    value: *mut ManagedBytes,
    offset: i64,
    width: usize,
    big_endian: u8,
    data: i64,
) -> u8 {
    let Some(bytes) = (unsafe { bytes_mut(value) }) else {
        return 0;
    };
    let Ok(offset) = usize::try_from(offset) else {
        return 0;
    };
    let width = width.clamp(1, 8);
    if offset.saturating_add(width) > bytes.len() {
        return 0;
    }
    for index in 0..width {
        let shift = if big_endian != 0 {
            8 * (width - 1 - index)
        } else {
            8 * index
        };
        bytes[offset + index] = u8::try_from((data >> shift) & 0xFF).unwrap_or(0);
    }
    1
}
