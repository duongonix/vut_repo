//! Converts an async body into a suspendable poll state machine.
//!
//! This covers B2.6b: bodies whose `await`s can all suspend are rewritten into
//! `(frame, out) -> i32`, resuming from a state word stored in the frame. Every
//! block is split at its await sites, so straight-line bodies, `if`/`else`,
//! `match`, and `for` loops (conditional and iterable) all convert. Values that
//! live across a suspension are spilled into the frame beforehand (see
//! [`super::spill`]) and iterator state is frame-resident, so no value or
//! iterator crosses the boundary. Vutcon tasks and native futures are both poll
//! tasks, so `await` of either suspends the same way.
use vut_ast::BinaryOp;
use vut_hir::TypeId;

use super::super::{
    BasicBlock, BlockId, Function, Instruction, Local, LocalId, LocalStorage, Terminator, ValueId,
};
use super::{AwaitLive, FrameLayout};

/// One await site in the function.
struct AwaitSite {
    block: usize,
    index: usize,
    result: ValueId,
    handle: ValueId,
    result_type: TypeId,
}

/// The pieces of an async body needed to build a state machine.
struct Plan {
    frame: LocalId,
    sites: Vec<AwaitSite>,
}

/// Rewrites `function` into a poll body when its awaits do not cross a
/// suspension with an SSA value.
///
/// Returns `true` when the body was converted.
pub(super) fn make_poll_body(
    function: &mut Function,
    awaits: &[AwaitLive],
    frame_ty: TypeId,
    spilled: bool,
) -> bool {
    let Some(plan) = plan(function, awaits, spilled) else {
        return false;
    };
    let next = super::values::next_value_id(function);
    let blocks = assemble(function, &plan, next);
    let span = first_span(function);
    let out_index = function.locals.len();
    function.locals.push(Local {
        name: "$out".to_string(),
        ty: Some(frame_ty),
        span,
        storage: LocalStorage::Stack,
    });
    debug_assert_eq!(function.entry, BlockId(0), "the dispatch entry owns id 0");
    function.blocks = blocks;
    function.is_poll = true;
    function.out_param = Some(out_index);
    function.parameter_count = 0;
    true
}

/// Collects the await sites when the body can be turned into a state machine.
fn plan(function: &Function, awaits: &[AwaitLive], spilled: bool) -> Option<Plan> {
    let frame = LocalId(function.frame_param?);
    // Every await (Vutcon task or native future) is a suspension site.
    let mut sites = Vec::new();
    for (block_index, block) in function.blocks.iter().enumerate() {
        for (index, instruction) in block.instructions.iter().enumerate() {
            if let Instruction::AwaitFuture {
                value,
                handle,
                result_type,
            } = instruction
            {
                sites.push(AwaitSite {
                    block: block_index,
                    index,
                    result: *value,
                    handle: *handle,
                    result_type: *result_type,
                });
            }
        }
    }
    if sites.is_empty() || sites.len() != awaits.len() {
        return None;
    }
    if awaits.iter().any(|site| site.crossing_values != 0) && !spilled {
        return None;
    }
    Some(Plan { frame, sites })
}

/// Block-id layout of the generated state machine.
///
/// Block `0` becomes the dispatch entry; the original entry block's leading
/// segment is relocated to `start`. Other original blocks keep their ids.
struct Layout {
    base: usize,
    count: usize,
    start: usize,
}

impl Layout {
    fn poll(&self, k: usize) -> BlockId {
        BlockId(self.base + 1 + 2 * k)
    }

    fn suspend(&self, k: usize) -> BlockId {
        BlockId(self.base + 2 + 2 * k)
    }

    fn cont(&self, k: usize) -> BlockId {
        BlockId(self.base + 1 + 2 * self.count + k)
    }

    fn dispatch(&self, j: usize) -> BlockId {
        BlockId(self.base + 1 + 3 * self.count + j)
    }
}

/// Builds the state-machine block list, preserving the original block ids for
/// their leading (pre-await) segments.
fn assemble(function: &Function, plan: &Plan, next: usize) -> Vec<BasicBlock> {
    let layout = Layout {
        base: function.blocks.len(),
        count: plan.sites.len(),
        start: function.blocks.len(),
    };
    let total = layout.base + 4 * layout.count;

    let mut by_block: Vec<Vec<usize>> = vec![Vec::new(); layout.base];
    for (k, site) in plan.sites.iter().enumerate() {
        by_block[site.block].push(k);
    }

    let mut out: Vec<Option<BasicBlock>> = (0..total).map(|_| None).collect();
    for (index, block) in function.blocks.iter().enumerate() {
        // The original entry block cannot stay at id 0 (the dispatch entry owns
        // it), so its leading segment is moved to `start`.
        let destination = if index == 0 { layout.start } else { index };
        split_block(
            destination,
            block,
            &by_block[index],
            plan,
            &layout,
            &mut out,
        );
    }

    let mut values = next;
    push_await_blocks(&mut out, plan, &layout, &mut values);
    let state = ValueId(values);
    values += 1;
    let zero = ValueId(values);
    values += 1;
    let not_started = ValueId(values);
    values += 1;
    push_dispatch_blocks(&mut out, &layout, state, &mut values);

    let resume = if layout.count == 1 {
        layout.poll(0)
    } else {
        layout.dispatch(0)
    };
    out[0] = Some(BasicBlock {
        instructions: vec![
            Instruction::FrameState {
                value: state,
                frame: plan.frame,
            },
            Instruction::ConstInt {
                value: zero,
                literal: 0,
            },
            Instruction::Binary {
                value: not_started,
                op: BinaryOp::NotEqual,
                left: state,
                right: zero,
            },
        ],
        terminator: Terminator::Branch {
            condition: not_started,
            then_block: resume,
            else_block: BlockId(layout.start),
        },
    });

    let blocks = out
        .into_iter()
        .map(|block| block.expect("every state-machine block is built"))
        .collect::<Vec<_>>();
    renumber(&blocks, 0)
}

/// Builds the per-await poll and suspend blocks.
fn push_await_blocks(
    out: &mut [Option<BasicBlock>],
    plan: &Plan,
    layout: &Layout,
    values: &mut usize,
) {
    for k in 0..layout.count {
        let site = &plan.sites[k];
        let ready = ValueId(*values);
        *values += 1;
        out[layout.poll(k).0] = Some(BasicBlock {
            instructions: vec![Instruction::PollFuture {
                value: Some(site.result),
                ready,
                frame: plan.frame,
                result_type: site.result_type,
            }],
            terminator: Terminator::Branch {
                condition: ready,
                then_block: layout.cont(k),
                else_block: layout.suspend(k),
            },
        });
        out[layout.suspend(k).0] = Some(BasicBlock {
            instructions: vec![Instruction::SetFrameState {
                frame: plan.frame,
                state: u32::try_from(k + 1).unwrap_or(u32::MAX),
            }],
            terminator: Terminator::PollReturn(0),
        });
    }
}

/// Builds the state-dispatch chain that resumes at the recorded await.
fn push_dispatch_blocks(
    out: &mut [Option<BasicBlock>],
    layout: &Layout,
    state: ValueId,
    values: &mut usize,
) {
    for j in 0..layout.count.saturating_sub(1) {
        let literal = ValueId(*values);
        *values += 1;
        let matches = ValueId(*values);
        *values += 1;
        let else_block = if j + 2 == layout.count {
            layout.poll(layout.count - 1)
        } else {
            layout.dispatch(j + 1)
        };
        out[layout.dispatch(j).0] = Some(BasicBlock {
            instructions: vec![
                Instruction::ConstInt {
                    value: literal,
                    literal: i64::try_from(j + 1).unwrap_or(i64::MAX),
                },
                Instruction::Binary {
                    value: matches,
                    op: BinaryOp::Equal,
                    left: state,
                    right: literal,
                },
            ],
            terminator: Terminator::Branch {
                condition: matches,
                then_block: layout.poll(j),
                else_block,
            },
        });
    }
}

/// Reorders blocks into reverse postorder so a block that defines an SSA value
/// is compiled before the blocks that use it. Terminators are remapped.
fn renumber(blocks: &[BasicBlock], entry: usize) -> Vec<BasicBlock> {
    let mut order = Vec::new();
    let mut visited = vec![false; blocks.len()];
    let mut stack = vec![(entry, 0_usize)];
    visited[entry] = true;
    while let Some((node, next)) = stack.pop() {
        let successors = successors(&blocks[node].terminator);
        if next < successors.len() {
            stack.push((node, next + 1));
            let target = successors[next];
            if !visited[target] {
                visited[target] = true;
                stack.push((target, 0));
            }
        } else {
            order.push(node);
        }
    }
    order.reverse();
    let mut index = vec![0_usize; blocks.len()];
    for (position, &old) in order.iter().enumerate() {
        index[old] = position;
    }
    let mut result = Vec::with_capacity(blocks.len());
    for &old in &order {
        let mut block = blocks[old].clone();
        block.terminator = remap_ids(block.terminator.clone(), &index);
        result.push(block);
    }
    for (old, block) in blocks.iter().enumerate() {
        if !visited[old] {
            let mut block = block.clone();
            block.terminator = remap_ids(block.terminator.clone(), &index);
            result.push(block);
        }
    }
    result
}

fn successors(terminator: &Terminator) -> Vec<usize> {
    match terminator {
        Terminator::Jump(target) => vec![target.0],
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![then_block.0, else_block.0],
        Terminator::Return(_) | Terminator::PollReturn(_) | Terminator::Unreachable => Vec::new(),
    }
}

fn remap_ids(terminator: Terminator, index: &[usize]) -> Terminator {
    match terminator {
        Terminator::Jump(target) => Terminator::Jump(BlockId(index[target.0])),
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => Terminator::Branch {
            condition,
            then_block: BlockId(index[then_block.0]),
            else_block: BlockId(index[else_block.0]),
        },
        other => other,
    }
}

/// Splits `block` at its await sites: the leading segment goes to `destination`,
/// each later segment becomes the continuation of the previous await.
fn split_block(
    destination: usize,
    block: &BasicBlock,
    awaits: &[usize],
    plan: &Plan,
    layout: &Layout,
    out: &mut [Option<BasicBlock>],
) {
    if awaits.is_empty() {
        out[destination] = Some(BasicBlock {
            instructions: block.instructions.clone(),
            terminator: remap(block.terminator.clone(), layout),
        });
        return;
    }
    let mut start = 0;
    for (segment, &k) in awaits.iter().enumerate() {
        let mut instructions = block.instructions[start..plan.sites[k].index].to_vec();
        instructions.push(Instruction::SetFrameChild {
            frame: plan.frame,
            value: plan.sites[k].handle,
        });
        let id = if segment == 0 {
            destination
        } else {
            layout.cont(awaits[segment - 1]).0
        };
        out[id] = Some(BasicBlock {
            instructions,
            terminator: Terminator::Jump(layout.poll(k)),
        });
        start = plan.sites[k].index + 1;
    }
    out[layout.cont(*awaits.last().expect("non-empty awaits")).0] = Some(BasicBlock {
        instructions: block.instructions[start..].to_vec(),
        terminator: remap(block.terminator.clone(), layout),
    });
}

/// Redirects references to the original entry block to the relocated `start`.
fn remap(terminator: Terminator, layout: &Layout) -> Terminator {
    let start = BlockId(layout.start);
    match terminator {
        Terminator::Jump(BlockId(0)) => Terminator::Jump(start),
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => Terminator::Branch {
            condition,
            then_block: if then_block.0 == 0 { start } else { then_block },
            else_block: if else_block.0 == 0 { start } else { else_block },
        },
        other => other,
    }
}

fn first_span(function: &Function) -> vut_source::Span {
    function.locals.first().map_or_else(
        || vut_source::Span::new(vut_source::SourceId::from_index(0), 0, 0),
        |local| local.span,
    )
}

/// Rewrites an async body's persisted locals to frame storage.
pub(super) fn apply_layout(function: &mut Function, layout: &FrameLayout) {
    for (&local, slot) in &layout.locals {
        if let Some(entry) = function.locals.get_mut(local) {
            entry.storage = LocalStorage::Frame(slot.offset);
        }
    }
}
