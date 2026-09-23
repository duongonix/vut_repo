#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn lexical_path_surface_preserves_roots_and_parent_components() {
    let source = r#"import path
fn show(value: str?):
  if value == null:
    out("none")
    return
  out(value)
fn is_empty(value: str?) -> bool:
  if value == null:
    return false
  value == ""
fn main():
  out(path.normalize("a/./x/../b"))
  out(path.normalize("../a/../../b"))
  out(is_empty(path.parent("file")))
  show(path.file_name("a/file.txt"))
  show(path.stem(".hidden"))
  show(path.extension("a/file.txt"))
  out(path.with_extension("a/file.txt", ""))
  out(path.is_relative("relative"))
  out(path.components("a/b").len())
  out(path.separator())
  out(path.join("a", "b"))
"#;
    let (code, stdout, stderr) = stdlib_common::run("path", source);
    assert_eq!(code, Some(0), "{stderr}\n{stdout}");
    let sep = if cfg!(windows) { "\\" } else { "/" };
    assert_eq!(
        stdout,
        format!(
            "a{sep}b\n..{sep}..{sep}b\ntrue\nfile.txt\n.hidden\ntxt\na{sep}file\ntrue\n2\n{sep}\na{sep}b\n"
        )
    );
}

#[test]
fn platform_roots_drive_and_unc_are_lexical() {
    let source = if cfg!(windows) {
        r#"import path
fn main():
  out(path.is_absolute("C:\\a"))
  out(path.is_absolute("C:a"))
  out(path.is_absolute("\\a"))
  out(path.normalize("C:\\a\\..\\b"))
  out(path.normalize("\\\\server\\share\\a\\..\\b"))
  out(path.join("C:\\base", "\\child"))
  out(path.parent("C:\\") == null)
"#
    } else {
        r#"import path
fn main():
  out(path.is_absolute("/a"))
  out(path.normalize("/a/../../b"))
  out(path.join("/base", "/child"))
  out(path.parent("/") == null)
"#
    };
    let (code, stdout, stderr) = stdlib_common::run("path_root", source);
    assert_eq!(code, Some(0), "{stderr}\n{stdout}");
    assert_eq!(
        stdout,
        if cfg!(windows) {
            "true\nfalse\nfalse\nC:\\b\n\\\\server\\share\\b\nC:\\child\ntrue\n"
        } else {
            "true\n/b\n/child\ntrue\n"
        }
    );
}
