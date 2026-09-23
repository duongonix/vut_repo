#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn duration_timestamp_and_monotonic_clock_execute() {
    let source = r"import time
fn main():
  out(time.microseconds(2).as_nanoseconds())
  out(time.minutes(2).as_seconds())
  out(time.hours(1).as_milliseconds())
  a = time.seconds(3)
  b = time.milliseconds(500)
  out(a.add(b).as_microseconds())
  out(b.subtract(a).as_milliseconds())
  stamp = time.from_unix_seconds(-2)
  out(stamp.unix_seconds())
  out(stamp.add(a).unix_milliseconds())
  out(stamp.duration_since(time.from_unix_milliseconds(-2500)).as_milliseconds())
  out(stamp.subtract(b).unix_milliseconds())
  start = time.instant()
  time.sleep(time.milliseconds(2))
  out(start.elapsed().as_nanoseconds() > 0)
  out(time.elapsed(start, time.instant()).as_nanoseconds() > 0)
  out(time.now().unix_seconds() > 0)
";
    let (code, stdout, stderr) = stdlib_common::run("time", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(
        stdout,
        "2000\n120\n3600000\n3500000\n-2500\n-2\n1000\n500\n-2500\ntrue\ntrue\ntrue\n"
    );
}

#[test]
fn overflowing_time_operations_use_canonical_trap() {
    for expression in [
        "time.hours(9223372036854775807)",
        "time.nanoseconds(9223372036854775807).add(time.nanoseconds(1))",
        "time.nanoseconds(-9223372036854775808).subtract(time.nanoseconds(1))",
        "time.from_unix_seconds(9223372036854775807)",
    ] {
        let source = format!("import time\nfn main():\n  value = {expression}\n  out(value)\n");
        let (code, _, stderr) = stdlib_common::run("time_overflow", &source);
        assert_ne!(code, Some(0));
        assert!(stderr.contains("numeric conversion"), "{stderr}");
    }
}
