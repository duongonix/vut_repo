//! Explicit Child control; destructors perform no process-control operations.
use super::handles::{child, error, reply};
use crate::abi;
use std::ffi::c_void;
use vut_runtime::bytes::ManagedBytes;

#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a borrowed live `ChildHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_child_ok_v1(pointer: *mut c_void) -> usize {
    usize::from(unsafe { child(pointer) }.is_some_and(|value| value.0.is_ok()))
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a borrowed live `ChildHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_child_error_v1(pointer: *mut c_void) -> *mut ManagedBytes {
    error(unsafe { child(pointer) }.map(|value| &value.0))
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a borrowed live successful `ChildHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_child_id_v1(pointer: *mut c_void) -> u32 {
    unsafe { child(pointer) }
        .and_then(|value| value.0.as_ref().ok())
        .map_or(0, std::process::Child::id)
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a borrowed live `ChildHandle` pointer, exclusively accessed.
pub unsafe extern "C" fn vut_rt_process_child_wait_v1(pointer: *mut c_void) -> *mut c_void {
    match unsafe { child(pointer) }.and_then(|value| value.0.as_mut().ok()) {
        Some(value) => reply(value.wait().map(Some)),
        None => reply(Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid Child",
        ))),
    }
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a borrowed live `ChildHandle` pointer, exclusively accessed.
pub unsafe extern "C" fn vut_rt_process_child_try_wait_v1(pointer: *mut c_void) -> *mut c_void {
    match unsafe { child(pointer) }.and_then(|value| value.0.as_mut().ok()) {
        Some(value) => reply(value.try_wait()),
        None => reply(Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid Child",
        ))),
    }
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a borrowed live `ChildHandle` pointer, exclusively accessed.
pub unsafe extern "C" fn vut_rt_process_child_kill_v1(pointer: *mut c_void) -> *mut ManagedBytes {
    let result = match unsafe { child(pointer) }.and_then(|value| value.0.as_mut().ok()) {
        Some(value) => value.kill(),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid Child",
        )),
    };
    match result {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}
