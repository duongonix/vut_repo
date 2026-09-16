use std::{fs, path::PathBuf, time::SystemTime};
use vut_compiler::{CompilerConfig, CompilerSession};

fn check(source: &str) -> (PathBuf, vut_compiler::CheckedProgram) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-list-{nonce}"));
    fs::create_dir(&root).expect("create source root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root, &[])
        .expect("check source");
    (root, checked)
}

fn cleanup(root: &PathBuf) {
    fs::remove_dir_all(root).expect("remove source root");
}

#[test]
fn list_builtin_methods_are_typed() {
    let (root, checked) = check(
        "fn main():\n  items = @(1, 2, 3)\n  items.reserve(8)\n  items.push(4)\n  items.set(1, 20)\n  items.insert(2, 30)\n  removed = items.remove(0)\n  sliced = items.slice(1, 3)\n  has_twenty = items.contains(20)\n  out(\"$(items.len()) $(items.capacity()) $(items.at(0)) $removed $(sliced.len()) $has_twenty $(items.is_empty())\")\n  items.clear()\n  out(\"$(items.len())\")\n",
    );
    cleanup(&root);
    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(
        !checked.semantics.diagnostics.has_errors(),
        "{:?}",
        checked.semantics.diagnostics.as_slice()
    );
}

#[test]
fn list_builtin_methods_are_generic_over_element_type() {
    let (root, checked) = check(
        "fn main():\n  names = @(\"a\", \"b\")\n  names.push(\"c\")\n  names.set(0, \"z\")\n  out(\"$(names.len()) $(names.at(0))\")\n",
    );
    cleanup(&root);
    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(
        !checked.semantics.diagnostics.has_errors(),
        "{:?}",
        checked.semantics.diagnostics.as_slice()
    );
}

#[test]
fn list_builtin_methods_reject_wrong_element_types() {
    for source in [
        "fn main():\n  items = @(1, 2, 3)\n  items.push(\"bad\")\n",
        "fn main():\n  items = @(1, 2, 3)\n  items.set(0, \"bad\")\n",
        "fn main():\n  items = @(1, 2, 3)\n  items.contains(\"bad\")\n",
    ] {
        let (root, checked) = check(source);
        cleanup(&root);
        assert!(checked.semantics.diagnostics.has_errors(), "{source}");
    }
}
