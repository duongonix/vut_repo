//! Backward liveness of SSA values over the CFG.
//!
//! Used by ADCE and by passes that need to know whether a value is read after a
//! point (release sinking, dead-store elimination).

use std::collections::HashSet;

use super::cfg::Cfg;
use crate::optimize::effects;
use crate::{BlockId, Function, ValueId};

#[derive(Clone, Debug, Default)]
pub struct Liveness {
    live_in: Vec<HashSet<ValueId>>,
    live_out: Vec<HashSet<ValueId>>,
}

impl Liveness {
    #[must_use]
    pub fn analyze(cfg: &Cfg, function: &Function) -> Self {
        let count = cfg.block_count();
        let mut use_before_def: Vec<HashSet<ValueId>> = vec![HashSet::new(); count];
        let mut defs: Vec<HashSet<ValueId>> = vec![HashSet::new(); count];
        for (block_index, block) in function.blocks.iter().enumerate() {
            for instruction in &block.instructions {
                let mut operands = Vec::new();
                effects::operands(instruction, &mut operands);
                for operand in operands {
                    if !defs[block_index].contains(&operand) {
                        use_before_def[block_index].insert(operand);
                    }
                }
                let mut defined = Vec::new();
                effects::defined_values(instruction, &mut defined);
                defs[block_index].extend(defined);
            }
            let mut terminator = Vec::new();
            effects::terminator_uses(&block.terminator, &mut terminator);
            for value in terminator {
                if !defs[block_index].contains(&value) {
                    use_before_def[block_index].insert(value);
                }
            }
        }

        let mut live_in = use_before_def.clone();
        let mut live_out = vec![HashSet::new(); count];
        let mut changed = true;
        while changed {
            changed = false;
            for block in 0..count {
                let mut out: HashSet<ValueId> = HashSet::new();
                for &successor in cfg.successors(BlockId(block)) {
                    out.extend(live_in[successor.0].iter().copied());
                }
                let mut next_in = use_before_def[block].clone();
                for value in &out {
                    if !defs[block].contains(value) {
                        next_in.insert(*value);
                    }
                }
                if next_in != live_in[block] || out != live_out[block] {
                    live_in[block] = next_in;
                    live_out[block] = out;
                    changed = true;
                }
            }
        }
        Self { live_in, live_out }
    }

    #[must_use]
    pub fn live_in(&self, block: BlockId) -> &HashSet<ValueId> {
        match self.live_in.get(block.0) {
            Some(set) => set,
            None => empty(),
        }
    }

    #[must_use]
    pub fn live_out(&self, block: BlockId) -> &HashSet<ValueId> {
        match self.live_out.get(block.0) {
            Some(set) => set,
            None => empty(),
        }
    }
}

fn empty() -> &'static HashSet<ValueId> {
    static EMPTY: std::sync::OnceLock<HashSet<ValueId>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}
