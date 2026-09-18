//! Reference-counted boxes backing `interface` and `dyn` values.
//!
//! The compiler boxes a concrete value (always an aggregate `data`/`enum`
//! block) into inline storage and attaches a static vtable. The vtable's slot
//! zero is the compiler-generated drop function; the remaining slots are the
//! concrete method addresses in the interface's canonical (sorted) order.
//!
//! ```text
//! [ references | vtable | data_offset | data_size | data... ]
//! ```
//!
//! `data_offset` is derived from the fixed box alignment so that releasing a
//! box can reconstruct its allocation layout without storing the alignment.
use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

const BOX_ALIGN: usize = 16;
const REFS_OFFSET: usize = 0;
const VTABLE_OFFSET: usize = mem::size_of::<usize>();
const DATA_OFFSET_OFFSET: usize = VTABLE_OFFSET * 2;
const DATA_SIZE_OFFSET: usize = VTABLE_OFFSET * 3;
const HEADER_SIZE: usize = VTABLE_OFFSET * 4;

fn box_layout(data_size: usize) -> (Layout, usize) {
    let offset = (HEADER_SIZE + BOX_ALIGN - 1) & !(BOX_ALIGN - 1);
    let total = offset.saturating_add(data_size).max(1);
    let layout =
        Layout::from_size_align(total, BOX_ALIGN).expect("interface box layout must be valid");
    (layout, offset)
}

fn read_word(base: *mut u8, offset: usize) -> usize {
    unsafe { std::ptr::read_unaligned(base.add(offset).cast::<usize>()) }
}

fn write_word(base: *mut u8, offset: usize, value: usize) {
    unsafe { std::ptr::write_unaligned(base.add(offset).cast::<usize>(), value) };
}

/// Allocates an interface box with `data_size` bytes of inline storage.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_interface_new(data_size: usize, _data_align: usize) -> *mut u8 {
    let (layout, offset) = box_layout(data_size);
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        handle_alloc_error(layout);
    }
    write_word(base, REFS_OFFSET, 1);
    write_word(base, VTABLE_OFFSET, 0);
    write_word(base, DATA_OFFSET_OFFSET, offset);
    write_word(base, DATA_SIZE_OFFSET, data_size);
    base
}

/// Returns the address of the box's inline concrete-value storage.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_interface_data(base: *mut u8) -> *mut u8 {
    let offset = read_word(base, DATA_OFFSET_OFFSET);
    unsafe { base.add(offset) }
}

/// Attaches the static vtable that describes the boxed concrete type.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_interface_set_vtable(base: *mut u8, vtable: *const ()) {
    write_word(base, VTABLE_OFFSET, vtable as usize);
}

/// Reads the attached vtable.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_interface_vtable(base: *mut u8) -> *const () {
    read_word(base, VTABLE_OFFSET) as *const ()
}

/// Increments the box reference count.
#[unsafe(no_mangle)]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the reference counter lives at offset zero of a 16-byte-aligned box"
)]
pub extern "C" fn vut_rt_interface_retain(base: *mut u8) {
    if base.is_null() {
        return;
    }
    let references = unsafe { &*base.add(REFS_OFFSET).cast::<AtomicUsize>() };
    references.fetch_add(1, Ordering::Relaxed);
}

/// Decrements the box reference count, destroying the box at zero.
#[unsafe(no_mangle)]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the reference counter lives at offset zero of a 16-byte-aligned box"
)]
pub extern "C" fn vut_rt_interface_release(base: *mut u8) {
    if base.is_null() {
        return;
    }
    let references = unsafe { &*base.add(REFS_OFFSET).cast::<AtomicUsize>() };
    if references.fetch_sub(1, Ordering::AcqRel) != 1 {
        return;
    }
    let vtable = read_word(base, VTABLE_OFFSET) as *const usize;
    if !vtable.is_null() {
        let drop_fn = unsafe { mem::transmute::<usize, unsafe extern "C" fn(*mut u8)>(*vtable) };
        let data = vut_rt_interface_data(base);
        unsafe { drop_fn(data) };
    }
    let data_size = read_word(base, DATA_SIZE_OFFSET);
    let (layout, _) = box_layout(data_size);
    unsafe { dealloc(base, layout) };
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn drop_stub(_: *mut u8) {}

    #[test]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "the box payload is 16-byte aligned; the test writes a u64 into it"
    )]
    fn box_round_trip_and_release() {
        unsafe {
            let base = vut_rt_interface_new(8, 8);
            assert!(!base.is_null());
            let data = vut_rt_interface_data(base);
            std::ptr::write(data.cast::<u64>(), 42);
            assert_eq!(std::ptr::read(data.cast::<u64>()), 42);
            let vtable: [usize; 1] = [drop_stub as *const () as usize];
            vut_rt_interface_set_vtable(base, vtable.as_ptr().cast());
            assert_eq!(vut_rt_interface_vtable(base), vtable.as_ptr().cast());
            vut_rt_interface_retain(base);
            vut_rt_interface_release(base);
            vut_rt_interface_release(base);
        }
    }
}
