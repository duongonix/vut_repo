//! Dominator tree (Cooper–Harvey–Kennedy, iterative).
//!
//! Shared by the verifier and every pass that needs dominance facts (bounds
//! elimination, LICM, RC sinking, SROA).

use super::cfg::Cfg;
use crate::BlockId;

#[derive(Clone, Debug)]
pub struct DominatorTree {
    idom: Vec<Option<BlockId>>,
    entry: Option<BlockId>,
}

impl DominatorTree {
    #[must_use]
    pub fn build(cfg: &Cfg, entry: BlockId) -> Self {
        let count = cfg.block_count();
        let order = reverse_postorder_order(cfg, entry);
        let mut idom = vec![None; count];
        if entry.0 < count {
            idom[entry.0] = Some(entry);
        }
        let rpo = cfg.reverse_postorder(entry);
        let mut changed = true;
        while changed {
            changed = false;
            for &block in &rpo {
                if block == entry {
                    continue;
                }
                let mut new_idom: Option<BlockId> = None;
                for &predecessor in cfg.predecessors(block) {
                    if idom[predecessor.0].is_none() {
                        continue;
                    }
                    new_idom = Some(match new_idom {
                        None => predecessor,
                        Some(current) => intersect(current, predecessor, &idom, &order),
                    });
                }
                if let Some(new_idom) = new_idom
                    && idom[block.0] != Some(new_idom)
                {
                    idom[block.0] = Some(new_idom);
                    changed = true;
                }
            }
        }
        Self {
            idom,
            entry: Some(entry),
        }
    }

    #[must_use]
    pub fn entry(&self) -> Option<BlockId> {
        self.entry
    }

    #[must_use]
    pub fn immediate_dominator(&self, block: BlockId) -> Option<BlockId> {
        self.idom
            .get(block.0)
            .copied()
            .flatten()
            .filter(|parent| *parent != block)
    }

    /// `true` when `a` dominates `b` (or they are the same block).
    #[must_use]
    pub fn dominates(&self, a: BlockId, b: BlockId) -> bool {
        if a == b {
            return true;
        }
        if self.idom.get(a.0).copied().flatten().is_none() {
            return false;
        }
        let mut current = b;
        for _ in 0..=self.idom.len() {
            let Some(parent) = self.idom.get(current.0).copied().flatten() else {
                return false;
            };
            if parent == a {
                return true;
            }
            if parent == current {
                return false;
            }
            current = parent;
        }
        false
    }
}

fn reverse_postorder_order(cfg: &Cfg, entry: BlockId) -> Vec<usize> {
    let mut order = vec![usize::MAX; cfg.block_count()];
    for (index, block) in cfg.reverse_postorder(entry).iter().enumerate() {
        order[block.0] = index;
    }
    order
}

fn intersect(mut a: BlockId, mut b: BlockId, idom: &[Option<BlockId>], order: &[usize]) -> BlockId {
    while a != b {
        while order.get(a.0).copied().unwrap_or(usize::MAX)
            > order.get(b.0).copied().unwrap_or(usize::MAX)
        {
            let Some(next) = idom.get(a.0).copied().flatten() else {
                return a;
            };
            if next == a {
                return a;
            }
            a = next;
        }
        while order.get(b.0).copied().unwrap_or(usize::MAX)
            > order.get(a.0).copied().unwrap_or(usize::MAX)
        {
            let Some(next) = idom.get(b.0).copied().flatten() else {
                return b;
            };
            if next == b {
                return b;
            }
            b = next;
        }
    }
    a
}
