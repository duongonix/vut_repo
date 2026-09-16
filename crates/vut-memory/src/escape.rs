use std::collections::HashSet;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscapeClass {
    Local,
    Returned,
    PassedToUnknown,
    Stored,
}
#[derive(Clone, Debug, Default)]
pub struct EscapeFacts {
    escaping: HashSet<usize>,
}
impl EscapeFacts {
    pub fn mark(&mut self, local: usize) {
        self.escaping.insert(local);
    }
    #[must_use]
    pub fn class(&self, local: usize) -> EscapeClass {
        if self.escaping.contains(&local) {
            EscapeClass::Stored
        } else {
            EscapeClass::Local
        }
    }
}
