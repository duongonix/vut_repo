//! Native command configuration and spawn/status/output operations.
use super::handles::{ChildHandle, Reply, ReplyHandle, failure};
use crate::{abi, resource};
use std::{
    ffi::c_void,
    io,
    process::{Command, Stdio},
};
use vut_runtime::abi::{ManagedList, ManagedString};

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

fn stdio(mode: usize) -> io::Result<Stdio> {
    match mode {
        0 => Ok(Stdio::inherit()),
        1 => Ok(Stdio::null()),
        2 => Ok(Stdio::piped()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid Stdio mode",
        )),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "flat versioned C ABI command configuration"
)]
unsafe fn configured(
    program: *const ManagedString,
    args: *const ManagedList,
    environment: *const ManagedList,
    removed: *const ManagedList,
    cwd: *const ManagedString,
    stdin: usize,
    stdout: usize,
    stderr: usize,
) -> io::Result<Command> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "invalid command configuration");
    let program = unsafe { abi::text(program) }.ok_or_else(invalid)?;
    let arguments = unsafe { read_arguments(args) }.ok_or_else(invalid)?;
    let environment = unsafe { read_arguments(environment) }.ok_or_else(invalid)?;
    let removed = unsafe { read_arguments(removed) }.ok_or_else(invalid)?;
    let cwd = unsafe { abi::text(cwd) }.ok_or_else(invalid)?;
    if environment.len() % 2 != 0 {
        return Err(invalid());
    }
    let mut command = Command::new(program);
    command.args(arguments);
    for key in removed {
        command.env_remove(key);
    }
    for pair in environment.as_chunks::<2>().0 {
        command.env(&pair[0], &pair[1]);
    }
    if !cwd.is_empty() {
        command.current_dir(cwd);
    }
    // 3 preserves std::process defaults: inherit for spawn/status, capture for output.
    if stdin != 3 {
        command.stdin(stdio(stdin)?);
    }
    if stdout != 3 {
        command.stdout(stdio(stdout)?);
    }
    if stderr != 3 {
        command.stderr(stdio(stderr)?);
    }
    Ok(command)
}

macro_rules! operation {
    ($name:ident, $body:expr) => {
        #[unsafe(no_mangle)]
        /// # Safety
        /// Strings and list(str) arguments must be live borrowed managed handles.
        pub unsafe extern "C" fn $name(
            program: *const ManagedString,
            args: *const ManagedList,
            environment: *const ManagedList,
            removed: *const ManagedList,
            cwd: *const ManagedString,
            stdin: usize,
            stdout: usize,
            stderr: usize,
        ) -> *mut c_void {
            let command = unsafe {
                configured(
                    program,
                    args,
                    environment,
                    removed,
                    cwd,
                    stdin,
                    stdout,
                    stderr,
                )
            };
            ($body)(command)
        }
    };
}
operation!(vut_rt_process_command_spawn_v1, |command: io::Result<
    Command,
>| {
    resource::owned(ChildHandle(
        command
            .and_then(|mut c| c.spawn())
            .map_err(|error| failure(&error)),
    ))
});
operation!(vut_rt_process_command_status_v1, |command: io::Result<
    Command,
>| {
    super::handles::reply(
        command
            .and_then(|mut c| c.spawn()?.wait_with_output())
            .map(|output| Some(output.status)),
    )
});
operation!(vut_rt_process_command_output_v1, |command: io::Result<
    Command,
>| {
    resource::owned(ReplyHandle(
        command
            .and_then(|mut c| c.output())
            .map(|output| Reply {
                status: Some(output.status),
                stdout: output.stdout,
                stderr: output.stderr,
            })
            .map_err(|error| failure(&error)),
    ))
});
