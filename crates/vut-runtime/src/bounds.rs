use std::fmt;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundsError {
    pub index: usize,
    pub len: usize,
}
impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "index {} is outside collection length {}",
            self.index, self.len
        )
    }
}
impl std::error::Error for BoundsError {}
pub fn check_bounds(index: usize, len: usize) -> Result<(), BoundsError> {
    if index < len {
        Ok(())
    } else {
        Err(BoundsError { index, len })
    }
}
