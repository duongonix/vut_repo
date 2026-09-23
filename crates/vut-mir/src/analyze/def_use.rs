//! Definition and use sites for SSA values.

use std::collections::HashMap;

use crate::optimize::effects;
use crate::{BlockId, Function, ValueId};

/// Where a value is defined, and everywhere it is read.
#[derive(Clone, Debug, Default)]
pub struct DefUse {
    definitions: HashMap<ValueId, (BlockId, usize)>,
    uses: HashMap<ValueId, Vec<(BlockId, usize)>>,
}

impl DefUse {
    #[must_use]
    pub fn build(function: &Function) -> Self {
        let mut definitions = HashMap::new();
        let mut uses: HashMap<ValueId, Vec<(BlockId, usize)>> = HashMap::new();
        for (block_index, block) in function.blocks.iter().enumerate() {
            let block_id = BlockId(block_index);
            for (index, instruction) in block.instructions.iter().enumerate() {
                let mut defined = Vec::new();
                effects::defined_values(instruction, &mut defined);
                for value in defined {
                    definitions.insert(value, (block_id, index));
                }
                let mut operands = Vec::new();
                effects::operands(instruction, &mut operands);
                for value in operands {
                    uses.entry(value).or_default().push((block_id, index));
                }
            }
            let mut terminator = Vec::new();
            effects::terminator_uses(&block.terminator, &mut terminator);
            for value in terminator {
                uses.entry(value).or_default().push((block_id, usize::MAX));
            }
        }
        Self { definitions, uses }
    }

    #[must_use]
    pub fn definition(&self, value: ValueId) -> Option<(BlockId, usize)> {
        self.definitions.get(&value).copied()
    }

    #[must_use]
    pub fn uses(&self, value: ValueId) -> &[(BlockId, usize)] {
        self.uses.get(&value).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn is_defined(&self, value: ValueId) -> bool {
        self.definitions.contains_key(&value)
    }
}
