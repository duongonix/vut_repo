//! Step 0: proves the standard library compiles, links and executes natively.
use std::fs;

#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

use stdlib_common::run;

#[test]
fn fs_module_round_trips_a_file() {
    let dir = std::env::temp_dir().join(format!("vut-fs-sanity-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    let file = dir.join("sanity.txt");
    let path = file.to_string_lossy().replace('\\', "/");
    let source = format!(
        "import fs\nfn main():\n  p = \"{path}\"\n  match fs.write_str(p, \"hello stdlib\"):\n    ok(flag): out(\"wrote\")\n    err(error): out(\"write-failed\")\n  match fs.read_str(p):\n    ok(text): out(text)\n    err(error): out(\"read-failed\")\n  match fs.remove_file(p):\n    ok(flag): out(\"removed\")\n    err(error): out(\"remove-failed\")\n"
    );
    let (code, stdout, stderr) = run("fs_sanity", &source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "wrote\nhello stdlib\nremoved\n");
    fs::remove_dir_all(&dir).ok();
}
