use std::{fs, path::PathBuf, time::SystemTime};
use vut_compiler::{CompilerConfig, CompilerSession};

fn project(source: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-method-syntax-{nonce}"));
    fs::create_dir(&root).expect("create test source root");
    fs::write(root.join("main.vut"), source).expect("write test module");
    root
}

fn remove_project(root: &PathBuf) {
    fs::remove_file(root.join("main.vut")).expect("remove test module");
    fs::remove_dir_all(root).expect("remove test source root");
}

#[test]
fn compile_pass_method_requires_fn_and_has_implicit_self() {
    let root = project(
        "data Counter:\n  value: int = 0\nfn Counter.increment():\n  self.value = self.value + 1\nfn Counter.add(amount: int):\n  self.value = self.value + amount\nfn Counter.get() -> int:\n  self.value\n",
    );
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session
        .check_source_root(&root, &[])
        .expect("check source root");
    remove_project(&root);

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
    assert_eq!(checked.resolution.modules[0].methods.len(), 3);
}

#[test]
fn compile_fail_removed_method_syntax_has_actionable_diagnostic() {
    let root = project(
        "data Counter:\n  value: int = 0\nCounter.increment():\n  self.value = self.value + 1\n",
    );
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session
        .check_source_root(&root, &[])
        .expect("check source root");
    remove_project(&root);

    let diagnostic = checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .find(|item| item.code.as_deref() == Some("E0110"))
        .expect("old method syntax must fail with E0110");
    assert_eq!(diagnostic.title, "method declarations require `fn`");
    assert_eq!(
        diagnostic.help.as_deref(),
        Some("write `fn Type.method(...):`")
    );
}
