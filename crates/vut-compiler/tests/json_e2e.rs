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
    let root = std::env::temp_dir().join(format!("vut-json-e2e-{nonce}-{sequence}"));
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

#[test]
fn parses_and_stringifies_scalars_by_literal() {
    let source = "import json\n\nfn show(source: str, label: str):\n  match json.parse(source):\n    ok(value): out(\"$label=$(json.stringify(value))\")\n    err(error): out(\"$label=err\")\n\nfn main():\n  show(\"null\", \"null\")\n  show(\"true\", \"bool\")\n  show(\"42\", \"int\")\n  show(\"1.5\", \"float\")\n  show(\"\\\"hi\\\"\", \"str\")\n  show(\"[1,2]\", \"arr\")\n  show(\"{}\", \"empty\")\n  show(\"{\\\"a\\\":1}\", \"obj\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "null=null\nbool=true\nint=42\nfloat=1.5\nstr=\"hi\"\narr=[1,2]\nempty={}\nobj={\"a\":1}\n"
    );
}

#[test]
fn parses_string_escapes_and_unicode() {
    let source = "import json\nimport json at Value\n\nfn text_of(value: Value) -> str:\n  match value.as_str():\n    ok(text): text\n    err(error): \"<error>\"\n\nfn main():\n  match json.parse(\"\\\"line\\\\nq\\\\\\\" \\\\u0041 \\\\ud83d\\\\ude00\\\"\"):\n    ok(value): out(text_of(value))\n    err(error): out(\"err\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "line\nq\" A \u{1f600}\n");
}

#[test]
fn rejects_invalid_documents_with_positioned_errors() {
    let source = "import json\n\nfn main():\n  match json.parse(\"{\\\"a\\\":}\"):\n    ok(value): out(\"ok\")\n    err(error): out(\"err at $(error.line):$(error.column)\")\n  match json.parse(\"9223372036854775808\"):\n    ok(value): out(\"ok big\")\n    err(error): out(\"overflow\")\n  match json.parse(\"[1,]\"):\n    ok(value): out(\"ok trail\")\n    err(error): out(\"trailing\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "err at 1:6\noverflow\ntrailing\n");
}

#[test]
fn accesses_nested_values_and_reports_types() {
    let source = "import json\nimport json at Value\n\nfn probe(value: Value):\n  name = value.is_str()\n  tags = value.is_array()\n  inner = value.is_object()\n  out(\"str=$name array=$tags object=$inner\")\n\nfn run(value: Value):\n  probe(value)\n  match value.get(\"tags\"):\n    ok(tags): match tags.as_array():\n      ok(items): out(\"tags=$(items.len())\")\n      err(error): out(\"tags not array\")\n    err(error): out(\"missing tags\")\n  has_nested = value.contains(\"nested\")\n  out(\"has_nested=$has_nested\")\n\nfn main():\n  source = \"{\\\"name\\\":\\\"Vut\\\",\\\"tags\\\":[\\\"a\\\",\\\"b\\\"],\\\"nested\\\":{\\\"x\\\":null}}\"\n  match json.parse(source):\n    ok(value): run(value)\n    err(error): out(\"parse error\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "str=0 array=0 object=1\ntags=2\nhas_nested=1\n");
}

#[test]
fn pretty_printing_uses_two_space_indentation() {
    let source = "import json\n\nfn main():\n  match json.parse(\"{\\\"a\\\":[1,{\\\"b\\\":true}]}\"):\n    ok(value): out(json.stringify_pretty(value))\n    err(error): out(\"err\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "{\n  \"a\": [\n    1,\n    {\n      \"b\": true\n    }\n  ]\n}\n"
    );
}

#[test]
fn typed_encode_via_encodable_interface() {
    let source = "import json\nimport json at Value, Encodable\n\ndata User:\n  name: str\n\nfn User.to_json() -> Value:\n  entries: map[str, Value] = ()\n  entries.set(\"name\", json.value_str(value = self.name))\n  json.value_object(value = entries)\n\nfn main():\n  user = User(name: \"Nam\")\n  out(json.encode(user))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "{\"name\":\"Nam\"}\n");
}

#[test]
fn round_trips_numbers_without_losing_integer_precision() {
    let source = "import json\nimport json at Value\n\nfn main():\n  match json.parse(\"9223372036854775807\"):\n    ok(value): match value.as_int():\n      ok(number): out(\"int=$number\")\n      err(error): out(\"not int\")\n    err(error): out(\"parse error\")\n  match json.parse(\"1.5\"):\n    ok(value): match value.as_float():\n      ok(number): out(\"float=$number\")\n      err(error): out(\"not float\")\n    err(error): out(\"parse error\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "int=9223372036854775807\nfloat=1.5\n");
}
