//! Zero-sized values through collection and optional storage.
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn unit_collections_and_optional_payloads_execute() {
    let source = r#"fn accept(value: unit) -> int:
  7
fn main():
  values: list[unit] = @[unit, unit]
  values.push(unit)
  values.set(0, unit)
  out(accept(values.at(0)))
  out(accept(values.pop()))
  fixed: array[unit, 2] = [unit, unit]
  fixed.set(1, unit)
  out(accept(fixed.at(1)))
  table: map[str, unit] = ("a": unit)
  table.set("b", unit)
  out(accept(table.get_or("b", unit)))
  present: unit? = unit
  absent: unit? = null
  out(present != null)
  out(absent == null)
  if present != null:
    out(accept(present))
"#;
    let (code, stdout, stderr) = stdlib_common::run("unit_storage", source);
    assert_eq!(code, Some(0), "{stderr}\n{stdout}");
    assert_eq!(stdout, "7\n7\n7\n7\ntrue\ntrue\n7\n");
}

#[test]
fn discarded_owned_results_and_terminating_branch_cleanup() {
    let source = r#"fn choose(value: bool) -> str:
  if value:
    temporary = "branch"
    return temporary
  "continuation"
fn main():
  names = @["one", "two", "three"]
  names.pop()
  names.remove(0)
  out(choose(false))
  out(choose(true))
"#;
    let (code, stdout, stderr) = stdlib_common::run("discard_cleanup", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "continuation\nbranch\n");
}
