//! Native/platform primitives for the official Vut standard library.
//!
//! This crate implements only what genuinely requires OS/runtime access:
//! filesystem syscalls, environment access, platform information, clock/sleep,
//! process primitives, standard streams, native handles and the shared result
//! ABI. All high-level stdlib behavior lives in `vut-stdlib/std` as Vut code.
//!
//! Symbol convention: `vut_rt_<module>_<operation>_v1`.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::semicolon_if_nothing_returned,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

pub mod abi;
mod count;
mod env;
mod fs;
mod guard;
pub mod http;
mod io;
mod math;
mod os;
mod process;
mod random;
mod resource;
mod time;
