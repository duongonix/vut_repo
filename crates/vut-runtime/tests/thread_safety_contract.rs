//! Cross-thread safety of managed values.
//!
//! Vut values are single-owner across tasks, but native operations and the
//! runtime itself retain/release and copy-on-write from multiple threads. These
//! tests contend on the atomic reference counts and on the COW helpers.
use std::sync::Mutex;

use vut_runtime::{VutList, VutMap, abi};

static LOCK: Mutex<()> = Mutex::new(());

#[test]
fn concurrent_string_retain_release_is_balanced() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::live_string_allocations();
    let handle = unsafe { abi::vut_rt_string_from_utf8_v1(b"threaded".as_ptr(), 8) };
    assert!(!handle.is_null());
    let address = handle as usize;
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(move || {
                let handle = address as *mut abi::ManagedString;
                for _ in 0..2_000 {
                    // SAFETY: the handle is live for the whole test.
                    unsafe { abi::vut_rt_retain_string_v1(handle) };
                    unsafe { abi::vut_rt_release_string_v1(handle) };
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("thread");
    }
    // SAFETY: this releases the original reference exactly once.
    unsafe { abi::vut_rt_release_string_v1(handle) };
    assert_eq!(abi::live_string_allocations(), baseline);
}

#[test]
fn concurrent_list_retain_release_is_balanced() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::live_list_allocations();
    let list = unsafe { abi::vut_rt_list_new_v1(8, 8, std::ptr::null(), std::ptr::null(), 0) };
    assert!(!list.is_null());
    let address = list as usize;
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(move || {
                let list = address as *mut abi::ManagedList;
                for _ in 0..2_000 {
                    // SAFETY: the handle is live for the whole test.
                    unsafe { abi::vut_rt_list_retain_v1(list) };
                    unsafe { abi::vut_rt_list_release_v1(list) };
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("thread");
    }
    // SAFETY: this releases the original reference exactly once.
    unsafe { abi::vut_rt_list_release_v1(list) };
    assert_eq!(abi::live_list_allocations(), baseline);
}

#[test]
fn copy_on_write_helpers_are_independent_across_threads() {
    let original = VutList::from(vec![1, 2, 3]);
    let shared = original.clone();
    let threads: Vec<_> = (0..4)
        .map(|index| {
            let mut local = shared.clone();
            std::thread::spawn(move || {
                local.push(index);
                local.as_slice().to_vec()
            })
        })
        .collect();
    let mut results: Vec<Vec<i32>> = threads
        .into_iter()
        .map(|thread| thread.join().expect("thread"))
        .collect();
    results.sort();
    assert_eq!(
        results,
        vec![
            vec![1, 2, 3, 0],
            vec![1, 2, 3, 1],
            vec![1, 2, 3, 2],
            vec![1, 2, 3, 3]
        ]
    );
    assert_eq!(original.as_slice(), &[1, 2, 3]);
    assert_eq!(shared.as_slice(), &[1, 2, 3]);

    let map = VutMap::new();
    let map_shared = map.clone();
    let map_thread = std::thread::spawn(move || {
        let mut local = map_shared.clone();
        local.set("k", 1);
        local.get(&"k").copied()
    });
    assert_eq!(map_thread.join().expect("thread"), Some(1));
    assert_eq!(map.get(&"k"), None);
}
