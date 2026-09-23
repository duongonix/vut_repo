//! Unit tests for the individual optimization passes and the verifier.
//!
//! Kept as one consolidated suite: every test shares the same MIR builder
//! helpers, and splitting them across files would duplicate that setup without
//! improving clarity.
use std::collections::{HashMap, HashSet};

use crate::{
    AbiClass, BasicBlock, BlockId, FieldLayout, Function, Instruction, LayoutTable, Local,
    LocalStorage, OwnershipKind, Program, Terminator, TypeInfo, ValueId, ValueRepr,
};
use vut_diagnostics::DiagnosticSink;
use vut_hir::TypeId;
use vut_resolver::SymbolId;
use vut_source::{SourceId, Span};

use super::{OptimizationLevel, optimize};

fn layouts(types: Vec<TypeInfo>) -> LayoutTable {
    LayoutTable {
        types,
        pointer_size: 8,
        fields: HashMap::new(),
        arrays: HashMap::new(),
        lists: HashMap::new(),
        maps: HashMap::new(),
        channels: HashMap::new(),
        results: HashMap::new(),
        enums: HashMap::new(),
        optionals: HashMap::new(),
        callables: HashMap::new(),
        aggregates: HashSet::new(),
        unsigned: HashSet::new(),
    }
}

fn scalar_type() -> TypeInfo {
    TypeInfo {
        size: 8,
        alignment: 8,
        is_copy: true,
        needs_drop: false,
        contains_managed: false,
        contains_linear: false,
        abi: AbiClass::Scalar,
        repr: ValueRepr::Integer,
        ownership: OwnershipKind::None,
    }
}

fn program_with(blocks: Vec<BasicBlock>, locals: Vec<Local>, table: LayoutTable) -> Program {
    Program {
        functions: vec![Function {
            symbol: SymbolId(0),
            receiver: None,
            parameter_count: 0,
            locals,
            blocks,
            entry: BlockId(0),
            return_type: None,
            is_async: false,
            frame_param: None,
            is_poll: false,
            out_param: None,
        }],
        external_functions: Vec::new(),
        layouts: table,
        interface_vtables: Vec::new(),
        diagnostics: DiagnosticSink::new(),
        frames: HashMap::new(),
        awaits: HashMap::new(),
        closure_layouts: HashMap::new(),
    }
}

fn block(instructions: Vec<Instruction>, terminator: Terminator) -> BasicBlock {
    BasicBlock {
        instructions,
        terminator,
    }
}

fn local() -> Local {
    Local {
        name: "x".to_owned(),
        ty: Some(TypeId(0)),
        span: Span::new(SourceId::from_index(0), 0, 0),
        storage: LocalStorage::Stack,
    }
}

fn instructions(program: &Program) -> &Vec<Instruction> {
    &program.functions[0].blocks[0].instructions
}

#[test]
fn dce_removes_dead_pure_but_keeps_observable() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 5,
                },
                Instruction::Binary {
                    operand_type: None,
                    value: ValueId(1),
                    op: crate::BinaryOp::Add,
                    left: ValueId(0),
                    right: ValueId(0),
                },
                Instruction::Spawn {
                    value: ValueId(2),
                    start: ValueId(0),
                    callable: SymbolId(9),
                    result_type: TypeId(0),
                },
            ],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let _ = optimize(&mut program, OptimizationLevel::O2);
    let remaining = instructions(&program);
    assert!(
        !remaining
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Binary { .. })),
        "the dead binary op must be removed"
    );
    assert!(
        remaining
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Spawn { .. })),
        "the observable spawn must be kept"
    );
}

#[test]
fn dce_keeps_division_even_when_unused() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 0,
                },
                Instruction::Binary {
                    operand_type: None,
                    value: ValueId(2),
                    op: crate::BinaryOp::Divide,
                    left: ValueId(0),
                    right: ValueId(1),
                },
            ],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let _ = optimize(&mut program, OptimizationLevel::O2);
    assert!(
        instructions(&program)
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Binary { .. })),
        "a potentially-trapping division must be kept"
    );
}

#[test]
fn const_prop_folds_across_blocks_and_removes_dead_branch() {
    let mut program = program_with(
        vec![
            block(
                vec![
                    Instruction::ConstInt {
                        value: ValueId(0),
                        literal: 2,
                    },
                    Instruction::ConstInt {
                        value: ValueId(1),
                        literal: 3,
                    },
                    Instruction::Binary {
                        operand_type: None,
                        value: ValueId(2),
                        op: crate::BinaryOp::Add,
                        left: ValueId(0),
                        right: ValueId(1),
                    },
                    Instruction::ConstBool {
                        value: ValueId(3),
                        literal: true,
                    },
                ],
                Terminator::Branch {
                    condition: ValueId(3),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            ),
            block(Vec::new(), Terminator::Return(None)),
            block(Vec::new(), Terminator::Return(None)),
        ],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.constants_folded >= 1);
    assert_eq!(program.functions[0].blocks.len(), 2, "dead branch removed");
    assert!(matches!(
        program.functions[0].blocks[0].terminator,
        Terminator::Jump(BlockId(1))
    ));
}

#[test]
fn rc_cancels_retain_release_pair() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::Retain {
                    value: ValueId(0),
                    ty: TypeId(0),
                },
                Instruction::Release {
                    value: ValueId(0),
                    ty: TypeId(0),
                },
            ],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = super::rc::run(&mut program);
    assert_eq!(report.retains_elided, 1);
    assert!(instructions(&program).is_empty());
}

#[test]
fn rc_keeps_pair_across_make_unique() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::Retain {
                    value: ValueId(0),
                    ty: TypeId(0),
                },
                Instruction::MakeUnique {
                    value: ValueId(1),
                    operand: ValueId(0),
                    ty: TypeId(0),
                },
                Instruction::Release {
                    value: ValueId(0),
                    ty: TypeId(0),
                },
            ],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = super::rc::run(&mut program);
    assert_eq!(report.retains_elided, 0, "MakeUnique observes the count");
}

#[test]
fn copy_prop_forwards_scalar_copy() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::Copy {
                    value: ValueId(0),
                    local: crate::LocalId(0),
                },
                Instruction::Copy {
                    value: ValueId(1),
                    local: crate::LocalId(0),
                },
            ],
            Terminator::Return(Some(ValueId(1))),
        )],
        vec![local()],
        layouts(vec![scalar_type()]),
    );
    let report = super::copy_prop::run(&mut program);
    assert_eq!(report.copies_propagated, 1);
    assert!(matches!(
        program.functions[0].blocks[0].terminator,
        Terminator::Return(Some(ValueId(0)))
    ));
}

#[test]
fn bounds_marks_provable_in_bounds() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 3,
                },
                Instruction::VariadicAt {
                    value: ValueId(2),
                    data: ValueId(0),
                    len: ValueId(1),
                    index: ValueId(0),
                    element: TypeId(0),
                    in_bounds: false,
                },
            ],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = super::bounds::run(&mut program);
    assert_eq!(report.bounds_checks_elided, 1);
    let marked = instructions(&program).iter().any(|instruction| {
        matches!(
            instruction,
            Instruction::VariadicAt {
                in_bounds: true,
                ..
            }
        )
    });
    assert!(marked);
}

#[test]
fn async_opt_dedups_reloads() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ReloadValue {
                    value: ValueId(0),
                    slot: 0,
                },
                Instruction::ReloadValue {
                    value: ValueId(1),
                    slot: 0,
                },
            ],
            Terminator::Return(Some(ValueId(1))),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = super::async_opt::run(&mut program);
    assert_eq!(report.async_reloads_elided, 1);
    assert!(matches!(
        program.functions[0].blocks[0].terminator,
        Terminator::Return(Some(ValueId(0)))
    ));
}

#[test]
fn verifier_rejects_undefined_operand() {
    let program = program_with(
        vec![block(
            vec![Instruction::Binary {
                operand_type: None,
                value: ValueId(0),
                op: crate::BinaryOp::Add,
                left: ValueId(5),
                right: ValueId(6),
            }],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    assert!(super::verify::verify(&program).is_err());
}

#[test]
fn verifier_accepts_valid_program() {
    let program = program_with(
        vec![block(
            vec![Instruction::ConstInt {
                value: ValueId(0),
                literal: 1,
            }],
            Terminator::Return(Some(ValueId(0))),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    assert!(super::verify::verify(&program).is_ok());
}

#[test]
fn verifier_rejects_use_after_move() {
    let locals = vec![local()];
    let program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 2,
                },
                Instruction::Store {
                    local: crate::LocalId(0),
                    value: ValueId(0),
                },
                Instruction::Move {
                    value: ValueId(1),
                    local: crate::LocalId(0),
                },
                Instruction::Drop(crate::LocalId(0)),
            ],
            Terminator::Return(None),
        )],
        locals,
        layouts(vec![scalar_type()]),
    );
    assert!(super::verify::verify(&program).is_err());
}

#[test]
fn algebra_simplifies_add_zero() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::Allocate {
                    value: ValueId(0),
                    ty: TypeId(0),
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 0,
                },
                Instruction::Binary {
                    operand_type: Some(TypeId(0)),
                    value: ValueId(2),
                    op: crate::BinaryOp::Add,
                    left: ValueId(0),
                    right: ValueId(1),
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.constants_propagated >= 1);
    assert!(matches!(
        program.functions[0].blocks[0].terminator,
        Terminator::Return(Some(ValueId(0)))
    ));
}

#[test]
fn dead_store_elimination_removes_an_overwritten_store() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 2,
                },
                Instruction::Store {
                    local: crate::LocalId(0),
                    value: ValueId(0),
                },
                Instruction::Store {
                    local: crate::LocalId(0),
                    value: ValueId(1),
                },
                Instruction::Copy {
                    value: ValueId(2),
                    local: crate::LocalId(0),
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        vec![local()],
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.dead_stores_removed >= 1);
}

#[test]
fn dead_store_elimination_keeps_a_store_that_is_read() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 2,
                },
                Instruction::Store {
                    local: crate::LocalId(0),
                    value: ValueId(0),
                },
                Instruction::Borrow {
                    value: ValueId(2),
                    local: crate::LocalId(0),
                },
                Instruction::Store {
                    local: crate::LocalId(0),
                    value: ValueId(1),
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        vec![local()],
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert_eq!(report.dead_stores_removed, 0);
}

#[test]
fn cse_reuses_a_repeated_length_read() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstNull {
                    value: ValueId(0),
                    ty: None,
                },
                Instruction::RuntimeCall {
                    value: Some(ValueId(1)),
                    result_type: None,
                    function: crate::BuiltinFunction::ListLen,
                    arguments: vec![ValueId(0)],
                },
                Instruction::RuntimeCall {
                    value: Some(ValueId(2)),
                    result_type: None,
                    function: crate::BuiltinFunction::ListLen,
                    arguments: vec![ValueId(0)],
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.cse_eliminated >= 1);
}

#[test]
fn optimization_is_deterministic_and_never_reverts_valid_mir() {
    let build = || {
        program_with(
            vec![block(
                vec![
                    Instruction::ConstInt {
                        value: ValueId(0),
                        literal: 2,
                    },
                    Instruction::ConstInt {
                        value: ValueId(1),
                        literal: 3,
                    },
                    Instruction::Binary {
                        operand_type: Some(TypeId(0)),
                        value: ValueId(2),
                        op: crate::BinaryOp::Add,
                        left: ValueId(0),
                        right: ValueId(1),
                    },
                ],
                Terminator::Return(Some(ValueId(2))),
            )],
            Vec::new(),
            layouts(vec![scalar_type()]),
        )
    };
    let mut first = build();
    let mut second = build();
    let report_first = optimize(&mut first, OptimizationLevel::O2);
    let report_second = optimize(&mut second, OptimizationLevel::O2);
    assert_eq!(report_first.reverted_phases, 0);
    assert_eq!(report_second.reverted_phases, 0);
    assert!(report_first.constants_folded >= 1);
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

fn copy_data_layouts() -> LayoutTable {
    let data = TypeInfo {
        size: 8,
        alignment: 8,
        is_copy: true,
        needs_drop: false,
        contains_managed: false,
        contains_linear: false,
        abi: AbiClass::Aggregate,
        repr: ValueRepr::Pointer,
        ownership: OwnershipKind::None,
    };
    let mut table = layouts(vec![data, scalar_type()]);
    table.fields.insert(
        TypeId(0),
        vec![FieldLayout {
            name: "a".into(),
            ty: TypeId(1),
            offset: 0,
        }],
    );
    table
}

fn function_full(
    symbol: usize,
    parameter_count: usize,
    locals: Vec<Local>,
    blocks: Vec<BasicBlock>,
    return_type: Option<TypeId>,
) -> Function {
    Function {
        symbol: SymbolId(symbol),
        receiver: None,
        parameter_count,
        locals,
        blocks,
        entry: BlockId(0),
        return_type,
        is_async: false,
        frame_param: None,
        is_poll: false,
        out_param: None,
    }
}

fn program_of(functions: Vec<Function>, table: LayoutTable) -> Program {
    Program {
        functions,
        external_functions: Vec::new(),
        layouts: table,
        interface_vtables: Vec::new(),
        diagnostics: DiagnosticSink::new(),
        frames: HashMap::new(),
        awaits: HashMap::new(),
        closure_layouts: HashMap::new(),
    }
}

#[test]
fn loop_invariant_arithmetic_is_hoisted() {
    let mut program = program_with(
        vec![
            block(
                vec![
                    Instruction::Allocate {
                        value: ValueId(0),
                        ty: TypeId(0),
                    },
                    Instruction::ConstInt {
                        value: ValueId(1),
                        literal: 0,
                    },
                ],
                Terminator::Jump(BlockId(1)),
            ),
            block(
                vec![Instruction::Binary {
                    operand_type: Some(TypeId(0)),
                    value: ValueId(2),
                    op: crate::BinaryOp::Less,
                    left: ValueId(0),
                    right: ValueId(1),
                }],
                Terminator::Branch {
                    condition: ValueId(2),
                    then_block: BlockId(2),
                    else_block: BlockId(3),
                },
            ),
            block(
                vec![
                    Instruction::Binary {
                        operand_type: Some(TypeId(0)),
                        value: ValueId(3),
                        op: crate::BinaryOp::Add,
                        left: ValueId(0),
                        right: ValueId(0),
                    },
                    Instruction::Store {
                        local: crate::LocalId(0),
                        value: ValueId(3),
                    },
                ],
                Terminator::Jump(BlockId(1)),
            ),
            block(Vec::new(), Terminator::Return(None)),
        ],
        vec![local()],
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.instructions_hoisted >= 1);
}

#[test]
fn sroa_replaces_non_escaping_aggregate_fields() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 5,
                },
                Instruction::Construct {
                    value: ValueId(1),
                    ty: TypeId(0),
                    fields: vec![("a".into(), ValueId(0))],
                },
                Instruction::Field {
                    value: ValueId(2),
                    base: ValueId(1),
                    name: "a".into(),
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Vec::new(),
        copy_data_layouts(),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.aggregates_scalarized >= 1);
    assert!(matches!(
        program.functions[0].blocks[0].terminator,
        Terminator::Return(Some(ValueId(0)))
    ));
}

#[test]
fn rc_cancels_pair_across_an_unconditional_jump() {
    let mut program = program_with(
        vec![
            block(
                vec![
                    Instruction::ConstNull {
                        value: ValueId(0),
                        ty: None,
                    },
                    Instruction::Retain {
                        value: ValueId(0),
                        ty: TypeId(0),
                    },
                ],
                Terminator::Jump(BlockId(1)),
            ),
            block(
                vec![Instruction::Release {
                    value: ValueId(0),
                    ty: TypeId(0),
                }],
                Terminator::Return(None),
            ),
        ],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.retains_elided >= 1);
    assert!(report.releases_elided >= 1);
}

#[test]
fn constant_string_concatenation_is_folded() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstString {
                    value: ValueId(0),
                    literal: "x".into(),
                },
                Instruction::ConstString {
                    value: ValueId(1),
                    literal: "y".into(),
                },
                Instruction::ConcatString {
                    value: ValueId(2),
                    left: ValueId(0),
                    right: ValueId(1),
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.strings_folded >= 1);
    assert!(matches!(
        program.functions[0].blocks[0].instructions[0],
        Instruction::ConstString { ref literal, .. } if literal == "xy"
    ));
}

#[test]
fn tiny_pure_function_is_inlined() {
    let caller = function_full(
        0,
        0,
        Vec::new(),
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 2,
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 3,
                },
                Instruction::Call {
                    value: Some(ValueId(2)),
                    result_type: Some(TypeId(0)),
                    target: SymbolId(1),
                    arguments: vec![ValueId(0), ValueId(1)],
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Some(TypeId(0)),
    );
    let helper = function_full(
        1,
        2,
        vec![local(), local()],
        vec![block(
            vec![
                Instruction::Copy {
                    value: ValueId(0),
                    local: crate::LocalId(0),
                },
                Instruction::Copy {
                    value: ValueId(1),
                    local: crate::LocalId(1),
                },
                Instruction::Binary {
                    operand_type: Some(TypeId(0)),
                    value: ValueId(2),
                    op: crate::BinaryOp::Add,
                    left: ValueId(0),
                    right: ValueId(1),
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Some(TypeId(0)),
    );
    let mut program = program_of(vec![caller, helper], layouts(vec![scalar_type()]));
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.call_sites_inlined >= 1);
    assert!(
        !program.functions[0].blocks[0]
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Call { .. }))
    );
}

#[test]
fn indirect_call_to_known_function_is_devirtualized() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::MakeFunction {
                    value: ValueId(0),
                    symbol: SymbolId(1),
                },
                Instruction::ConstInt {
                    value: ValueId(1),
                    literal: 0,
                },
                Instruction::CallIndirect {
                    value: Some(ValueId(2)),
                    result_type: Some(TypeId(0)),
                    callable_ty: None,
                    callee: ValueId(0),
                    arguments: vec![ValueId(1)],
                },
            ],
            Terminator::Return(Some(ValueId(2))),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.call_indirect_devirtualized >= 1);
    assert!(program.functions[0].blocks[0]
        .instructions
        .iter()
        .any(|instruction| matches!(instruction, Instruction::Call { target, .. } if *target == SymbolId(1))));
}

#[test]
fn overwritten_frame_state_store_is_removed() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::SetFrameState {
                    frame: crate::LocalId(0),
                    state: 1,
                },
                Instruction::SetFrameState {
                    frame: crate::LocalId(0),
                    state: 2,
                },
            ],
            Terminator::Return(None),
        )],
        vec![local()],
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.async_frame_states_elided >= 1);
}

#[test]
fn dead_scalar_spill_is_removed() {
    let mut program = program_with(
        vec![block(
            vec![
                Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                },
                Instruction::SpillValue {
                    slot: 0,
                    value: ValueId(0),
                },
            ],
            Terminator::Return(None),
        )],
        Vec::new(),
        layouts(vec![scalar_type()]),
    );
    let report = optimize(&mut program, OptimizationLevel::O2);
    assert!(report.async_spills_elided >= 1);
}
