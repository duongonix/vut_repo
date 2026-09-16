//! Child-process primitives (`vut_rt_process_*`).
//!
//! v1 exposes synchronous execution plus current-process identity/exit.
//! Asynchronous child handles and pipes are deferred until the language gains a
//! managed native-resource type with deterministic cleanup.
use std::path::Path;
use std::process::Command;

use vut_runtime::abi::{ManagedList, ManagedString};
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

unsafe fn read_arguments(list: *const ManagedList) -> Option<Vec<String>> {
    if list.is_null() {
        return Some(Vec::new());
    }
    let length = unsafe { vut_runtime::abi::vut_rt_list_len_v1(list) };
    let mut arguments = Vec::with_capacity(length);
    for index in 0..length {
        let mut handle: *mut ManagedString = std::ptr::null_mut();
        let copied = unsafe {
            vut_runtime::abi::vut_rt_list_at_v1(
                list,
                index,
                std::ptr::from_mut(&mut handle).cast::<u8>(),
            )
        };
        if copied == 0 {
            return None;
        }
        let value = unsafe { handle.as_ref() }?.as_str().to_owned();
        unsafe { vut_runtime::abi::vut_rt_release_string_v1(handle) };
        arguments.push(value);
    }
    Some(arguments)
}

unsafe fn build_command(
    program: *const ManagedString,
    args: *const ManagedList,
    cwd: *const ManagedString,
) -> Option<Command> {
    let program = unsafe { abi::text(program) }?;
    let arguments = unsafe { read_arguments(args) }?;
    let mut command = Command::new(program);
    command.args(arguments);
    if let Some(cwd) = unsafe { abi::text(cwd) }
        && !cwd.is_empty()
    {
        command.current_dir(Path::new(cwd));
    }
    Some(command)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_process_id_v1() -> u32 {
    std::process::id()
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_process_exit_v1(code: i32) -> ! {
    std::process::exit(code)
}

#[unsafe(no_mangle)]
/// # Safety
/// `program`/`cwd` must be null or live managed string handles and `args` a
/// live `list(str)` handle.
pub unsafe extern "C" fn vut_rt_process_status_v1(
    program: *const ManagedString,
    args: *const ManagedList,
    cwd: *const ManagedString,
) -> *mut ManagedBytes {
    let Some(mut command) = (unsafe { build_command(program, args, cwd) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid command");
    };
    match command.status() {
        Ok(status) => abi::ok(&status.code().unwrap_or(-1).to_le_bytes()),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

/// Runs the program and returns `[exit_code i32][stdout_len u32][stdout][stderr]`.
#[unsafe(no_mangle)]
/// # Safety
/// `program`/`cwd` must be null or live managed string handles and `args` a
/// live `list(str)` handle.
pub unsafe extern "C" fn vut_rt_process_output_v1(
    program: *const ManagedString,
    args: *const ManagedList,
    cwd: *const ManagedString,
) -> *mut ManagedBytes {
    let Some(mut command) = (unsafe { build_command(program, args, cwd) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid command");
    };
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return abi::err(abi::io_status(&error), &error.to_string()),
    };
    let exit = output.status.code().unwrap_or(-1);
    let stdout_len = u32::try_from(output.stdout.len()).unwrap_or(u32::MAX);
    let mut payload = Vec::with_capacity(8 + output.stdout.len() + output.stderr.len());
    payload.extend_from_slice(&exit.to_le_bytes());
    payload.extend_from_slice(&stdout_len.to_le_bytes());
    payload.extend_from_slice(&output.stdout);
    payload.extend_from_slice(&output.stderr);
    abi::ok(&payload)
}
