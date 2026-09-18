//! `fs` module end-to-end tests: real compilation, native execution, and leak
//! detection (a clean exit code means no live managed allocation).
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

use std::fs;

use stdlib_common::run;

fn scratch(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("vut-fs-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir.to_string_lossy().replace('\\', "/")
}

#[test]
fn fs_module_round_trips_a_file() {
    let dir = scratch("roundtrip");
    let path = format!("{dir}/sanity.txt");
    let source = format!(
        "import fs\nfn main():\n  p = \"{path}\"\n  match fs.write_str(p, \"hello stdlib\"):\n    ok(flag): out(\"wrote\")\n    err(error): out(\"write-failed\")\n  match fs.read_str(p):\n    ok(text): out(text)\n    err(error): out(\"read-failed\")\n  match fs.remove_file(p):\n    ok(flag): out(\"removed\")\n    err(error): out(\"remove-failed\")\n"
    );
    let (code, stdout, stderr) = run("fs_sanity", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "wrote\nhello stdlib\nremoved\n");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_resource_write_append_and_sync() {
    let dir = scratch("file");
    let path = format!("{dir}/a.txt");
    let source = format!(
        "import fs\nimport fs at File\nfn main():\n  p = \"{path}\"\n  match File.create(p):\n    ok(file):\n      written = file.write(\"hello\".to_bytes())\n      synced = file.sync()\n      out(\"created\")\n    err(error):\n      out(\"create-failed\")\n  match fs.append_str(p, \" world\"):\n    ok(flag): out(\"appended\")\n    err(error): out(\"append-failed\")\n  match fs.read_str(p):\n    ok(text): out(text)\n    err(error): out(\"read-failed\")\n"
    );
    let (code, stdout, stderr) = run("fs_file", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "created\nappended\nhello world\n");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn metadata_permissions_and_read_dir() {
    let dir = scratch("meta");
    let path = format!("{dir}/b.txt");
    let source = format!(
        "import fs\nfn main():\n  p = \"{path}\"\n  match fs.write_str(p, \"abcde\"):\n    ok(flag): out(\"wrote\")\n    err(error): out(\"write-failed\")\n  match fs.metadata(p):\n    ok(meta): out(\"len=$(meta.len()) file=$(meta.is_file()) dir=$(meta.is_dir())\")\n    err(error): out(\"meta-failed\")\n  match fs.permissions(p):\n    ok(perm): out(\"readonly=$(perm.readonly())\")\n    err(error): out(\"perm-failed\")\n  match fs.read_dir(\"{dir}\"):\n    ok(entries):\n      out(\"entries=$(entries.len())\")\n      for entry in entries:\n        out(\"$(entry.name()) file=$(entry.is_file())\")\n    err(error): out(\"readdir-failed\")\n"
    );
    let (code, stdout, stderr) = run("fs_meta", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(
        stdout,
        "wrote\nlen=5 file=1 dir=0\nreadonly=0\nentries=1\nb.txt file=1\n"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_satisfies_io_reader() {
    let dir = scratch("reader");
    let path = format!("{dir}/c.txt");
    let source = format!(
        "import fs\nimport io\nimport fs at File\nfn read_via_io(p: str) -> result(str, fs.FsError):\n  file = File.open(p)?\n  outcome = io.read_to_str(file)\n  value: result(str, fs.FsError) = match outcome:\n    ok(text): ok(text)\n    err(error): err(fs.FsError(kind = fs.FsErrorKind.io, message = \"io failed\"))\n  value\nfn main():\n  p = \"{path}\"\n  ignored = fs.write_str(p, \"streamed text\")\n  match read_via_io(p):\n    ok(text): out(text)\n    err(error): out(\"read-failed\")\n"
    );
    let (code, stdout, stderr) = run("fs_reader", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "streamed text\n");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_file_reports_not_found() {
    let dir = scratch("missing");
    let path = format!("{dir}/nope.txt");
    let source = format!(
        "import fs\nfn main():\n  match fs.read_str(\"{path}\"):\n    ok(text): out(\"unexpected\")\n    err(error): out(\"not-found\")\n"
    );
    let (code, stdout, stderr) = run("fs_missing", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "not-found\n");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn open_missing_file_reports_error_without_leaking() {
    let dir = scratch("openmissing");
    let path = format!("{dir}/nope.txt");
    let source = format!(
        "import fs\nimport fs at File\nfn main():\n  match File.open(\"{path}\"):\n    ok(file): out(\"unexpected\")\n    err(error): out(\"open-failed\")\n"
    );
    let (code, stdout, stderr) = run("fs_open_missing", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "open-failed\n");
    fs::remove_dir_all(&dir).ok();
}
