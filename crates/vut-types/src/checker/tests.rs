//! Type checker tests.
mod numeric_target;

use super::*;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_resolver::{ModuleInput, ModulePath, Resolver};
use vut_source::SourceId;

fn analyze(source: &str) -> SemanticResult {
    let source_id = SourceId::from_index(0);
    let (tokens, lexical) = Lexer::new(source_id, source).lex();
    assert!(!lexical.has_errors(), "{:?}", lexical.as_slice());
    let (file, syntax) = Parser::new(source_id, source, tokens).parse();
    assert!(!syntax.has_errors(), "{:?}", syntax.as_slice());
    let resolution = Resolver::new(vec![ModuleInput {
        logical_path: ModulePath(vec!["main".into()]),
        filesystem_path: None,
        file,
    }])
    .resolve();
    assert!(
        !resolution.diagnostics.has_errors(),
        "{:?}",
        resolution.diagnostics.as_slice()
    );
    Analyzer::new(&resolution).analyze()
}
fn codes(result: &SemanticResult) -> Vec<&str> {
    result
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|d| d.code.as_deref())
        .collect()
}

/// Returns all diagnostic codes from both the resolver and the checker. Used by
/// tests that expect a resolution-stage error (an unknown interface parent).
fn all_codes(source: &str) -> Vec<String> {
    let source_id = SourceId::from_index(0);
    let (tokens, lexical) = Lexer::new(source_id, source).lex();
    assert!(!lexical.has_errors(), "{:?}", lexical.as_slice());
    let (file, syntax) = Parser::new(source_id, source, tokens).parse();
    assert!(!syntax.has_errors(), "{:?}", syntax.as_slice());
    let resolution = Resolver::new(vec![ModuleInput {
        logical_path: ModulePath(vec!["main".into()]),
        filesystem_path: None,
        file,
    }])
    .resolve();
    let semantic = Analyzer::new(&resolution).analyze();
    let mut codes: Vec<String> = resolution
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|d| d.code.as_deref().map(str::to_owned))
        .collect();
    codes.extend(
        semantic
            .diagnostics
            .as_slice()
            .iter()
            .filter_map(|d| d.code.as_deref().map(str::to_owned)),
    );
    codes
}

#[test]
fn type_checks_lambda_callbacks_and_rejects_invalid_calls() {
    let valid = analyze(
        "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  out(\"$(apply(3, x => x * 2))\")\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
    assert!(
        valid
            .types
            .iter()
            .any(|ty| matches!(ty, Type::Callable { .. }))
    );

    let non_callable = analyze("fn main():\n  value = 1\n  value(2)\n");
    assert!(
        codes(&non_callable).contains(&"E1014"),
        "{:?}",
        codes(&non_callable)
    );

    let infer = analyze("fn main():\n  unknown = x => x + 1\n");
    assert!(codes(&infer).contains(&"E1012"), "{:?}", codes(&infer));
}

#[test]
fn receiver_function_types_keep_receiver_distinct_from_parameters() {
    let result = analyze(
        "data Scope:\n  label: str\n\
         type Receiver = fn(Scope)() -> void\n\
         type Plain = fn() -> void\n\
         fn main():\n  out(\"x\")\n",
    );
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
    let receivers: Vec<bool> = result
        .types
        .iter()
        .filter_map(|ty| match ty {
            Type::Callable { receiver, .. } => Some(receiver.is_some()),
            _ => None,
        })
        .collect();
    assert!(
        receivers.contains(&true),
        "receiver callable missing: {receivers:?}"
    );
    assert!(
        receivers.contains(&false),
        "plain callable missing: {receivers:?}"
    );
}

#[test]
fn trailing_body_uses_the_callee_receiver_function_parameter() {
    let trailing = analyze(
        "data Scope:\n  label: str\n\
         fn render(body: fn(Scope)() -> void) -> void:\n  out(\"render\")\n\
         fn main():\n  render():\n    out(\"hi\")\n",
    );
    assert!(
        !trailing.diagnostics.has_errors(),
        "{:?}",
        trailing.diagnostics.as_slice()
    );
    assert!(
        trailing
            .function_signatures
            .values()
            .any(|signature| signature.receiver.is_some()),
        "the trailing lambda signature must carry the inferred receiver"
    );

    let mismatch = analyze(
        "fn consume(callback: fn() -> void) -> void:\n  out(\"x\")\n\
         fn main():\n  consume():\n    out(\"y\")\n",
    );
    assert!(
        codes(&mismatch).contains(&"E1020"),
        "{:?}",
        codes(&mismatch)
    );
}

#[test]
fn receiver_and_plain_callables_are_not_interchangeable() {
    let mismatched = analyze(
        "data Scope:\n  label: str\n\
         fn use_plain(callback: fn() -> void) -> void:\n  out(\"x\")\n\
         fn main():\n  receiver_fn: fn(Scope)() -> void = fn():\n    out(\"a\")\n  use_plain(receiver_fn)\n",
    );
    assert!(
        mismatched.diagnostics.has_errors(),
        "a receiver function must not satisfy a plain function type: {:?}",
        mismatched.diagnostics.as_slice()
    );
}

#[test]
fn checks_result_constructors_match_and_question_operator() {
    let valid = analyze(
        "fn load() -> result[int, str]:\n  ok(1)\nfn main() -> result[int, str]:\n  value = load()?\n  checked: result[int, str] = ok(value)\n  match checked:\n    ok(number): ok(number)\n    err(message): err(message)\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
    assert!(
        valid
            .types
            .iter()
            .any(|ty| matches!(ty, Type::Result(_, _)))
    );
}

#[test]
fn rejects_uncontextual_result_constructors_bad_question_and_incomplete_match() {
    let uncontextual = analyze("fn main():\n  value = ok(1)\n");
    assert!(codes(&uncontextual).contains(&"E1004"));
    let bad_question =
        analyze("fn load() -> result[int, str]:\n  ok(1)\nfn main() -> int:\n  load()?\n");
    assert!(codes(&bad_question).contains(&"E6005"));
    let incomplete = analyze(
        "fn main() -> int:\n  value: result[int, str] = ok(1)\n  match value:\n    ok(number): number\n",
    );
    assert!(codes(&incomplete).contains(&"E5009"));
}

#[test]
fn checks_usize_isize_and_bool_across_ffi() {
    let valid = analyze("extern \"C\" fn a(x: usize, y: isize) -> usize\n");
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );

    // `bool` maps to the target C ABI's C `_Bool` and is FFI-safe in v1.
    let booly = analyze("extern \"C\" fn a(x: bool) -> bool\n");
    assert!(!codes(&booly).contains(&"E8004"), "{:?}", codes(&booly));

    let abi = analyze("extern \"Rust\" fn a()\n");
    assert!(codes(&abi).contains(&"E8005"), "{:?}", codes(&abi));
}

#[test]
fn rejects_result_in_extern_signatures_for_ffi_v1() {
    let result = analyze("extern \"C\" fn native(value: result[i32, i32]) -> result[i32, i32]\n");
    let codes = codes(&result);
    assert!(codes.contains(&"E8004"), "{codes:?}");
}

#[test]
fn checks_core_bytes_type_methods_and_conversions() {
    let valid = analyze(
        "data Packet:\n  payload: bytes\nfn take(blob: bytes) -> bytes:\n  blob\nfn inspect(blob: bytes) -> int:\n  match blob.to_str():\n    ok(value): 0\n    err(error): error.valid_up_to + error.error_len\nfn main():\n  raw = bytes()\n  raw.reserve(8)\n  text = \"Xin chào\".to_bytes()\n  copied = text\n  copied.set(0, 86)\n  values: list[u8] = copied.to_list()\n  rebuilt = bytes.from_list(values)\n  packets: list[bytes] = @[rebuilt]\n  table: map[str, bytes] = (\"payload\": text)\n  maybe: bytes? = text\n  result_value: result[bytes, str] = ok(text)\n  entry: bytes? = table.get(\"payload\")\n  if entry != null:\n    entry.len()\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );

    let invalid = analyze("fn main():\n  blob = bytes()\n  blob.set(0, \"A\")\n");
    assert!(codes(&invalid).contains(&"E1003"));

    let ffi = analyze(
        "extern \"C\" fn bad(blob: map[str, i32])\nextern \"C\" fn good(blob: ptr[u8], len: u64)\n",
    );
    assert!(
        codes(&ffi).contains(&"E8004"),
        "{:?}",
        ffi.diagnostics.as_slice()
    );
}

#[test]
fn checks_functions_methods_data_and_lists() {
    let result = analyze(
        "data Counter:\n  value: int = 0\nfn Counter.add(amount: int) -> int:\n  self.value = self.value + amount\n  self.value\nfn main() -> int:\n  counter = Counter(value: 1)\n  values: list[int] = @[1, 2, 3]\n  counter.add(values.len)\n",
    );
    assert!(result.diagnostics.has_errors());
    assert!(codes(&result).contains(&"E2004"));
}

#[test]
fn omitted_return_type_defaults_to_void_and_explicit_void_is_supported() {
    let valid = analyze(
        "fn implicit():\n  out(\"ok\")\nfn explicit() -> void:\n  implicit()\nfn main():\n  explicit()\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
    let invalid = analyze("fn accidentally_returns_value():\n  42\n");
    assert!(codes(&invalid).contains(&"E6005"));
}

#[test]
fn structurally_satisfies_interfaces_and_caches_result() {
    let result = analyze(
        "interface Speaker:\n  speak() -> str\ndata Dog:\n  name: str\nfn Dog.speak() -> str:\n  \"woof\"\nfn hear(value: Speaker) -> str:\n  value.speak()\nfn main() -> str:\n  dog = Dog(name: \"Milo\")\n  hear(dog)\n",
    );
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
    assert_eq!(result.interface_shapes.len(), 1);
    assert!(
        result
            .interface_satisfaction
            .values()
            .any(|value| *value == Satisfaction::Satisfied)
    );
}

#[test]
fn reports_missing_mismatched_and_private_interface_methods() {
    let missing = analyze(
        "interface Speaker:\n  speak() -> str\ndata Dog:\n  name: str\nfn hear(value: Speaker):\n  value.speak()\nfn main():\n  hear(Dog(name: \"Milo\"))\n",
    );
    assert!(codes(&missing).contains(&"E4102"));
    let mismatch = analyze(
        "interface Speaker:\n  speak() -> str\ndata Dog:\n  name: str\nfn Dog.speak(value: int) -> str:\n  \"woof\"\nfn hear(value: Speaker):\n  value.speak()\nfn main():\n  hear(Dog(name: \"Milo\"))\n",
    );
    assert!(codes(&mismatch).contains(&"E4103"));
    let private = analyze(
        "interface Speaker:\n  _speak() -> str\ndata Dog:\n  name: str\nfn Dog._speak() -> str:\n  \"woof\"\nfn hear(value: Speaker):\n  value._speak()\nfn main():\n  hear(Dog(name: \"Milo\"))\n",
    );
    assert!(codes(&private).contains(&"E4104"));
}

#[test]
fn detects_interface_composition_conflicts_and_cycles() {
    let conflict = analyze(
        "interface TextReader:\n  read() -> str\ninterface ByteReader:\n  read() -> bytes\ninterface Reader: TextReader, ByteReader:\n  close()\n",
    );
    assert!(codes(&conflict).contains(&"E4010"));
    let cycle = analyze("interface A: B:\n  a()\ninterface B: A:\n  b()\n");
    assert!(codes(&cycle).contains(&"E4011"));
}

#[test]
fn rejects_fixed_type_heterogeneous_list_and_truthiness() {
    let result = analyze(
        "fn invalid():\n  value = 1\n  value = \"one\"\n  items = @[1, \"two\"]\n  if 1:\n    value\n",
    );
    let actual = codes(&result);
    assert!(actual.contains(&"E1003"), "{actual:?}");
    assert!(actual.contains(&"E1005"), "{actual:?}");
    assert!(actual.contains(&"E5007"), "{actual:?}");
}

#[test]
fn checks_calls_constructors_returns_and_loop_control() {
    let result = analyze(
        "data User:\n  name: str\n  age: int = 0\nfn greet(name: str) -> str:\n  return 1\nfn invalid():\n  user = User(age: \"old\", extra: 1)\n  greet()\n  break\n  continue\n",
    );
    let actual = codes(&result);
    for expected in ["E6005", "E2004", "E1003", "E6002", "E5005", "E5006"] {
        assert!(actual.contains(&expected), "missing {expected}: {actual:?}");
    }
}

#[test]
fn checks_ranges_iterables_and_enum_match_exhaustiveness() {
    let result = analyze(
        "enum Status:\n  pending\n  done\nfn test():\n  status = Status.pending\n  for value, index in 0..10:\n    value + index\n  text = match status:\n    pending: \"wait\"\n",
    );
    assert!(codes(&result).contains(&"E5009"));
}

#[test]
fn checks_optional_dyn_empty_list_and_numeric_bounds() {
    let result = analyze(
        "fn types():\n  small: u8 = 300\n  name: str = null\n  maybe: str? = null\n  items: list[int] = @[]\n  value: dyn = 1\n  value = \"text\"\n",
    );
    let actual = codes(&result);
    assert!(actual.contains(&"E1008"), "{actual:?}");
    assert!(actual.contains(&"E1006"), "{actual:?}");
    assert!(!actual.contains(&"E1004"), "{actual:?}");
}

#[test]
fn type_checks_builtin_methods_and_their_generic_receivers() {
    let valid = analyze(
        "fn main():\n  values: list[int] = @[1, 2, 3]\n  size: int = values.len()\n  empty: bool = values.is_empty()\n  text = \" Vut \"\n  clean: str = text.trim().to_upper()\n  found: bool = text.contains(\"Vut\")\n  number: str = size.to_str()\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );

    let invalid = analyze(
        "fn main():\n  text = \"Vut\"\n  bad = text.contains(1)\n  values: list[int] = @[1]\n  also_bad = values.len(1)\n",
    );
    let actual = codes(&invalid);
    assert!(actual.contains(&"E1003"), "{actual:?}");
    assert!(actual.contains(&"E6003"), "{actual:?}");
}

#[test]
fn rejects_duplicate_constructor_fields_and_if_branch_mismatch() {
    let result = analyze(
        "data User:\n  name: str\nfn invalid():\n  user = User(name: \"A\", name: \"B\")\n  value = if true:\n    1\n  else:\n    \"one\"\n",
    );
    let actual = codes(&result);
    assert!(actual.contains(&"E6004"), "{actual:?}");
    assert!(actual.contains(&"E1010"), "{actual:?}");
}

#[test]
fn integer_literal_uses_declared_function_return_type() {
    let result = analyze("fn ok() -> i32:\n  1\n");
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
}

#[test]
fn validates_builtin_attributes_and_preserves_metadata() {
    let result = analyze(
        "@repr(C)\ndata Point:\n  x: f32\n  y: f32\n@link_name(\"native_add\")\nextern \"C\" fn add(a: i32, b: i32) -> i32\n",
    );
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
    assert_eq!(result.attributes.data_repr.len(), 1);
    assert!(
        result
            .attributes
            .link_names
            .values()
            .any(|name| name == "native_add")
    );
}

#[test]
fn rejects_invalid_attributes() {
    let result = analyze(
        "@unknown\nfn a():\n  1\n@repr(\"C\")\ndata Bad:\n  x: i32\n@repr(C)\n@repr(C)\ndata Dup:\n  x: i32\n@repr(C)\nfn wrong():\n  1\n@link_name(foo)\nextern \"C\" fn bad()\n",
    );
    let actual = codes(&result);
    assert!(actual.contains(&"E0111"), "{actual:?}");
    assert!(result.diagnostics.error_count() >= 5);
}

#[test]
fn type_checks_async_await_and_result_interaction() {
    let result = analyze(
        "data User:\n  name: str\n  age: int\n\
fn User.label() -> str:\n  \"$(self.name) ($(self.age))\"\n\
async fn read_value() -> int:\n  7\n\
async fn read_async() -> result[int, str]:\n  ok(1)\n\
async fn compute() -> int:\n  first = await read_value()\n  second = await read_value()\n  first + second\n\
async fn load() -> result[int, str]:\n  value = await read_async()?\n  ok(value)\n\
async fn make_user() -> User:\n  User(name: \"Nam\", age: await read_value())\n\
async fn main():\n  total = await compute()\n  loaded = await load()\n  match loaded:\n    ok(value): out(\"$value\")\n    err(message): out(message)\n  user = await make_user()\n  out(user.label())\n  out(\"$total\")\n",
    );
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
    assert_eq!(result.async_symbols.len(), 6);
}

#[test]
fn rejects_invalid_async_await_uses() {
    let outside = analyze("fn main():\n  value = await load()\nasync fn load() -> int:\n  1\n");
    assert!(codes(&outside).contains(&"E6011"), "{:?}", codes(&outside));

    let nonawaitable = analyze("async fn main():\n  value = await 123\n");
    assert!(
        codes(&nonawaitable).contains(&"E6012"),
        "{:?}",
        codes(&nonawaitable)
    );

    let ordinary_use = analyze(
        "async fn main():\n  value = load()\n  out(\"$value\")\nasync fn load() -> int:\n  1\n",
    );
    assert!(
        codes(&ordinary_use).contains(&"E6014"),
        "{:?}",
        codes(&ordinary_use)
    );

    let bad_return =
        analyze("async fn wrapper() -> int:\n  return load()\nasync fn load() -> int:\n  1\n");
    assert!(
        codes(&bad_return).contains(&"E6013"),
        "{:?}",
        codes(&bad_return)
    );

    let function_value = analyze("async fn load() -> int:\n  1\nfn main():\n  f = load\n");
    assert!(
        codes(&function_value).contains(&"E6014"),
        "{:?}",
        codes(&function_value)
    );

    let inside_lambda =
        analyze("async fn main():\n  make = () => await load()\nasync fn load() -> int:\n  1\n");
    assert!(
        codes(&inside_lambda).contains(&"E6011"),
        "{:?}",
        codes(&inside_lambda)
    );
}

#[test]
fn type_checks_payload_enums_and_patterns() {
    let valid = analyze(
        "enum Shape:\n  point\n  circle(radius: float)\n  rect(width: float, height: float)\n\
fn area(shape: Shape) -> float:\n  match shape:\n    point: 0.0\n    circle(radius = r): r\n    rect(width = w, height = h): w + h\n\
enum Option:\n  none\n  some(value: str)\n\
fn classify(o: Option) -> str:\n  match o:\n    some(value = \"hi\"): \"exact\"\n    some(value = _): \"some\"\n    none: \"none\"\n\
fn ints(v: int) -> str:\n  match v:\n    0: \"z\"\n    1 or 2: \"s\"\n    3..=9: \"m\"\n    _ if v < 0: \"n\"\n    _: \"l\"\n\
fn call() -> float:\n  shape = Shape.circle(radius: 1.0)\n  area(shape)\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
}

#[test]
fn rejects_invalid_payload_enum_usage() {
    let missing = analyze("enum E:\n  a\n  b\nfn f(e: E) -> int:\n  match e:\n    a: 1\n");
    assert!(codes(&missing).contains(&"E5009"), "{:?}", codes(&missing));

    let unreachable =
        analyze("enum E:\n  a\n  b\nfn f(e: E) -> int:\n  match e:\n    _: 0\n    a: 1\n");
    assert!(
        codes(&unreachable).contains(&"W5004"),
        "{:?}",
        codes(&unreachable)
    );

    let payload = analyze("enum E:\n  a(value: int)\nfn f(e: E) -> int:\n  match e:\n    a: 1\n");
    assert!(codes(&payload).contains(&"E7106"), "{:?}", codes(&payload));

    let unknown =
        analyze("enum E:\n  a\nfn f(e: E) -> int:\n  match e:\n    z(v = 1): 1\n    _: 0\n");
    assert!(codes(&unknown).contains(&"E7101"), "{:?}", codes(&unknown));

    let guard = analyze("enum E:\n  a\nfn f(e: E) -> int:\n  match e:\n    a if 1: 0\n");
    assert!(codes(&guard).contains(&"E7110"), "{:?}", codes(&guard));

    let recursive = analyze("enum E:\n  node(next: E)\n  leaf\n");
    assert!(
        codes(&recursive).contains(&"E7105"),
        "{:?}",
        codes(&recursive)
    );

    let list_pattern = analyze("fn f(xs: list[int]) -> int:\n  match xs:\n    @[a]: a\n    _: 0\n");
    assert!(
        codes(&list_pattern).contains(&"E7112"),
        "{:?}",
        codes(&list_pattern)
    );

    let construct = analyze("enum E:\n  a(value: int)\nfn f() -> E:\n  E.a(value: 1)\n");
    assert!(
        !construct.diagnostics.has_errors(),
        "{:?}",
        construct.diagnostics.as_slice()
    );

    let bad_construct = analyze("enum E:\n  a(value: int)\nfn f() -> E:\n  E.a()\n");
    assert!(
        codes(&bad_construct).contains(&"E7106"),
        "{:?}",
        codes(&bad_construct)
    );
}

#[test]
fn resolves_instance_methods_declared_by_generic_bounds() {
    let valid = analyze(
        "interface Encodable:\n  to_json() -> int\ndata User:\n  age: int\nfn User.to_json() -> int:\n  self.age\nfn encode[T: Encodable](value: T) -> int:\n  value.to_json()\nfn main():\n  out(\"$(encode(User(age: 7)))\")\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
    assert_eq!(valid.bound_calls.len(), 1);
}

#[test]
fn resolves_multiple_bounds_and_nested_container_receivers() {
    let valid = analyze(
        "interface Ping:\n  ping() -> int\ninterface Pong:\n  pong() -> int\ndata D:\n  n: int\nfn D.ping() -> int:\n  self.n\nfn D.pong() -> int:\n  self.n\nfn use[T: Ping + Pong](items: list[T]) -> int:\n  items.at(0).ping() + items.at(0).pong()\nfn main():\n  items: list[D] = @[]\n  items.push(D(n: 1))\n  out(\"$(use(items))\")\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
    assert_eq!(valid.bound_calls.len(), 2);
}

#[test]
fn rejects_method_not_declared_by_any_bound() {
    let result = analyze(
        "interface Encodable:\n  to_json() -> int\ndata User:\n  age: int\nfn User.to_json() -> int:\n  self.age\nfn bad[T: Encodable](value: T) -> int:\n  value.missing()\nfn main():\n  out(\"$(bad(User(age: 1)))\")\n",
    );
    assert!(codes(&result).contains(&"E1018"), "{:?}", codes(&result));
    assert!(!codes(&result).contains(&"E2005"), "{:?}", codes(&result));
}

#[test]
fn rejects_conflicting_bounds_for_the_same_method_name() {
    let result = analyze(
        "interface A:\n  m() -> int\ninterface B:\n  m() -> str\ndata D:\n  n: int\nfn D.m() -> int:\n  self.n\nfn use[T: A + B](value: T) -> int:\n  value.m()\n",
    );
    assert!(codes(&result).contains(&"E1019"), "{:?}", codes(&result));
}

#[test]
fn reports_unsatisfied_bound_with_the_missing_method() {
    let result = analyze(
        "interface Ping:\n  ping() -> int\ndata D:\n  n: int\nfn other[T: Ping](value: T) -> int:\n  value.ping()\nfn main():\n  out(\"$(other(42))\")\n",
    );
    assert!(codes(&result).contains(&"E1017"), "{:?}", codes(&result));
}

#[test]
fn resolves_static_bound_calls_with_self() {
    let result = analyze(
        "interface Decodable:\n  static from_text(text: str) -> result[Self, int]\ndata User:\n  name: str\nstatic fn User.from_text(text: str) -> result[User, int]:\n  ok(User(name: text))\nfn decode[T: Decodable](text: str) -> result[T, int]:\n  T.from_text(text)\nfn main():\n  value: result[User, int] = decode(\"hi\")\n  match value:\n    ok(user): out(user.name)\n    err(code): out(\"err\")\n",
    );
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
    assert!(result.bound_calls.values().any(|call| call.is_static));
}

#[test]
fn rejects_static_method_used_as_instance_method() {
    let result = analyze(
        "data User:\n  name: str\nstatic fn User.make(text: str) -> User:\n  User(name: text)\nfn bad(value: User) -> User:\n  value.make(\"x\")\nfn main():\n  out(\"x\")\n",
    );
    assert!(codes(&result).contains(&"E2005"), "{:?}", codes(&result));
}

#[test]
fn reports_unsatisfied_static_bound() {
    let result = analyze(
        "interface Decodable:\n  static from_text(text: str) -> result[Self, int]\ndata Plain:\n  n: int\nfn decode[T: Decodable](text: str) -> result[T, int]:\n  T.from_text(text)\nfn main():\n  value: result[Plain, int] = decode(\"hi\")\n  match value:\n    ok(item): out(\"ok\")\n    err(code): out(\"err\")\n",
    );
    assert!(codes(&result).contains(&"E1017"), "{:?}", codes(&result));
}

#[test]
fn match_arm_block_value_is_the_final_expression() {
    let valid = analyze(
        "enum Maybe:\n  none\n  some(value: int)\nfn value(input: Maybe) -> int:\n  match input:\n    some(inner):\n      doubled = inner * 2\n      doubled\n    none: 0\nfn main():\n  out(\"$(value(Maybe.some(value: 3)))\")\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );

    let void_block = analyze(
        "fn value(x: int) -> int:\n  match x:\n    0:\n      a = 1\n    1: 1\nfn main():\n  out(\"$(value(0))\")\n",
    );
    assert!(
        void_block.diagnostics.has_errors(),
        "a block without a final expression is `void` and must not unify with an `int` arm"
    );
}

#[test]
fn variadic_functions_accept_zero_or_more_matching_arguments() {
    let valid = analyze(
        "fn sum(...values: int) -> int:\n  total = 0\n  for value in values:\n    total = total + value\n  total\nfn main():\n  a = sum()\n  b = sum(1, 2, 3)\n  out(\"$a $b\")\n",
    );
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );

    let mismatch = analyze(
        "fn sum(...values: int) -> int:\n  values.len()\nfn main():\n  result = sum(\"bad\")\n  out(\"$result\")\n",
    );
    assert!(
        codes(&mismatch).contains(&"E1003"),
        "{:?}",
        codes(&mismatch)
    );

    let not_last =
        analyze("fn bad(...values: int, tail: int) -> int:\n  tail\nfn main():\n  out(\"x\")\n");
    assert!(
        codes(&not_last).contains(&"E7013"),
        "{:?}",
        codes(&not_last)
    );
}

#[test]
fn list_higher_order_builtins_infer_lambda_parameters() {
    let result = analyze(
        "fn main():\n  items = @[1, 2, 3]\n  labels = items.map(x => x.to_str())\n  evens = items.filter(x => x % 2 == 0)\n  total = items.fold(0, (acc, x) => acc + x)\n  any = items.any(x => x > 1)\n  all = items.all(x => x > 0)\n  index = items.find_index(x => x == 2)\n  out(labels)\n  out(evens)\n  out(total)\n  out(any)\n  out(all)\n  out(index)\n",
    );
    assert!(
        !result.diagnostics.has_errors(),
        "{:?}",
        result.diagnostics.as_slice()
    );
    assert!(
        !codes(&result).contains(&"E1012"),
        "lambda parameters are inferred from the receiver element type: {:?}",
        codes(&result)
    );
}

#[test]
fn join_is_only_defined_for_list_str() {
    let invalid = analyze("fn main():\n  nums = @[1, 2]\n  value = nums.join(\"-\")\n");
    assert!(codes(&invalid).contains(&"E1028"), "{:?}", codes(&invalid));

    let valid = analyze("fn main():\n  words = @[\"a\", \"b\"]\n  value = words.join(\"-\")\n");
    assert!(
        !valid.diagnostics.has_errors(),
        "{:?}",
        valid.diagnostics.as_slice()
    );
}

#[test]
fn collection_callbacks_are_arity_checked() {
    let missing = analyze("fn main():\n  items = @[1, 2]\n  value = items.map()\n");
    assert!(codes(&missing).contains(&"E1003"), "{:?}", codes(&missing));

    let bad_fold = analyze("fn main():\n  items = @[1, 2]\n  value = items.fold(0)\n");
    assert!(
        codes(&bad_fold).contains(&"E1003"),
        "{:?}",
        codes(&bad_fold)
    );
}

#[test]
fn rejects_constant_reassignment_with_e1104() {
    let declared = analyze("fn main():\n  MAX_SIZE = 100\n  out(\"$(MAX_SIZE)\")\n");
    assert!(
        !declared.diagnostics.has_errors(),
        "{:?}",
        declared.diagnostics.as_slice()
    );
    assert!(!codes(&declared).contains(&"E1104"));

    let reassigned = analyze("fn main():\n  MAX_SIZE = 100\n  MAX_SIZE = 200\n");
    assert!(
        codes(&reassigned).contains(&"E1104"),
        "{:?}",
        codes(&reassigned)
    );

    let annotated = analyze("fn main():\n  MAX_SIZE: int = 1\n  MAX_SIZE = 2\n");
    assert!(
        codes(&annotated).contains(&"E1104"),
        "{:?}",
        codes(&annotated)
    );

    let lowercase = analyze("fn main():\n  max_size = 1\n  max_size = 2\n");
    assert!(
        !codes(&lowercase).contains(&"E1104"),
        "{:?}",
        codes(&lowercase)
    );
}

#[test]
fn optional_misuse_uses_e1006_and_e1007() {
    let null_into_plain = analyze("fn main():\n  name: str = null\n");
    assert!(
        codes(&null_into_plain).contains(&"E1006"),
        "{:?}",
        codes(&null_into_plain)
    );
    assert!(!codes(&null_into_plain).contains(&"E1003"));

    let optional_into_plain = analyze(
        "fn use(value: str):\n  out(value)\nfn main():\n  name: str? = \"hi\"\n  use(name)\n",
    );
    assert!(
        codes(&optional_into_plain).contains(&"E1007"),
        "{:?}",
        codes(&optional_into_plain)
    );

    let narrowed = analyze(
        "fn use(value: str):\n  out(value)\nfn main():\n  name: str? = \"hi\"\n  if name != null:\n    use(name)\n",
    );
    assert!(
        !narrowed.diagnostics.has_errors(),
        "{:?}",
        narrowed.diagnostics.as_slice()
    );
}

#[test]
fn rejects_unknown_and_invalid_interface_parents() {
    let unknown = all_codes("interface Child: Missing:\n  run() -> int\n");
    assert!(unknown.contains(&"E4001".to_owned()), "{unknown:?}");

    let invalid = all_codes(
        "data NotAnInterface:\n  value: int\ninterface Child: NotAnInterface:\n  run() -> int\n",
    );
    assert!(invalid.contains(&"E4004".to_owned()), "{invalid:?}");
}

#[test]
fn interface_satisfaction_uses_compatible_signatures() {
    // A covariant return (`Dog` where `Animal` is required) satisfies the
    // interface; a contravariant parameter mismatch does not.
    let covariant = analyze(
        "interface Animal:\n  speak() -> str\ninterface Producer:\n  make() -> Animal\ndata Dog:\n  name: str\nfn Dog.speak() -> str:\n  \"woof\"\nfn Dog.make() -> Dog:\n  Dog(name: \"d\")\nfn accept(producer: Producer) -> str:\n  \"ok\"\nfn main():\n  out(accept(Dog(name: \"d\")))\n",
    );
    assert!(
        !covariant.diagnostics.has_errors(),
        "{:?}",
        covariant.diagnostics.as_slice()
    );

    let wrong_parameter = analyze(
        "interface Animal:\n  speak() -> str\ninterface Consumer:\n  take(value: Animal)\ndata Dog:\n  name: str\nfn Dog.speak() -> str:\n  \"woof\"\nfn Dog.take(value: Dog):\n  value.name\nfn accept(consumer: Consumer) -> int:\n  0\nfn main():\n  value = accept(Dog(name: \"d\"))\n  out(\"$(value)\")\n",
    );
    assert!(
        wrong_parameter.diagnostics.has_errors(),
        "{:?}",
        wrong_parameter.diagnostics.as_slice()
    );
}
