//! Small, safe native support runtime for generated Vut programs.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]
pub mod abi;
mod allocator;
mod async_handle;
mod awaiting;
mod bounds;
pub mod bytes;
mod collections;
mod dynamic;
pub mod executor;
mod frame;
mod io;
mod resource;
mod string;
mod task;
mod utf8;
mod vutcon;
pub use allocator::HeapBlock;
pub use async_handle::{
    ASYNC_CANCELLED, ASYNC_PENDING, ASYNC_READY, AsyncDropFn, AsyncHandle, AsyncPollFn,
    AsyncWakeFn, live_async_handles,
};
pub use awaiting::{vut_rt_async_await_child_v1, vut_rt_future_await_v1};
pub use bounds::{BoundsError, check_bounds};
pub use collections::{VutList, VutMap};
pub use dynamic::{DynValue, InterfaceValue, MethodTable};
pub use io::{read_from, read_line, write_stdout, write_to};
pub use string::{VutBytes, VutString};
pub use task::{
    is_task, task_completed, vut_rt_task_new_v1, vut_rt_task_poll_v1, vut_rt_task_result_v1,
};
pub fn panic(message: &str) -> ! {
    panic!("Vut runtime panic: {message}")
}
pub fn start(entry: fn() -> i32) -> i32 {
    entry()
}

#[cfg(test)]
mod tests {
    use super::*;
    static ABI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    #[test]
    fn collections_are_typed_and_bounds_safe() {
        let mut list = VutList::from(vec![1, 2]);
        assert_eq!(list.at(2), None);
        assert!(list.set(2, 3).is_err());
        let mut map = VutMap::new();
        map.set("a", 1);
        assert_eq!(map.get(&"a"), Some(&1));
    }
    #[test]
    fn utf8_and_dyn_metadata_are_explicit() {
        let text = VutString::from("Việt");
        assert_eq!((text.byte_len(), text.char_len()), (6, 4));
        let value = DynValue::new(7_i64);
        assert_eq!(value.downcast_ref::<i64>(), Some(&7));
    }
    #[test]
    fn heap_reallocation_preserves_existing_bytes() {
        let mut block = HeapBlock::allocate(4);
        block.realloc(8);
        assert_eq!(block.as_slice(), &[0; 8]);
    }
    #[test]
    fn managed_strings_retain_and_release_exactly_once() {
        let _guard = ABI_TEST_LOCK.lock().unwrap();
        let before = abi::live_string_allocations();
        // SAFETY: the byte slice remains live during construction and the
        // returned handle is balanced by exactly two releases after one retain.
        unsafe {
            let value = abi::vut_rt_string_from_utf8_v1(b"owned".as_ptr(), 5);
            assert!(!value.is_null());
            assert_eq!(abi::live_string_allocations(), before + 1);
            abi::vut_rt_retain_string_v1(value);
            abi::vut_rt_release_string_v1(value);
            assert_eq!(abi::live_string_allocations(), before + 1);
            abi::vut_rt_release_string_v1(value);
        }
        assert_eq!(abi::live_string_allocations(), before);
    }
    #[test]
    fn managed_string_abi_exposes_unicode_operations_without_leaks() {
        let _guard = ABI_TEST_LOCK.lock().unwrap();
        let before = abi::live_string_allocations();
        // SAFETY: every handle is created by this ABI, remains live for each
        // call, and is released exactly once before leaving the block.
        unsafe {
            let value = abi::vut_rt_string_from_utf8_v1("  Việt Vut  ".as_ptr(), 14);
            let pattern = abi::vut_rt_string_from_utf8_v1("Việt".as_ptr(), 6);
            assert_eq!(abi::vut_rt_string_byte_len_v1(value), 14);
            assert_eq!(abi::vut_rt_string_char_len_v1(value), 12);
            assert_eq!(abi::vut_rt_string_contains_v1(value, pattern), 1);
            let trimmed = abi::vut_rt_string_trim_v1(value);
            assert_eq!(abi::vut_rt_string_starts_with_v1(trimmed, pattern), 1);
            abi::vut_rt_release_string_v1(trimmed);
            abi::vut_rt_release_string_v1(pattern);
            abi::vut_rt_release_string_v1(value);
        }
        assert_eq!(abi::live_string_allocations(), before);
    }
    #[test]
    fn collection_mutation_is_copy_on_write() {
        let original = VutList::from(vec![1, 2]);
        let mut changed = original.clone();
        changed.push(3);
        assert_eq!(original.as_slice(), &[1, 2]);
        assert_eq!(changed.as_slice(), &[1, 2, 3]);

        let mut original = VutMap::new();
        original.set("name", "Vut");
        let mut changed = original.clone();
        changed.set("name", "changed");
        assert_eq!(original.get(&"name"), Some(&"Vut"));
        assert_eq!(changed.get(&"name"), Some(&"changed"));
    }
    #[test]
    fn list_api_is_bounds_safe_and_complete() {
        let mut values = VutList::with_capacity(4);
        assert!(values.is_empty());
        values.push(1);
        values.push(3);
        values.insert(1, 2).unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.capacity() >= 3);
        assert_eq!(values.at(1), Some(&2));
        assert!(values.set(3, 4).is_err());
        values.set(2, 4).unwrap();
        assert_eq!(values.slice(1, 3).unwrap().as_slice(), &[2, 4]);
        assert!(values.slice(3, 2).is_err());
        assert!(values.contains(&4));
        assert_eq!(values.remove(1), Some(2));
        assert_eq!(values.pop(), Some(4));
        values.clear();
        assert!(values.is_empty());
    }
    #[test]
    fn map_api_is_copy_on_write_and_complete() {
        let mut values = VutMap::new();
        values.reserve(4);
        values.set("one", 1);
        values.set("two", 2);
        assert_eq!(values.len(), 2);
        assert!(values.capacity() >= 2);
        assert!(values.contains_key(&"one"));
        assert_eq!(values.get(&"two"), Some(&2));
        let original = values.clone();
        assert_eq!(values.remove(&"one"), Some(1));
        assert!(original.contains_key(&"one"));
        assert_eq!(values.keys().len(), 1);
        assert_eq!(values.values().len(), 1);
        values.clear();
        assert!(values.is_empty());
    }
    #[test]
    fn string_and_bytes_apis_are_unicode_safe() {
        let text = VutString::from("  Việt Vut\nSecond  ");
        assert_eq!(text.byte_len(), 21);
        assert_eq!(text.char_len(), 19);
        assert!(text.contains("Việt"));
        assert!(text.trim().starts_with("Việt"));
        assert!(text.trim().ends_with("Second"));
        assert_eq!(text.trim_start().as_str(), "Việt Vut\nSecond  ");
        assert_eq!(text.trim_end().as_str(), "  Việt Vut\nSecond");
        assert_eq!(VutString::from("VUT").to_lower().as_str(), "vut");
        assert_eq!(VutString::from("vut").to_upper().as_str(), "VUT");
        assert_eq!(VutString::from("a-b").replace("-", ":").as_str(), "a:b");
        assert_eq!(VutString::from("a,b").split(",").len(), 2);
        assert_eq!(text.lines().len(), 2);
        assert_eq!(VutString::from("42").to_int(), Ok(42));
        assert_eq!(VutString::from("1.5").to_float(), Ok(1.5));
        assert!(VutString::from("x").to_int().is_err());
        let bytes = VutString::from("Việt").to_bytes();
        assert_eq!(bytes.at(0), Some(b'V'));
        assert_eq!(bytes.first(), Some(b'V'));
        assert_eq!(bytes.last(), Some(b't'));
        assert!(bytes.at(99).is_none());
        assert!(bytes.slice(1, 99).is_none());
        assert_eq!(bytes.to_string().unwrap().as_str(), "Việt");
        assert!(VutBytes::new(vec![0xff]).to_string().is_err());
    }

    #[test]
    fn bytes_buffer_is_contiguous_mutable_and_cow() {
        let original = VutBytes::new(vec![1, 2, 3]);
        let mut copy = original.clone();
        copy.reserve(32);
        assert!(copy.capacity() >= 32);
        assert!(copy.set(0, 9));
        assert_eq!(original.at(0), Some(1));
        assert_eq!(copy.at(0), Some(9));
        assert_eq!(copy.slice(1, 3).unwrap().to_list().len(), 2);
        copy.clear();
        assert!(copy.is_empty());
    }

    #[test]
    fn cow_collections_survive_repeated_clone_and_mutation() {
        let original_list = VutList::from((0..256).collect::<Vec<_>>());
        let mut original_map = VutMap::with_capacity(256);
        for value in 0..256 {
            original_map.set(value, value * 2);
        }
        for iteration in 0..2_000 {
            let mut list = original_list.clone();
            list.set(iteration % 256, iteration).unwrap();
            list.push(iteration);
            assert_eq!(list.len(), 257);

            let mut map = original_map.clone();
            map.set(iteration % 256, iteration);
            map.remove(&((iteration + 1) % 256));
            assert_eq!(map.len(), 255);
        }
        assert_eq!(original_list.len(), 256);
        assert_eq!(original_map.len(), 256);
    }

    #[test]
    fn managed_string_allocation_stress_returns_to_baseline() {
        let _guard = ABI_TEST_LOCK.lock().unwrap();
        let before = abi::live_string_allocations();
        for _ in 0..20_000 {
            // SAFETY: all inputs remain live during each call and every
            // returned managed handle is released exactly once.
            unsafe {
                let value = abi::vut_rt_string_from_utf8_v1(b"  stress  ".as_ptr(), 10);
                let trimmed = abi::vut_rt_string_trim_v1(value);
                let upper = abi::vut_rt_string_to_upper_v1(trimmed);
                abi::vut_rt_release_string_v1(upper);
                abi::vut_rt_release_string_v1(trimmed);
                abi::vut_rt_release_string_v1(value);
            }
        }
        assert_eq!(abi::live_string_allocations(), before);
    }
}
