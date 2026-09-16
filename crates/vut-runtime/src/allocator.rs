#[derive(Debug)]
pub struct HeapBlock(Vec<u8>);
impl HeapBlock {
    pub fn allocate(size: usize) -> Self {
        Self(vec![0; size])
    }
    pub fn realloc(&mut self, size: usize) {
        self.0.resize(size, 0);
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}
