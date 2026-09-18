//! Managed map storage and its C ABI operations.
use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::LIVE_MAPS;
use super::string::{ManagedString, string_ref};

const MAP_KEY_STRING: usize = 1;

type RetainFn = unsafe extern "C" fn(*const u8);
type ReleaseFn = unsafe extern "C" fn(*mut u8);

#[repr(C)]
pub struct ManagedMap {
    references: AtomicUsize,
    key_size: usize,
    key_align: usize,
    value_size: usize,
    value_align: usize,
    key_kind: usize,
    value_retain: Option<RetainFn>,
    value_release: Option<ReleaseFn>,
    entries: HashMap<Vec<u8>, Vec<u8>>,
}

unsafe impl Send for ManagedMap {}
unsafe impl Sync for ManagedMap {}

impl ManagedMap {
    #[expect(
        clippy::too_many_arguments,
        reason = "map allocation mirrors key/value layout and ownership callbacks"
    )]
    fn allocate(
        key_size: usize,
        key_align: usize,
        value_size: usize,
        value_align: usize,
        key_kind: usize,
        value_retain: *const (),
        value_release: *const (),
        capacity: usize,
    ) -> *mut Self {
        LIVE_MAPS.fetch_add(1, Ordering::Relaxed);
        Box::into_raw(Box::new(Self {
            references: AtomicUsize::new(1),
            key_size: key_size.max(1),
            key_align: key_align.max(1),
            value_size: value_size.max(1),
            value_align: value_align.max(1),
            key_kind,
            value_retain: unsafe { retain_fn(value_retain) },
            value_release: unsafe { release_fn(value_release) },
            entries: HashMap::with_capacity(capacity),
        }))
    }

    unsafe fn read_key(&self, key: *const u8) -> Option<Vec<u8>> {
        if key.is_null() {
            return None;
        }
        if self.key_kind == MAP_KEY_STRING {
            let handle = unsafe { std::ptr::read_unaligned(key.cast::<*const ManagedString>()) };
            return unsafe { string_ref(handle) }.map(|value| value.as_str().as_bytes().to_vec());
        }
        Some(unsafe { std::slice::from_raw_parts(key, self.key_size).to_vec() })
    }

    unsafe fn read_value(&self, value: *const u8) -> Option<Vec<u8>> {
        (!value.is_null())
            .then(|| unsafe { std::slice::from_raw_parts(value, self.value_size).to_vec() })
    }

    unsafe fn retain_value(&self, slot: *const u8) {
        if let Some(retain) = self.value_retain {
            unsafe { retain(slot) };
        }
    }

    unsafe fn release_value(&self, slot: *mut u8) {
        if let Some(release) = self.value_release {
            unsafe { release(slot) };
        }
    }
}

unsafe fn retain_fn(pointer: *const ()) -> Option<RetainFn> {
    if pointer.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*const (), RetainFn>(pointer) })
    }
}

unsafe fn release_fn(pointer: *const ()) -> Option<ReleaseFn> {
    if pointer.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute::<*const (), ReleaseFn>(pointer) })
    }
}

impl Drop for ManagedMap {
    fn drop(&mut self) {
        for value in self.entries.values() {
            unsafe { self.release_value(value.as_ptr().cast_mut()) };
        }
        LIVE_MAPS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// Sizes/alignment must describe the key and value types. `value_retain` and
/// `value_release` are optional compiler-generated callbacks for managed values
/// and must be null for trivial values.
pub unsafe extern "C" fn vut_rt_map_new_v1(
    key_size: usize,
    key_align: usize,
    value_size: usize,
    value_align: usize,
    key_kind: usize,
    value_retain: *const (),
    value_release: *const (),
    capacity: usize,
) -> *mut ManagedMap {
    ManagedMap::allocate(
        key_size,
        key_align,
        value_size,
        value_align,
        key_kind,
        value_retain,
        value_release,
        capacity,
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle.
pub unsafe extern "C" fn vut_rt_map_retain_v1(map: *mut ManagedMap) {
    if !map.is_null() {
        unsafe { &*map }.references.fetch_add(1, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or own one live managed-map reference.
pub unsafe extern "C" fn vut_rt_map_release_v1(map: *mut ManagedMap) {
    if !map.is_null() && unsafe { &*map }.references.fetch_sub(1, Ordering::AcqRel) == 1 {
        drop(unsafe { Box::from_raw(map) });
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle.
pub unsafe extern "C" fn vut_rt_map_len_v1(map: *const ManagedMap) -> usize {
    if map.is_null() {
        0
    } else {
        unsafe { (*map).entries.len() }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle.
pub unsafe extern "C" fn vut_rt_map_is_empty_v1(map: *const ManagedMap) -> u8 {
    u8::from(unsafe { vut_rt_map_len_v1(map) } == 0)
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle.
pub unsafe extern "C" fn vut_rt_map_capacity_v1(map: *const ManagedMap) -> usize {
    if map.is_null() {
        0
    } else {
        unsafe { (*map).entries.capacity() }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle.
pub unsafe extern "C" fn vut_rt_map_reserve_v1(map: *mut ManagedMap, capacity: usize) {
    if !map.is_null() {
        let map_ref = unsafe { &mut *map };
        if capacity > map_ref.entries.capacity() {
            map_ref.entries.reserve(capacity - map_ref.entries.len());
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be live, `key` readable for key size, and `out` writable for value size.
pub unsafe extern "C" fn vut_rt_map_get_v1(
    map: *const ManagedMap,
    key: *const u8,
    out: *mut u8,
) -> u8 {
    if map.is_null() || key.is_null() || out.is_null() {
        return 0;
    }
    let map_ref = unsafe { &*map };
    let Some(key) = (unsafe { map_ref.read_key(key) }) else {
        return 0;
    };
    let Some(value) = map_ref.entries.get(&key) else {
        return 0;
    };
    unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), out, map_ref.value_size) };
    unsafe { map_ref.retain_value(out) };
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be live, `key` readable for key size, and `value` readable for value size.
pub unsafe extern "C" fn vut_rt_map_set_v1(
    map: *mut ManagedMap,
    key: *const u8,
    value: *const u8,
) -> u8 {
    if map.is_null() || key.is_null() || value.is_null() {
        return 0;
    }
    let map_ref = unsafe { &mut *map };
    let Some(key) = (unsafe { map_ref.read_key(key) }) else {
        return 0;
    };
    let Some(value) = (unsafe { map_ref.read_value(value) }) else {
        return 0;
    };
    unsafe { map_ref.retain_value(value.as_ptr()) };
    if let Some(old) = map_ref.entries.insert(key, value) {
        unsafe { map_ref.release_value(old.as_ptr().cast_mut()) };
    }
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be live and `key` readable for key size.
pub unsafe extern "C" fn vut_rt_map_contains_key_v1(map: *const ManagedMap, key: *const u8) -> u8 {
    if map.is_null() || key.is_null() {
        return 0;
    }
    let map_ref = unsafe { &*map };
    let Some(key) = (unsafe { map_ref.read_key(key) }) else {
        return 0;
    };
    u8::from(map_ref.entries.contains_key(&key))
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be live, `key` readable for key size, and `out` writable for value size.
pub unsafe extern "C" fn vut_rt_map_remove_v1(
    map: *mut ManagedMap,
    key: *const u8,
    out: *mut u8,
) -> u8 {
    if map.is_null() || key.is_null() || out.is_null() {
        return 0;
    }
    let map_ref = unsafe { &mut *map };
    let Some(key) = (unsafe { map_ref.read_key(key) }) else {
        return 0;
    };
    let Some(value) = map_ref.entries.remove(&key) else {
        return 0;
    };
    unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), out, map_ref.value_size) };
    1
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle.
pub unsafe extern "C" fn vut_rt_map_clear_v1(map: *mut ManagedMap) {
    if !map.is_null() {
        let map_ref = unsafe { &mut *map };
        for value in map_ref.entries.values() {
            unsafe { map_ref.release_value(value.as_ptr().cast_mut()) };
        }
        map_ref.entries.clear();
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle. Returns a list of the
/// map's keys in the key element layout (string keys are rebuilt as handles).
/// The returned list owns one reference.
pub unsafe extern "C" fn vut_rt_map_keys_v1(
    map: *const ManagedMap,
) -> *mut super::list::ManagedList {
    if map.is_null() {
        return unsafe { super::list::build_string_list(Vec::new()) };
    }
    let map_ref = unsafe { &*map };
    if map_ref.key_kind == MAP_KEY_STRING {
        let keys: Vec<String> = map_ref
            .entries
            .keys()
            .map(|key| String::from_utf8_lossy(key).into_owned())
            .collect();
        return unsafe { super::list::build_string_list(keys) };
    }
    let list = unsafe {
        super::vut_rt_list_new_v1(
            map_ref.key_size,
            map_ref.key_align,
            std::ptr::null(),
            std::ptr::null(),
            map_ref.entries.len(),
        )
    };
    if list.is_null() {
        return std::ptr::null_mut();
    }
    for key in map_ref.entries.keys() {
        unsafe { super::vut_rt_list_push_v1(list, key.as_ptr()) };
    }
    list
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be null or a live managed-map handle. Returns a list of the
/// map's values in the value element layout. The returned list owns one
/// reference; managed values are retained into it.
pub unsafe extern "C" fn vut_rt_map_values_v1(
    map: *const ManagedMap,
) -> *mut super::list::ManagedList {
    if map.is_null() {
        return unsafe { super::list::build_string_list(Vec::new()) };
    }
    let map_ref = unsafe { &*map };
    let retain = map_ref
        .value_retain
        .map_or(std::ptr::null(), |retain| retain as *const ());
    let release = map_ref
        .value_release
        .map_or(std::ptr::null(), |release| release as *const ());
    let list = unsafe {
        super::vut_rt_list_new_v1(
            map_ref.value_size,
            map_ref.value_align,
            retain,
            release,
            map_ref.entries.len(),
        )
    };
    if list.is_null() {
        return std::ptr::null_mut();
    }
    for value in map_ref.entries.values() {
        unsafe { super::vut_rt_list_push_v1(list, value.as_ptr()) };
    }
    list
}

#[unsafe(no_mangle)]
/// # Safety
/// `map` must be live, `key` readable for key size, `out` writable for value
/// size, and `default` readable for value size. Returns the mapped value, or
/// `default` when the key is absent.
pub unsafe extern "C" fn vut_rt_map_get_or_v1(
    map: *const ManagedMap,
    key: *const u8,
    out: *mut u8,
    default: *const u8,
) -> u8 {
    if map.is_null() || key.is_null() || out.is_null() || default.is_null() {
        return 0;
    }
    let map_ref = unsafe { &*map };
    let Some(key) = (unsafe { map_ref.read_key(key) }) else {
        return 0;
    };
    let source = map_ref.entries.get(&key).map_or(default, Vec::as_ptr);
    unsafe { std::ptr::copy_nonoverlapping(source, out, map_ref.value_size) };
    unsafe { map_ref.retain_value(out) };
    1
}
