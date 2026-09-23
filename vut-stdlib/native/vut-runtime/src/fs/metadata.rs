//! Owned filesystem metadata snapshots and fallible, signed Unix timestamps.
use std::ffi::c_void;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use vut_runtime::abi::ManagedString;
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

struct Snapshot {
    metadata: io::Result<std::fs::Metadata>,
    times: [io::Result<i64>; 3],
}

fn timestamp(time: io::Result<SystemTime>) -> io::Result<i64> {
    let time = time?;
    let nanos = match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => i128::try_from(duration.as_nanos()),
        Err(error) => i128::try_from(error.duration().as_nanos()).map(|nanos| -nanos),
    }
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "timestamp is out of range"))?;
    i64::try_from(nanos)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "timestamp is out of range"))
}

pub(super) fn owned(metadata: io::Result<std::fs::Metadata>) -> *mut c_void {
    let times = match &metadata {
        Ok(value) => [
            timestamp(value.modified()),
            timestamp(value.created()),
            timestamp(value.accessed()),
        ],
        Err(_) => std::array::from_fn(|_| Err(io::Error::other("metadata unavailable"))),
    };
    crate::resource::owned(Snapshot { metadata, times })
}

// SAFETY: only adopted Snapshot resources may be borrowed through this module.
unsafe fn snapshot<'a>(handle: *mut c_void) -> Option<&'a Snapshot> {
    unsafe { handle.cast::<Snapshot>().as_ref() }
}

#[unsafe(no_mangle)]
/// # Safety
/// `path` must be null or a live managed string.
pub unsafe extern "C" fn vut_rt_fs_metadata_v2(path: *const ManagedString) -> *mut c_void {
    owned(match unsafe { abi::text(path) } {
        Some(path) => std::fs::metadata(path),
        None => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid path")),
    })
}

#[unsafe(no_mangle)]
/// # Safety
/// `handle` must be null or a borrowed live Snapshot resource.
pub unsafe extern "C" fn vut_rt_fs_metadata_ok_v2(handle: *mut c_void) -> usize {
    usize::from(unsafe { snapshot(handle) }.is_some_and(|value| value.metadata.is_ok()))
}

fn error_reply(result: io::Result<()>) -> *mut ManagedBytes {
    match result {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `handle` must be null or a borrowed live Snapshot resource.
pub unsafe extern "C" fn vut_rt_fs_metadata_error_v2(handle: *mut c_void) -> *mut ManagedBytes {
    let result = unsafe { snapshot(handle) }.map_or_else(
        || {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid metadata handle",
            ))
        },
        |value| {
            value
                .metadata
                .as_ref()
                .map(|_| ())
                .map_err(|error| io::Error::new(error.kind(), error.to_string()))
        },
    );
    error_reply(result)
}

macro_rules! getter {
    ($name:ident, $ty:ty, $get:expr) => {
        #[unsafe(no_mangle)]
        /// # Safety
        /// `handle` must be null or a borrowed live Snapshot resource.
        pub unsafe extern "C" fn $name(handle: *mut c_void) -> $ty {
            unsafe { snapshot(handle) }
                .and_then(|value| value.metadata.as_ref().ok())
                .map_or(0, $get)
        }
    };
}
getter!(vut_rt_fs_metadata_len_v2, u64, std::fs::Metadata::len);
getter!(vut_rt_fs_metadata_is_file_v2, usize, |value| usize::from(
    value.is_file()
));
getter!(vut_rt_fs_metadata_is_dir_v2, usize, |value| usize::from(
    value.is_dir()
));
getter!(vut_rt_fs_metadata_readonly_v2, usize, |value| usize::from(
    value.permissions().readonly()
));

#[unsafe(no_mangle)]
/// # Safety
/// `handle` must be null or a borrowed live Snapshot resource.
pub unsafe extern "C" fn vut_rt_fs_metadata_time_v2(handle: *mut c_void, index: usize) -> i64 {
    unsafe { snapshot(handle) }
        .and_then(|value| value.times.get(index))
        .and_then(|value| value.as_ref().ok())
        .copied()
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
/// # Safety
/// `handle` must be null or a borrowed live Snapshot resource.
pub unsafe extern "C" fn vut_rt_fs_metadata_time_error_v2(
    handle: *mut c_void,
    index: usize,
) -> *mut ManagedBytes {
    let result = unsafe { snapshot(handle) }
        .and_then(|value| value.times.get(index))
        .map_or_else(
            || {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid timestamp selector",
                ))
            },
            |value| {
                value
                    .as_ref()
                    .map(|_| ())
                    .map_err(|error| io::Error::new(error.kind(), error.to_string()))
            },
        );
    error_reply(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_pre_epoch_are_values_not_error_sentinels() {
        assert_eq!(timestamp(Ok(UNIX_EPOCH)).unwrap(), 0);
        assert_eq!(
            timestamp(Ok(UNIX_EPOCH - std::time::Duration::from_nanos(100))).unwrap(),
            -100
        );
        assert!(timestamp(Err(io::Error::from(io::ErrorKind::Unsupported))).is_err());
    }
}
