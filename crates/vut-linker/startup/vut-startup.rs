//! Vut startup object source.
//!
//! This file is compiled once per target at distribution time (and, as a
//! development fallback, on demand) into `vut-startup.o` / `vut-startup.obj`.
//! It provides the platform `main` symbol that forwards to the
//! compiler-generated `vut_entry`.
//!
//! It is never compiled at end-user link time in a release installation; the
//! prebuilt object is shipped inside the distribution. Keeping it as a single
//! source of truth avoids divergence between the object and this contract.

unsafe extern "C" {
    fn vut_entry() -> i64;
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    // `vut_entry` returns the process exit code as a 64-bit value; the platform
    // exit convention consumes the low 32 bits.
    unsafe { vut_entry() as i32 }
}
