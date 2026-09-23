//! Unit values through native functions, generic payloads and Result.
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn unit_function_and_result_payloads() {
    let source = "fn value() -> unit:\n  unit\n\nfn identity[T](value: T) -> T:\n  value\n\nfn operation() -> result[unit, str]:\n  ok(value())\n\nfn main():\n  token: unit = identity(unit)\n  match operation():\n    ok(done): out(\"done\")\n    err(error): out(error)\n";
    let (code, stdout, stderr) = stdlib_common::run("unit", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "done\n");
}

#[test]
fn unit_generic_data_and_enum_payloads() {
    let source = "data Box[T]:\n  token: T\n  number: int\n\nenum Payload[T]:\n  present(value: T)\n\nfn accept(token: unit) -> int:\n  7\n\nfn main():\n  box = Box(token: unit, number: 42)\n  box.token = unit\n  out(box.number)\n  out(accept(box.token))\n  payload: Payload[unit] = Payload.present(value: unit)\n  match payload:\n    present(token): out(accept(token))\n";
    let (code, stdout, stderr) = stdlib_common::run("unit_payload", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "42\n7\n7\n");
}
