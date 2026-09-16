//! M-LINK.0 startup object: platform C entry `main` that forwards to `vut_entry`.
//!
//! Compiled once per target to `vut-startup.obj` / `vut-startup.o` and shipped
//! in the distribution (provisional name for the spike).

unsafe extern "C" {
    fn vut_entry() -> i64;
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    // `vut_entry` returns the process exit code as a 64-bit value; the low 32
    // bits are used by the platform process exit convention.
    unsafe { vut_entry() as i32 }
}
