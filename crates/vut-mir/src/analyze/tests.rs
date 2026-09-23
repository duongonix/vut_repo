//! Unit tests for the reusable analyses.

use super::{Cfg, DefUse, DominatorTree, Liveness, LoopInfo};
use crate::{BasicBlock, BlockId, Function, Instruction, Terminator, ValueId};
use vut_resolver::SymbolId;

fn function(blocks: Vec<BasicBlock>) -> Function {
    Function {
        symbol: SymbolId(0),
        receiver: None,
        parameter_count: 0,
        locals: Vec::new(),
        blocks,
        entry: BlockId(0),
        return_type: None,
        is_async: false,
        frame_param: None,
        is_poll: false,
        out_param: None,
    }
}

fn block(instructions: Vec<Instruction>, terminator: Terminator) -> BasicBlock {
    BasicBlock {
        instructions,
        terminator,
    }
}

fn jump(target: usize) -> Terminator {
    Terminator::Jump(BlockId(target))
}

fn branch(condition: usize, then_block: usize, else_block: usize) -> Terminator {
    Terminator::Branch {
        condition: ValueId(condition),
        then_block: BlockId(then_block),
        else_block: BlockId(else_block),
    }
}

fn bool_const(value: usize) -> Instruction {
    Instruction::ConstBool {
        value: ValueId(value),
        literal: true,
    }
}

fn diamond() -> Function {
    function(vec![
        block(Vec::new(), jump(1)),
        block(vec![bool_const(0)], branch(0, 2, 3)),
        block(Vec::new(), jump(4)),
        block(Vec::new(), jump(4)),
        block(Vec::new(), Terminator::Return(None)),
    ])
}

#[test]
fn cfg_exposes_edges_and_reachability() {
    let function = diamond();
    let cfg = Cfg::build(&function);
    assert_eq!(cfg.successors(BlockId(0)), [BlockId(1)]);
    assert_eq!(cfg.predecessors(BlockId(4)), [BlockId(2), BlockId(3)]);
    let rpo = cfg.reverse_postorder(BlockId(0));
    assert_eq!(rpo.first(), Some(&BlockId(0)));
    assert_eq!(rpo.len(), 5);
    assert!(cfg.reachable(BlockId(0)).iter().all(|reachable| *reachable));
}

#[test]
fn dominators_identify_the_merge_point() {
    let function = diamond();
    let cfg = Cfg::build(&function);
    let dominators = DominatorTree::build(&cfg, function.entry);
    assert!(dominators.dominates(BlockId(0), BlockId(4)));
    assert!(dominators.dominates(BlockId(1), BlockId(4)));
    assert!(!dominators.dominates(BlockId(2), BlockId(4)));
    assert!(!dominators.dominates(BlockId(3), BlockId(4)));
    assert_eq!(dominators.immediate_dominator(BlockId(4)), Some(BlockId(1)));
    assert_eq!(dominators.immediate_dominator(BlockId(1)), Some(BlockId(0)));
}

#[test]
fn def_use_records_definitions_and_uses() {
    let function = function(vec![
        block(
            vec![Instruction::ConstInt {
                value: ValueId(0),
                literal: 7,
            }],
            jump(1),
        ),
        block(Vec::new(), Terminator::Return(Some(ValueId(0)))),
    ]);
    let def_use = DefUse::build(&function);
    assert_eq!(def_use.definition(ValueId(0)), Some((BlockId(0), 0)));
    assert_eq!(def_use.uses(ValueId(0)), [(BlockId(1), usize::MAX)]);
    assert!(def_use.is_defined(ValueId(0)));
}

#[test]
fn liveness_propagates_values_across_blocks() {
    let function = function(vec![
        block(
            vec![Instruction::ConstInt {
                value: ValueId(0),
                literal: 7,
            }],
            jump(1),
        ),
        block(Vec::new(), Terminator::Return(Some(ValueId(0)))),
    ]);
    let cfg = Cfg::build(&function);
    let liveness = Liveness::analyze(&cfg, &function);
    assert!(liveness.live_out(BlockId(0)).contains(&ValueId(0)));
    assert!(liveness.live_in(BlockId(1)).contains(&ValueId(0)));
}

#[test]
fn loops_find_the_back_edge_and_body() {
    let function = function(vec![
        block(Vec::new(), jump(1)),
        block(vec![bool_const(0)], branch(0, 2, 3)),
        block(Vec::new(), jump(1)),
        block(Vec::new(), Terminator::Return(None)),
    ]);
    let cfg = Cfg::build(&function);
    let dominators = DominatorTree::build(&cfg, function.entry);
    let loops = LoopInfo::analyze(&cfg, &dominators);
    assert_eq!(loops.back_edges(), [(BlockId(2), BlockId(1))]);
    let body = loops.body(BlockId(1)).expect("loop body");
    assert!(body.contains(&BlockId(1)));
    assert!(body.contains(&BlockId(2)));
    assert!(!body.contains(&BlockId(3)));
    assert_eq!(loops.headers(), vec![BlockId(1)]);
}

#[test]
fn acyclic_functions_have_no_loops() {
    let function = diamond();
    let cfg = Cfg::build(&function);
    let dominators = DominatorTree::build(&cfg, function.entry);
    assert!(LoopInfo::analyze(&cfg, &dominators).is_empty());
}
