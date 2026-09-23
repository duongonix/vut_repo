#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

use std::fs;
use vut_compiler::{CompilerConfig, CompilerSession};

fn emit_object_bytes(name: &str, source: &str) -> Vec<u8> {
    let scratch = stdlib_common::scratch(name);
    let root = &scratch.root;
    fs::write(root.join("main.vut"), source).expect("write source");
    let config = CompilerConfig {
        runtime_library: Some(stdlib_common::runtime_library()),
        ..CompilerConfig::default()
    };
    let mut session = CompilerSession::new(config);
    session.emit_object(root, &[]).expect("emit object")
}

#[test]
fn float_ops_are_lowered_to_instructions_not_runtime_calls() {
    let object = emit_object_bytes(
        "intrinsics",
        r"fn main():
  x = 2.0
  out(x.sqrt())
  out(x.abs())
  out(x.floor())
  out(x.ceil())
  out(x.trunc())
  out(x.fma(3.0, 4.0))
  out(x.copysign(-1.0))
",
    );
    let text = String::from_utf8_lossy(&object);
    for symbol in [
        "vut_rt_f64_sqrt_v1",
        "vut_rt_f64_abs_v1",
        "vut_rt_f64_floor_v1",
        "vut_rt_f64_ceil_v1",
        "vut_rt_f64_trunc_v1",
    ] {
        assert!(
            !text.contains(symbol),
            "intrinsic-capable op still references {symbol}"
        );
    }
}

#[test]
fn round_still_calls_the_runtime() {
    // `round` must not lower to `nearest` (round-half-to-even differs from
    // round-half-away-from-zero), so it keeps the runtime call.
    let object = emit_object_bytes("round_runtime", "fn main():\n  x = 2.5\n  out(x.round())\n");
    let text = String::from_utf8_lossy(&object);
    assert!(
        text.contains("vut_rt_f64_round_v1"),
        "round should keep the runtime call"
    );
}
