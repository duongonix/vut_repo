//! Target CPU / feature selection for Cranelift.
//!
//! The CPU target is independent of the optimization level: the default is the
//! portable baseline, and native/preset features are applied only when the user
//! opts in explicitly. This keeps release binaries portable unless asked
//! otherwise.

use super::CodegenError;
use cranelift_codegen::{isa, settings::Configurable as _};

/// Applies `cpu` and explicit `features` to the ISA builder.
pub(super) fn configure(
    builder: &mut isa::Builder,
    triple: &str,
    cpu: Option<&str>,
    features: &[String],
) -> Result<(), CodegenError> {
    if let Some(cpu) = cpu {
        for (flag, enabled) in cpu_flags(triple, cpu)? {
            set(builder, flag, enabled)?;
        }
    }
    for feature in features {
        let (flag, enabled) = parse_feature(feature)?;
        set(builder, &flag, enabled)?;
    }
    Ok(())
}

fn set(builder: &mut isa::Builder, flag: &str, enabled: bool) -> Result<(), CodegenError> {
    builder
        .set(flag, if enabled { "true" } else { "false" })
        .map_err(|error| CodegenError::Backend(format!("unknown target feature `{flag}`: {error}")))
}

fn parse_feature(feature: &str) -> Result<(String, bool), CodegenError> {
    let (enabled, name) = match feature.strip_prefix('+') {
        Some(name) => (true, name),
        None => match feature.strip_prefix('-') {
            Some(name) => (false, name),
            None => (true, feature),
        },
    };
    if name.is_empty() {
        return Err(CodegenError::Backend(format!(
            "invalid target feature `{feature}`"
        )));
    }
    let flag = if name.starts_with("has_") {
        name.to_owned()
    } else {
        format!("has_{name}")
    };
    Ok((flag, enabled))
}

fn cpu_flags(triple: &str, cpu: &str) -> Result<Vec<(&'static str, bool)>, CodegenError> {
    match cpu {
        "native" => native_flags(triple),
        "x86-64-v2" => Ok(preset(&[
            "has_sse3",
            "has_ssse3",
            "has_sse41",
            "has_sse42",
            "has_popcnt",
            "has_cmpxchg16b",
        ])),
        "x86-64-v3" => Ok(preset(&[
            "has_sse3",
            "has_ssse3",
            "has_sse41",
            "has_sse42",
            "has_popcnt",
            "has_cmpxchg16b",
            "has_avx",
            "has_avx2",
            "has_bmi1",
            "has_bmi2",
            "has_fma",
            "has_lzcnt",
        ])),
        "x86-64-v4" => Ok(preset(&[
            "has_sse3",
            "has_ssse3",
            "has_sse41",
            "has_sse42",
            "has_popcnt",
            "has_cmpxchg16b",
            "has_avx",
            "has_avx2",
            "has_bmi1",
            "has_bmi2",
            "has_fma",
            "has_lzcnt",
            "has_avx512f",
            "has_avx512vl",
            "has_avx512dq",
        ])),
        other => Err(CodegenError::InvalidTarget(format!(
            "unknown target CPU `{other}`; use `native` or a known preset"
        ))),
    }
}

fn preset(flags: &[&'static str]) -> Vec<(&'static str, bool)> {
    flags.iter().map(|flag| (*flag, true)).collect()
}

fn native_flags(triple: &str) -> Result<Vec<(&'static str, bool)>, CodegenError> {
    if triple != target_lexicon::HOST.to_string() {
        return Err(CodegenError::InvalidTarget(
            "`--target-cpu native` requires the host target".into(),
        ));
    }
    #[cfg(target_arch = "x86_64")]
    {
        Ok(vec![
            ("has_sse3", std::arch::is_x86_feature_detected!("sse3")),
            ("has_ssse3", std::arch::is_x86_feature_detected!("ssse3")),
            ("has_sse41", std::arch::is_x86_feature_detected!("sse4.1")),
            ("has_sse42", std::arch::is_x86_feature_detected!("sse4.2")),
            ("has_avx", std::arch::is_x86_feature_detected!("avx")),
            ("has_avx2", std::arch::is_x86_feature_detected!("avx2")),
            ("has_fma", std::arch::is_x86_feature_detected!("fma")),
            ("has_bmi1", std::arch::is_x86_feature_detected!("bmi1")),
            ("has_bmi2", std::arch::is_x86_feature_detected!("bmi2")),
            ("has_popcnt", std::arch::is_x86_feature_detected!("popcnt")),
            ("has_lzcnt", std::arch::is_x86_feature_detected!("lzcnt")),
            (
                "has_cmpxchg16b",
                std::arch::is_x86_feature_detected!("cmpxchg16b"),
            ),
        ])
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        Err(CodegenError::InvalidTarget(
            "`--target-cpu native` is not implemented for this architecture".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_signed_features() {
        assert_eq!(parse_feature("+avx2").unwrap(), ("has_avx2".into(), true));
        assert_eq!(parse_feature("-avx2").unwrap(), ("has_avx2".into(), false));
        assert_eq!(parse_feature("has_avx").unwrap(), ("has_avx".into(), true));
        assert!(parse_feature("+").is_err());
    }

    #[test]
    fn presets_and_unknown_cpus() {
        assert!(cpu_flags("x86_64-pc-windows-msvc", "x86-64-v3").is_ok());
        assert!(cpu_flags("x86_64-pc-windows-msvc", "bogus").is_err());
    }
}
