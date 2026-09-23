//! Machine-readable VPM format compatibility policy.
use std::fmt;

pub const CURRENT_LOCKFILE_VERSION: u32 = 2;
pub const MINIMUM_LOCKFILE_VERSION: u32 = 1;
pub const PACKAGE_LAYOUT_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityError {
    pub component: &'static str,
    pub found: u32,
    pub minimum: u32,
    pub maximum: u32,
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "incompatible {} version {}; supported range is {}..={}",
            self.component, self.found, self.minimum, self.maximum
        )
    }
}
impl std::error::Error for CompatibilityError {}

pub fn require_lockfile(version: u32) -> Result<(), CompatibilityError> {
    if (MINIMUM_LOCKFILE_VERSION..=CURRENT_LOCKFILE_VERSION).contains(&version) {
        Ok(())
    } else {
        Err(CompatibilityError {
            component: "lockfile",
            found: version,
            minimum: MINIMUM_LOCKFILE_VERSION,
            maximum: CURRENT_LOCKFILE_VERSION,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatibility_range_is_explicit_and_forward_safe() {
        assert!(require_lockfile(1).is_ok());
        assert!(require_lockfile(2).is_ok());
        assert!(require_lockfile(0).is_err());
        assert!(require_lockfile(3).is_err());
    }
}
