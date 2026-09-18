//! Lowering unit tests.

use super::*;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_resolver::{ModuleInput, ModulePath, Resolver};
use vut_source::SourceId;
use vut_types::Analyzer;
fn program(source: &str) -> Program {
    let id = SourceId::from_index(0);
    let (tokens, _) = Lexer::new(id, source).lex();
    let (file, _) = Parser::new(id, source, tokens).parse();
    let resolution = Resolver::new(vec![ModuleInput {
        logical_path: ModulePath(vec!["main".into()]),
        filesystem_path: None,
        file,
    }])
    .resolve();
    let hir = vut_hir::lower(&resolution);
    let sema = Analyzer::new(&resolution).analyze();
    lower(&hir, &sema, 8)
}
#[test]
fn lowers_arithmetic_to_backend_independent_values() {
    let p = program("fn add(a: int, b: int) -> int:\n  a + b\n");
    assert!(
        p.functions[0].blocks[0]
            .instructions
            .iter()
            .any(|i| matches!(
                i,
                Instruction::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ))
    );
}
#[test]
fn classifies_managed_and_copy_layouts() {
    let p = program("fn main():\n  1\n");
    assert!(p.layouts.types.iter().any(|i| i.is_copy && !i.needs_drop));
    assert!(
        p.layouts
            .types
            .iter()
            .any(|i| i.contains_managed && i.needs_drop)
    );
}
#[test]
fn last_use_moves_managed_values_and_eliminates_moved_drop() {
    let p = program("fn main() -> str:\n  value = \"owned\"\n  value\n");
    let instructions = &p.functions[0].blocks[0].instructions;
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::Move { .. }))
    );
    assert!(
        !instructions
            .iter()
            .any(|item| matches!(item, Instruction::Drop(_)))
    );
}
#[test]
fn inserts_deterministic_cleanup_for_unused_managed_locals() {
    let p = program("fn main() -> int:\n  value = \"owned\"\n  1\n");
    assert!(
        p.functions[0].blocks[0]
            .instructions
            .iter()
            .any(|item| matches!(item, Instruction::Drop(_)))
    );
}
#[test]
fn lowers_source_calls_and_methods_to_direct_symbols() {
    let p = program(
        "fn answer() -> int:\n  42\ndata Box:\n  value: int\nfn Box.get() -> int:\n  self.value\nfn main() -> int:\n  answer()\n",
    );
    assert!(
        p.functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.instructions)
            .any(|item| matches!(item, Instruction::Call { .. }))
    );
    assert!(
        p.functions
            .iter()
            .any(|function| function.receiver.is_some())
    );
}
#[test]
fn lowers_if_and_loop_control_to_real_cfg_edges() {
    let p = program("fn main() -> int:\n  for true:\n    if true:\n      break\n  1\n");
    let function = &p.functions[0];
    assert!(function.blocks.len() >= 7);
    assert!(
        function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Branch { .. }))
    );
    assert!(
        function
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Jump(_)))
            .count()
            >= 3
    );
}
#[test]
fn early_return_inserts_cleanup_before_its_exit_edge() {
    let p = program(
        "fn main() -> int:\n  resource = \"owned\"\n  if true:\n    return 1\n  resource\n  0\n",
    );
    assert!(p.functions[0].blocks.iter().any(|block| {
        matches!(block.terminator, Terminator::Return(Some(_)))
            && block
                .instructions
                .iter()
                .any(|item| matches!(item, Instruction::Drop(_)))
    }));
}
#[test]
fn lowers_iterators_and_match_decisions_explicitly() {
    let p = program(
        "enum Status:\n  pending\n  done\nfn main() -> str:\n  values: list(int) = @(1, 2)\n  for value, index in values:\n    value + index\n  status = Status.pending\n  match status:\n    pending: \"wait\"\n    done: \"done\"\n",
    );
    let instructions = p.functions[0]
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::IteratorInit { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ConstructEnum { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::EnumTag { .. }))
    );
}
#[test]
fn preserves_and_types_every_template_segment() {
    let p = program("fn main() -> str:\n  number = 42\n  \"value=$number next=$(number + 1)\"\n");
    let instructions: Vec<_> = p.functions[0]
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .collect();
    assert_eq!(
        instructions
            .iter()
            .filter(|item| matches!(item, Instruction::FormatValue { .. }))
            .count(),
        2
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ConcatString { .. }))
    );
}

#[test]
fn lowers_extern_link_name_metadata() {
    let p = program("@link_name(\"native_add\")\nextern \"C\" fn add(a: i32, b: i32) -> i32\n");
    assert!(p.functions.is_empty());
    assert_eq!(p.external_functions.len(), 1);
    assert_eq!(p.external_functions[0].link_name, "native_add");
    assert_eq!(p.external_functions[0].parameters.len(), 2);
}

#[test]
fn default_extern_link_name_is_the_declared_name() {
    let p = program("extern \"C\" fn native_add(a: i32, b: i32) -> i32\n");
    assert_eq!(p.external_functions.len(), 1);
    assert_eq!(p.external_functions[0].link_name, "native_add");
}

#[test]
fn lays_out_nested_repr_types_in_dependency_order() {
    let p = program(
        "@repr(C)\ndata Rect:\n  min: Vec2\n  max: Vec2\n@repr(C)\ndata Vec2:\n  x: f32\n  y: f32\n",
    );
    let (rect_id, rect) = p
        .layouts
        .fields
        .iter()
        .find(|(_, fields)| fields.len() == 2 && fields[0].name == "min")
        .map(|(id, fields)| (*id, fields.clone()))
        .expect("Rect layout");
    assert_eq!(rect[0].offset, 0);
    assert_eq!(rect[1].offset, 8);
    assert_eq!(p.layouts.types[rect_id.0].size, 16);
    assert_eq!(p.layouts.types[rect_id.0].alignment, 4);
    let vec2 = p
        .layouts
        .fields
        .values()
        .find(|fields| fields.len() == 2 && fields[0].name == "x")
        .expect("Vec2 layout");
    assert_eq!(vec2[0].offset, 0);
    assert_eq!(vec2[1].offset, 4);
}

#[test]
fn lowers_result_construction_match_and_question_operator() {
    let p = program(
        "fn load() -> result(int, str):\n  ok(1)\nfn main() -> result(int, str):\n  value = load()?\n  checked: result(int, str) = ok(value)\n  match checked:\n    ok(number): ok(number)\n    err(message): err(message)\n",
    );
    let instructions = p
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ConstructResult { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ResultState { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ResultPayload { .. }))
    );
}

#[test]
fn lowers_async_await_to_a_start_future_marker() {
    let p = program(
        "async fn value() -> int:\n  7\nasync fn main():\n  x = await value()\n  out(\"$x\")\n",
    );
    let instructions = p
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::StartFuture { .. })),
        "an async call must lower to StartFuture"
    );
    assert!(
        p.functions.iter().any(|function| function.is_async),
        "async declarations carry is_async in MIR"
    );
    assert!(
        p.functions.len() >= 2,
        "async functions must lower to ordinary MIR functions"
    );
}

#[test]
fn lowers_payload_enum_construction_and_matching() {
    let p = program(
        "enum Shape:\n  point\n  circle(radius: float)\nfn area(s: Shape) -> float:\n  match s:\n    point: 0.0\n    circle(radius = r): r\nfn main() -> float:\n  area(Shape.circle(radius = 1.0))\n",
    );
    let instructions = p
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ConstructEnum { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::EnumTag { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::EnumPayload { .. }))
    );
}

#[test]
fn computes_frame_layout_for_async_function() {
    let p = program(
        "async fn work(a: int, b: int) -> int:\n  a + b\nasync fn main():\n  out(\"$(await work(1, 2))\")\n",
    );
    let work = p
        .functions
        .iter()
        .find(|function| function.is_async && p.frames[&function.symbol].parameters.len() == 2)
        .expect("async work with two parameters");
    let layout = p.frames.get(&work.symbol).expect("frame layout");
    assert_eq!(layout.parameters.len(), 2);
    assert!(layout.completion.is_some());
    assert!(layout.size >= future::FRAME_HEADER_SIZE);
    assert!(layout.align >= 8);
    assert!(
        work.frame_param.is_some(),
        "async bodies read parameters from the frame"
    );
    assert_eq!(work.parameter_count, 0);
}

#[test]
fn tracks_locals_live_across_await() {
    let p = program(
        "async fn inner() -> int:\n  1\nasync fn work() -> int:\n  a = 10\n  b = await inner()\n  a + b\nasync fn main():\n  out(\"$(await work())\")\n",
    );
    let work = p
        .functions
        .iter()
        .find(|function| function.is_async && !p.awaits[&function.symbol].is_empty())
        .expect("async function with an await");
    let sites = &p.awaits[&work.symbol];
    assert_eq!(sites.len(), 1, "one await site");
    assert!(
        !sites[0].locals.is_empty(),
        "the local used after the await must be live across it"
    );
    let layout = p.frames.get(&work.symbol).expect("frame layout");
    assert!(
        !layout.locals.is_empty(),
        "live-across-await locals get frame slots"
    );
}

#[test]
fn tracks_multiple_sequential_awaits() {
    let p = program(
        "async fn one() -> int:\n  1\nasync fn two() -> int:\n  2\nasync fn work() -> int:\n  a = 1\n  x = await one()\n  y = await two()\n  a + x + y\nasync fn main():\n  out(\"$(await work())\")\n",
    );
    let work = p
        .functions
        .iter()
        .find(|function| function.is_async && p.awaits[&function.symbol].len() == 2)
        .expect("async function with two awaits");
    assert_eq!(p.awaits[&work.symbol].len(), 2);
    let states: Vec<u32> = p.awaits[&work.symbol]
        .iter()
        .map(|site| site.state)
        .collect();
    assert_eq!(states, vec![1, 2], "resume states are assigned in order");
}

#[test]
fn tracks_awaits_inside_conditionals_and_loops() {
    let p = program(
        "async fn one() -> int:\n  1\nasync fn conditional(flag: bool) -> int:\n  if flag:\n    return await one()\n  0\nasync fn looping() -> int:\n  total = 0\n  for index in @(0, 1):\n    total = total + await one()\n  total\nasync fn main():\n  out(\"$(await conditional(true))\")\n",
    );
    assert!(
        p.awaits.values().flatten().count() >= 2,
        "awaits inside if and loop are detected"
    );
}

#[test]
fn detects_values_live_across_await() {
    let p = program(
        "fn compute() -> int:\n  5\nfn add(a: int, b: int) -> int:\n  a + b\nasync fn one() -> int:\n  1\nasync fn combine() -> int:\n  add(compute(), await one())\nasync fn main():\n  out(\"$(await combine())\")\n",
    );
    let combine = p
        .functions
        .iter()
        .find(|function| function.is_async && p.awaits[&function.symbol].len() == 1)
        .expect("async function with one await");
    let site = &p.awaits[&combine.symbol][0];
    assert!(
        site.crossing_values >= 1,
        "compute() result is live across the await"
    );
}

#[test]
fn spills_values_live_across_await_into_the_frame() {
    let p = program(
        "fn compute() -> int:\n  5\nfn add(a: int, b: int) -> int:\n  a + b\nasync fn one() -> int:\n  1\nasync fn combine() -> int:\n  add(compute(), await one())\nasync fn main():\n  out(\"$(await combine())\")\n",
    );
    let combine = p
        .functions
        .iter()
        .find(|function| function.is_async && p.awaits[&function.symbol].len() == 1)
        .expect("async function with one await");
    assert!(combine.is_poll, "a crossing value is spilled, not blocked");
    let instructions = combine
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::SpillValue { .. })),
        "the crossing value is stored in the frame"
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::ReloadValue { .. })),
        "the crossing value is reloaded before its use"
    );
    assert!(
        !p.frames[&combine.symbol].regions.is_empty(),
        "the spill gets a frame region"
    );
}

#[test]
fn statement_await_has_no_crossing_values() {
    let p = program(
        "async fn one() -> int:\n  1\nasync fn work() -> int:\n  value = await one()\n  value\nasync fn main():\n  out(\"$(await work())\")\n",
    );
    let work = p
        .functions
        .iter()
        .find(|function| function.is_async && p.awaits[&function.symbol].len() == 1)
        .expect("async function with one await");
    let site = &p.awaits[&work.symbol][0];
    assert_eq!(
        site.crossing_values, 0,
        "a simple awaited binding leaves no value live across the await"
    );
}

#[test]
fn converts_straight_line_async_body_into_a_poll_state_machine() {
    let p = program(
        "async fn one() -> int:\n  1\nasync fn work() -> int:\n  a = 10\n  x = await one()\n  y = await one()\n  a + x + y\nasync fn main():\n  out(\"$(await work())\")\n",
    );
    let work = p
        .functions
        .iter()
        .find(|function| function.is_async && p.awaits[&function.symbol].len() == 2)
        .expect("async function with two awaits");
    assert!(
        work.is_poll,
        "a straight-line async body becomes a poll body"
    );
    assert!(
        work.out_param.is_some(),
        "a poll body receives an out pointer"
    );
    let instructions = work
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .collect::<Vec<_>>();
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::FrameState { .. })),
        "the poll body dispatches on the frame state word"
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::SetFrameState { .. })),
        "suspending records the resume state"
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::PollFuture { .. })),
        "awaits poll without blocking"
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::SetFrameChild { .. })),
        "in-flight child handles are spilled to the frame"
    );
    assert!(
        work.blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::PollReturn(0))),
        "a pending poll returns the pending status"
    );
    assert!(
        work.locals
            .iter()
            .any(|local| matches!(local.storage, LocalStorage::Frame(_))),
        "locals live across an await are frame-resident"
    );
}

fn is_poll(function: &Function) -> bool {
    function.is_poll
}

#[test]
fn converts_conditional_async_body_into_a_poll_state_machine() {
    let p = program(
        "async fn one() -> int:\n  1\nasync fn conditional(flag: bool) -> int:\n  value = 0\n  if flag:\n    value = await one()\n  value\nasync fn main():\n  out(\"$(await conditional(true))\")\n",
    );
    let work = p
        .functions
        .iter()
        .find(|function| {
            function.is_async
                && p.awaits.contains_key(&function.symbol)
                && !p.awaits[&function.symbol].is_empty()
                && function.blocks.len() > 1
                && is_poll(function)
        })
        .expect("an async body with a conditional await is a poll body");
    assert!(work.is_poll);
}

#[test]
fn converts_match_async_body_into_a_poll_state_machine() {
    let p = program(
        "enum Choice:\n  yes\n  no\nasync fn one() -> int:\n  1\nasync fn pick(choice: Choice) -> int:\n  match choice:\n    yes: await one()\n    no: 0\nasync fn main():\n  out(\"$(await pick(Choice.yes))\")\n",
    );
    assert!(
        p.functions
            .iter()
            .any(|function| function.is_async && function.is_poll),
        "an async body awaiting inside match becomes a poll body"
    );
}

#[test]
fn converts_loop_async_body_into_a_poll_state_machine() {
    let p = program(
        "async fn one() -> int:\n  1\nasync fn looping() -> int:\n  total = 0\n  for total < 3:\n    await one()\n    total = total + 1\n  total\nasync fn main():\n  out(\"$(await looping())\")\n",
    );
    let looping = p
        .functions
        .iter()
        .find(|function| function.is_async && function.blocks.len() > 2)
        .expect("an async body with a loop");
    assert!(
        looping.is_poll,
        "a conditional loop body awaiting a statement becomes a poll body"
    );
    assert!(
        looping
            .locals
            .iter()
            .any(|local| matches!(local.storage, LocalStorage::Frame(_))),
        "loop-carried locals survive the suspension in the frame"
    );
}

#[test]
fn converts_iterable_loop_over_a_local_into_a_poll_state_machine() {
    // The iterator state is frame-resident so an iterable loop can suspend.
    let p = program(
        "async fn one() -> int:\n  1\nasync fn looping() -> int:\n  total = 0\n  values: list(int) = @(0, 1, 2)\n  for value in values:\n    await one()\n    total = total + 1\n  total\nasync fn main():\n  out(\"$(await looping())\")\n",
    );
    let looping = p
        .functions
        .iter()
        .find(|function| function.is_async && function.blocks.len() > 2)
        .expect("an async body with an iterable loop");
    assert!(
        looping.is_poll,
        "an iterable loop over a local becomes a poll body"
    );
    assert!(
        p.frames[&looping.symbol]
            .regions
            .iter()
            .any(|region| region.size >= 32),
        "the iterator state gets a frame region"
    );
}
#[test]
fn higher_order_builtins_lower_inline_without_a_runtime_callback() {
    let p = program(
        "fn double(x: int) -> int:\n  x * 2\nfn keep(x: int) -> bool:\n  x > 0\nfn combine(acc: int, x: int) -> int:\n  acc + x\nfn compare(a: int, b: int) -> int:\n  a - b\nfn main():\n  items = @(1, 2, 3)\n  items.map(double)\n  items.filter(keep)\n  items.fold(0, combine)\n  items.any(keep)\n  items.all(keep)\n  items.find_index(keep)\n  items.sort_by(compare)\n",
    );
    let instructions: Vec<_> = p
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .collect();
    let indirect = instructions
        .iter()
        .filter(|item| matches!(item, Instruction::CallIndirect { .. }))
        .count();
    assert!(
        indirect >= 7,
        "each higher-order builtin invokes its callback through a normal call: {indirect}"
    );
    assert!(
        !instructions.iter().any(
            |item| matches!(item, Instruction::RuntimeCall { function, .. } if matches!(
                function,
                BuiltinFunction::ListMap
                    | BuiltinFunction::ListFilter
                    | BuiltinFunction::ListFold
                    | BuiltinFunction::ListAny
                    | BuiltinFunction::ListAll
                    | BuiltinFunction::ListFindIndexBy
                    | BuiltinFunction::ListSortBy
            ))
        ),
        "higher-order builtins never reach a native runtime call"
    );
    assert!(
        instructions
            .iter()
            .any(|item| matches!(item, Instruction::IteratorInit { .. }))
    );
}
#[test]
fn short_circuit_builtins_jump_directly_to_the_loop_exit() {
    let p = program(
        "fn main():\n  items = @(1, 2, 3)\n  items.any(keep)\n\nfn keep(x: int) -> bool:\n  x > 2\n",
    );
    let function = p
        .functions
        .iter()
        .find(|function| {
            function
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .any(|item| matches!(item, Instruction::IteratorInit { .. }))
        })
        .expect("the `any` call lowers to an iterator loop");
    let has_value = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|item| match item {
            Instruction::IteratorNext { has_value, .. } => Some(*has_value),
            _ => None,
        })
        .expect("the loop calls IteratorNext");
    let exit = function
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Terminator::Branch {
                condition,
                else_block,
                ..
            } if *condition == has_value => Some(*else_block),
            _ => None,
        })
        .expect("the loop header branches to the exit");
    assert!(
        function
            .blocks
            .iter()
            .any(|block| matches!(&block.terminator, Terminator::Jump(target) if *target == exit)),
        "a short-circuiting builtin must jump straight to the loop exit"
    );
}
