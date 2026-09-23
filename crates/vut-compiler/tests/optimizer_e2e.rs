//! Optimizer correctness: the release optimizer must preserve observable
//! behavior and never remove scheduling/ownership instructions.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{BuildMode, CompilerConfig, CompilerSession, OptimizationLevel};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-opt-{label}-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    root
}

fn run_with(source: &str, mode: BuildMode) -> (Option<i32>, String) {
    let root = scratch("e2e");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        build_mode: mode,
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

fn run_at(source: &str, level: OptimizationLevel) -> (Option<i32>, String) {
    let root = scratch("level");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        optimization: Some(level),
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

fn run_both(source: &str) -> (String, String) {
    let (reference_code, reference) = run_with(source, BuildMode::Debug);
    let (optimized_code, optimized) = run_with(source, BuildMode::Release);
    assert_eq!(
        reference_code, optimized_code,
        "exit code differs: reference={reference_code:?} optimized={optimized_code:?}\nreference:\n{reference}\noptimized:\n{optimized}"
    );
    (reference, optimized)
}

fn mir_for(source: &str) -> vut_mir::Program {
    let root = scratch("mir");
    fs::write(root.join("main.vut"), source).expect("write source");
    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root, &[])
        .expect("check source");
    fs::remove_dir_all(&root).ok();
    checked.mir
}

fn count_list_at_unchecked(program: &vut_mir::Program) -> usize {
    program
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| {
            matches!(
                instruction,
                vut_mir::Instruction::RuntimeCall {
                    function: vut_types::BuiltinFunction::ListAtUnchecked,
                    ..
                }
            )
        })
        .count()
}

fn has_spawn(program: &vut_mir::Program) -> bool {
    program.functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, vut_mir::Instruction::Spawn { .. }))
        })
    })
}

#[test]
fn optimized_matches_reference_for_optionals() {
    let source = "fn describe(value: str?) -> str:\n  if value != null:\n    return value\n  \"none\"\n\nfn main():\n  out(describe(\"hi\"))\n  out(describe(null))\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_interface_dispatch() {
    let source = "interface Animal:\n  speak() -> str\n  legs() -> int\n\ndata Dog:\n  name: str\n\ndata Bird:\n  name: str\n\nfn Dog.speak() -> str:\n  \"woof\"\n\nfn Dog.legs() -> int:\n  4\n\nfn Bird.speak() -> str:\n  \"tweet\"\n\nfn Bird.legs() -> int:\n  2\n\nfn describe(animal: Animal) -> str:\n  \"$(animal.speak()) has $(animal.legs()) legs\"\n\nfn main():\n  out(describe(Dog(name: \"Rex\")))\n  out(describe(Bird(name: \"Pip\")))\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_variadics() {
    let source = "data Point:\n  x: int\n  y: int\n\nfn sum(...values: int) -> int:\n  total = 0\n  for value in values:\n    total = total + value\n  total\n\nfn sum_x(...points: Point) -> int:\n  total = 0\n  for point in points:\n    total = total + point.x\n  total\n\nfn main():\n  items = @[10, 20, 30]\n  out(\"spread=$(sum(...items))\")\n  out(\"points=$(sum_x(Point(x: 1, y: 2), Point(x: 3, y: 4)))\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_map() {
    let source = "fn main():\n  scores: map[str, int] = (\"ann\": 10, \"bob\": 20)\n  ann: int? = scores.get(\"ann\")\n  if ann != null:\n    out(\"ann=$ann\")\n  missing: int? = scores.get(\"zed\")\n  if missing == null:\n    out(\"missing\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_result_and_async() {
    let source = "fn load() -> result[int, int]:\n  ok(7)\n\nasync fn get() -> result[int, int]:\n  job: vutcon[result[int, int]] = vut(() => load())\n  inner = await job?\n  ok(inner)\n\nasync fn main():\n  match await get():\n    ok(v): out(\"ok=$v\")\n    err(e): out(\"err=$e\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
    assert_eq!(optimized, "ok=7\n");
}

#[test]
fn all_optimization_levels_agree() {
    let source = "fn classify(n: int) -> int:\n  if n < 0:\n    return -1\n  if n == 0:\n    return 0\n  return n * 2\n\nasync fn main():\n  total = 0\n  for i in 0..20:\n    total = total + classify(i)\n  values = @[total, 1, 2]\n  values.push(5)\n  values.sort()\n  out(\"$(values.len()):$total\")\n  name = \"vut\"\n  out(\"hi, \" + name)\n  ch = channel[int](capacity: 1)\n  ch.send(total)\n  ch.close()\n  a = ch.recv()\n  if a != null:\n    out(\"ch=$a\")\n";
    let levels = [
        OptimizationLevel::O0,
        OptimizationLevel::O1,
        OptimizationLevel::O2,
        OptimizationLevel::O3,
    ];
    let mut expected: Option<(Option<i32>, String)> = None;
    for level in levels {
        let result = run_at(source, level);
        match &expected {
            None => expected = Some(result),
            Some(reference) => assert_eq!(
                reference, &result,
                "level {level:?} disagrees with the reference"
            ),
        }
    }
}

#[test]
fn loop_list_bounds_check_is_eliminated() {
    let source = "fn main():\n  xs = @[1, 2, 3, 4]\n  total = 0\n  for i in 0..xs.len():\n    total = total + xs.at(i)\n  out(\"total=$total\")\n";
    let mut mir = mir_for(source);
    let report = vut_mir::optimize(&mut mir, vut_mir::OptimizationLevel::O2);
    assert!(
        count_list_at_unchecked(&mir) >= 1,
        "expected the loop's list access to be unchecked: {report:?}"
    );
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
    assert_eq!(optimized, "total=10\n");
}

#[test]
fn out_of_bounds_list_access_still_traps() {
    // The index is not provably in bounds, so the check must remain and trap.
    let source = "fn main():\n  xs = @[1, 2, 3]\n  index = 3\n  out(\"$(xs.at(index))\")\n";
    let (code, _) = run_with(source, BuildMode::Release);
    assert_ne!(code, Some(0), "an out-of-bounds access must trap");
}

#[test]
fn verifier_accepts_lowered_mir() {
    let source = "fn main():\n  xs = @[1, 2, 3, 4]\n  total = 0\n  for i in 0..xs.len():\n    total = total + xs.at(i)\n  out(\"total=$total\")\n";
    let mir = mir_for(source);
    if let Err(errors) = vut_mir::verify(&mir) {
        for error in errors.iter().take(10) {
            eprintln!("VERIFY: {error}");
        }
        panic!("verifier rejected lowered MIR: {} errors", errors.len());
    }
}

#[test]
fn discarded_vutcon_spawn_survives_optimization() {
    // The result of `vut(...)` is unused. The optimizer must not delete the
    // `Spawn`: it schedules a task, which is an observable effect.
    let source = "fn side() -> int:\n  out(\"side\")\n  1\n\nasync fn main():\n  vut(() => side())\n  out(\"main\")\n";
    let mut mir = mir_for(source);
    assert!(has_spawn(&mir), "the unoptimized MIR must contain a Spawn");
    let _ = vut_mir::optimize(&mut mir, vut_mir::OptimizationLevel::O2);
    assert!(
        has_spawn(&mir),
        "the optimizer must not remove a discarded Spawn"
    );
}

#[test]
fn optimizer_preserves_discarded_spawn_output() {
    let source = "fn side() -> int:\n  out(\"side\")\n  1\n\nasync fn main():\n  vut(() => side())\n  out(\"main\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_inlined_receiver_methods() {
    // Inlining a receiver method must bind its `Borrow(self)` to the call
    // argument, not to the caller's local at the same index. Before the fix
    // the inlined `Borrow` read the caller's local 0 and the field projection
    // was looked up on the wrong type.
    let source = "data Count:\n  value: int\n\nfn Count.get() -> int:\n  self.value\n\nfn main():\n  first = 111\n  c = Count(value: 7)\n  out(\"$(c.get()):$first\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
    assert_eq!(optimized, "7:111\n");
}

#[test]
fn optimized_matches_reference_for_scalar_and_control_flow() {
    let source = "fn classify(n: int) -> int:\n  if n < 0:\n    return -1\n  if n == 0:\n    return 0\n  return n * 2\n\nfn main():\n  total = 0\n  for i in 0..10:\n    total = total + classify(i)\n  out(\"total=$total\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
    assert_eq!(optimized, "total=90\n");
}

#[test]
fn optimized_matches_reference_for_collections() {
    let source = "fn main():\n  values = @[3, 1, 2]\n  values.push(4)\n  values.sort()\n  doubled = values.map(v => v * 2)\n  out(\"$(doubled.len())\")\n  for v in doubled:\n    out(\"$v\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_managed_values() {
    let source = "fn main():\n  name = \"vut\"\n  greeting = \"hi, \" + name\n  copy = greeting\n  out(greeting)\n  out(copy)\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
}

#[test]
fn optimized_matches_reference_for_channels_and_vutcons() {
    let source = "async fn main():\n  ch = channel[int](capacity: 2)\n  ch.send(7)\n  ch.close()\n  a = ch.recv()\n  if a != null:\n    out(\"got=$a\")\n  b = ch.recv()\n  if b == null:\n    out(\"absent\")\n";
    let (reference, optimized) = run_both(source);
    assert_eq!(reference, optimized);
    assert_eq!(optimized, "got=7\nabsent\n");
}
