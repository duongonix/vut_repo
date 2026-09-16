use std::collections::HashMap;
#[derive(Clone, Debug, Default)]
pub struct LastUse {
    remaining: HashMap<String, usize>,
}
impl LastUse {
    #[must_use]
    pub fn from_counts(remaining: HashMap<String, usize>) -> Self {
        Self { remaining }
    }
    pub fn consume(&mut self, name: &str) -> bool {
        let remaining = self.remaining.entry(name.to_owned()).or_default();
        *remaining = remaining.saturating_sub(1);
        *remaining == 0
    }
    #[must_use]
    pub fn remaining(&self, name: &str) -> usize {
        self.remaining.get(name).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifies_last_consumption() {
        let mut uses = LastUse::from_counts(HashMap::from([("value".into(), 2)]));
        assert!(!uses.consume("value"));
        assert!(uses.consume("value"));
    }
}
