//! Checked float conversion boundaries through real native executables.
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn integer_to_f32_uses_single_ieee_rounding() {
    let source = "fn main():\n  value: u64 = 9223372586610589697\n  expected: f32 = 9223373136366403584.0\n  out(value.to_f32() == expected)\n";
    let (code, stdout, stderr) = stdlib_common::run("single_rounding", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "true\n");
}

const DESTINATIONS: &[(&str, &str, &str)] = &[
    ("i8", "-128.5", "128.0"),
    ("i16", "-32768.5", "32768.0"),
    ("i32", "-2147483648.5", "2147483648.0"),
    ("i64", "-9223372036854775808.0", "9223372036854775808.0"),
    ("int", "-9223372036854775808.0", "9223372036854775808.0"),
    ("u8", "255.9", "256.0"),
    ("u16", "65535.9", "65536.0"),
    ("u32", "4294967295.9", "4294967296.0"),
    ("u64", "18446744073709549568.0", "18446744073709551616.0"),
];

fn destinations() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut cases = DESTINATIONS.to_vec();
    cases.push(if usize::BITS == 64 {
        ("usize", "18446744073709549568.0", "18446744073709551616.0")
    } else {
        ("usize", "4294967295.9", "4294967296.0")
    });
    cases
}

fn signed_edges(ty: &str) -> (&'static str, &'static str) {
    match ty {
        "i8" => ("127.9", "-129.0"),
        "i16" => ("32767.9", "-32769.0"),
        "i32" => ("2147483647.9", "-2147483649.0"),
        // The adjacent representable f64 values, not rounded integer maxima.
        _ => ("9223372036854774784.0", "-9223372036854777856.0"),
    }
}

#[test]
fn finite_truncation_precedes_range_check_for_every_destination() {
    for (ty, boundary, _) in destinations() {
        let negative = if ty.starts_with('u') { "-0.5" } else { "-1.9" };
        let expected = if ty.starts_with('u') { "0" } else { "-1" };
        let top = if ty.starts_with('u') {
            boundary
        } else {
            signed_edges(ty).0
        };
        let source = format!(
            "fn main():\n  a: float = 1.9\n  b: float = {negative}\n  edge: float = {boundary}\n  out(a.to_{ty}())\n  out(b.to_{ty}())\n  converted = edge.to_{ty}()\n  out(converted.to_float() == edge.trunc())\n  top: f64 = {top}\n  out(top.to_{ty}().to_float() == top.trunc())\n  narrow: f32 = -0.5\n  out(narrow.to_{ty}())\n"
        );
        let (code, stdout, stderr) = stdlib_common::run(&format!("truncate_{ty}"), &source);
        assert_eq!(code, Some(0), "{ty}: {stderr}");
        assert_eq!(stdout, format!("1\n{expected}\ntrue\ntrue\n0\n"), "{ty}");
    }
}

#[test]
fn invalid_values_use_numeric_trap_for_every_destination() {
    for (ty, _, high) in destinations() {
        let low = if ty.starts_with('u') {
            "-1.0".to_owned()
        } else {
            signed_edges(ty).1.to_owned()
        };
        for expression in [
            high.to_owned(),
            low,
            "(0.0 - 1.0).sqrt()".into(),
            "1.0 / 0.0".into(),
            "(0.0 - 1.0) / 0.0".into(),
        ] {
            let source =
                format!("fn main():\n  value: float = {expression}\n  out(value.to_{ty}())\n");
            let (code, _, stderr) = stdlib_common::run(&format!("trap_{ty}"), &source);
            assert_ne!(code, Some(0), "{ty}: {expression}");
            assert!(stderr.contains("numeric"), "wrong trap for {ty}: {stderr}");
        }
    }
}
