//! Register opaque pointees before resolving any cross-module resource fields.
use super::Analyzer;
use vut_ast::Item;

impl Analyzer<'_> {
    pub(super) fn collect_opaque_pointees(&mut self) {
        for module in &self.resolution.modules {
            for item in &module.file.items {
                if let Item::Data(data) = item
                    && data.opaque
                    && let Some(symbol) = module.symbols.get(&data.name.text)
                {
                    self.opaque_data.insert(*symbol);
                }
            }
        }
    }
}
