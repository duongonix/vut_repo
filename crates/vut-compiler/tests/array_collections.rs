use std::{fs, path::PathBuf, time::SystemTime};
use vut_compiler::{CompilerConfig, CompilerSession};

fn check(source: &str) -> (PathBuf, vut_compiler::CheckedProgram) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-array-{nonce}"));
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
fn list_and_array_literals_never_contextually_interchange() {
    let (root, checked) = check(
        "fn main():\n  list_value: list[int] = @[1, 2, 3]\n  array_value: array[int, 3] = [1, 2, 3]\n",
    );
    cleanup(&root);
    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(!checked.semantics.diagnostics.has_errors());

    let (root, checked) = check(
        "fn main():\n  wrong_array: array[int, 3] = @[1, 2, 3]\n  wrong_list: list[int] = [1, 2, 3]\n",
    );
    cleanup(&root);
    assert!(checked.semantics.diagnostics.has_errors());
}

#[test]
fn array_rejects_empty_mixed_length_and_grow_operations() {
    for source in [
        "fn main():\n  values = []\n",
        "fn main():\n  values = [1, \"two\"]\n",
        "fn main():\n  values: array[int, 4] = [1, 2, 3]\n",
        "fn main():\n  values = [1, 2, 3]\n  values.push(4)\n",
    ] {
        let (root, checked) = check(source);
        cleanup(&root);
        assert!(checked.semantics.diagnostics.has_errors(), "{source}");
    }
}

#[test]
fn array_operations_and_iteration_are_fully_typed() {
    let (root, checked) = check(
        "fn main():\n  values = [1, 2, 3]\n  values.set(0, 9)\n  values.fill(7)\n  out(\"$(values.len()) $(values.first()) $(values.last()) $(values.at(1))\")\n  for value, index in values:\n    out(\"$index:$value\")\n",
    );
    cleanup(&root);
    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(
        !checked.semantics.diagnostics.has_errors(),
        "{:?}",
        checked.semantics.diagnostics.as_slice()
    );
}
