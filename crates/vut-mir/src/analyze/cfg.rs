//! Control-flow graph for one function.
//!
//! Every optimizer pass that needs predecessors, successors, reachability, or a
//! reverse postorder uses this instead of re-deriving them.

use crate::{BlockId, Function, Terminator};

#[derive(Clone, Debug)]
pub struct Cfg {
    successors: Vec<Vec<BlockId>>,
    predecessors: Vec<Vec<BlockId>>,
}

impl Cfg {
    #[must_use]
    pub fn build(function: &Function) -> Self {
        let count = function.blocks.len();
        let mut successors = vec![Vec::new(); count];
        let mut predecessors = vec![Vec::new(); count];
        for (index, block) in function.blocks.iter().enumerate() {
            for successor in terminator_successors(&block.terminator) {
                if successor.0 < count {
                    successors[index].push(successor);
                    predecessors[successor.0].push(BlockId(index));
                }
            }
        }
        Self {
            successors,
            predecessors,
        }
    }

    #[must_use]
    pub fn block_count(&self) -> usize {
        self.successors.len()
    }

    #[must_use]
    pub fn successors(&self, block: BlockId) -> &[BlockId] {
        self.successors.get(block.0).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn predecessors(&self, block: BlockId) -> &[BlockId] {
        self.predecessors.get(block.0).map_or(&[], Vec::as_slice)
    }

    /// Reverse postorder from `entry`; blocks unreachable from `entry` are
    /// omitted. Deterministic (successors are visited in index order).
    #[must_use]
    pub fn reverse_postorder(&self, entry: BlockId) -> Vec<BlockId> {
        if entry.0 >= self.block_count() {
            return Vec::new();
        }
        let mut visited = vec![false; self.block_count()];
        let mut postorder = Vec::new();
        visited[entry.0] = true;
        let mut stack = vec![(entry, 0_usize)];
        while let Some((block, next)) = stack.pop() {
            let successors = self.successors(block);
            if next < successors.len() {
                let child = successors[next];
                stack.push((block, next + 1));
                if !visited[child.0] {
                    visited[child.0] = true;
                    stack.push((child, 0));
                }
            } else {
                postorder.push(block);
            }
        }
        postorder.reverse();
        postorder
    }

    #[must_use]
    pub fn reachable(&self, entry: BlockId) -> Vec<bool> {
        let mut reachable = vec![false; self.block_count()];
        if entry.0 >= self.block_count() {
            return reachable;
        }
        reachable[entry.0] = true;
        let mut stack = vec![entry];
        while let Some(block) = stack.pop() {
            for &successor in self.successors(block) {
                if !reachable[successor.0] {
                    reachable[successor.0] = true;
                    stack.push(successor);
                }
            }
        }
        reachable
    }
}

/// Successor blocks of a terminator.
#[must_use]
pub fn terminator_successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Jump(target) => vec![*target],
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![*then_block, *else_block],
        Terminator::Return(_) | Terminator::PollReturn(_) | Terminator::Unreachable => Vec::new(),
    }
}
