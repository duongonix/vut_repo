//! Console I/O ABI used by generated programs.
use super::string::{ManagedString, string_ref};

#[unsafe(no_mangle)]
/// # Safety
/// `value` must point to a live runtime-owned string.
pub unsafe extern "C" fn vut_rt_print_v1(value: *const ManagedString) {
    if let Some(text) = unsafe { string_ref(value) } {
        let _ = crate::write_stdout(text.as_str(), false);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must point to a live runtime-owned string.
pub unsafe extern "C" fn vut_rt_out_v1(value: *const ManagedString) {
    if let Some(text) = unsafe { string_ref(value) } {
        let _ = crate::write_stdout(text.as_str(), true);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `prompt` must point to a live runtime-owned string.
pub unsafe extern "C" fn vut_rt_input_v1(prompt: *const ManagedString) -> *mut ManagedString {
    let Some(prompt) = (unsafe { string_ref(prompt) }) else {
        return std::ptr::null_mut();
    };
    match crate::read_line(prompt.as_str()) {
        Ok(value) => ManagedString::allocate(value),
        Err(_) => std::ptr::null_mut(),
    }
}
