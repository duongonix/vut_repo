//! Inline compiler session and end-to-end tests.

use super::*;
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[test]
fn session_keeps_state_local() {
    let mut session = CompilerSession::new(CompilerConfig::default());
    let id = session
        .sources_mut()
        .add_text("main.vut", "name = \"Nam\"".into());
    assert_eq!(session.sources().get(id).unwrap().name(), "main.vut");
    assert!(!session.diagnostics().has_errors());
}
#[test]
fn target_and_build_mode_are_explicit() {
    let session = CompilerSession::new(CompilerConfig::for_target(
        "x86_64-unknown-linux-gnu",
        BuildMode::Release,
    ));
    assert_eq!(session.config().target, "x86_64-unknown-linux-gnu");
    assert_eq!(session.config().build_mode, BuildMode::Release);
}

#[test]
fn session_runs_discovery_through_name_resolution() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-compiler-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("helper.vut"), "fn value() -> int:\n  1\n").unwrap();
    fs::write(
        root.join("main.vut"),
        "import helper\nfn run() -> int:\n  helper.value()\n",
    )
    .unwrap();

    let mut session = CompilerSession::new(CompilerConfig::default());
    let resolution = session.resolve_source_root(&root, &[]).unwrap();
    assert!(!resolution.diagnostics.has_errors());
    assert_eq!(resolution.modules.len(), 2);
    assert_eq!(session.sources().len(), 2);

    fs::remove_file(root.join("helper.vut")).unwrap();
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn source_pipeline_emits_a_native_object() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-native-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.vut"), "fn main() -> int:\n  42\n").unwrap();
    let mut session = CompilerSession::new(CompilerConfig::default());
    let object = session.emit_object(&root, &[]).unwrap();
    assert!(object.len() > 64);
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incremental_object_cache_hits_invalidates_by_mode_and_recovers_corruption() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-incremental-{nonce}"));
    let cache = root.join("cache");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.vut"), "fn main() -> int:\n  40 + 2\n").unwrap();
    let debug = CompilerConfig {
        output_dir: Some(cache.clone()),
        ..CompilerConfig::default()
    };
    let mut cold = CompilerSession::new(debug.clone());
    let debug_key = cold
        .cache_key(&[(root.clone(), Vec::new())], "object")
        .unwrap();
    let first = cold.emit_object(&root, &[]).unwrap();
    let mut warm = CompilerSession::new(debug);
    assert_eq!(warm.emit_object(&root, &[]).unwrap(), first);
    assert_eq!(
        warm.sources().len(),
        0,
        "warm cache hit must skip the frontend"
    );

    let release = CompilerConfig {
        output_dir: Some(cache.clone()),
        build_mode: BuildMode::Release,
        ..CompilerConfig::default()
    };
    let mut different_mode = CompilerSession::new(release);
    different_mode.emit_object(&root, &[]).unwrap();
    assert_eq!(
        different_mode.sources().len(),
        1,
        "release must not reuse debug objects"
    );

    let cache_file = cache.join(format!("{}.bin", debug_key.as_str()));
    fs::write(cache_file, "corrupt").unwrap();
    let debug = CompilerConfig {
        output_dir: Some(cache),
        ..CompilerConfig::default()
    };
    let mut recovered = CompilerSession::new(debug);
    recovered.emit_object(&root, &[]).unwrap();
    assert_eq!(
        recovered.sources().len(),
        1,
        "corrupt entries must be rebuilt"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_pipeline_links_and_runs_a_native_executable() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-run-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("main.vut"),
        "fn helper() -> int:\n  1\nfn main() -> int:\n  42\n",
    )
    .unwrap();
    let executable = root.join("program.exe");
    let mut session = CompilerSession::new(CompilerConfig::default());
    session.emit_executable(&root, &[], &executable).unwrap();
    let status = std::process::Command::new(&executable).status().unwrap();
    assert_eq!(status.code(), Some(42));
    fs::remove_file(executable).unwrap();
    let _ = fs::remove_file(root.join("program.pdb"));
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_result_match_executes_end_to_end() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-result-run-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn main() -> int:\n  value: result(int, int) = ok(42)\n  match value:\n    ok(number): number\n    err(code): code\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let mut session = CompilerSession::new(CompilerConfig::default());
    session.emit_executable(&root, &[], &executable).unwrap();
    let status = std::process::Command::new(&executable).status().unwrap();
    assert_eq!(status.code(), Some(42));
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn void_main_uses_zero_exit_code() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-void-main-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.vut"), "fn main():\n  print(\"ok\")\n").unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"ok");
    fs::remove_file(executable).unwrap();
    let _ = fs::remove_file(root.join("program.pdb"));
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn native_template_calls_the_versioned_runtime() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-runtime-link-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("main.vut"),
        "fn main() -> int:\n  number = 41\n  out(\"answer=$(number + 1)\")\n  0\n",
    )
    .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    let mut session = CompilerSession::new(config);
    let checked = session.check_source_root(&root, &[]).unwrap();
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
    session.emit_executable(&root, &[], &executable).unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"answer=42\n");
    fs::remove_file(executable).unwrap();
    let _ = fs::remove_file(root.join("program.pdb"));
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn builtin_list_string_and_numeric_methods_execute_natively() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-builtin-methods-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn main():\n  values: list(int) = @(10, 20, 30)\n  text = \"  Vut  \"\n  out(\"$(values.len())\")\n  out(text.trim().to_upper())\n  number = 42\n  out(number.to_str())\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"3\nVUT\n42\n");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn native_string_addition_concatenates() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-string-plus-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn main():\n  hello = \"hello\"\n  hi = \"hi\"\n  c = \"xin chao \" + hello + hi\n  out(c)\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"xin chao hellohi\n");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn builtin_list_mutation_methods_execute_natively() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-list-methods-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn main():\n  items = @(1, 2, 3)\n  items.reserve(8)\n  items.push(4)\n  items.set(1, 20)\n  items.insert(2, 30)\n  removed = items.remove(0)\n  sliced = items.slice(1, 3)\n  out(\"$(items.len()) $(items.capacity()) $(items.at(0)) $removed $(sliced.len()) $(items.contains(20))\")\n  items.clear()\n  out(\"$(items.is_empty())\")\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"4 8 20 1 2 1\n1\n");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn builtin_map_methods_execute_natively() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-map-methods-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn main():\n  scores = map((1, 10), (2, 20))\n  scores.reserve(16)\n  scores.set(1, 30)\n  scores.set(3, 40)\n  old = scores.remove(2)\n  enough = scores.capacity() >= 16\n  out(\"$(scores.len()) $enough $(scores.get(1)) $old $(scores.contains_key(3))\")\n  scores.clear()\n  out(\"$(scores.is_empty())\")\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"2 1 30 20 1\n1\n");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn native_list_iterator_executes_real_cfg() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-iterator-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.vut"), "fn main() -> int:\n  values: list(int) = @(40, 42)\n  for value, index in values:\n    if index == 1:\n      return value\n  0\n").unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    assert_eq!(
        std::process::Command::new(&executable)
            .status()
            .unwrap()
            .code(),
        Some(42)
    );
    fs::remove_file(executable).unwrap();
    let _ = fs::remove_file(root.join("program.pdb"));
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn native_method_uses_receiver_first_abi() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-method-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.vut"), "data Box:\n  value: int\nfn Box.get() -> int:\n  self.value\nfn main() -> int:\n  box = Box(value = 42)\n  box.get()\n").unwrap();
    let executable = root.join("program.exe");
    CompilerSession::new(CompilerConfig::default())
        .emit_executable(&root, &[], &executable)
        .unwrap();
    assert_eq!(
        std::process::Command::new(&executable)
            .status()
            .unwrap()
            .code(),
        Some(42)
    );
    fs::remove_file(executable).unwrap();
    let _ = fs::remove_file(root.join("program.pdb"));
    fs::remove_file(root.join("main.vut")).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn complete_mvp_program_compiles_and_executes() {
    use std::io::Write as _;
    use std::process::Stdio;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-mvp-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/mvp_integration.vut"),
        root.join("main.vut"),
    )
    .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let mut child = std::process::Command::new(&executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"Nam\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Name: Hello Nam\n0: 10\n1: 20\n2: 30\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_ownership_balances_alias_overwrite_and_early_return() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-ownership-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn early(flag: bool) -> int:\n  temporary = \"owned\"\n  if flag:\n    return 7\n  0\nfn main():\n  value = \"first\"\n  out(value)\n  alias = value\n  value = \"second\"\n  out(alias)\n  out(value)\n  _result = early(true)\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "memory leak exit: {output:?}"
    );
    assert_eq!(output.stdout, b"first\nfirst\nsecond\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_typed_numeric_operator_reaches_native_codegen() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-numeric-operators-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn main() -> int:\n  remainder: float = 7.5 % 2.0\n  integer = ((8 + 4) - 2) * 3 / 2 % 8\n  logic = true and not false or false\n  if remainder == 1.5 and integer == 7 and logic:\n    return 0\n  return 1\n",
        )
        .unwrap();
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    assert_eq!(
        std::process::Command::new(&executable)
            .status()
            .unwrap()
            .code(),
        Some(0)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn whole_and_aliased_module_imports_call_exported_functions() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    for (case, source) in [
        (
            "whole",
            "import hello\nfn main() -> int:\n  hello.answer()\n",
        ),
        (
            "alias",
            "import hello as h\nfn main() -> int:\n  h.answer()\n",
        ),
    ] {
        let root = std::env::temp_dir().join(format!(
            "vut-module-call-{case}-{nonce}-{}",
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("hello.vut"), "fn answer() -> int:\n  42\n").unwrap();
        fs::write(root.join("main.vut"), source).unwrap();
        let executable = root.join(if cfg!(windows) {
            format!("{case}.exe")
        } else {
            case.to_owned()
        });
        CompilerSession::new(CompilerConfig::default())
            .emit_executable(&root, &[], &executable)
            .unwrap();
        assert_eq!(
            std::process::Command::new(executable)
                .status()
                .unwrap()
                .code(),
            Some(42),
            "failed module import case: {case}"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn native_bytes_flow_executes_end_to_end() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-bytes-flow-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
            root.join("main.vut"),
            "fn decode(blob: bytes) -> str:\n  match blob.to_str():\n    ok(value): value\n    err(error): \"invalid utf-8\"\nfn main():\n  text = \"Xin chào Vut\"\n  blob = text.to_bytes()\n  out(\"bytes: $(blob.len())\")\n  copied = blob\n  copied.set(0, 86)\n  out(decode(blob))\n  out(decode(copied))\n  values: list(u8) = copied.to_list()\n  raw_values: list(u8) = @(255)\n  raw = bytes.from_list(raw_values)\n  out(decode(raw))\n  out(\"$(values.len())\")\n",
        )
        .unwrap();
    let executable = root.join("program.exe");
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        "bytes: 13\nXin chào Vut\nVin chào Vut\ninvalid utf-8\n13\n".as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn async_await_program_compiles_and_executes() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-async-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("main.vut"),
        "async fn fetch_name() -> str:\n  \"Nam\"\n\
async fn read_age() -> int:\n  20\n\
async fn add_twice(value: int) -> int:\n  first = await read_age()\n  second = await read_age()\n  first + second + value\n\
async fn load_name() -> result(str, str):\n  name = await fetch_name()\n  ok(name)\n\
async fn load_code() -> result(int, str):\n  age = await read_age()\n  code = await add_twice(age)\n  ok(code)\n\
async fn main():\n  greeting = await fetch_name()\n  out(\"Hello $greeting\")\n  total = await add_twice(2)\n  out(\"total: $total\")\n  named = await load_name()\n  match named:\n    ok(text): out(\"name: $text\")\n    err(message): out(\"error: $message\")\n  coded = await load_code()\n  match coded:\n    ok(value): out(\"code: $value\")\n    err(message): out(\"error: $message\")\n",
    )
    .unwrap();
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        "Hello Nam\ntotal: 42\nname: Nam\ncode: 60\n".as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn async_method_program_compiles_and_executes() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-async-method-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("main.vut"),
        "data Counter:\n  value: int\n\
fn Counter.get() -> int:\n  self.value\n\
async fn Counter.next() -> int:\n  base = self.get()\n  base + 1\n\
async fn main():\n  counter = Counter(value = 41)\n  out(\"$(counter.get())\")\n  following = await counter.next()\n  out(\"$following\")\n",
    )
    .unwrap();
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, "41\n42\n".as_bytes());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_collection_of_managed_enums_balances_ownership() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-managed-collection-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("main.vut"),
        "data Item:\n  name: str\n  count: int\n\
enum Shape:\n  circle(radius: float)\n  label(text: str)\n\
fn main():\n  items: list(Item) = @(Item(name = \"a\", count = 1), Item(name = \"b\", count = 2))\n  for item in items:\n    out(\"$(item.name):$(item.count)\")\n  nested: list(list(str)) = @(@(\"x\", \"y\"), @(\"z\"))\n  for inner in nested:\n    for word in inner:\n      out(word)\n  table: map(str, Shape) = map((\"c\", Shape.circle(radius = 1.0)), (\"l\", Shape.label(text = \"hi\")))\n  out(\"map: $(table.len())\")\n  shapes: list(Shape) = @(Shape.label(text = \"one\"), Shape.circle(radius = 2.0))\n  for shape in shapes:\n    match shape:\n      circle(radius = r): out(\"circle $(r)\")\n      label(text = t): out(\"label $t\")\n",
    )
    .unwrap();
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        "a:1\nb:2\nx\ny\nz\nmap: 2\nlabel one\ncircle 2\n".as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn payload_enum_program_compiles_and_executes() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-enum-{nonce}"));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("main.vut"),
        "enum Shape:\n  point\n  circle(radius: float)\n  rect(width: float, height: float)\n\
enum Option:\n  none\n  some(value: str)\n\
fn area(shape: Shape) -> float:\n  match shape:\n    point: 0.0\n    circle(radius = r): 3.0 * r * r\n    rect(width = w, height = h): w * h\n\
fn classify(o: Option) -> str:\n  match o:\n    some(value = \"hi\"): \"exact\"\n    some(value = _): \"some\"\n    none: \"none\"\n\
fn describe(value: int) -> str:\n  match value:\n    0: \"zero\"\n    1 or 2: \"small\"\n    3..=9: \"medium\"\n    _ if value < 0: \"negative\"\n    _: \"large\"\n\
fn main():\n  out(\"$(area(Shape.circle(radius = 2.0)))\")\n  out(\"$(area(Shape.rect(width = 3.0, height = 4.0)))\")\n  out(\"$(area(Shape.point))\")\n  out(classify(Option.some(value = \"hi\")))\n  out(classify(Option.some(value = \"bye\")))\n  out(classify(Option.none))\n  out(describe(0))\n  out(describe(2))\n  out(describe(5))\n  out(describe(-1))\n  out(describe(100))\n",
    )
    .unwrap();
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib"),
        ),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .unwrap();
    let output = std::process::Command::new(&executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        "12\n12\n0\nexact\nsome\nnone\nzero\nsmall\nmedium\nnegative\nlarge\n".as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}
