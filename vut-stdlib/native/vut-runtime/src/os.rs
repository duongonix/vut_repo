//! Operating-system information primitives (`vut_rt_os_*`).
use std::path::Path;

use vut_runtime::abi::ManagedString;
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_name_v1() -> *mut ManagedString {
    abi::managed(std::env::consts::OS)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_arch_v1() -> *mut ManagedString {
    abi::managed(std::env::consts::ARCH)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_family_v1() -> *mut ManagedString {
    abi::managed(if cfg!(windows) { "windows" } else { "unix" })
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_home_dir_v1() -> *mut ManagedBytes {
    home_dir().map_or_else(
        || abi::err(abi::NOT_FOUND, "home directory is not available"),
        |path| abi::ok_text(&path.to_string_lossy()),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_temp_dir_v1() -> *mut ManagedBytes {
    match std::env::temp_dir().to_str() {
        Some(path) => abi::ok_text(path),
        None => abi::err(abi::INVALID_DATA, "temporary directory is not valid UTF-8"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_current_dir_v1() -> *mut ManagedBytes {
    match std::env::current_dir() {
        Ok(path) => abi::ok_text(&path.to_string_lossy()),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_os_set_current_dir_v1(
    path: *const ManagedString,
) -> *mut ManagedBytes {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    match std::env::set_current_dir(Path::new(path)) {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_current_exe_v1() -> *mut ManagedBytes {
    match std::env::current_exe() {
        Ok(path) => abi::ok_text(&path.to_string_lossy()),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_os_cpu_count_v1() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}
