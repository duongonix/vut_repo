//! Expression inlining for tiny pure scalar functions.
//!
//! A function that has a single block, uses only its parameters (no extra
//! locals), performs only pure instructions, and takes and returns copyable
//! scalars is inlined at every direct call site. The callee body is cloned with
//! fresh values, parameter reads are bound to the arguments, and the call's
//! result is aliased to the cloned return value. Because the callee is pure and
//! scalar, inlining cannot change ownership, retention, or side effects.
use std::collections::HashMap;

use super::{OptimizationReport, effects};
use crate::analyze::CallGraph;
use crate::{Function, Instruction, LayoutTable, Program, SymbolId, Terminator, ValueId};

/// Maximum callee instruction count eligible for inlining.
const INLINE_INSTRUCTION_LIMIT: usize = 24;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    let callgraph = CallGraph::build(program);
    let inlinable = collect_inlinable(program);
    if inlinable.is_empty() {
        return report;
    }
    let Program { functions, .. } = program;
    for caller in functions.iter_mut() {
        report.call_sites_inlined += inline_calls(caller, &inlinable, &callgraph);
    }
    report
}

struct InlineInfo {
    parameter_count: usize,
    instructions: Vec<Instruction>,
    return_value: Option<ValueId>,
}

fn collect_inlinable(program: &Program) -> HashMap<SymbolId, InlineInfo> {
    let mut inlinable = HashMap::new();
    for function in &program.functions {
        if function.blocks.len() != 1
            || function.locals.len() != function.parameter_count
            || function.is_async
        {
            continue;
        }
        let block = &function.blocks[0];
        if block.instructions.len() > INLINE_INSTRUCTION_LIMIT
            || block.instructions.iter().any(effects::is_observable)
        {
            continue;
        }
        if !function
            .locals
            .iter()
            .all(|local| is_scalar(local.ty, &program.layouts))
            || !is_scalar(function.return_type, &program.layouts)
        {
            continue;
        }
        let Terminator::Return(return_value) = &block.terminator else {
            continue;
        };
        inlinable.insert(
            function.symbol,
            InlineInfo {
                parameter_count: function.parameter_count,
                instructions: block.instructions.clone(),
                return_value: *return_value,
            },
        );
    }
    inlinable
}

fn is_scalar(ty: Option<vut_hir::TypeId>, layouts: &LayoutTable) -> bool {
    ty.is_none_or(|ty| {
        let info = layouts.types[ty.0];
        info.is_copy && !info.needs_drop && !info.contains_managed
    })
}

fn inline_calls(
    caller: &mut Function,
    inlinable: &HashMap<SymbolId, InlineInfo>,
    callgraph: &CallGraph,
) -> usize {
    let _ = callgraph;
    let mut inlined = 0;
    let mut next_value = max_value(caller).map_or(0, |value| value + 1);
    let mut aliases: Vec<(ValueId, ValueId)> = Vec::new();
    for block in &mut caller.blocks {
        let mut rewritten: Vec<Instruction> = Vec::with_capacity(block.instructions.len());
        for instruction in &block.instructions {
            let Instruction::Call {
                value,
                target,
                arguments,
                ..
            } = instruction
            else {
                rewritten.push(instruction.clone());
                continue;
            };
            let Some(info) = inlinable.get(target) else {
                rewritten.push(instruction.clone());
                continue;
            };
            if *target == caller.symbol || arguments.len() != info.parameter_count {
                rewritten.push(instruction.clone());
                continue;
            }
            let (cloned, return_value) = clone_body(info, arguments, &mut next_value);
            rewritten.extend(cloned);
            if let (Some(result), Some(return_value)) = (value, return_value) {
                aliases.push((*result, return_value));
            }
            inlined += 1;
        }
        block.instructions = rewritten;
    }
    for (from, to) in aliases {
        effects::rewrite_uses(caller, from, to);
    }
    inlined
}

fn clone_body(
    info: &InlineInfo,
    arguments: &[ValueId],
    next_value: &mut usize,
) -> (Vec<Instruction>, Option<ValueId>) {
    let mut rename: HashMap<ValueId, ValueId> = HashMap::new();
    let mut cloned = Vec::with_capacity(info.instructions.len());
    for instruction in &info.instructions {
        // A parameter read becomes the corresponding argument.
        if let Some(local) = read_local(instruction)
            && local < info.parameter_count
            && let Some(argument) = arguments.get(local)
        {
            let mut defined = Vec::new();
            effects::defined_values(instruction, &mut defined);
            for value in defined {
                rename.insert(value, *argument);
            }
            continue;
        }
        let mut instruction = instruction.clone();
        clone_instruction(&mut instruction, &mut rename, next_value);
        cloned.push(instruction);
    }
    let return_value = info
        .return_value
        .map(|value| rename.get(&value).copied().unwrap_or(value));
    (cloned, return_value)
}

fn clone_instruction(
    instruction: &mut Instruction,
    rename: &mut HashMap<ValueId, ValueId>,
    next_value: &mut usize,
) {
    // Operands first, using the mapping built so far.
    let mut operands = Vec::new();
    effects::operands(instruction, &mut operands);
    if !operands.is_empty() {
        effects::for_each_operand_mut(instruction, &mut |value| {
            if let Some(mapped) = rename.get(value) {
                *value = *mapped;
            }
        });
    }
    // Then assign fresh ids to the values it defines.
    let mut defined = Vec::new();
    effects::defined_values(instruction, &mut defined);
    for value in defined {
        let fresh = ValueId(*next_value);
        *next_value += 1;
        replace_defined(instruction, value, fresh);
        rename.insert(value, fresh);
    }
}

/// The local read by a `Copy`/`Move`/`Borrow`.
///
/// A borrowed receiver (`self.field`) is read with `Borrow`, not `Copy`/`Move`.
/// Treating it as an ordinary instruction would leave the cloned `Borrow`
/// pointing at the *caller's* local with the same index, so it must be bound to
/// the call argument like any other parameter read.
fn read_local(instruction: &Instruction) -> Option<usize> {
    match instruction {
        Instruction::Copy { local, .. }
        | Instruction::Move { local, .. }
        | Instruction::Borrow { local, .. } => Some(local.0),
        _ => None,
    }
}

fn replace_defined(instruction: &mut Instruction, from: ValueId, to: ValueId) {
    match instruction {
        Instruction::ConstNull { value, .. }
        | Instruction::ConstInt { value, .. }
        | Instruction::ConstFloat { value, .. }
        | Instruction::ConstBool { value, .. }
        | Instruction::ConstString { value, .. }
        | Instruction::FormatValue { value, .. }
        | Instruction::ConcatString { value, .. }
        | Instruction::Copy { value, .. }
        | Instruction::Borrow { value, .. }
        | Instruction::Move { value, .. }
        | Instruction::Allocate { value, .. }
        | Instruction::LoadRaw { value, .. }
        | Instruction::MakeUnique { value, .. }
        | Instruction::Binary { value, .. }
        | Instruction::Unary { value, .. }
        | Instruction::ConstructEnum { value, .. }
        | Instruction::EnumPayload { value, .. }
        | Instruction::TypeRetain { value, .. }
        | Instruction::TypeRelease { value, .. }
        | Instruction::ConstructResult { value, .. }
        | Instruction::ResultPayload { value, .. }
        | Instruction::Spawn { value, .. }
        | Instruction::FrameState { value, .. }
        | Instruction::BorrowField { value, .. }
        | Instruction::ResourceDeref { value, .. }
        | Instruction::AwaitFuture { value, .. }
        | Instruction::MakeFunction { value, .. }
        | Instruction::MakeClosure { value, .. }
        | Instruction::Field { value, .. }
        | Instruction::CopyAggregate { value, .. }
        | Instruction::Construct { value, .. }
        | Instruction::ConstructArray { value, .. }
        | Instruction::ConstructVariadicBuffer { value, .. }
        | Instruction::VariadicAt { value, .. }
        | Instruction::ConstructInterface { value, .. }
        | Instruction::OptionalWrap { value, .. }
        | Instruction::OptionalUnwrap { value, .. }
        | Instruction::OptionalIsPresent { value, .. }
        | Instruction::OptionalFromValue { value, .. } => {
            if *value == from {
                *value = to;
            }
        }
        Instruction::EnumTag { result, .. } | Instruction::ResultState { result, .. }
            if *result == from =>
        {
            *result = to;
        }
        // A pure scalar builtin (see `effects::is_pure_scalar_builtin`) may be
        // cloned into the caller; its optional result slot must be remapped.
        Instruction::RuntimeCall { value, .. } if *value == Some(from) => {
            *value = Some(to);
        }
        _ => {}
    }
}

fn max_value(function: &Function) -> Option<usize> {
    let mut max = None;
    for block in &function.blocks {
        for instruction in &block.instructions {
            let mut defined = Vec::new();
            effects::defined_values(instruction, &mut defined);
            for value in defined {
                max = Some(max.map_or(value.0, |current: usize| current.max(value.0)));
            }
        }
    }
    max
}
