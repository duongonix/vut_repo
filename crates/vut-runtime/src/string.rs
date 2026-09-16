use std::{fmt, sync::Arc};
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VutString(Arc<str>);
impl VutString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Arc::from(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn byte_len(&self) -> usize {
        self.0.len()
    }
    pub fn char_len(&self) -> usize {
        self.0.chars().count()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn contains(&self, pattern: &str) -> bool {
        self.0.contains(pattern)
    }
    pub fn starts_with(&self, pattern: &str) -> bool {
        self.0.starts_with(pattern)
    }
    pub fn ends_with(&self, pattern: &str) -> bool {
        self.0.ends_with(pattern)
    }
    #[must_use]
    pub fn trim(&self) -> Self {
        Self::from(self.0.trim())
    }
    #[must_use]
    pub fn trim_start(&self) -> Self {
        Self::from(self.0.trim_start())
    }
    #[must_use]
    pub fn trim_end(&self) -> Self {
        Self::from(self.0.trim_end())
    }
    #[must_use]
    pub fn to_lower(&self) -> Self {
        Self::from(self.0.to_lowercase())
    }
    #[must_use]
    pub fn to_upper(&self) -> Self {
        Self::from(self.0.to_uppercase())
    }
    #[must_use]
    pub fn replace(&self, from: &str, to: &str) -> Self {
        Self::from(self.0.replace(from, to))
    }
    pub fn split(&self, separator: &str) -> crate::VutList<Self> {
        crate::VutList::from(self.0.split(separator).map(Self::from).collect::<Vec<_>>())
    }
    pub fn lines(&self) -> crate::VutList<Self> {
        crate::VutList::from(self.0.lines().map(Self::from).collect::<Vec<_>>())
    }
    pub fn to_int(&self) -> Result<i64, std::num::ParseIntError> {
        self.0.parse()
    }
    pub fn to_float(&self) -> Result<f64, std::num::ParseFloatError> {
        self.0.parse()
    }
    pub fn to_bytes(&self) -> VutBytes {
        VutBytes(Arc::new(self.0.as_bytes().to_vec()))
    }
    #[must_use]
    pub fn concat(&self, other: &Self) -> Self {
        Self(Arc::from(format!("{}{other}", self.0)))
    }
}
impl fmt::Display for VutString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl From<String> for VutString {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}
impl From<&str> for VutString {
    fn from(value: &str) -> Self {
        Self(Arc::from(value))
    }
}
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VutBytes(Arc<Vec<u8>>);
impl VutBytes {
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(Arc::new(value.into()))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    pub fn reserve(&mut self, capacity: usize) {
        let value = Arc::make_mut(&mut self.0);
        if capacity > value.capacity() {
            value.reserve(capacity - value.len());
        }
    }
    pub fn at(&self, index: usize) -> Option<u8> {
        self.0.get(index).copied()
    }
    pub fn first(&self) -> Option<u8> {
        self.at(0)
    }
    pub fn last(&self) -> Option<u8> {
        self.0.last().copied()
    }
    pub fn slice(&self, start: usize, end: usize) -> Option<Self> {
        (start <= end && end <= self.0.len()).then(|| Self::new(self.0[start..end].to_vec()))
    }
    pub fn clear(&mut self) {
        Arc::make_mut(&mut self.0).clear();
    }
    pub fn to_list(&self) -> crate::VutList<u8> {
        crate::VutList::from(self.0.as_ref().clone())
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
    pub fn push(&mut self, value: u8) {
        Arc::make_mut(&mut self.0).push(value);
    }
    pub fn pop(&mut self) -> Option<u8> {
        Arc::make_mut(&mut self.0).pop()
    }
    pub fn set(&mut self, index: usize, value: u8) -> bool {
        let Some(slot) = Arc::make_mut(&mut self.0).get_mut(index) else {
            return false;
        };
        *slot = value;
        true
    }
    pub fn to_string(&self) -> Result<VutString, std::str::Utf8Error> {
        std::str::from_utf8(&self.0).map(VutString::from)
    }
}
impl From<Vec<u8>> for VutBytes {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}
