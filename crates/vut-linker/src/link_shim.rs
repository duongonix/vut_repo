//! Portable startup shim used to let `rustc` supply each platform's native
//! system libraries while Vut supplies the compiled entry function.
//!
//! `main` is defined on every platform so that the C runtime performs normal
//! process teardown. This matters when native libraries (for example the HTTP
//! backend) start background threads: a custom entry point that merely returns
//! would leave those threads alive and prevent the process from exiting.

unsafe extern "C" {
    fn vut_entry() -> i64;
}

fn main() {
    // SAFETY: the compiler-generated object always exports this C-compatible
    // entry returning the process exit code as a 64-bit value.
    std::process::exit(unsafe { vut_entry() as i32 });
}
