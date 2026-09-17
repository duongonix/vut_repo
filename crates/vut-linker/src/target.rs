//! Target classification for linker selection.
//!
//! The linker must not assume the host platform: the same backend code has to
//! map a target triple to a kernel, flavor, architecture, runtime artifact
//! names, and the matching system libraries.
use std::str::FromStr;

use target_lexicon::{Architecture, Environment, OperatingSystem, Triple};

use crate::error::{LinkError, LinkFailure};

/// Operating-system kernel family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kernel {
    Windows,
    Unix,
}

/// ABI/runtime flavor that determines the linker driver and system libraries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flavor {
    /// Windows MSVC ABI (`link.exe`, MSVC/UCRT).
    Msvc,
    /// Windows GNU ABI or a glibc Unix target (`cc`, GNU system libraries).
    Gnu,
    /// Linux musl (static libc).
    Musl,
    /// Apple Darwin (clang, libSystem).
    Darwin,
    /// Unknown flavor; only target-independent linking is possible.
    Unknown,
}

/// Target CPU architecture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Other,
}

/// Parsed view of a target triple used for linker decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetProfile {
    triple: String,
    kernel: Kernel,
    flavor: Flavor,
    arch: Arch,
}

impl TargetProfile {
    /// Parses a target triple into a linker profile.
    ///
    /// # Errors
    /// Returns [`LinkFailure::UnsupportedTarget`] when the triple is malformed.
    pub fn parse(triple: &str) -> Result<Self, LinkError> {
        let parsed = Triple::from_str(triple).map_err(|error| {
            LinkError::classified(
                format!("invalid target `{triple}`: {error}"),
                LinkFailure::UnsupportedTarget,
            )
        })?;
        let environment = parsed.environment;
        let kernel = match parsed.operating_system {
            OperatingSystem::Windows | OperatingSystem::Cygwin => Kernel::Windows,
            _ => Kernel::Unix,
        };
        let flavor = match parsed.operating_system {
            OperatingSystem::Windows => {
                if matches!(environment, Environment::Gnu | Environment::GnuLlvm) {
                    Flavor::Gnu
                } else {
                    Flavor::Msvc
                }
            }
            OperatingSystem::Darwin(_) | OperatingSystem::MacOSX(_) => Flavor::Darwin,
            OperatingSystem::Linux => {
                if matches!(
                    environment,
                    Environment::Musl
                        | Environment::Musleabi
                        | Environment::Musleabihf
                        | Environment::Muslabi64
                ) {
                    Flavor::Musl
                } else {
                    Flavor::Gnu
                }
            }
            _ => Flavor::Unknown,
        };
        let arch = match parsed.architecture {
            Architecture::X86_64 => Arch::X86_64,
            Architecture::Aarch64(_) => Arch::Aarch64,
            _ => Arch::Other,
        };
        Ok(Self {
            triple: triple.to_owned(),
            kernel,
            flavor,
            arch,
        })
    }

    #[must_use]
    pub fn triple(&self) -> &str {
        &self.triple
    }
    #[must_use]
    pub fn kernel(&self) -> Kernel {
        self.kernel
    }
    #[must_use]
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }
    #[must_use]
    pub fn arch(&self) -> Arch {
        self.arch
    }
    #[must_use]
    pub fn is_windows(&self) -> bool {
        self.kernel == Kernel::Windows
    }
    #[must_use]
    pub fn is_darwin(&self) -> bool {
        self.flavor == Flavor::Darwin
    }
    /// Distribution startup object name for this target.
    #[must_use]
    pub fn startup_object_name(&self) -> &'static str {
        if self.is_windows() {
            "vut-startup.obj"
        } else {
            "vut-startup.o"
        }
    }
    /// Distribution core runtime archive name for this target.
    #[must_use]
    pub fn core_library_name(&self) -> &'static str {
        if self.is_windows() {
            "vut-core.lib"
        } else {
            "libvut-core.a"
        }
    }
    /// Distribution native stdlib archive name for this target.
    #[must_use]
    pub fn stdlib_library_name(&self) -> &'static str {
        if self.is_windows() {
            "vut-stdlib.lib"
        } else {
            "libvut-stdlib.a"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_release_targets_to_profiles() {
        let cases = [
            (
                "x86_64-pc-windows-msvc",
                Kernel::Windows,
                Flavor::Msvc,
                Arch::X86_64,
            ),
            (
                "aarch64-pc-windows-msvc",
                Kernel::Windows,
                Flavor::Msvc,
                Arch::Aarch64,
            ),
            (
                "x86_64-unknown-linux-gnu",
                Kernel::Unix,
                Flavor::Gnu,
                Arch::X86_64,
            ),
            (
                "aarch64-unknown-linux-gnu",
                Kernel::Unix,
                Flavor::Gnu,
                Arch::Aarch64,
            ),
            (
                "x86_64-unknown-linux-musl",
                Kernel::Unix,
                Flavor::Musl,
                Arch::X86_64,
            ),
            (
                "x86_64-apple-darwin",
                Kernel::Unix,
                Flavor::Darwin,
                Arch::X86_64,
            ),
            (
                "aarch64-apple-darwin",
                Kernel::Unix,
                Flavor::Darwin,
                Arch::Aarch64,
            ),
        ];
        for (triple, kernel, flavor, arch) in cases {
            let profile = TargetProfile::parse(triple).expect("parse target");
            assert_eq!(profile.kernel(), kernel, "{triple}");
            assert_eq!(profile.flavor(), flavor, "{triple}");
            assert_eq!(profile.arch(), arch, "{triple}");
        }
    }

    #[test]
    fn artifact_names_follow_the_platform() {
        let windows = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(windows.startup_object_name(), "vut-startup.obj");
        assert_eq!(windows.core_library_name(), "vut-core.lib");
        assert_eq!(windows.stdlib_library_name(), "vut-stdlib.lib");

        let unix = TargetProfile::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(unix.startup_object_name(), "vut-startup.o");
        assert_eq!(unix.core_library_name(), "libvut-core.a");
        assert_eq!(unix.stdlib_library_name(), "libvut-stdlib.a");
    }

    #[test]
    fn rejects_malformed_triples() {
        let error = TargetProfile::parse("not a triple").expect_err("must fail");
        assert_eq!(error.failure(), LinkFailure::UnsupportedTarget);
    }
}
