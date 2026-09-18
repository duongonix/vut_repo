//! `io` module end-to-end tests: real compilation, native execution, and leak
//! detection (a clean exit code means no live managed allocation).
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

use stdlib_common::{run, run_with_stdin};

#[test]
fn stdout_write_and_flush() {
    let source = "import io\nfn main():\n  match io.stdout().write(\"hello\".to_bytes()):\n    ok(n): out(\"wrote=$(n)\")\n    err(error): out(\"write-failed\")\n  flushed = io.stdout().flush()\n  out(\"done\")\n";
    let (code, stdout, stderr) = run("io_stdout", source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "hellowrote=5\ndone\n");
}

#[test]
fn stderr_write_is_routed_to_stderr() {
    let source = "import io\nfn main():\n  match io.stderr().write(\"oops\".to_bytes()):\n    ok(n): out(\"n=$(n)\")\n    err(error): out(\"err\")\n";
    let (code, stdout, stderr) = run("io_stderr", source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "n=4\n");
    assert_eq!(stderr, "oops");
}

#[test]
fn stdin_read_to_str_reads_to_end() {
    let source = "import io\nfn main():\n  value: result(str, io.IoError) = io.read_to_str(io.stdin())\n  match value:\n    ok(text): out(text)\n    err(error): out(\"read-failed\")\n";
    let (code, stdout, stderr) = run_with_stdin("io_stdin_all", source, "hello\nworld");
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "hello\nworld\n");
}

#[test]
fn buffered_reader_reads_lines() {
    let source = "import io\nfn main():\n  reader = io.buffered(io.stdin())\n  lines: list(str) = @()\n  count = 0\n  for count < 3:\n    text = \"err\"\n    matched = reader.read_line()\n    match matched:\n      ok(value):\n        if value != null:\n          text = value\n        else:\n          text = \"eof\"\n      err(error):\n        text = \"err\"\n    lines.push(text)\n    count = count + 1\n  for text in lines:\n    out(text)\n";
    let (code, stdout, stderr) = run_with_stdin("io_lines", source, "first\nsecond\n");
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "first\nsecond\neof\n");
}

#[test]
fn copy_streams_stdin_to_stdout() {
    let source = "import io\nfn main():\n  value: result(int, io.IoError) = io.copy(io.stdin(), io.stdout())\n  match value:\n    ok(copied): out(\"copied=$(copied)\")\n    err(error): out(\"copy-failed\")\n";
    let (code, stdout, stderr) = run_with_stdin("io_copy", source, "abcdef");
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "abcdefcopied=6\n");
}

#[test]
fn buffered_writer_accumulates_until_flush() {
    let source = "import io\nfn main():\n  writer = io.buffered_writer(io.stdout())\n  written = writer.write_all(\"buffered\".to_bytes())\n  flushed = writer.flush()\n  out(\"done\")\n";
    let (code, stdout, stderr) = run("io_buffered_writer", source);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "buffereddone\n");
}

#[test]
fn read_exact_reports_unexpected_eof() {
    let source = "import io\nfn main():\n  reader = io.buffered(io.stdin())\n  value: result(bytes, io.IoError) = reader.read_exact(5)\n  match value:\n    ok(blob): out(\"got\")\n    err(error): out(\"eof-error\")\n";
    let (code, stdout, stderr) = run_with_stdin("io_read_exact", source, "abc");
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "eof-error\n");
}

#[test]
fn read_exact_returns_exact_bytes_then_leaves_tail() {
    // Reads 2 bytes then expects the buffered reader to carry the tail forward.
    let source = "import io\nfn main():\n  reader = io.buffered(io.stdin())\n  value: result(bytes, io.IoError) = reader.read_exact(2)\n  head: bytes = match value:\n    ok(blob): blob\n    err(error): bytes()\n  match head.to_str():\n    ok(text): out(text)\n    err(error): out(\"head-failed\")\n  tail: result(str, io.IoError) = io.read_to_str(reader)\n  match tail:\n    ok(text): out(text)\n    err(error): out(\"tail-failed\")\n";
    let (code, stdout, stderr) = run_with_stdin("io_read_exact_tail", source, "abcdef");
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "ab\ncdef\n");
}
