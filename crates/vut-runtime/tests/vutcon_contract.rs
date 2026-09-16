use std::ffi::c_void;
use std::sync::Mutex;

use vut_runtime::{ASYNC_READY, AsyncHandle, abi};

static LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" fn poll_forty_two(_op: *mut c_void, out: *mut u8) -> i32 {
    // SAFETY: the runtime provides room for one i64 result.
    unsafe { out.cast::<i64>().write_unaligned(42) };
    ASYNC_READY
}

unsafe extern "C" fn poll_write_pair(_op: *mut c_void, out: *mut u8) -> i32 {
    let pair = [7_i64, 9_i64];
    // SAFETY: the runtime provides room for two i64 values.
    unsafe {
        std::ptr::copy_nonoverlapping(pair.as_ptr().cast::<u8>(), out, 16);
    }
    ASYNC_READY
}

unsafe extern "C" fn drop_noop(_op: *mut c_void) {}

#[test]
fn spawn_runs_a_sync_task_and_copies_its_result() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_vutcon_live_count_v1();
    // SAFETY: the callbacks and result layout match the spawn contract.
    unsafe {
        let task =
            abi::vut_rt_vutcon_spawn_v1(std::ptr::null_mut(), poll_forty_two, drop_noop, 8, 8);
        assert_eq!(abi::vut_rt_vutcon_live_count_v1(), baseline + 1);
        abi::vut_rt_executor_run_v1(task);
        let mut out = 0_i64;
        abi::vut_rt_task_result_v1(task, std::ptr::from_mut(&mut out).cast());
        assert_eq!(out, 42);
        abi::vut_rt_async_drop_v1(task);
        assert_eq!(abi::vut_rt_vutcon_live_count_v1(), baseline);
    }
}

#[test]
fn dropped_unawaited_task_is_released_exactly_once() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_vutcon_live_count_v1();
    // SAFETY: the task is dropped exactly once without being run.
    unsafe {
        let task =
            abi::vut_rt_vutcon_spawn_v1(std::ptr::null_mut(), poll_forty_two, drop_noop, 8, 8);
        abi::vut_rt_async_drop_v1(task);
        assert_eq!(abi::vut_rt_vutcon_live_count_v1(), baseline);
    }
}

#[test]
fn aggregate_task_result_is_copied_into_destination() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_vutcon_live_count_v1();
    // SAFETY: the poll function writes exactly 16 bytes into the task buffer.
    unsafe {
        let task =
            abi::vut_rt_vutcon_spawn_v1(std::ptr::null_mut(), poll_write_pair, drop_noop, 16, 8);
        abi::vut_rt_executor_run_v1(task);
        let mut out = [0_i64; 2];
        abi::vut_rt_task_result_v1(task, out.as_mut_ptr().cast::<u8>());
        assert_eq!(out, [7, 9]);
        abi::vut_rt_async_drop_v1(task);
    }
    assert_eq!(abi::vut_rt_vutcon_live_count_v1(), baseline);
}

#[test]
fn executor_drain_releases_detached_tasks() {
    let _guard = LOCK.lock().unwrap();
    let baseline = abi::vut_rt_vutcon_live_count_v1();
    // SAFETY: each task is scheduled but never awaited, then drained.
    unsafe {
        let _first =
            abi::vut_rt_vutcon_spawn_v1(std::ptr::null_mut(), poll_forty_two, drop_noop, 8, 8);
        let _second =
            abi::vut_rt_vutcon_spawn_v1(std::ptr::null_mut(), poll_forty_two, drop_noop, 8, 8);
        abi::vut_rt_executor_drain_v1();
    }
    assert_eq!(abi::vut_rt_vutcon_live_count_v1(), baseline);
}

#[test]
fn handle_pointer_type_is_exposed() {
    // A compile-time check that the opaque handle type is nameable.
    let _null: *mut AsyncHandle = std::ptr::null_mut();
}
