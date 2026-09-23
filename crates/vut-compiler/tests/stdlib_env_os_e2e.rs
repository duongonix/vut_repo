#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn environment_map_absence_errors_and_arguments() {
    let source = r#"import env
fn main():
  out(env.has("VUT_M13_ABSENT_471"))
  match env.get("VUT_M13_ABSENT_471"):
    ok(value): out(value == null)
    err(error): out(error.message)
  match env.set("VUT_M13_ENV", "a=b"):
    ok(done): out("set")
    err(error): out(error.message)
  match env.all():
    ok(values): out(values.get_or("VUT_M13_ENV", "missing"))
    err(error): out(error.message)
  match env.remove("VUT_M13_ENV"):
    ok(done): out("removed")
    err(error): out(error.message)
  match env.set("BAD=NAME", "x"):
    ok(done): out("bad")
    err(error):
      match error.kind:
        invalid_name: out("invalid name")
        _: out("wrong error")
  match env.set("VUT_M13_ENV", "x\0y"):
    ok(done): out("bad")
    err(error):
      match error.kind:
        invalid_value: out("invalid value")
        _: out("wrong error")
  out(env.args().len() > 0)
  out(env.arg(9999) == null)
"#;
    let (code, stdout, stderr) = stdlib_common::run("env", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(
        stdout,
        "false\ntrue\nset\na=b\nremoved\ninvalid name\ninvalid value\ntrue\ntrue\n"
    );
}

#[test]
fn os_paths_platform_info_and_typed_failure() {
    let source = r#"import os
fn main():
  out(os.name())
  out(os.arch())
  out(os.family())
  out(os.cpu_count() > 0)
  match os.home_dir():
    ok(value): out(not value.is_empty())
    err(error): out(error.message)
  match os.temp_dir():
    ok(value): out(not value.is_empty())
    err(error): out(error.message)
  match os.current_dir():
    ok(value): out(not value.is_empty())
    err(error): out(error.message)
  match os.current_exe():
    ok(value): out(not value.is_empty())
    err(error): out(error.message)
  match os.set_current_dir("vut-no-such-directory-m13"):
    ok(done): out("bad")
    err(error):
      match error.kind:
        not_found: out("not found")
        _: out("wrong error")
"#;
    let (code, stdout, stderr) = stdlib_common::run("os", source);
    assert_eq!(code, Some(0), "{stderr}");
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let family = if cfg!(windows) { "windows" } else { "unix" };
    assert_eq!(
        stdout,
        format!("{platform}\n{arch}\n{family}\ntrue\ntrue\ntrue\ntrue\ntrue\nnot found\n")
    );
}
