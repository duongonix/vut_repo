//! Managed string values and their C ABI operations.
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{LIVE_STRINGS, ManagedList};
use crate::bytes::{ManagedBytes, bytes_ref};

#[repr(C)]
pub struct ManagedString {
    references: AtomicUsize,
    value: crate::VutString,
}

impl ManagedString {
    pub(crate) fn allocate(value: crate::VutString) -> *mut Self {
        LIVE_STRINGS.fetch_add(1, Ordering::Relaxed);
        Box::into_raw(Box::new(Self {
            references: AtomicUsize::new(1),
            value,
        }))
    }

    /// Views the UTF-8 contents of a live managed string.
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

/// Borrows a live managed string handle as a Vut string.
///
/// # Safety
/// A non-null `value` must be a live managed string handle for this call.
pub unsafe fn string_value<'a>(value: *const ManagedString) -> Option<&'a crate::VutString> {
    unsafe { string_ref(value) }
}

/// Creates a new managed string from Rust text.
#[must_use]
pub fn managed_string(value: &str) -> *mut ManagedString {
    ManagedString::allocate(crate::VutString::from(value))
}

/// Borrows the string behind a managed handle, or `None` for a null handle.
///
/// # Safety
/// A non-null `value` must point to a live managed string for the returned
/// borrow's lifetime.
pub(crate) unsafe fn string_ref<'a>(value: *const ManagedString) -> Option<&'a crate::VutString> {
    // SAFETY: upheld by the caller; the returned borrow cannot outlive the handle.
    (!value.is_null()).then(|| unsafe { &(*value).value })
}

unsafe fn transform_string(
    value: *const ManagedString,
    transform: impl FnOnce(&crate::VutString) -> crate::VutString,
) -> *mut ManagedString {
    unsafe { string_ref(value) }.map_or(std::ptr::null_mut(), |value| {
        ManagedString::allocate(transform(value))
    })
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle.
pub unsafe extern "C" fn vut_rt_string_to_bytes_v1(
    value: *const ManagedString,
) -> *mut ManagedBytes {
    ManagedBytes::allocate(
        unsafe { string_ref(value) }
            .map(|value| value.as_str().as_bytes().to_vec())
            .unwrap_or_default(),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-bytes handle.
pub unsafe extern "C" fn vut_rt_bytes_to_str_v1(value: *const ManagedBytes) -> *mut ManagedString {
    let Some(bytes) = (unsafe { bytes_ref(value) }) else {
        return std::ptr::null_mut();
    };
    match crate::utf8::validate(bytes) {
        Ok(()) => ManagedString::allocate(crate::VutString::from(
            std::str::from_utf8(bytes).unwrap_or_default(),
        )),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_byte_len_v1(value: *const ManagedString) -> usize {
    unsafe { string_ref(value) }.map_or(0, crate::VutString::byte_len)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_char_len_v1(value: *const ManagedString) -> usize {
    unsafe { string_ref(value) }.map_or(0, crate::VutString::char_len)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_is_empty_v1(value: *const ManagedString) -> u8 {
    u8::from(unsafe { string_ref(value) }.is_none_or(crate::VutString::is_empty))
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-string handles for this call.
pub unsafe extern "C" fn vut_rt_string_eq_v1(
    left: *const ManagedString,
    right: *const ManagedString,
) -> u8 {
    // Null never equals a live value, and content equality is required: string
    // identity must not be substituted for value equality.
    u8::from(
        unsafe { string_ref(left) }
            .zip(unsafe { string_ref(right) })
            .is_some_and(|(left, right)| left.as_str() == right.as_str()),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-string handles for this call.
pub unsafe extern "C" fn vut_rt_string_contains_v1(
    value: *const ManagedString,
    pattern: *const ManagedString,
) -> u8 {
    u8::from(
        unsafe { string_ref(value) }
            .zip(unsafe { string_ref(pattern) })
            .is_some_and(|(value, pattern)| value.contains(pattern.as_str())),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-string handles for this call.
pub unsafe extern "C" fn vut_rt_string_starts_with_v1(
    value: *const ManagedString,
    pattern: *const ManagedString,
) -> u8 {
    u8::from(
        unsafe { string_ref(value) }
            .zip(unsafe { string_ref(pattern) })
            .is_some_and(|(value, pattern)| value.starts_with(pattern.as_str())),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-string handles for this call.
pub unsafe extern "C" fn vut_rt_string_ends_with_v1(
    value: *const ManagedString,
    pattern: *const ManagedString,
) -> u8 {
    u8::from(
        unsafe { string_ref(value) }
            .zip(unsafe { string_ref(pattern) })
            .is_some_and(|(value, pattern)| value.ends_with(pattern.as_str())),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-string handles for this call.
pub unsafe extern "C" fn vut_rt_string_find_v1(
    value: *const ManagedString,
    needle: *const ManagedString,
) -> i64 {
    unsafe { string_ref(value) }
        .zip(unsafe { string_ref(needle) })
        .and_then(|(value, needle)| value.as_str().find(needle.as_str()))
        .map_or(-1, |index| i64::try_from(index).unwrap_or(i64::MAX))
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_substring_v1(
    value: *const ManagedString,
    start: i64,
    end: i64,
) -> *mut ManagedString {
    let Some(value) = (unsafe { string_ref(value) }) else {
        return std::ptr::null_mut();
    };
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return std::ptr::null_mut();
    };
    value
        .as_str()
        .get(start..end)
        .map_or(std::ptr::null_mut(), |slice| {
            ManagedString::allocate(crate::VutString::from(slice))
        })
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed-string handles for this call.
pub unsafe extern "C" fn vut_rt_string_split_v1(
    value: *const ManagedString,
    separator: *const ManagedString,
) -> *mut ManagedList {
    let Some((value, separator)) =
        (unsafe { string_ref(value) }).zip(unsafe { string_ref(separator) })
    else {
        return std::ptr::null_mut();
    };
    let parts: Vec<String> = value
        .as_str()
        .split(separator.as_str())
        .map(str::to_owned)
        .collect();
    let list = unsafe {
        super::vut_rt_list_new_v1(
            8,
            8,
            super::ownership::vut_rt_slot_retain_string_v1 as unsafe extern "C" fn(*const u8)
                as *const (),
            super::ownership::vut_rt_slot_release_string_v1 as unsafe extern "C" fn(*mut u8)
                as *const (),
            parts.len(),
        )
    };
    if list.is_null() {
        return std::ptr::null_mut();
    }
    for part in &parts {
        let handle = ManagedString::allocate(crate::VutString::from(part.as_str()));
        let slot = std::ptr::from_ref(&handle).cast::<u8>();
        unsafe { super::vut_rt_list_push_v1(list, slot) };
        unsafe { vut_rt_release_string_v1(handle) };
    }
    list
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_trim_v1(value: *const ManagedString) -> *mut ManagedString {
    unsafe { transform_string(value, crate::VutString::trim) }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_trim_start_v1(
    value: *const ManagedString,
) -> *mut ManagedString {
    unsafe { transform_string(value, crate::VutString::trim_start) }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_trim_end_v1(
    value: *const ManagedString,
) -> *mut ManagedString {
    unsafe { transform_string(value, crate::VutString::trim_end) }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_to_lower_v1(
    value: *const ManagedString,
) -> *mut ManagedString {
    unsafe { transform_string(value, crate::VutString::to_lower) }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_to_upper_v1(
    value: *const ManagedString,
) -> *mut ManagedString {
    unsafe { transform_string(value, crate::VutString::to_upper) }
}

#[unsafe(no_mangle)]
/// # Safety
/// Every argument must be null or a live managed-string handle for this call.
pub unsafe extern "C" fn vut_rt_string_replace_v1(
    value: *const ManagedString,
    from: *const ManagedString,
    to: *const ManagedString,
) -> *mut ManagedString {
    let Some((value, from, to)) = unsafe { string_ref(value) }
        .zip(unsafe { string_ref(from) })
        .zip(unsafe { string_ref(to) })
        .map(|((value, from), to)| (value, from, to))
    else {
        return std::ptr::null_mut();
    };
    ManagedString::allocate(value.replace(from.as_str(), to.as_str()))
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_format_i64_v1(value: i64) -> *mut ManagedString {
    ManagedString::allocate(crate::VutString::from(value.to_string()))
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_format_f64_v1(value: f64) -> *mut ManagedString {
    ManagedString::allocate(crate::VutString::from(value.to_string()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `bytes` must identify a readable allocation of at least `len` bytes.
pub unsafe extern "C" fn vut_rt_string_from_utf8_v1(
    bytes: *const u8,
    len: usize,
) -> *mut ManagedString {
    if bytes.is_null() && len != 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: the compiler emits a static allocation containing at least `len` bytes.
    let bytes = unsafe { std::slice::from_raw_parts(bytes, len) };
    match std::str::from_utf8(bytes) {
        Ok(value) => ManagedString::allocate(crate::VutString::from(value)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Both handles must be null or point to live runtime-owned strings.
pub unsafe extern "C" fn vut_rt_concat_v1(
    left: *const ManagedString,
    right: *const ManagedString,
) -> *mut ManagedString {
    if left.is_null() || right.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: generated Vut code only passes live runtime-owned string handles.
    let value = unsafe { (&*left).value.concat(&(&*right).value) };
    ManagedString::allocate(value)
}

#[unsafe(no_mangle)]
/// # Safety
/// A non-null handle must identify a live managed string.
pub unsafe extern "C" fn vut_rt_retain_string_v1(value: *mut ManagedString) {
    if !value.is_null() {
        // SAFETY: callers uphold the live-handle contract.
        unsafe { &*value }
            .references
            .fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// A non-null handle must own one live reference.
pub unsafe extern "C" fn vut_rt_release_string_v1(value: *mut ManagedString) {
    if !value.is_null() {
        // SAFETY: callers uphold the live-reference contract.
        if unsafe { &*value }.references.fetch_sub(1, Ordering::AcqRel) == 1 {
            LIVE_STRINGS.fetch_sub(1, Ordering::Relaxed);
            // SAFETY: this thread observed and removed the final reference.
            drop(unsafe { Box::from_raw(value) });
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Equivalent to releasing one owned managed-string reference.
pub unsafe extern "C" fn vut_rt_drop_string_v1(value: *mut ManagedString) {
    // SAFETY: forwarded unchanged to the release operation.
    unsafe { vut_rt_release_string_v1(value) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle.
pub unsafe extern "C" fn vut_rt_string_to_i64_v1(value: *const ManagedString) -> i64 {
    unsafe { string_ref(value) }
        .and_then(|value| value.as_str().trim().parse::<i64>().ok())
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed-string handle.
pub unsafe extern "C" fn vut_rt_string_to_f64_v1(value: *const ManagedString) -> f64 {
    unsafe { string_ref(value) }
        .and_then(|value| value.as_str().trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

#[unsafe(no_mangle)]
/// # Safety
/// `code` must be a valid Unicode scalar value or it yields the empty string.
pub unsafe extern "C" fn vut_rt_char_from_code_v1(code: i64) -> *mut ManagedString {
    let text = u32::try_from(code)
        .ok()
        .and_then(char::from_u32)
        .map_or_else(String::new, |value| value.to_string());
    managed_string(&text)
}

#[unsafe(no_mangle)]
/// # Safety
/// Widens a signed 64-bit integer to an IEEE-754 double.
#[allow(clippy::cast_precision_loss)] // `int.to_float` is an intentional widening conversion
pub unsafe extern "C" fn vut_rt_int_to_float_v1(value: i64) -> f64 {
    value as f64
}
