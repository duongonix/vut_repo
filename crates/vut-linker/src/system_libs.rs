//! System libraries and frameworks required per target.
//!
//! The lists are derived from the M-LINK.0 measurements (the arguments rustc and
//! the platform linkers actually needed), not from guesswork. They cover the
//! Rust standard library plus the native stdlib stack (getrandom/ring/aws-lc,
//! tokio/mio) whose `#[link]` metadata is lost when archives are passed to a C
//! linker directly.
use crate::target::{Flavor, TargetProfile};

/// Platform system libraries needed by the runtime archives.
#[must_use]
pub fn system_libraries(profile: &TargetProfile) -> Vec<&'static str> {
    match profile.flavor() {
        Flavor::Msvc => vec![
            // CRT (Rust links the dynamic MSVC CRT by default).
            "msvcrt",
            "vcruntime",
            "ucrt",
            "oldnames",
            // Rust std base.
            "kernel32",
            "ntdll",
            "userenv",
            "ws2_32",
            "dbghelp",
            // getrandom: ring (advapi32) and aws-lc-rs (bcrypt).
            "bcrypt",
            "advapi32",
        ],
        Flavor::Gnu if profile.is_windows() => vec![
            "ws2_32", "userenv", "bcrypt", "advapi32", "ntdll", "kernel32",
        ],
        Flavor::Gnu => vec!["gcc_s", "util", "rt", "pthread", "m", "dl", "c"],
        Flavor::Musl => vec!["c"],
        Flavor::Darwin => vec!["System", "c", "m"],
        Flavor::Unknown => Vec::new(),
    }
}

/// Frameworks required on Apple platforms (getrandom uses the Security
/// framework; CoreFoundation accompanies it).
#[must_use]
pub fn frameworks(profile: &TargetProfile) -> Vec<&'static str> {
    if profile.is_darwin() {
        vec!["Security", "CoreFoundation"]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msvc_requires_crt_and_crypto_libraries() {
        let profile = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        let libraries = system_libraries(&profile);
        for required in [
            "msvcrt",
            "vcruntime",
            "ucrt",
            "kernel32",
            "ws2_32",
            "bcrypt",
            "advapi32",
        ] {
            assert!(libraries.contains(&required), "missing {required}");
        }
    }

    #[test]
    fn linux_gnu_requires_pthread_and_gcc_s() {
        let profile = TargetProfile::parse("x86_64-unknown-linux-gnu").unwrap();
        let libraries = system_libraries(&profile);
        assert!(libraries.contains(&"pthread"));
        assert!(libraries.contains(&"gcc_s"));
        assert!(libraries.contains(&"dl"));
    }

    #[test]
    fn darwin_adds_security_framework() {
        let profile = TargetProfile::parse("aarch64-apple-darwin").unwrap();
        assert!(system_libraries(&profile).contains(&"System"));
        assert_eq!(frameworks(&profile), vec!["Security", "CoreFoundation"]);
    }

    #[test]
    fn non_apple_targets_have_no_frameworks() {
        let profile = TargetProfile::parse("x86_64-unknown-linux-gnu").unwrap();
        assert!(frameworks(&profile).is_empty());
    }
}
