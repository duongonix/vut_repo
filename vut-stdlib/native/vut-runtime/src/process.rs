//! Child-process primitives; cleanup never controls process lifetime.
mod child;
mod command;
mod handles;
mod pipes;

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_process_id_v1() -> u32 {
    std::process::id()
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_process_exit_v1(code: i32) -> ! {
    std::process::exit(code)
}
