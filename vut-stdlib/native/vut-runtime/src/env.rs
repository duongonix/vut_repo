//! Environment and process-argument primitives (`vut_rt_env_*`).
use vut_runtime::abi::ManagedString;
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

#[unsafe(no_mangle)]
/// # Safety
/// `name` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_env_get_v1(name: *const ManagedString) -> *mut ManagedBytes {
    let Some(name) = (unsafe { abi::text(name) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid environment variable name");
    };
    match std::env::var(name) {
        Ok(value) => abi::ok_text(&value),
        Err(std::env::VarError::NotPresent) => {
            abi::err(abi::NOT_FOUND, "environment variable is not set")
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            abi::err(abi::INVALID_DATA, "environment variable is not valid UTF-8")
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `name` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_env_has_v1(name: *const ManagedString) -> usize {
    usize::from((unsafe { abi::text(name) }).is_some_and(|name| std::env::var_os(name).is_some()))
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed string handles.
pub unsafe extern "C" fn vut_rt_env_set_v1(
    name: *const ManagedString,
    value: *const ManagedString,
) -> *mut ManagedBytes {
    let (Some(name), Some(value)) = (unsafe { abi::text(name) }, unsafe { abi::text(value) })
    else {
        return abi::err(
            abi::INVALID_INPUT,
            "invalid environment variable name or value",
        );
    };
    // SAFETY: environment mutation is confined to the runtime's single caller.
    unsafe { std::env::set_var(name, value) };
    abi::ok(&[])
}

#[unsafe(no_mangle)]
/// # Safety
/// `name` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_env_remove_v1(name: *const ManagedString) -> *mut ManagedBytes {
    let Some(name) = (unsafe { abi::text(name) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid environment variable name");
    };
    // SAFETY: see `vut_rt_env_set_v1`.
    unsafe { std::env::remove_var(name) };
    abi::ok(&[])
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_env_all_v1() -> *mut ManagedBytes {
    let mut entries: Vec<(String, String)> = std::env::vars_os()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect();
    entries.sort();
    let fields: Vec<String> = entries
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    abi::ok(&abi::join_fields(&fields))
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_env_args_v1() -> *mut ManagedBytes {
    let fields: Vec<String> = std::env::args_os()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();
    abi::ok(&abi::join_fields(&fields))
}
