//! Natural loops derived from the CFG and dominator tree.

use std::collections::{HashMap, HashSet};

use super::cfg::Cfg;
use super::dominators::DominatorTree;
use crate::BlockId;

#[derive(Clone, Debug, Default)]
pub struct LoopInfo {
    back_edges: Vec<(BlockId, BlockId)>,
    bodies: HashMap<BlockId, HashSet<BlockId>>,
}

impl LoopInfo {
    #[must_use]
    pub fn analyze(cfg: &Cfg, dominators: &DominatorTree) -> Self {
        let mut back_edges = Vec::new();
        for block in 0..cfg.block_count() {
            let latch = BlockId(block);
            for &successor in cfg.successors(latch) {
                if dominators.dominates(successor, latch) {
                    back_edges.push((latch, successor));
                }
            }
        }
        let mut bodies: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        for &(latch, header) in &back_edges {
            let body = bodies.entry(header).or_default();
            body.insert(header);
            let mut stack = vec![latch];
            while let Some(node) = stack.pop() {
                if node == header || !body.insert(node) {
                    continue;
                }
                for &predecessor in cfg.predecessors(node) {
                    stack.push(predecessor);
                }
            }
        }
        Self { back_edges, bodies }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.back_edges.is_empty()
    }

    #[must_use]
    pub fn back_edges(&self) -> &[(BlockId, BlockId)] {
        &self.back_edges
    }

    /// Loop headers in deterministic (block index) order.
    #[must_use]
    pub fn headers(&self) -> Vec<BlockId> {
        let mut headers: Vec<BlockId> = self.bodies.keys().copied().collect();
        headers.sort_by_key(|block| block.0);
        headers
    }

    #[must_use]
    pub fn body(&self, header: BlockId) -> Option<&HashSet<BlockId>> {
        self.bodies.get(&header)
    }

    #[must_use]
    pub fn contains(&self, header: BlockId, block: BlockId) -> bool {
        self.bodies
            .get(&header)
            .is_some_and(|body| body.contains(&block))
    }
}
