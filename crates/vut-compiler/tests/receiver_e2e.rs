use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn run(source: &str) -> (Option<i32>, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-receiver-e2e-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .expect("emit executable");
    let output = Command::new(&executable).output().expect("run executable");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    fs::remove_dir_all(&root).ok();
    (output.status.code(), stdout)
}

fn diagnostic_codes(source: &str) -> Vec<String> {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-receiver-diag-{nonce}"));
    fs::create_dir(&root).expect("create root");
    let path = root.join("main.vut");
    fs::write(&path, source).expect("write source");
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session.check_source_file(Path::new(&path)).expect("check");
    let codes = checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .chain(checked.semantics.diagnostics.as_slice())
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    fs::remove_dir_all(&root).ok();
    codes
}

const NESTED_DSL: &str = "data AppScope:\n  label: str\n\ndata ColumnScope:\n  label: str\n\nfn AppScope.Column(body: fn(ColumnScope)() -> void) -> void:\n  scope = ColumnScope(label = \"col\")\n  body.call(scope)\n\nfn ColumnScope.Button(text: str) -> void:\n  out(\"button=$text\")\n\nfn App(body: fn(AppScope)() -> void) -> void:\n  scope = AppScope(label = \"app\")\n  body.call(scope)\n\nfn main():\n  App():\n    Column():\n      Button(\"A\")\n";

#[test]
fn nested_receiver_dsl_resolves_statically_and_runs() {
    let (code, stdout) = run(NESTED_DSL);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "button=A\n");
}

#[test]
fn trailing_receiver_with_parameters_and_control_flow() {
    let source = "data UserCardScope:\n  label: str\n\ndata User:\n  name: str\n\ndata Event:\n  clicked: bool\n\nfn UserCard(user: User, body: fn(UserCardScope)(Event, int) -> void) -> void:\n  scope = UserCardScope(label = \"card\")\n  event = Event(clicked = true)\n  body.call(scope, event, 7)\n\nfn UserCardScope.Text(text: str) -> void:\n  out(\"text=$text\")\n\nfn main():\n  user = User(name = \"ann\")\n  UserCard(user) (event, index):\n    Text(\"$index\")\n    if event.clicked:\n      Text(\"clicked\")\n    for value in @(1, 2):\n      Text(\"loop=$value\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "text=7\ntext=clicked\ntext=loop=1\ntext=loop=2\n");
}

#[test]
fn dot_call_invokes_a_trailing_receiver_function() {
    let source = "data Scope:\n  label: str\n\nfn capture(body: fn(Scope)() -> int) -> fn(Scope)() -> int:\n  body\n\nfn main():\n  scope = Scope(label = \"s\")\n  body = capture():\n    41\n  out(\"value=$(body.call(scope))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "value=41\n");
}

#[test]
fn explicit_self_refers_to_the_receiver() {
    let source = "data Scope:\n  label: str\n\nfn Render(body: fn(Scope)() -> void) -> void:\n  scope = Scope(label = \"hi\")\n  body.call(scope)\n\nfn main():\n  Render():\n    out(\"label=$(self.label)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "label=hi\n");
}

#[test]
fn receiver_body_without_managed_state_does_not_allocate() {
    let source = "data Scope:\n  label: str\n\nfn Render(body: fn(Scope)() -> void) -> void:\n  scope = Scope(label = \"x\")\n  body.call(scope)\n\nfn main():\n  Render():\n    out(\"body\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "no receiver heap allocation must be reported: {stdout}"
    );
    assert_eq!(stdout, "body\n");
}

#[test]
fn dot_call_on_a_plain_function_is_rejected() {
    let codes = diagnostic_codes(
        "fn double(x: int) -> int:\n  x * 2\n\nfn main():\n  f = double\n  f.call(1)\n",
    );
    assert!(codes.iter().any(|code| code == "E1025"), "{codes:?}");
}

#[test]
fn trailing_body_requires_a_receiver_function_parameter() {
    let codes = diagnostic_codes(
        "fn consume(callback: fn() -> void) -> void:\n  callback()\n\nfn main():\n  consume():\n    out(\"x\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1020"), "{codes:?}");
}

#[test]
fn dot_call_with_the_wrong_receiver_is_rejected() {
    let codes = diagnostic_codes(
        "data A:\n  x: int\n\ndata B:\n  y: int\n\nfn use(body: fn(A)() -> void) -> void:\n  out(\"u\")\n\nfn main():\n  body: fn(A)() -> void = fn():\n    out(\"hi\")\n  b = B(y = 2)\n  body.call(b)\n",
    );
    assert!(codes.iter().any(|code| code == "E1021"), "{codes:?}");
}

#[test]
fn unknown_implicit_receiver_method_is_reported() {
    let codes = diagnostic_codes(
        "data Scope:\n  label: str\n\nfn Render(body: fn(Scope)() -> void) -> void:\n  out(\"r\")\n\nfn main():\n  Render():\n    Missing()\n",
    );
    assert!(codes.iter().any(|code| code == "E1023"), "{codes:?}");
}

#[test]
fn capturing_receiver_body_is_still_rejected() {
    let codes = diagnostic_codes(
        "data Scope:\n  label: str\n\nfn Render(body: fn(Scope)() -> void) -> void:\n  out(\"r\")\n\nfn main():\n  title = \"x\"\n  Render():\n    out(title)\n",
    );
    assert!(codes.iter().any(|code| code == "E1013"), "{codes:?}");
}
