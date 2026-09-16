//! Slot-level ownership callbacks for runtime-internal collection construction.
//!
//! The compiler generates per-type retain/release callbacks for managed
//! collection elements. These fixed callbacks cover the primitive managed
//! kinds that the runtime itself stores (for example `string.split`).
use super::list::{ManagedList, vut_rt_list_release_v1, vut_rt_list_retain_v1};
use super::map::{ManagedMap, vut_rt_map_release_v1, vut_rt_map_retain_v1};
use super::string::{ManagedString, vut_rt_release_string_v1, vut_rt_retain_string_v1};
use crate::bytes::{ManagedBytes, vut_rt_bytes_release_v1, vut_rt_bytes_retain_v1};

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-string handle.
pub unsafe extern "C" fn vut_rt_slot_retain_string_v1(slot: *const u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedString>()) };
    unsafe { vut_rt_retain_string_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-string handle.
pub unsafe extern "C" fn vut_rt_slot_release_string_v1(slot: *mut u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedString>()) };
    unsafe { vut_rt_release_string_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-list handle.
pub unsafe extern "C" fn vut_rt_slot_retain_list_v1(slot: *const u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedList>()) };
    unsafe { vut_rt_list_retain_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-list handle.
pub unsafe extern "C" fn vut_rt_slot_release_list_v1(slot: *mut u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedList>()) };
    unsafe { vut_rt_list_release_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-bytes handle.
pub unsafe extern "C" fn vut_rt_slot_retain_bytes_v1(slot: *const u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedBytes>()) };
    unsafe { vut_rt_bytes_retain_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-bytes handle.
pub unsafe extern "C" fn vut_rt_slot_release_bytes_v1(slot: *mut u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedBytes>()) };
    unsafe { vut_rt_bytes_release_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-map handle.
pub unsafe extern "C" fn vut_rt_slot_retain_map_v1(slot: *const u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedMap>()) };
    unsafe { vut_rt_map_retain_v1(handle) };
}

#[unsafe(no_mangle)]
/// # Safety
/// `slot` must point to one initialized managed-map handle.
pub unsafe extern "C" fn vut_rt_slot_release_map_v1(slot: *mut u8) {
    let handle = unsafe { std::ptr::read_unaligned(slot.cast::<*mut ManagedMap>()) };
    unsafe { vut_rt_map_release_v1(handle) };
}
