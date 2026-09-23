use std::{fs, path::PathBuf, time::SystemTime};
use vut_compiler::{CompilerConfig, CompilerSession};

fn temp(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{nonce}"))
}

#[test]
fn imports_file_folder_mod_children_alias_and_selected_symbols() {
    let root = temp("vut-modules-pass");
    fs::create_dir_all(root.join("src/service")).expect("create source tree");
    fs::create_dir_all(root.join("src/app")).expect("create app tree");
    fs::write(
        root.join("src/service.vut"),
        "fn source() -> str:\n  \"file\"\nfn get() -> int:\n  10\n",
    )
    .expect("write service file");
    fs::write(
        root.join("src/service/mod.vut"),
        "fn source() -> str:\n  \"folder\"\nfn get() -> int:\n  20\n",
    )
    .expect("write service mod");
    fs::write(
        root.join("src/service/client.vut"),
        "fn status() -> int:\n  30\n",
    )
    .expect("write child");
    fs::write(root.join("src/app/mod.vut"), "fn boot() -> int:\n  1\n").expect("write app mod");
    fs::write(
        root.join("src/main.vut"),
        "import service\nimport service as h\nimport service at get\nimport service.client as client\nimport app\nfn main() -> int:\n  get() + client.status()\n",
    )
    .expect("write main");

    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root.join("src"), &[])
        .expect("check source");

    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(
        !checked.semantics.diagnostics.has_errors(),
        "{:?}",
        checked.semantics.diagnostics.as_slice()
    );
    let service = checked
        .resolution
        .modules
        .iter()
        .find(|module| module.logical_path.display() == "service")
        .expect("service module");
    assert!(
        service
            .filesystem_path
            .as_ref()
            .unwrap()
            .ends_with("service.vut")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn resolves_qualified_type_references_across_modules() {
    let root = temp("vut-qualified-types");
    fs::create_dir_all(root.join("src/service")).expect("create source tree");
    fs::write(
        root.join("src/service.vut"),
        "data Client:\n  name: str\n\nfn make() -> Client:\n  Client(name: \"x\")\n",
    )
    .expect("write service");
    fs::write(
        root.join("src/service/status.vut"),
        "data Status:\n  code: int\n\nfn ok() -> Status:\n  Status(code: 200)\n",
    )
    .expect("write status");
    fs::write(
        root.join("src/main.vut"),
        "import service\nimport service.status as status\n\nfn accepts(client: service.Client, s: status.Status) -> int:\n  s.code\n\nfn main() -> int:\n  accepts(service.make(), status.ok())\n",
    )
    .expect("write main");

    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root.join("src"), &[])
        .expect("check source");
    assert!(
        !checked.resolution.diagnostics.has_errors(),
        "{:?}",
        checked.resolution.diagnostics.as_slice()
    );
    assert!(
        !checked.semantics.diagnostics.has_errors(),
        "{:?}",
        checked.semantics.diagnostics.as_slice()
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn qualified_type_reference_to_private_symbol_is_rejected() {
    let root = temp("vut-qualified-private");
    fs::create_dir_all(root.join("src")).expect("create source tree");
    fs::write(
        root.join("src/service.vut"),
        "data _Hidden:\n  value: int\n",
    )
    .expect("write service");
    fs::write(
        root.join("src/main.vut"),
        "import service\n\nfn accepts(value: service._Hidden) -> int:\n  1\n\nfn main() -> int:\n  accepts(service._Hidden(value: 1))\n",
    )
    .expect("write main");

    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root.join("src"), &[])
        .expect("check source");
    assert!(checked.resolution.diagnostics.has_errors());
    assert!(
        checked
            .resolution
            .diagnostics
            .as_slice()
            .iter()
            .any(|diagnostic| diagnostic.title == "private symbol `_Hidden`"),
        "{:?}",
        checked.resolution.diagnostics.as_slice()
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn local_namespace_conflicts_with_dependency_namespace() {
    let root = temp("vut-module-conflict");
    fs::create_dir_all(root.join("src")).expect("create source tree");
    fs::create_dir_all(root.join("dep/src")).expect("create dependency tree");
    fs::write(root.join("src/service.vut"), "fn local() -> int:\n  1\n").expect("write local");
    fs::write(root.join("src/main.vut"), "import service\n").expect("write main");
    fs::write(root.join("dep/src/mod.vut"), "fn remote() -> int:\n  2\n").expect("write dep");

    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_roots(&[
            (root.join("src"), Vec::new()),
            (root.join("dep/src"), vec!["service".into()]),
        ])
        .expect("check source roots");

    assert!(checked.resolution.diagnostics.has_errors());
    assert!(
        checked
            .resolution
            .diagnostics
            .as_slice()
            .iter()
            .any(|diagnostic| diagnostic.title == "module namespace conflict")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn dependency_alias_avoids_local_namespace_conflict() {
    let root = temp("vut-module-alias");
    fs::create_dir_all(root.join("src")).expect("create source tree");
    fs::create_dir_all(root.join("dep/src")).expect("create dependency tree");
    fs::write(root.join("src/service.vut"), "fn local() -> int:\n  1\n").expect("write local");
    fs::write(
        root.join("src/main.vut"),
        "import service\nimport webservice\n",
    )
    .expect("write main");
    fs::write(root.join("dep/src/mod.vut"), "fn remote() -> int:\n  2\n").expect("write dep");

    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_roots(&[
            (root.join("src"), Vec::new()),
            (root.join("dep/src"), vec!["webservice".into()]),
        ])
        .expect("check source roots");

    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(!checked.semantics.diagnostics.has_errors());
    fs::remove_dir_all(root).expect("cleanup");
}
