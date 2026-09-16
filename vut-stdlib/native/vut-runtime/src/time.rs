//! Clock and sleep primitives (`vut_rt_time_*`).
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn epoch_nanos() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX),
        Err(error) => -i64::try_from(error.duration().as_nanos()).unwrap_or(i64::MAX),
    }
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
    i64::try_from(monotonic_origin().elapsed().as_nanos()).unwrap_or(i64::MAX)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_time_sleep_nanos_v1(nanos: i64) {
    if nanos > 0 {
        std::thread::sleep(std::time::Duration::from_nanos(nanos.unsigned_abs()));
    }
}
