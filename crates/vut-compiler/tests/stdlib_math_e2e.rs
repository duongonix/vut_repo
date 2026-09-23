#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn constants_basic_rounding_and_integer_algorithms_are_exact() {
    let source = r"import math
fn main():
  out(math.PI > 3.14 and math.PI < 3.15)
  out(math.TAU > 6.28 and math.TAU < 6.29)
  out(math.E > 2.71 and math.E < 2.72)
  out(math.abs(-3.5) == 3.5)
  out(math.min(2.0, 5.0) == 2.0)
  out(math.max(2.0, 5.0) == 5.0)
  out(math.clamp(10.0, 0.0, 5.0) == 5.0)
  out(math.sign(-3.0) == -1.0)
  out(math.sign(2.0) == 1.0)
  out(math.sign(0.0) == 0.0)
  out(math.floor(2.7) == 2.0)
  out(math.ceil(2.1) == 3.0)
  out(math.round(2.5) == 3.0)
  out(math.round(-2.5) == -3.0)
  out(math.trunc(-2.7) == -2.0)
  out(math.fract(2.75) == 0.75)
  out(math.fract(-2.75) == -0.75)
  out(math.abs(math.degrees(math.radians(90.0)) - 90.0) < 0.000001)
  out(math.gcd(12, 18) == 6)
  out(math.gcd(-12, 18) == 6)
  out(math.lcm(4, 6) == 12)
  out(math.lcm(0, 5) == 0)
  out(math.is_power_of_two(1))
  out(math.is_power_of_two(64))
  out(not math.is_power_of_two(6))
  out(not math.is_power_of_two(0))
  out(not math.is_power_of_two(-8))
";
    let (code, stdout, stderr) = stdlib_common::run("math_basic", source);
    assert_eq!(code, Some(0), "{stderr}");
    let expected = "true\n".repeat(27);
    assert_eq!(stdout, expected);
}

#[test]
fn transcendentals_match_expected_values() {
    let source = r"import math
fn main():
  out(math.abs(math.sin(0.0)) < 0.0000001)
  out(math.abs(math.cos(0.0) - 1.0) < 0.0000001)
  out(math.abs(math.sin(math.PI / 2.0) - 1.0) < 0.0000001)
  out(math.abs(math.atan2(1.0, 1.0) - math.PI / 4.0) < 0.0000001)
  out(math.abs(math.exp(0.0) - 1.0) < 0.0000001)
  out(math.abs(math.exp2(3.0) - 8.0) < 0.0000001)
  out(math.abs(math.log(math.E) - 1.0) < 0.0000001)
  out(math.abs(math.log2(8.0) - 3.0) < 0.0000001)
  out(math.abs(math.log10(1000.0) - 3.0) < 0.0000001)
  out(math.abs(math.cbrt(27.0) - 3.0) < 0.0000001)
  out(math.abs(math.sqrt(16.0) - 4.0) < 0.0000001)
  out(math.abs(math.pow(2.0, 10.0) - 1024.0) < 0.0000001)
  out(math.powi(2.0, 10) == 1024.0)
  out(math.powi(2.0, -1) == 0.5)
  out(math.abs(math.sinh(0.0)) < 0.0000001)
  out(math.abs(math.cosh(0.0) - 1.0) < 0.0000001)
  out(math.abs(math.tanh(0.0)) < 0.0000001)
  out(math.abs(math.asin(1.0) - math.PI / 2.0) < 0.0000001)
  out(math.abs(math.acos(1.0)) < 0.0000001)
  out(math.abs(math.atan(1.0) - math.PI / 4.0) < 0.0000001)
  out(math.is_finite(1.0))
  out(not math.is_nan(1.0))
";
    let (code, stdout, stderr) = stdlib_common::run("math_transcendentals", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "true\n".repeat(22));
}

#[test]
fn float_utilities_are_exact() {
    let source = r"import math
fn main():
  out(math.copysign(3.0, -1.0) == -3.0)
  out(math.copysign(-3.0, 1.0) == 3.0)
  out(math.abs(math.hypot(3.0, 4.0) - 5.0) < 0.0000001)
  out(math.fma(2.0, 3.0, 4.0) == 10.0)
";
    let (code, stdout, stderr) = stdlib_common::run("math_float_utilities", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "true\n".repeat(4));
}

#[test]
fn random_is_deterministic_for_a_seed() {
    let source = r"import math
fn main():
  a = math.rng(42)
  b = math.rng(42)
  out(a.random() == b.random())
  out(a.random_int(1, 6) == b.random_int(1, 6))
  out(a.random_float(0.0, 1.0) == b.random_float(0.0, 1.0))
  out(a.random_bool() == b.random_bool())
  r = math.rng(7)
  ok = true
  for i in @[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]:
    v = r.random_int(10, 20)
    if v < 10 or v > 20:
      ok = false
  out(ok)
  f = math.random_float(2.0, 3.0)
  out(f >= 2.0 and f < 3.0)
";
    let (code, stdout, stderr) = stdlib_common::run("math_random", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "true\n".repeat(6));
}

#[test]
fn integer_lcm_overflow_wraps() {
    let source = r"import math
fn main():
  out(math.lcm(9223372036854775807, 3))
";
    let (code, stdout, stderr) = stdlib_common::run("math_lcm_overflow", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "9223372036854775805\n");
}
