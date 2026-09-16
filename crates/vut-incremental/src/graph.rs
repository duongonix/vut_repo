use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Default)]
pub struct DependencyGraph {
    reverse: BTreeMap<String, BTreeSet<String>>,
}
impl DependencyGraph {
    pub fn add_dependency(&mut self, module: impl Into<String>, dependency: impl Into<String>) {
        self.reverse
            .entry(dependency.into())
            .or_default()
            .insert(module.into());
    }
    #[must_use]
    pub fn invalidate_from(&self, changed: &str) -> BTreeSet<String> {
        let mut invalid = BTreeSet::from([changed.to_owned()]);
        let mut queue = VecDeque::from([changed.to_owned()]);
        while let Some(item) = queue.pop_front() {
            if let Some(dependents) = self.reverse.get(&item) {
                for dependent in dependents {
                    if invalid.insert(dependent.clone()) {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
        invalid
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalidates_transitive_reverse_dependents() {
        let mut graph = DependencyGraph::default();
        graph.add_dependency("b", "d");
        graph.add_dependency("a", "b");
        assert_eq!(
            graph.invalidate_from("d"),
            BTreeSet::from(["a".into(), "b".into(), "d".into()])
        );
    }
}
