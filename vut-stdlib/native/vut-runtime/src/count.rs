//! Fallible byte counts: exact u64 C scalars, canonical owned resource cleanup.
use crate::{abi, resource};
use std::{ffi::c_void, io};
use vut_runtime::bytes::ManagedBytes;

struct Count(Result<u64, (i32, String)>);

pub(crate) fn reply(value: io::Result<u64>) -> *mut c_void {
    resource::owned(Count(
        value.map_err(|error| (abi::io_status(&error), error.to_string())),
    ))
}

pub(crate) fn invalid(message: &str) -> *mut c_void {
    reply(Err(io::Error::new(io::ErrorKind::InvalidInput, message)))
}

unsafe fn borrow<'a>(pointer: *const c_void) -> Option<&'a Count> {
    // SAFETY: the caller borrows a live resource payload for this call.
    unsafe { pointer.cast::<Count>().as_ref() }
}

#[unsafe(no_mangle)]
/// # Safety
/// A non-null pointer must borrow a live Count resource payload.
pub unsafe extern "C" fn vut_rt_count_ok_v1(pointer: *const c_void) -> usize {
    usize::from(unsafe { borrow(pointer) }.is_some_and(|value| value.0.is_ok()))
}

#[unsafe(no_mangle)]
/// # Safety
/// A non-null pointer must borrow a live Count resource payload.
pub unsafe extern "C" fn vut_rt_count_value_v1(pointer: *const c_void) -> u64 {
    unsafe { borrow(pointer) }
        .and_then(|value| value.0.as_ref().ok())
        .copied()
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
/// # Safety
/// A non-null pointer must borrow a live Count resource payload.
pub unsafe extern "C" fn vut_rt_count_error_v1(pointer: *const c_void) -> *mut ManagedBytes {
    match unsafe { borrow(pointer) }.map(|value| &value.0) {
        Some(Err((status, message))) => abi::err(*status, message),
        Some(Ok(_)) => abi::ok(&[]),
        None => abi::err(abi::INVALID_INPUT, "invalid byte count reply"),
    }
}
