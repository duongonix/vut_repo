//! Filesystem primitives (`vut_rt_fs_*`).
use std::ffi::c_void;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use vut_runtime::abi::ManagedString;
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

unsafe extern "C" {
    fn vut_rt_resource_new_v1(
        ptr: *mut c_void,
        drop_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> *mut c_void;
}

fn reply(result: std::io::Result<()>) -> *mut ManagedBytes {
    match result {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

/// Nanoseconds since the Unix epoch, or `0` when the timestamp is unavailable
/// or precedes the epoch. `0` is the "unavailable" sentinel for file times.
fn system_time_nanos(value: &std::io::Result<SystemTime>) -> usize {
    match value {
        Ok(time) => match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => usize::try_from(duration.as_nanos()).unwrap_or(usize::MAX),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// An owned open-file handle, or a deferred open error.
///
/// The FFI cannot return `result(resource(T), E)`, so `open` always returns a
/// resource; the caller checks [`vut_rt_fs_file_ok_v1`] and reads the error with
/// [`vut_rt_fs_file_error_v1`]. The resource owns the file exactly once.
struct FileHandle {
    file: Option<std::fs::File>,
    error: Option<(i32, String)>,
}

/// Closes and frees an owned file handle resource.
///
/// # Safety
/// `ptr` must be null or a pointer produced by [`vut_rt_fs_open_v1`].
unsafe extern "C" fn drop_file_handle(ptr: *mut c_void) {
    if !ptr.is_null() {
        // SAFETY: the pointer was produced by `Box::into_raw` in `vut_rt_fs_open_v1`.
        drop(unsafe { Box::from_raw(ptr.cast::<FileHandle>()) });
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_open_v1(
    path: *const ManagedString,
    read: usize,
    write: usize,
    append: usize,
    truncate: usize,
    create: usize,
    create_new: usize,
) -> *mut c_void {
    let handle = match unsafe { abi::text(path) } {
        None => FileHandle {
            file: None,
            error: Some((abi::INVALID_INPUT, "invalid path".to_owned())),
        },
        Some(path) => {
            let mut options = fs::OpenOptions::new();
            options
                .read(read != 0)
                .write(write != 0)
                .append(append != 0)
                .truncate(truncate != 0)
                .create(create != 0)
                .create_new(create_new != 0);
            match options.open(path) {
                Ok(file) => FileHandle {
                    file: Some(file),
                    error: None,
                },
                Err(error) => FileHandle {
                    file: None,
                    error: Some((abi::io_status(&error), error.to_string())),
                },
            }
        }
    };
    let raw = Box::into_raw(Box::new(handle)).cast::<c_void>();
    unsafe { vut_rt_resource_new_v1(raw, Some(drop_file_handle)) }
}

/// Returns `1` when the handle wraps an open file, `0` when it carries an error.
#[unsafe(no_mangle)]
/// # Safety
/// `handle` must be null or a raw `FileHandle` from an adopted resource.
pub unsafe extern "C" fn vut_rt_fs_file_ok_v1(handle: *mut c_void) -> usize {
    if handle.is_null() {
        return 0;
    }
    // SAFETY: the handle is a live `FileHandle` owned by a resource.
    let handle = unsafe { &*handle.cast::<FileHandle>() };
    usize::from(handle.file.is_some())
}

/// Returns the deferred open error as a result envelope (`ok([])` when open
/// succeeded).
#[unsafe(no_mangle)]
/// # Safety
/// `handle` must be null or a raw `FileHandle` from an adopted resource.
pub unsafe extern "C" fn vut_rt_fs_file_error_v1(handle: *mut c_void) -> *mut ManagedBytes {
    if handle.is_null() {
        return abi::err(abi::INVALID_INPUT, "invalid file handle");
    }
    // SAFETY: the handle is a live `FileHandle` owned by a resource.
    let handle = unsafe { &*handle.cast::<FileHandle>() };
    match &handle.error {
        Some((code, message)) => abi::err(*code, message),
        None => abi::ok(&[]),
    }
}

/// Borrows the open file, or `None` when the handle carries an error.
///
/// # Safety
/// `handle` must be null or a live raw `FileHandle`.
unsafe fn open_file<'a>(handle: *mut c_void) -> Option<&'a mut std::fs::File> {
    if handle.is_null() {
        return None;
    }
    // SAFETY: the handle is a live `FileHandle` owned by a resource.
    unsafe { &mut *handle.cast::<FileHandle>() }.file.as_mut()
}

/// Reads one bounded chunk from an open file handle. An empty payload is EOF.
#[unsafe(no_mangle)]
/// # Safety
/// `file` must be null or a raw `FileHandle` from an adopted resource.
pub unsafe extern "C" fn vut_rt_fs_file_read_v1(file: *mut c_void) -> *mut ManagedBytes {
    use std::io::Read as _;
    const CHUNK: usize = 64 * 1024;
    let Some(file) = (unsafe { open_file(file) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid file handle");
    };
    let mut buffer = vec![0_u8; CHUNK];
    match file.read(&mut buffer) {
        Ok(read) => abi::ok(&buffer[..read]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `file` must be null or a raw `FileHandle`; `data` must be null or a live
/// managed bytes handle.
pub unsafe extern "C" fn vut_rt_fs_file_write_v1(
    file: *mut c_void,
    data: *const ManagedBytes,
) -> *mut ManagedBytes {
    use std::io::Write as _;
    let Some(file) = (unsafe { open_file(file) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid file handle");
    };
    let Some(data) = (unsafe { abi::bytes(data) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid bytes");
    };
    match file.write_all(data) {
        Ok(()) => abi::ok(&(data.len() as i64).to_le_bytes()),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `file` must be null or a raw `FileHandle`.
pub unsafe extern "C" fn vut_rt_fs_file_flush_v1(file: *mut c_void) -> *mut ManagedBytes {
    use std::io::Write as _;
    let Some(file) = (unsafe { open_file(file) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid file handle");
    };
    reply(file.flush())
}

#[unsafe(no_mangle)]
/// # Safety
/// `file` must be null or a raw `FileHandle`.
pub unsafe extern "C" fn vut_rt_fs_file_sync_v1(file: *mut c_void) -> *mut ManagedBytes {
    let Some(file) = (unsafe { open_file(file) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid file handle");
    };
    reply(file.sync_all())
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
pub unsafe extern "C" fn vut_rt_fs_metadata_len_v1(path: *const ManagedString) -> u64 {
    (unsafe { abi::text(path) })
        .and_then(|path| fs::metadata(path).ok())
        .map_or(0, |metadata| metadata.len())
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
pub unsafe extern "C" fn vut_rt_fs_metadata_modified_v1(path: *const ManagedString) -> usize {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return 0;
    };
    system_time_nanos(&fs::metadata(path).and_then(|metadata| metadata.modified()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_created_v1(path: *const ManagedString) -> usize {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return 0;
    };
    system_time_nanos(&fs::metadata(path).and_then(|metadata| metadata.created()))
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_fs_metadata_accessed_v1(path: *const ManagedString) -> usize {
    let Some(path) = (unsafe { abi::text(path) }) else {
        return 0;
    };
    system_time_nanos(&fs::metadata(path).and_then(|metadata| metadata.accessed()))
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
