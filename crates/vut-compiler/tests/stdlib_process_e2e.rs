#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

fn driver() -> (stdlib_common::Scratch, String) {
    let scratch = stdlib_common::scratch("process_driver");
    let source = scratch.root.join("driver.rs");
    std::fs::write(&source, include_str!("fixtures/process_driver.rs")).unwrap();
    let executable = scratch.root.join(if cfg!(windows) {
        "driver.exe"
    } else {
        "driver"
    });
    let status = std::process::Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .status()
        .unwrap();
    assert!(status.success());
    (scratch, executable.to_string_lossy().replace('\\', "/"))
}

#[test]
fn arguments_working_directory_and_nonzero_status() {
    let (scratch, program) = driver();
    std::fs::write(scratch.root.join("marker"), b"marker").unwrap();
    let directory = scratch.root.to_string_lossy().replace('\\', "/");
    let source = format!(
        r#"import process
fn main():
  command = process.Command(program: "{program}")
  command.args(@["config", "argument with spaces"])
  command.current_dir("{directory}")
  match command.output():
    ok(output):
      match output.stdout.to_str():
        ok(text): print(text)
        err(error): out("encoding failure")
    err(error): out(error.message)
  failure = process.Command(program: "{program}")
  failure.arg("fail")
  match failure.status():
    ok(status):
      out(status.success())
      code = status.code()
      if code != null:
        out(code)
    err(error): out(error.message)
"#
    );
    let (code, stdout, stderr) = stdlib_common::run("process_config", &source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "argument with spaces\ntrue\nfalse\n7\n");
}

#[test]
fn dropped_child_survives_early_return() {
    let (scratch, program) = driver();
    let marker = scratch
        .root
        .join("alive")
        .to_string_lossy()
        .replace('\\', "/");
    let source = format!(
        "import process\n\nfn launch() -> result[unit, process.ProcessError]:\n  command = process.Command(program: \"{program}\")\n  command.arg(\"marker\")\n  command.arg(\"{marker}\")\n  child = command.spawn()?\n  return ok(unit)\n\nfn main():\n  match launch():\n    ok(done): out(\"released\")\n    err(error): out(error.message)\n"
    );
    let (code, stdout, stderr) = stdlib_common::run("process_drop", &source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "released\n");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !std::path::Path::new(&marker).exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(std::fs::read(marker).unwrap(), b"alive");
}

#[test]
fn spawn_wait_and_explicit_kill_wait() {
    let (_scratch, program) = driver();
    let source = format!(
        "import process\n\nfn exercise() -> result[unit, process.ProcessError]:\n  command = process.Command(program: \"{program}\")\n  command.arg(\"sleep\")\n  child = command.spawn()?\n  pending = child.try_wait()?\n  out(pending == null)\n  child.kill()?\n  status = child.wait()?\n  out(status.success())\n  second = process.Command(program: \"{program}\")\n  second.arg(\"success\")\n  status2 = second.status()?\n  out(status2.success())\n  ok(unit)\n\nfn main():\n  match exercise():\n    ok(done): out(\"done\")\n    err(error): out(error.message)\n"
    );
    let (code, stdout, stderr) = stdlib_common::run("process_wait", &source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "true\nfalse\ntrue\ndone\n");
}

#[test]
fn imported_status_receiver_has_concrete_layout() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/process.vut");
    let checked = vut_compiler::CompilerSession::new(vut_compiler::CompilerConfig::default())
        .check_source_file(&path)
        .unwrap();
    assert!(!checked.resolution.diagnostics.has_errors());
    assert!(!checked.semantics.diagnostics.has_errors());
    let mut receivers = 0;
    for function in &checked.mir.functions {
        if checked.resolution.symbols[function.symbol.0].name == "success" {
            receivers += 1;
            let ty = function.locals[0].ty.unwrap();
            assert!(
                checked.mir.layouts.field(ty, "_success").is_some(),
                "{}: {:?}, fields {:?}",
                function.symbol.0,
                checked.semantics.types[ty.0],
                checked.mir.layouts.fields.get(&ty)
            );
        }
    }
    assert_eq!(receivers, 1);
}

#[test]
fn failed_spawn_has_typed_error_and_no_resource_leak() {
    let source = "import process\n\nfn main():\n  command = process.Command(program: \"vut-no-such-program-m13\")\n  match command.spawn():\n    ok(child): out(\"unexpected\")\n    err(error):\n      match error.kind:\n        not_found: out(\"not_found\")\n        _: out(\"wrong_error\")\n";
    let (code, stdout, stderr) = stdlib_common::run("process_missing", source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "not_found\n");
}

#[test]
fn output_is_binary_and_moved_stdout_survives_child_drop() {
    let (_scratch, program) = driver();
    let source = format!(
        "import process\nimport io\n\nfn detached() -> result[process.ChildStdout, io.IoError]:\n  command = process.Command(program: \"{program}\")\n  command.arg(\"output\")\n  command.stdout(process.Stdio.piped)\n  command.stderr(process.Stdio.null_device)\n  match command.spawn():\n    ok(child): child.stdout()\n    err(error): err(io.IoError(kind: io.IoErrorKind.other, message: error.message))\n\nfn read_detached() -> result[bytes, io.IoError]:\n  pipe = detached()?\n  io.read_all(pipe)\n\nfn main():\n  command = process.Command(program: \"{program}\")\n  command.arg(\"output\")\n  match command.output():\n    ok(output):\n      out(output.stdout.len())\n      out(output.stdout.at(5))\n      out(output.stderr.len())\n    err(error): out(error.message)\n  match read_detached():\n    ok(blob): out(blob.len())\n    err(error): out(error.message)\n  out(\"done\")\n"
    );
    let (code, stdout, stderr) = stdlib_common::run("process_pipes", &source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "11\n0\n5\n11\ndone\n");
}

#[test]
fn moved_pipe_endpoints_close_once_and_stdin_drop_signals_eof() {
    let (_scratch, program) = driver();
    let source = format!(
        r#"import process
import io

fn feed(writer: process.ChildStdin) -> result[unit, io.IoError]:
  count = writer.write("payload".to_bytes())?
  writer.flush()?
  ok(unit)

fn pipes(child: process.Child) -> result[unit, io.IoError]:
  reader = child.stdout()?
  errors = child.stderr()?
  writer = child.stdin()?
  feed(writer)?
  blob = io.read_all(reader)?
  error_blob = io.read_all(errors)?
  out(blob.len())
  out(error_blob.len())
  match child.wait():
    ok(status): out(status.success())
    err(error): out(error.message)
  ok(unit)

fn exercise() -> result[unit, process.ProcessError]:
  command = process.Command(program: "{program}")
  command.arg("echo")
  command.stdin(process.Stdio.piped)
  command.stdout(process.Stdio.piped)
  command.stderr(process.Stdio.piped)
  child = command.spawn()?
  result = pipes(child)
  match result:
    ok(done): out("done")
    err(error): out(error.message)
  ok(unit)

fn main():
  match exercise():
    ok(done): out("complete")
    err(error): out(error.message)
"#
    );
    let (code, stdout, stderr) = stdlib_common::run("process_endpoints", &source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "7\n6\ntrue\ndone\ncomplete\n");
}

#[test]
fn environment_configuration_uses_last_operation() {
    let (_scratch, program) = driver();
    let source = format!(
        r#"import process

fn show(command: process.Command):
  match command.output():
    ok(output):
      match output.stdout.to_str():
        ok(text): print(text)
        err(error): out("invalid utf8")
    err(error): out(error.message)

fn main():
  removed = process.Command(program: "{program}")
  removed.arg("env")
  removed.env("VUT_M13_CHILD_TEST", "before")
  removed.env_remove("VUT_M13_CHILD_TEST")
  show(removed)
  set = process.Command(program: "{program}")
  set.arg("env")
  set.env_remove("VUT_M13_CHILD_TEST")
  set.env("VUT_M13_CHILD_TEST", "after")
  show(set)
"#
    );
    let (code, stdout, stderr) = stdlib_common::run("process_env_order", &source);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(stdout, "absent\nafter\n");
}

#[test]
fn repeated_child_resource_cleanup_has_no_leak_or_double_close() {
    let (_driver, program) = driver();
    let source = format!(
        r#"import process

fn exercise() -> result[unit, process.ProcessError]:
  command = process.Command(program: "{program}")
  command.arg("success")
  child = command.spawn()?
  status = child.wait()?
  out(status.success())
  ok(unit)

fn main():
  iteration = 0
  for iteration < 64:
    match exercise():
      ok(done): out("done")
      err(error): out(error.message)
    iteration = iteration + 1
"#
    );
    let (_scratch, executable) = stdlib_common::build("process_repeated", &source);
    for _ in 0..16 {
        let (code, stdout, stderr) = stdlib_common::run_executable(&executable, &[], None);
        assert_eq!(code, Some(0), "{stderr}");
        assert_eq!(stdout, "true\ndone\n".repeat(64));
    }
}
