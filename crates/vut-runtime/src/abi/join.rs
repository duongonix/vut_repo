//! `list[str].join(separator)` — concatenates managed strings with a separator.
use super::{ManagedList, ManagedString};

/// Joins a managed `list[str]` with `separator` between elements.
///
/// # Safety
/// `list` must be null or a live managed list whose element storage holds
/// managed-string handles; `separator` must be null or a live managed string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_list_join_v1(
    list: *const ManagedList,
    separator: *const ManagedString,
) -> *mut ManagedString {
    let separator = unsafe { super::string::string_ref(separator) }
        .map(|value| value.as_str().to_owned())
        .unwrap_or_default();
    let length = unsafe { super::list::vut_rt_list_len_v1(list) };
    let mut result = String::new();
    for index in 0..length {
        if index != 0 {
            result.push_str(&separator);
        }
        // `list_at` copies the element handle into `element` and retains it, so
        // this borrows one reference that must be released after use.
        let mut element: *mut ManagedString = std::ptr::null_mut();
        let copied = unsafe {
            super::list::vut_rt_list_at_v1(
                list,
                index,
                std::ptr::addr_of_mut!(element).cast::<u8>(),
            )
        };
        if copied == 0 || element.is_null() {
            continue;
        }
        if let Some(text) = unsafe { super::string::string_ref(element) } {
            result.push_str(text.as_str());
        }
        unsafe { super::string::vut_rt_release_string_v1(element) };
    }
    super::string::managed_string(&result)
}
