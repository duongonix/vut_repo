//! Move-out of child pipe endpoints and deterministic endpoint-only cleanup.
use super::handles::{Outcome, child, error};
use crate::{abi, resource};
use std::{
    ffi::c_void,
    process::{ChildStderr, ChildStdin, ChildStdout},
};
use vut_runtime::bytes::ManagedBytes;

enum Pipe {
    Stdin(ChildStdin),
    Stdout(ChildStdout),
    Stderr(ChildStderr),
}
struct PipeHandle(Outcome<Pipe>);
unsafe fn pipe<'a>(pointer: *mut c_void) -> Option<&'a mut PipeHandle> {
    // SAFETY: caller supplies a live borrowed `PipeHandle` pointer exclusively.
    unsafe { pointer.cast::<PipeHandle>().as_mut() }
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live exclusively borrowed `ChildHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_child_pipe_v1(
    pointer: *mut c_void,
    stream: usize,
) -> *mut c_void {
    let endpoint = unsafe { child(pointer) }
        .and_then(|value| value.0.as_mut().ok())
        .and_then(|child| match stream {
            0 => child.stdin.take().map(Pipe::Stdin),
            1 => child.stdout.take().map(Pipe::Stdout),
            2 => child.stderr.take().map(Pipe::Stderr),
            _ => None,
        });
    resource::owned(PipeHandle(endpoint.ok_or_else(|| {
        (
            abi::INVALID_INPUT,
            "pipe is unavailable or already taken".into(),
        )
    })))
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live borrowed `PipeHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_pipe_ok_v1(pointer: *mut c_void) -> usize {
    usize::from(unsafe { pipe(pointer) }.is_some_and(|value| value.0.is_ok()))
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live borrowed `PipeHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_pipe_error_v1(pointer: *mut c_void) -> *mut ManagedBytes {
    error(unsafe { pipe(pointer) }.map(|value| &value.0))
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live exclusively borrowed `PipeHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_pipe_read_v1(pointer: *mut c_void) -> *mut ManagedBytes {
    match unsafe { pipe(pointer) }.and_then(|value| value.0.as_mut().ok()) {
        Some(Pipe::Stdout(value)) => crate::io::read_chunk(value),
        Some(Pipe::Stderr(value)) => crate::io::read_chunk(value),
        _ => abi::err(abi::INVALID_INPUT, "pipe is not readable"),
    }
}
#[unsafe(no_mangle)]
/// # Safety
/// Both handles must be live; `pointer` is exclusively borrowed.
pub unsafe extern "C" fn vut_rt_process_pipe_write_v2(
    pointer: *mut c_void,
    data: *const ManagedBytes,
) -> *mut c_void {
    let Some(data) = (unsafe { abi::bytes(data) }) else {
        return crate::count::invalid("invalid bytes");
    };
    match unsafe { pipe(pointer) }.and_then(|value| value.0.as_mut().ok()) {
        Some(Pipe::Stdin(value)) => crate::io::write_bytes(value, data),
        _ => crate::count::invalid("pipe is not writable"),
    }
}
#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live exclusively borrowed `PipeHandle` pointer.
pub unsafe extern "C" fn vut_rt_process_pipe_flush_v1(pointer: *mut c_void) -> *mut ManagedBytes {
    use std::io::Write as _;
    match unsafe { pipe(pointer) }.and_then(|value| value.0.as_mut().ok()) {
        Some(Pipe::Stdin(value)) => match value.flush() {
            Ok(()) => abi::ok(&[]),
            Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
        },
        _ => abi::err(abi::INVALID_INPUT, "pipe is not writable"),
    }
}
