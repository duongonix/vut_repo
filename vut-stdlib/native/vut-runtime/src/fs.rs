//! Filesystem primitives (`vut_rt_fs_*`).
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use vut_runtime::abi::ManagedString;
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

fn reply(result: std::io::Result<()>) -> *mut ManagedBytes {
    match result {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

fn system_time_secs(value: &std::io::Result<SystemTime>) -> i64 {
    match value {
        Ok(time) => match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
            Err(shifted) => -i64::try_from(shifted.duration().as_secs()).unwrap_or(i64::MAX),
        },
        Err(_) => i64::MIN,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_exists_v1(path: *const ManagedString) -> usize {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return 0;
    };
    usize::from(Path::new(path).exists())
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_is_file_v1(path: *const ManagedString) -> usize {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return 0;
    };
    usize::from(Path::new(path).is_file())
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_is_dir_v1(path: *const ManagedString) -> usize {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return 0;
    };
    usize::from(Path::new(path).is_dir())
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_read_v1(path: *const ManagedString) -> *mut ManagedBytes {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    match fs::read(path) {
        Ok(data) => abi::ok(&data),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_read_str_v1(path: *const ManagedString) -> *mut ManagedBytes {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    match fs::read(path) {
        Ok(data) => match String::from_utf8(data) {
            Ok(value) => abi::ok_text(&value),
            Err(_) => abi::err(abi::INVALID_DATA, "file contents are not valid UTF-8"),
        },
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle and `data` a live bytes handle.
pub unsafe extern "C" fn vut_rt_fs_write_v1(
    path: *const ManagedString,
    data: *const ManagedBytes,
) -> *mut ManagedBytes {
    let (Some(path), Some(data)) = (unsafe { abi::text(path) }, unsafe { abi::bytes(data) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path or data");
    };
    reply(fs::write(path, data))
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed string handles.
pub unsafe extern "C" fn vut_rt_fs_write_str_v1(
    path: *const ManagedString,
    data: *const ManagedString,
) -> *mut ManagedBytes {
    let (Some(path), Some(data)) = (unsafe { abi::text(path) }, unsafe { abi::text(data) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path or text");
    };
    reply(fs::write(path, data.as_bytes()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle and `data` a live bytes handle.
pub unsafe extern "C" fn vut_rt_fs_append_v1(
    path: *const ManagedString,
    data: *const ManagedBytes,
) -> *mut ManagedBytes {
    let (Some(path), Some(data)) = (unsafe { abi::text(path) }, unsafe { abi::bytes(data) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path or data");
    };
    append(path, data)
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed string handles.
pub unsafe extern "C" fn vut_rt_fs_append_str_v1(
    path: *const ManagedString,
    data: *const ManagedString,
) -> *mut ManagedBytes {
    let (Some(path), Some(data)) = (unsafe { abi::text(path) }, unsafe { abi::text(data) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path or text");
    };
    append(path, data.as_bytes())
}

fn append(path: &str, data: &[u8]) -> *mut ManagedBytes {
    use std::io::Write as _;
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => reply(file.write_all(data)),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

macro_rules! path_operation {
    ($name:ident, $method:expr) => {
        #[unsafe(no_mangle)]
        /// # Safety
        /// `path` must be null or a live managed string handle.
        pub unsafe extern "C" fn $name(path: *const ManagedString) -> *mut ManagedBytes {
            let Some(path) = (unsafe { abi::text(path) }) else {
                return abi::err(abi::INVALID_INPUT, "invalid path");
            };
            reply($method(path))
        }
    };
}

path_operation!(vut_rt_fs_create_dir_v1, fs::create_dir);
path_operation!(vut_rt_fs_create_dir_all_v1, fs::create_dir_all);
path_operation!(vut_rt_fs_remove_dir_v1, fs::remove_dir);
path_operation!(vut_rt_fs_remove_dir_all_v1, fs::remove_dir_all);
path_operation!(vut_rt_fs_remove_file_v1, fs::remove_file);

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed string handles.
pub unsafe extern "C" fn vut_rt_fs_copy_v1(
    from: *const ManagedString,
    to: *const ManagedString,
) -> *mut ManagedBytes {
    let (Some(from), Some(to)) = (unsafe { abi::text(from) }, unsafe { abi::text(to) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    match fs::copy(from, to) {
        Ok(bytes) => abi::ok(&bytes.to_le_bytes()),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Both arguments must be null or live managed string handles.
pub unsafe extern "C" fn vut_rt_fs_rename_v1(
    from: *const ManagedString,
    to: *const ManagedString,
) -> *mut ManagedBytes {
    let (Some(from), Some(to)) = (unsafe { abi::text(from) }, unsafe { abi::text(to) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    reply(fs::rename(from, to))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_read_dir_v1(path: *const ManagedString) -> *mut ManagedBytes {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => return abi::err(abi::io_status(&error), &error.to_string()),
    };
    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => names.push(entry.file_name().to_string_lossy().into_owned()),
            Err(error) => return abi::err(abi::io_status(&error), &error.to_string()),
        }
    }
    names.sort();
    abi::ok(&abi::join_fields(&names))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_len_v1(path: *const ManagedString) -> i64 {
    (unsafe { abi::text(path) })
        .and_then(|path| fs::metadata(path).ok())
        .map_or(-1, |metadata| {
            i64::try_from(metadata.len()).unwrap_or(i64::MAX)
        })
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_is_file_v1(path: *const ManagedString) -> usize {
    let Some(metadata) = (unsafe { abi::text(path) }).and_then(|path| fs::metadata(path).ok())
    else {
        return 0;
    };
    usize::from(metadata.is_file())
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_is_dir_v1(path: *const ManagedString) -> usize {
    let Some(metadata) = (unsafe { abi::text(path) }).and_then(|path| fs::metadata(path).ok())
    else {
        return 0;
    };
    usize::from(metadata.is_dir())
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_readonly_v1(path: *const ManagedString) -> usize {
    let Some(metadata) = (unsafe { abi::text(path) }).and_then(|path| fs::metadata(path).ok())
    else {
        return 0;
    };
    usize::from(metadata.permissions().readonly())
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_modified_v1(path: *const ManagedString) -> i64 {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return i64::MIN;
    };
    system_time_secs(&fs::metadata(path).and_then(|metadata| metadata.modified()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_created_v1(path: *const ManagedString) -> i64 {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return i64::MIN;
    };
    system_time_secs(&fs::metadata(path).and_then(|metadata| metadata.created()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_accessed_v1(path: *const ManagedString) -> i64 {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return i64::MIN;
    };
    system_time_secs(&fs::metadata(path).and_then(|metadata| metadata.accessed()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_set_readonly_v1(
    path: *const ManagedString,
    readonly: usize,
) -> *mut ManagedBytes {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid path");
    };
    match fs::metadata(path) {
        Ok(metadata) => {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(readonly != 0);
            reply(fs::set_permissions(path, permissions))
        }
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}
