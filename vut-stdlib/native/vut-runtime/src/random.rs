//! Pseudo-random number generation (`vut_rt_math_rng_*`, `vut_rt_math_random_*`).
//!
//! **Not cryptographically secure.** The engine is xoshiro256** seeded with
//! splitmix64, matching the documented `math` semantics. A fixed seed produces a
//! reproducible sequence across platforms.
use crate::resource;
use std::{cell::RefCell, ffi::c_void};

pub(super) struct RngHandle(Rng);

struct Rng {
    state: [u64; 4],
}

impl Rng {
    fn from_seed(seed: u64) -> Self {
        let mut state = [0_u64; 4];
        let mut source = seed;
        for slot in &mut state {
            source = source.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut mixed = source;
            mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = mixed ^ (mixed >> 31);
        }
        if state == [0; 4] {
            state[0] = 0x9E37_79B9_7F4A_7C15;
        }
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let shift = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= shift;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }

    /// Uniform `f64` in `[0, 1)` using a 53-bit mantissa.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the value is masked to 53 bits, so the cast is exact"
    )]
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
    }

    /// Uniform integer in `[0, bound)` (Lemire's method with rejection).
    fn next_bounded(&mut self, bound: u64) -> u64 {
        let mut x = self.next_u64();
        let mut product = u128::from(x) * u128::from(bound);
        let threshold = bound.wrapping_neg() % bound;
        while (product as u64) < threshold {
            x = self.next_u64();
            product = u128::from(x) * u128::from(bound);
        }
        (product >> 64) as u64
    }

    /// Uniform integer in the inclusive range `[min, max]`. Returns `min` when
    /// the range is empty (`min >= max`).
    fn int_inclusive(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let span = (i128::from(max) - i128::from(min) + 1) as u128;
        if span > u128::from(u64::MAX) {
            return self.next_u64() as i64;
        }
        (i128::from(min) + i128::from(self.next_bounded(span as u64))) as i64
    }
}

fn entropy_seed() -> u64 {
    let mut bytes = [0_u8; 8];
    if getrandom::getrandom(&mut bytes).is_ok() {
        return u64::from_le_bytes(bytes);
    }
    // Fallback only when the OS RNG is unavailable.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() as u64)
}

thread_local! {
    static GLOBAL: RefCell<Rng> = RefCell::new(Rng::from_seed(entropy_seed()));
}

/// # Safety
/// `pointer` must be a live borrowed `RngHandle` resource pointer.
unsafe fn handle<'a>(pointer: *mut c_void) -> Option<&'a mut Rng> {
    // SAFETY: upheld by the caller.
    unsafe { pointer.cast::<RngHandle>().as_mut() }.map(|handle| &mut handle.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_math_rng_new_v1(seed: i64) -> *mut c_void {
    resource::owned(RngHandle(Rng::from_seed(seed as u64)))
}

#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live borrowed `RngHandle` resource pointer.
pub unsafe extern "C" fn vut_rt_math_rng_float_v1(pointer: *mut c_void) -> f64 {
    unsafe { handle(pointer) }.map_or(0.0, Rng::next_f64)
}

#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live borrowed `RngHandle` resource pointer.
pub unsafe extern "C" fn vut_rt_math_rng_int_v1(pointer: *mut c_void, min: i64, max: i64) -> i64 {
    unsafe { handle(pointer) }.map_or(min, |rng| rng.int_inclusive(min, max))
}

#[unsafe(no_mangle)]
/// # Safety
/// `pointer` must be a live borrowed `RngHandle` resource pointer.
pub unsafe extern "C" fn vut_rt_math_rng_bool_v1(pointer: *mut c_void) -> usize {
    unsafe { handle(pointer) }.map_or(0, |rng| usize::from(rng.next_u64() & 1 == 1))
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_math_random_float_v1() -> f64 {
    GLOBAL.with(|rng| rng.borrow_mut().next_f64())
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_math_random_int_v1(min: i64, max: i64) -> i64 {
    GLOBAL.with(|rng| rng.borrow_mut().int_inclusive(min, max))
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_math_random_bool_v1() -> usize {
    GLOBAL.with(|rng| usize::from(rng.borrow_mut().next_u64() & 1 == 1))
}
