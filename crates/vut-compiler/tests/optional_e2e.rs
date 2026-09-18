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
    let root = std::env::temp_dir().join(format!("vut-optional-e2e-{nonce}-{sequence}"));
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
fn optional_parameter_if_narrowing_does_not_leak() {
    let source = "fn describe(value: str?) -> str:\n  if value != null:\n    return value\n  \"none\"\n\nfn main():\n  out(describe(\"hi\"))\n  out(describe(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\nnone\n");
}

#[test]
fn optional_else_branch_narrows_to_present_type() {
    let source = "fn describe(value: str?) -> str:\n  text: str = if value == null:\n    \"none\"\n  else:\n    value\n  text\n\nfn main():\n  out(describe(\"hi\"))\n  out(describe(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\nnone\n");
}

#[test]
fn optional_guard_clause_narrows_after_terminating_if() {
    let source = "fn first(values: list(str?)) -> str:\n  for value in values:\n    if value == null:\n      continue\n    return value\n  \"\"\n\nfn main():\n  values: list(str?) = @()\n  values.push(\"a\")\n  values.push(null)\n  values.push(\"b\")\n  out(first(values))\n  empty: list(str?) = @()\n  empty.push(null)\n  out(first(empty))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a\n\n");
}

#[test]
fn optional_match_accepts_null_pattern() {
    let source = "fn is_none(value: str?) -> bool:\n  match value:\n    null: true\n    _: false\n\nfn main():\n  out(is_none(\"x\"))\n  out(is_none(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "false\ntrue\n");
}

#[test]
fn optional_repeated_narrowed_reads_balance_ownership() {
    let source = "fn size(value: str?) -> int:\n  if value != null:\n    return value.byte_len() + value.byte_len()\n  0\n\nfn main():\n  out(size(\"hello\"))\n  out(size(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "10\n0\n");
}

#[test]
fn optional_nested_control_flow_balances_ownership() {
    let source = "fn pick(value: str?, flag: bool) -> str:\n  if flag:\n    if value != null:\n      return value\n    else:\n      return \"absent\"\n  \"skipped\"\n\nfn main():\n  out(pick(\"a\", true))\n  out(pick(null, true))\n  out(pick(\"a\", false))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a\nabsent\nskipped\n");
}

#[test]
fn optional_bytes_narrowing_balances_ownership() {
    let source = "fn size(value: bytes?) -> int:\n  if value != null:\n    return value.len()\n  0\n\nfn main():\n  raw: list(u8) = @(1, 2, 3)\n  out(size(bytes.from_list(raw)))\n  out(size(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n0\n");
}

#[test]
fn scalar_optional_distinguishes_zero_value_from_absent() {
    let source = "fn main():\n  a: int? = 5\n  b: int? = 0\n  c: int? = null\n  if a != null:\n    out(\"a=$(a)\")\n  if b != null:\n    out(\"b=$(b)\")\n  if c == null:\n    out(\"c=absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=5\nb=0\nc=absent\n");
}

#[test]
fn scalar_optional_float_and_bool() {
    let source = "fn main():\n  f: float? = 1.5\n  g: bool? = true\n  h: bool? = null\n  if f != null:\n    out(f)\n  if g != null:\n    out(g)\n  if h == null:\n    out(\"absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1.5\ntrue\nabsent\n");
}

#[test]
fn scalar_optional_argument_and_narrowing() {
    let source = "fn plus(value: int?) -> int:\n  if value != null:\n    return value + 1\n  -1\n\nfn main():\n  x: int? = 4\n  out(plus(x))\n  out(plus(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\n-1\n");
}

#[test]
fn scalar_optional_return_across_functions() {
    let source = "fn make() -> int?:\n  5\n\nfn absent() -> int?:\n  null\n\nfn half(flag: bool) -> int?:\n  if flag:\n    return 9\n  null\n\nfn chain() -> int?:\n  make()\n\nfn main():\n  a: int? = make()\n  if a != null:\n    out(a)\n  b: int? = absent()\n  if b == null:\n    out(\"b=absent\")\n  c: int? = half(true)\n  if c != null:\n    out(c)\n  d: int? = half(false)\n  if d == null:\n    out(\"d=absent\")\n  e: int? = chain()\n  if e != null:\n    out(e)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\nb=absent\n9\nd=absent\n5\n");
}

#[test]
fn scalar_optional_float_return() {
    let source = "fn ratio(flag: bool) -> float?:\n  if flag:\n    return 0.5\n  null\n\nfn main():\n  a: float? = ratio(true)\n  if a != null:\n    out(a)\n  b: float? = ratio(false)\n  if b == null:\n    out(\"b=absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "0.5\nb=absent\n");
}

#[test]
fn map_get_and_remove_report_absence_as_null() {
    let source = "fn main():\n  scores: map(str, int) = map((\"ann\", 30))\n  a: int? = scores.get(\"ann\")\n  if a != null:\n    out(\"ann=$(a)\")\n  missing: int? = scores.get(\"bob\")\n  if missing == null:\n    out(\"bob=absent\")\n  removed: int? = scores.remove(\"ann\")\n  if removed != null:\n    out(\"removed=$(removed)\")\n  gone: int? = scores.remove(\"ann\")\n  if gone == null:\n    out(\"gone=absent\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ann=30\nbob=absent\nremoved=30\ngone=absent\n");
}

#[test]
fn map_get_and_remove_managed_values_balance_ownership() {
    let source = "fn main():\n  names: map(str, str) = map((\"a\", \"alpha\"))\n  hit: str? = names.get(\"a\")\n  if hit != null:\n    out(hit)\n  miss: str? = names.get(\"z\")\n  if miss == null:\n    out(\"missing\")\n  taken: str? = names.remove(\"a\")\n  if taken != null:\n    out(taken)\n  empty: str? = names.remove(\"a\")\n  if empty == null:\n    out(\"empty\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "alpha\nmissing\nalpha\nempty\n");
}

#[test]
fn scalar_optional_match_null_pattern() {
    let source = "fn describe(value: int?) -> str:\n  match value:\n    null: \"none\"\n    v: \"got $(v)\"\n\nfn main():\n  out(describe(7))\n  out(describe(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "got 7\nnone\n");
}

#[test]
fn managed_optional_match_binding_narrows() {
    let source = "fn describe(value: str?) -> str:\n  match value:\n    null: \"none\"\n    v: \"hi $(v)\"\n\nfn main():\n  out(describe(\"ann\"))\n  out(describe(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi ann\nnone\n");
}

#[test]
fn optional_data_match_binding_is_leak_free() {
    let source = "data Person:\n  name: str\n  age: int\n\nfn label(person: Person?) -> str:\n  match person:\n    null: \"none\"\n    v: \"$(v.name) $(v.age)\"\n\nfn main():\n  out(label(Person(name = \"Ann\", age = 30)))\n  out(label(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "Ann 30\nnone\n");
}

#[test]
fn optional_enum_variant_match_narrows() {
    let source = "enum Payload:\n  none\n  text(value: str)\n  count(value: int)\n\nfn label(payload: Payload?) -> str:\n  match payload:\n    null: \"absent\"\n    none: \"none\"\n    text(value): \"text $(value)\"\n    count(value): \"count $(value)\"\n\nfn main():\n  out(label(Payload.text(value = \"hi\")))\n  out(label(Payload.count(value = 4)))\n  out(label(Payload.none))\n  out(label(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "text hi\ncount 4\nnone\nabsent\n");
}

#[test]
fn optional_data_with_managed_fields_is_leak_free() {
    let source = "data Person:\n  name: str\n  age: int\n\nfn find(flag: bool) -> Person?:\n  if flag:\n    return Person(name = \"Ann\", age = 30)\n  null\n\nfn main():\n  a: Person? = find(true)\n  if a != null:\n    out(\"$(a.name) $(a.age)\")\n  b: Person? = find(false)\n  if b == null:\n    out(\"none\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "Ann 30\nnone\n");
}

#[test]
fn optional_list_handle_narrowing_is_leak_free() {
    let source = "fn total(values: list(int)?) -> int:\n  if values == null:\n    return 0\n  values.len()\n\nfn main():\n  items: list(int) = @(1, 2, 3)\n  out(total(items))\n  out(total(null))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n0\n");
}
