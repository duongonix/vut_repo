use std::{fs, path::PathBuf, time::SystemTime};

use vut_compiler::{CompilerConfig, CompilerSession};

/// The documented HTTP examples must stay valid Vut. They are compile-checked
/// only: running them would require network access.
#[test]
fn http_examples_type_check() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-http-example-{nonce}"));
    fs::create_dir(&root).expect("create root");
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/http.vut");
    fs::copy(&example, root.join("main.vut")).expect("copy example");

    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session
        .check_source_root(&root, &[])
        .expect("check example");
    fs::remove_dir_all(&root).ok();

    let mut diagnostics = Vec::new();
    for item in checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .chain(checked.semantics.diagnostics.as_slice())
    {
        if let Some(code) = item.code {
            diagnostics.push(code.as_str().to_owned());
        }
    }
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}
