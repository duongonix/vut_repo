//! Fallible process handles and direct-width reply accessors.
use crate::{abi, resource};
use std::{
    ffi::c_void,
    process::{Child, ExitStatus},
};
use vut_runtime::bytes::ManagedBytes;

pub(super) type Outcome<T> = Result<T, (i32, String)>;
pub(super) struct ChildHandle(pub Outcome<Child>);
// Rust Child drop closes owned handles/pipes; it never kills or waits.
pub(super) struct Reply {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub(super) struct ReplyHandle(pub Outcome<Reply>);

pub(super) fn failure(error: &std::io::Error) -> (i32, String) {
    (abi::io_status(error), error.to_string())
}
pub(super) fn reply(result: std::io::Result<Option<ExitStatus>>) -> *mut c_void {
    resource::owned(ReplyHandle(
        result
            .map(|status| Reply {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
            .map_err(|error| failure(&error)),
    ))
}
pub(super) unsafe fn child<'a>(pointer: *mut c_void) -> Option<&'a mut ChildHandle> {
    // SAFETY: only called with a borrowed live `ChildHandle` resource.
    unsafe { pointer.cast::<ChildHandle>().as_mut() }
}
unsafe fn response<'a>(pointer: *mut c_void) -> Option<&'a ReplyHandle> {
    // SAFETY: only called with a borrowed live ReplyHandle resource.
    unsafe { pointer.cast::<ReplyHandle>().as_ref() }
}
pub(super) fn error<T>(result: Option<&Outcome<T>>) -> *mut ManagedBytes {
    match result {
        Some(Err((code, message))) => abi::err(*code, message),
        Some(Ok(_)) => abi::ok(&[]),
        None => abi::err(abi::INVALID_INPUT, "invalid process handle"),
    }
}

macro_rules! accessor {
    ($name:ident, $return:ty, $fallback:expr, $body:expr) => {
        #[unsafe(no_mangle)]
        /// # Safety
        /// `pointer` must be a live borrowed `ReplyHandle` resource pointer.
        pub unsafe extern "C" fn $name(pointer: *mut c_void) -> $return {
            match unsafe { response(pointer) }.and_then(|value| value.0.as_ref().ok()) {
                Some(value) => ($body)(value),
                None => $fallback,
            }
        }
    };
}
accessor!(vut_rt_process_reply_ok_v1, usize, 0, |_: &Reply| 1);
accessor!(vut_rt_process_reply_ready_v1, usize, 0, |value: &Reply| {
    usize::from(value.status.is_some())
});
accessor!(
    vut_rt_process_reply_success_v1,
    usize,
    0,
    |value: &Reply| usize::from(value.status.is_some_and(|s| s.success()))
);
accessor!(
    vut_rt_process_reply_has_code_v1,
    usize,
    0,
    |value: &Reply| usize::from(value.status.and_then(|s| s.code()).is_some())
);
accessor!(vut_rt_process_reply_code_v1, i32, 0, |value: &Reply| value
    .status
    .and_then(|s| s.code())
    .unwrap_or(0));
accessor!(
    vut_rt_process_reply_stdout_v1,
    *mut ManagedBytes,
    std::ptr::null_mut(),
    |value: &Reply| vut_runtime::bytes::managed_bytes(value.stdout.clone())
);
accessor!(
    vut_rt_process_reply_stderr_v1,
    *mut ManagedBytes,
    std::ptr::null_mut(),
    |value: &Reply| vut_runtime::bytes::managed_bytes(value.stderr.clone())
);
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live borrowed `ReplyHandle` resource pointer.
pub unsafe extern "C" fn vut_rt_process_reply_error_v1(pointer: *mut c_void) -> *mut ManagedBytes {
    error(unsafe { response(pointer) }.map(|value| &value.0))
}
