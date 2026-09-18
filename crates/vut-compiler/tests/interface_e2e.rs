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
    let root = std::env::temp_dir().join(format!("vut-interface-e2e-{nonce}-{sequence}"));
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
fn interface_boxing_without_dispatch() {
    let source = "interface Animal:\n  speak() -> str\n\ndata Dog:\n  name: str\n\nfn Dog.speak() -> str:\n  \"woof\"\n\nfn consume(animal: Animal) -> str:\n  \"ok\"\n\nfn main():\n  out(consume(Dog(name = \"Rex\")))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ok\n");
}

#[test]
fn minimal_interface_dispatch() {
    let source = "interface Marker:\n  ping() -> int\n\ndata Empty:\n  x: int\n\nfn Empty.ping() -> int:\n  7\n\nfn call(m: Marker) -> int:\n  m.ping()\n\nfn main():\n  out(\"$(call(Empty(x = 1)))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn interface_dispatch_selects_the_dynamic_concrete_type() {
    let source = "interface Animal:\n  speak() -> str\n  legs() -> int\n\ndata Dog:\n  name: str\n\ndata Bird:\n  name: str\n\nfn Dog.speak() -> str:\n  \"woof\"\n\nfn Dog.legs() -> int:\n  4\n\nfn Bird.speak() -> str:\n  \"tweet\"\n\nfn Bird.legs() -> int:\n  2\n\nfn describe(animal: Animal) -> str:\n  \"$(animal.speak()) has $(animal.legs()) legs\"\n\nfn main():\n  out(describe(Dog(name = \"Rex\")))\n  out(describe(Bird(name = \"Pip\")))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "woof has 4 legs\ntweet has 2 legs\n");
}

#[test]
fn interface_parameter_accepts_a_bound_local() {
    let source = "interface Named:\n  label() -> str\n\ndata Person:\n  name: str\n\nfn Person.label() -> str:\n  self.name\n\nfn greet(item: Named) -> str:\n  \"hello $(item.label())\"\n\nfn main():\n  p = Person(name = \"Ada\")\n  out(greet(p))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hello Ada\n");
}

#[test]
fn concrete_values_convert_at_explicit_returns() {
    let source = "interface Shape:\n  area() -> int\n\ndata Square:\n  side: int\n\ndata Rect:\n  width: int\n  height: int\n\nfn Square.area() -> int:\n  self.side * self.side\n\nfn Rect.area() -> int:\n  self.width * self.height\n\nfn make(wide: bool) -> Shape:\n  if wide:\n    return Rect(width = 3, height = 4)\n  return Square(side = 5)\n\nfn main():\n  out(\"$(make(true).area())\")\n  out(\"$(make(false).area())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "12\n25\n");
}

#[test]
fn interface_local_binding_boxes_and_dispatches() {
    let source = "interface Animal:\n  speak() -> str\n\ndata Dog:\n  name: str\n\nfn Dog.speak() -> str:\n  \"woof\"\n\nfn main():\n  animal: Animal = Dog(name = \"Rex\")\n  out(animal.speak())\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "woof\n");
}

#[test]
fn interface_data_field_and_list_are_leak_free() {
    let source = "interface Animal:\n  speak() -> str\n\ndata Dog:\n  name: str\n\ndata Cat:\n  name: str\n\nfn Dog.speak() -> str:\n  \"woof\"\n\nfn Cat.speak() -> str:\n  \"meow\"\n\ndata Shelter:\n  residents: list(Animal)\n\nfn main():\n  animals: list(Animal) = @(Dog(name = \"Rex\"), Cat(name = \"Milo\"))\n  for animal in animals:\n    out(animal.speak())\n  shelter = Shelter(residents = animals)\n  out(\"$(shelter.residents.len())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "woof\nmeow\n2\n");
}
