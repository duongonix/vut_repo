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
    let root = std::env::temp_dir().join(format!("vut-ownership-e2e-{nonce}-{sequence}"));
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
fn enum_method_returning_managed_payload_is_balanced() {
    let source = "enum E:\n  nothing\n  text(value: str)\n\nfn E.get_text() -> str:\n  match self:\n    text(inner): inner\n    nothing: \"nothing\"\n\nfn main():\n  e: E = E.text(value = \"hi\")\n  out(\"method=$(e.get_text())\")\n  match e:\n    text(inner): out(\"direct=$inner\")\n    nothing: out(\"nothing\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "method=hi\ndirect=hi\n");
}

#[test]
fn enum_method_returning_borrowed_payload_repeatedly_is_balanced() {
    let source = "enum E:\n  nothing\n  text(value: str)\n\nfn E.get_text() -> str:\n  match self:\n    text(inner): inner\n    nothing: \"nothing\"\n\nfn main():\n  e: E = E.text(value = \"hi\")\n  index = 0\n  for:\n    if index >= 3:\n      break\n    out(e.get_text())\n    index = index + 1\n  out(e.get_text())\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\nhi\nhi\nhi\n");
}

#[test]
fn data_method_returning_runtime_managed_field_is_balanced() {
    let source = "data Tag:\n  text: str\n\nfn Tag.label() -> str:\n  self.text\n\nfn make(part: str) -> Tag:\n  Tag(text = part + \"!\")\n\nfn main():\n  tag = make(\"x\")\n  out(\"label=$(tag.label())\")\n  out(\"again=$(tag.label())\")\n  out(\"direct=$(tag.text)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "label=x!\nagain=x!\ndirect=x!\n");
}

#[test]
fn data_method_that_does_not_read_self_keeps_the_receiver_alive() {
    let source = "data Tag:\n  text: str\n\nfn Tag.size() -> int:\n  3\n\nfn make(part: str) -> Tag:\n  Tag(text = part + \"!\")\n\nfn main():\n  tag = make(\"x\")\n  out(\"size=$(tag.size())\")\n  out(\"text=$(tag.text)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "size=3\ntext=x!\n");
}

#[test]
fn data_default_fields_are_materialized() {
    let source = "data Counter:\n  value: int = 0\ndata Point:\n  x: int\n  y: int = 5\ndata User:\n  name: str = \"anon\"\n\nfn main():\n  counter = Counter()\n  out(\"counter=$(counter.value)\")\n  point = Point(x = 1)\n  out(\"point=$(point.x),$(point.y)\")\n  user = User()\n  out(\"user=$(user.name)\")\n  named = User(name = \"nam\")\n  out(\"named=$(named.name)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "counter=0\npoint=1,5\nuser=anon\nnamed=nam\n");
}

#[test]
fn data_default_expression_is_lowered_at_construction() {
    let source = "fn fallback() -> str:\n  \"x\" + \"y\"\n\ndata Holder:\n  text: str = fallback()\n\nfn main():\n  holder = Holder()\n  out(\"text=$(holder.text)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "text=xy\n");
}

#[test]
fn mutating_method_on_projected_field_receiver() {
    let source = "data Bag:\n  items: list(str)\n\nfn Bag.replace(value: str):\n  kept: list(str) = @()\n  kept.push(value)\n  self.items = kept\n\nfn Bag.add(value: str):\n  self.items.push(value)\n\nfn Bag.count() -> int:\n  self.items.len()\n\ndata Holder:\n  bag: Bag\n\nfn Holder.via_replace(value: str) -> int:\n  self.bag.replace(value)\n  self.bag.count()\n\nfn Holder.via_add(value: str) -> int:\n  self.bag.add(value)\n  self.bag.count()\n\nfn main():\n  h = Holder(bag = Bag(items = @(\"x\")))\n  r = h.via_replace(\"y\")\n  a = h.via_add(\"z\")\n  total = h.bag.items.len()\n  out(\"r=$r a=$a total=$total\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "r=1 a=2 total=2\n");
}

#[test]
fn data_field_assignment_mutates_in_place() {
    let source = "data Box:\n  text: str\n  count: usize\n\nfn Box.fill(value: str):\n  self.text = value\n\nfn Box.bump():\n  self.count = self.count + 1\n\nfn main():\n  b = Box(text = \"a\", count = 0)\n  b.fill(\"b\")\n  b.bump()\n  t = b.text\n  n = b.count\n  out(\"t=$t c=$n\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "t=b c=1\n");
}

#[test]
fn iterating_an_empty_string_list_is_safe() {
    let source = "fn main():\n  items: list(str) = @()\n  for item in items:\n    out(\"item=$item\")\n  out(\"ok\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ok\n");
}

#[test]
fn map_keys_returns_live_string_handles() {
    let source = "fn main():\n  m: map(str, int) = map()\n  m.set(\"a\", 1)\n  m.set(\"b\", 2)\n  keys = m.keys()\n  out(\"len=$(keys.len())\")\n  total = 0\n  for key in keys:\n    total = total + key.byte_len()\n  out(\"bytes=$total\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "len=2\nbytes=2\n");
}
