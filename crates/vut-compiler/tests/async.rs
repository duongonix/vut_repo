use std::{fs, path::PathBuf, time::SystemTime};
use vut_compiler::{CompilerConfig, CompilerSession};

fn project(source: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-async-syntax-{nonce}"));
    fs::create_dir(&root).expect("create test source root");
    fs::write(root.join("main.vut"), source).expect("write test module");
    root
}

fn remove_project(root: &PathBuf) {
    fs::remove_file(root.join("main.vut")).expect("remove test module");
    fs::remove_dir_all(root).expect("remove test source root");
}

fn check(source: &str) -> vut_compiler::CheckedProgram {
    let root = project(source);
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session
        .check_source_root(&root, &[])
        .expect("check source root");
    remove_project(&root);
    checked
}

/// Diagnostics whose primary span belongs to the checked module (source 0).
///
/// The standard library is injected into the module graph as additional
/// sources, so unrelated stdlib diagnostics must not affect these assertions.
fn main_source_codes(checked: &vut_compiler::CheckedProgram) -> Vec<String> {
    let main = checked
        .resolution
        .modules
        .iter()
        .find(|module| module.logical_path.0 == ["main".to_owned()])
        .map(|module| module.source);
    let Some(main) = main else {
        return Vec::new();
    };
    checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .chain(checked.semantics.diagnostics.as_slice().iter())
        .filter(|item| {
            item.primary
                .as_ref()
                .is_some_and(|label| label.span.source() == main)
        })
        .filter_map(|item| item.code.map(|code| code.as_str().to_owned()))
        .collect()
}

#[test]
fn compile_pass_async_functions_methods_await_and_result() {
    let checked = check(
        "data User:\n  name: str\n  age: int\n\
fn User.label() -> str:\n  \"$(self.name) ($(self.age))\"\n\
async fn fetch_name() -> str:\n  \"Nam\"\n\
async fn read_age() -> int:\n  20\n\
async fn User.with_name(name: str) -> User:\n  User(name: name, age: self.age)\n\
async fn load_name() -> result[str, str]:\n  name = await fetch_name()\n  ok(name)\n\
async fn compute() -> int:\n  first = await read_age()\n  second = await read_age()\n  first + second\n\
async fn main():\n  total = await compute()\n  loaded = await load_name()\n  match loaded:\n    ok(text): out(\"$text\")\n    err(message): out(message)\n  base = User(name: \"Nam\", age: 20)\n  out(base.label())\n  renamed = await base.with_name(\"Ha\")\n  out(renamed.name)\n  out(\"$total\")\n",
    );
    assert!(
        main_source_codes(&checked).is_empty(),
        "{:?}",
        main_source_codes(&checked)
    );
    assert!(checked.semantics.async_symbols.len() >= 5);
}

#[test]
fn compile_fail_await_outside_async_fn() {
    let checked = check("fn main():\n  value = await load()\nasync fn load() -> int:\n  1\n");
    assert!(main_source_codes(&checked).contains(&"E6011".to_owned()));
}

#[test]
fn compile_fail_await_non_awaitable_value() {
    let checked = check("async fn main():\n  value = await 123\n");
    assert!(main_source_codes(&checked).contains(&"E6012".to_owned()));
}

#[test]
fn compile_fail_async_return_without_await() {
    let checked =
        check("async fn wrapper() -> int:\n  return load()\nasync fn load() -> int:\n  1\n");
    assert!(main_source_codes(&checked).contains(&"E6013".to_owned()));
}

#[test]
fn compile_fail_awaitable_used_as_ordinary_value() {
    let checked = check(
        "async fn main():\n  value = load()\n  out(\"$value\")\nasync fn load() -> int:\n  1\n",
    );
    assert!(main_source_codes(&checked).contains(&"E6014".to_owned()));
}

#[test]
fn compile_fail_async_without_fn() {
    let checked = check("async data Point:\n  x: int\n");
    assert!(main_source_codes(&checked).contains(&"E0112".to_owned()));
}

#[test]
fn compile_pass_anonymous_async_fn_in_vut() {
    let checked = check(
        "async fn load() -> int:\n  1\nasync fn main():\n  job = vut(async fn():\n    return await load()\n  )\n  out(\"$(await job)\")\n",
    );
    assert!(
        main_source_codes(&checked).is_empty(),
        "{:?}",
        main_source_codes(&checked)
    );
}

#[test]
fn compile_fail_anonymous_async_fn_outside_vut() {
    let checked = check(
        "async fn load() -> int:\n  1\nasync fn main():\n  job = async fn():\n    return await load()\n",
    );
    assert!(main_source_codes(&checked).contains(&"E6024".to_owned()));
}

#[test]
fn compile_fail_await_in_sync_fn_lambda() {
    let checked = check(
        "async fn load() -> int:\n  1\nasync fn main():\n  job = vut(fn():\n    value = await load()\n    value\n  )\n  out(\"$(await job)\")\n",
    );
    assert!(main_source_codes(&checked).contains(&"E6011".to_owned()));
}
