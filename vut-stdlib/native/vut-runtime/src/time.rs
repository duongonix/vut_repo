//! Clock and sleep primitives (`vut_rt_time_*`).
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn epoch_nanos() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => checked_nanos(duration.as_nanos()),
        Err(error) => -checked_nanos(error.duration().as_nanos()),
    }
}

fn checked_nanos(value: u128) -> i64 {
    i64::try_from(value).unwrap_or_else(|_| vut_runtime::abi::vut_rt_numeric_panic_v1(i64::MAX))
}

fn monotonic_origin() -> Instant {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    *ORIGIN.get_or_init(Instant::now)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_time_now_secs_v1() -> i64 {
    epoch_nanos() / 1_000_000_000
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_time_now_millis_v1() -> i64 {
    epoch_nanos() / 1_000_000
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_time_now_nanos_v1() -> i64 {
    epoch_nanos()
}

/// Nanoseconds elapsed on a monotonic clock since the first instant query.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_time_instant_nanos_v1() -> i64 {
    checked_nanos(monotonic_origin().elapsed().as_nanos())
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_time_sleep_nanos_v1(nanos: i64) {
    if nanos > 0 {
        std::thread::sleep(std::time::Duration::from_nanos(nanos.unsigned_abs()));
    }
}
