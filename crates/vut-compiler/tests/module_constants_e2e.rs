#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn module_level_scalar_constants_are_inlined() {
    let source = r"MAX = 5
LIMIT: int = 7
SCALED = MAX * 2
PI_ISH = 3.5
FLAG = true

fn main():
  out(MAX)
  out(LIMIT)
  out(SCALED)
  out(PI_ISH)
  out(FLAG)
  out(MAX > 3 and MAX < 6)
";
    let (code, stdout, stderr) = stdlib_common::run("module_constants", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "5\n7\n10\n3.5\ntrue\ntrue\n");
}
