use crate::BoundsError;
use std::{collections::HashMap, hash::Hash, sync::Arc};
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VutList<T>(Arc<Vec<T>>);
impl<T: Clone> VutList<T> {
    pub fn new() -> Self {
        Self(Arc::new(Vec::new()))
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Arc::new(Vec::with_capacity(capacity)))
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
    /// Ensures the total capacity is at least `capacity`.
    pub fn reserve(&mut self, capacity: usize) {
        let values = Arc::make_mut(&mut self.0);
        if capacity > values.capacity() {
            values.reserve(capacity - values.len());
        }
    }
    pub fn push(&mut self, value: T) {
        Arc::make_mut(&mut self.0).push(value);
    }
    pub fn pop(&mut self) -> Option<T> {
        Arc::make_mut(&mut self.0).pop()
    }
    pub fn at(&self, index: usize) -> Option<&T> {
        self.0.get(index)
    }
    pub fn set(&mut self, index: usize, value: T) -> Result<(), BoundsError> {
        let len = self.0.len();
        let slot = Arc::make_mut(&mut self.0)
            .get_mut(index)
            .ok_or(BoundsError { index, len })?;
        *slot = value;
        Ok(())
    }
    pub fn insert(&mut self, index: usize, value: T) -> Result<(), BoundsError> {
        if index > self.0.len() {
            return Err(BoundsError {
                index,
                len: self.0.len(),
            });
        }
        Arc::make_mut(&mut self.0).insert(index, value);
        Ok(())
    }
    pub fn remove(&mut self, index: usize) -> Option<T> {
        (index < self.0.len()).then(|| Arc::make_mut(&mut self.0).remove(index))
    }
    pub fn clear(&mut self) {
        Arc::make_mut(&mut self.0).clear();
    }
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
    pub fn into_vec(self) -> Vec<T> {
        Arc::try_unwrap(self.0).unwrap_or_else(|shared| (*shared).clone())
    }
}
impl<T: Clone> VutList<T> {
    pub fn slice(&self, start: usize, end: usize) -> Result<Self, BoundsError> {
        if start > end || end > self.0.len() {
            return Err(BoundsError {
                index: end,
                len: self.0.len(),
            });
        }
        Ok(Self(Arc::new(self.0[start..end].to_vec())))
    }
}
impl<T: Clone + PartialEq> VutList<T> {
    pub fn contains(&self, value: &T) -> bool {
        self.0.contains(value)
    }
}
impl<T> From<Vec<T>> for VutList<T> {
    fn from(value: Vec<T>) -> Self {
        Self(Arc::new(value))
    }
}
#[derive(Clone, Debug, Default)]
pub struct VutMap<K, V>(Arc<HashMap<K, V>>);
impl<K: Clone + Eq + Hash, V: Clone> VutMap<K, V> {
    pub fn new() -> Self {
        Self(Arc::new(HashMap::new()))
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Arc::new(HashMap::with_capacity(capacity)))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }
    pub fn set(&mut self, key: K, value: V) -> Option<V> {
        Arc::make_mut(&mut self.0).insert(key, value)
    }
    pub fn remove(&mut self, key: &K) -> Option<V> {
        Arc::make_mut(&mut self.0).remove(key)
    }
    pub fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }
    pub fn clear(&mut self) {
        Arc::make_mut(&mut self.0).clear();
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    pub fn reserve(&mut self, additional: usize) {
        Arc::make_mut(&mut self.0).reserve(additional);
    }
    pub fn keys(&self) -> VutList<K> {
        VutList::from(self.0.keys().cloned().collect::<Vec<_>>())
    }
    pub fn values(&self) -> VutList<V> {
        VutList::from(self.0.values().cloned().collect::<Vec<_>>())
    }
}
impl<K, V> From<HashMap<K, V>> for VutMap<K, V> {
    fn from(value: HashMap<K, V>) -> Self {
        Self(Arc::new(value))
    }
}
