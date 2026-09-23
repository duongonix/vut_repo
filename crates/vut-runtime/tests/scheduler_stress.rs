//! Scheduler stress and lifecycle tests.
//!
//! These exercise mass scheduling, concurrent drop/cancel, and repeated runs
//! against the global M:N scheduler. They assert memory safety and leak-freedom,
//! not throughput (see the ignored benchmark for timing).
use std::ffi::c_void;
use std::sync::Mutex;

use vut_runtime::{AsyncHandle, abi};

static LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" fn poll_ready(_op: *mut c_void, out: *mut u8) -> i32 {
    // SAFETY: the runtime provides room for one i64 result.
    unsafe { out.cast::<i64>().write_unaligned(42) };
    1
}

unsafe extern "C" fn drop_noop(_op: *mut c_void) {}

fn spawn() -> *mut AsyncHandle {
    // SAFETY: the callbacks and result layout match the spawn contract.
    unsafe { abi::vut_rt_vutcon_spawn_v1(std::ptr::null_mut(), poll_ready, drop_noop, 8, 8) }
}

#[test]
fn mass_spawn_and_drain_is_leak_free() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_async_live_count_v1();
    // SAFETY: every handle is scheduled and released by run/drain.
    unsafe {
        let tasks: Vec<*mut AsyncHandle> = (0..1_000).map(|_| spawn()).collect();
        abi::vut_rt_executor_run_v1(tasks[0]);
        abi::vut_rt_executor_drain_v1();
    }
    assert_eq!(abi::vut_rt_async_live_count_v1(), baseline);
}

#[test]
fn concurrent_drop_during_run_is_safe() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_async_live_count_v1();
    // SAFETY: the root is run; the rest are dropped from another thread while
    // the scheduler is active, exercising deferred reclamation.
    unsafe {
        let tasks: Vec<*mut AsyncHandle> = (0..500).map(|_| spawn()).collect();
        let root = tasks[0];
        let victims: Vec<usize> = tasks[1..].iter().map(|task| *task as usize).collect();
        let dropper = std::thread::spawn(move || {
            for address in victims {
                abi::vut_rt_async_drop_v1(address as *mut AsyncHandle);
            }
        });
        abi::vut_rt_executor_run_v1(root);
        let _ = dropper.join();
        abi::vut_rt_executor_drain_v1();
    }
    assert_eq!(abi::vut_rt_async_live_count_v1(), baseline);
}

#[test]
fn repeated_runs_reuse_the_scheduler() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_async_live_count_v1();
    for _ in 0..50 {
        // SAFETY: each run schedules, drives, and drains exactly one task.
        unsafe {
            let task = spawn();
            let status = abi::vut_rt_executor_run_v1(task);
            assert_eq!(status, 1);
            let mut out = 0_i64;
            abi::vut_rt_task_result_v1(task, std::ptr::from_mut(&mut out).cast());
            assert_eq!(out, 42);
            abi::vut_rt_executor_drain_v1();
        }
    }
    assert_eq!(abi::vut_rt_async_live_count_v1(), baseline);
}
