//! Metadata errors and open-handle identity must survive native lowering.
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn metadata_snapshot_is_typed_and_open_file_survives_rename() {
    let scratch = stdlib_common::scratch("metadata_source");
    let original = scratch
        .root
        .join("before")
        .to_string_lossy()
        .replace('\\', "/");
    let renamed = scratch
        .root
        .join("after")
        .to_string_lossy()
        .replace('\\', "/");
    let source = format!(
        r#"import fs
import fs at File
fn exercise() -> result[unit, fs.FsError]:
  fs.write_str("{original}", "payload")?
  file = File.open("{original}")?
  fs.rename("{original}", "{renamed}")?
  snapshot = file.metadata()?
  size: u64 = snapshot.len()
  out(size)
  out(snapshot.is_file())
  match snapshot.modified():
    ok(stamp): out(stamp.unix_seconds() > 0)
    err(error): out(error.message)
  match fs.metadata("{original}"):
    ok(unexpected): out("unexpected metadata")
    err(error):
      match error.kind:
        not_found: out("not_found")
        _: out("wrong_error")
  ok(unit)
fn main():
  match exercise():
    ok(done): out("done")
    err(error): out(error.message)
"#
    );
    let (code, stdout, stderr) = stdlib_common::run("metadata", &source);
    assert_eq!(code, Some(0), "{stderr}\n{stdout}");
    assert_eq!(stdout, "7\ntrue\ntrue\nnot_found\ndone\n");
}
