//! Release-level manifest (`releases.json`).
//!
//! One `releases.json` describes every supported target for a release. The
//! installers resolve `OS + arch + libc (+ version)` through this file instead
//! of constructing asset URLs themselves (see `specs/deploy/installer.md`).
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use vut_linker::{Arch, Flavor, TargetProfile};

use crate::checksum;
use crate::manifest::Manifest;

/// Release manifest schema version.
pub const RELEASE_SCHEMA: u32 = 1;

/// Inputs that are release-wide rather than per-target.
pub struct Inputs<'a> {
    pub channel: &'a str,
    pub profile: &'a str,
    pub cranelift: Option<&'a str>,
    pub released_at: Option<&'a str>,
    pub repo: &'a str,
    pub url_base: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReleaseManifest {
    pub schema: u32,
    pub channel: String,
    pub version: String,
    pub released_at: String,
    pub compiler: CompilerInfo,
    pub runtime_abi: u32,
    pub stdlib_version: String,
    pub manifest_format: u32,
    pub targets: Vec<TargetEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompilerInfo {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cranelift: Option<String>,
    pub profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetEntry {
    pub triple: String,
    pub os: String,
    pub arch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libc: Option<String>,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub archive: String,
    pub abi_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_os: Option<String>,
    pub sdk: SdkHints,
}

/// Which platform SDK/toolchain a target needs for the final link.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SdkHints {
    pub windows_build_tools: bool,
    pub xcode_clt: bool,
    pub cc: bool,
}

impl ReleaseManifest {
    /// Renders the manifest as pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("release manifest always serializes")
    }
}

/// Builds a release manifest from `<dist>.manifest.json` files in `dir`.
///
/// # Errors
/// Returns an error when no per-target manifests are found, an archive is
/// missing, a manifest is unreadable, or the output cannot be written.
pub fn generate(
    dir: &Path,
    out: &Path,
    inputs: &Inputs<'_>,
) -> Result<ReleaseManifest, Box<dyn Error>> {
    let mut manifests = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(dist) = name.strip_suffix(".manifest.json") else {
            continue;
        };
        let text = std::fs::read_to_string(entry.path())?;
        let manifest: Manifest = serde_json::from_str(&text)?;
        manifests.push((dist.to_owned(), manifest));
    }
    if manifests.is_empty() {
        return Err(format!("no `*.manifest.json` files found in `{}`", dir.display()).into());
    }
    manifests.sort_by(|left, right| left.1.target.cmp(&right.1.target));

    let first = &manifests[0].1;
    let mut targets = Vec::with_capacity(manifests.len());
    for (dist, manifest) in &manifests {
        let archive = archive_for(dir, dist)?;
        let archive_name = archive
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("archive name is not valid UTF-8")?
            .to_owned();
        let size = std::fs::metadata(&archive)?.len();
        let sha256 = checksum::sha256_file(&archive)?;
        let profile = TargetProfile::parse(&manifest.target)?;
        let (os, arch, libc, sdk) = classify(&profile);
        let base = inputs.url_base.map_or_else(
            || {
                format!(
                    "https://github.com/{}/releases/download/v{}",
                    inputs.repo, manifest.vut
                )
            },
            ToOwned::to_owned,
        );
        targets.push(TargetEntry {
            triple: manifest.target.clone(),
            os,
            arch,
            libc,
            url: format!("{}/{}", base.trim_end_matches('/'), archive_name),
            sha256,
            size,
            archive: archive_name,
            abi_version: manifest.abi_version,
            minimum_os: manifest.minimum_os.clone(),
            sdk,
        });
    }

    let manifest = ReleaseManifest {
        schema: RELEASE_SCHEMA,
        channel: inputs.channel.to_owned(),
        version: first.vut.clone(),
        released_at: inputs
            .released_at
            .map_or_else(now_rfc3339, ToOwned::to_owned),
        compiler: CompilerInfo {
            version: first.vut.clone(),
            cranelift: inputs.cranelift.map(ToOwned::to_owned),
            profile: inputs.profile.to_owned(),
        },
        runtime_abi: first.abi_version,
        stdlib_version: first.stdlib.clone(),
        manifest_format: first.format_version,
        targets,
    };
    std::fs::write(out, manifest.to_json())?;
    Ok(manifest)
}

fn archive_for(dir: &Path, dist: &str) -> Result<PathBuf, Box<dyn Error>> {
    for extension in ["zip", "tar.gz"] {
        let candidate = dir.join(format!("{dist}.{extension}"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!("archive for `{dist}` not found in `{}`", dir.display()).into())
}

/// Maps a target triple to `(os, arch, libc, sdk hints)`.
fn classify(profile: &TargetProfile) -> (String, String, Option<String>, SdkHints) {
    let arch = match profile.arch() {
        Arch::X86_64 => "x86_64",
        Arch::Aarch64 => "aarch64",
        Arch::Other => "unknown",
    }
    .to_owned();
    let (os, libc, sdk) = match profile.flavor() {
        Flavor::Msvc => (
            "windows",
            None,
            SdkHints {
                windows_build_tools: true,
                xcode_clt: false,
                cc: false,
            },
        ),
        Flavor::Darwin => (
            "macos",
            None,
            SdkHints {
                windows_build_tools: false,
                xcode_clt: true,
                cc: false,
            },
        ),
        Flavor::Gnu if profile.is_windows() => (
            "windows",
            None,
            SdkHints {
                windows_build_tools: false,
                xcode_clt: false,
                cc: true,
            },
        ),
        Flavor::Gnu => (
            "linux",
            Some("glibc".to_owned()),
            SdkHints {
                windows_build_tools: false,
                xcode_clt: false,
                cc: true,
            },
        ),
        Flavor::Musl => (
            "linux",
            Some("musl".to_owned()),
            SdkHints {
                windows_build_tools: false,
                xcode_clt: false,
                cc: true,
            },
        ),
        Flavor::Unknown => (
            "unknown",
            None,
            SdkHints {
                windows_build_tools: false,
                xcode_clt: false,
                cc: false,
            },
        ),
    };
    (os.to_owned(), arch, libc, sdk)
}

/// Current UTC time as RFC 3339 (`YYYY-MM-DDThh:mm:ssZ`).
fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let time = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// Howard Hinnant's `civil_from_days` (days since 1970-01-01 -> Y/M/D).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, target: &str, archive: &str, abi: u32) {
        let dist = format!("vut-v0.1.0-{target}");
        let manifest = Manifest {
            format_version: 1,
            vut: "0.1.0".into(),
            vpm: "0.1.0".into(),
            stdlib: "0.1.0".into(),
            runtime: "0.1.0".into(),
            abi_version: abi,
            target: target.into(),
            archive: archive.into(),
            channel: "stable".into(),
            profile: "release".into(),
            cranelift: Some("0.135.2".into()),
            build_commit: Some("deadbeef".into()),
            minimum_os: None,
        };
        std::fs::write(
            dir.join(format!("{dist}.manifest.json")),
            manifest.to_json(),
        )
        .unwrap();
        std::fs::write(dir.join(archive), b"archive-bytes").unwrap();
    }

    #[test]
    fn merges_targets_sorted_with_urls_and_checksums() {
        let dir = std::env::temp_dir().join(format!("vut-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_manifest(
            &dir,
            "x86_64-unknown-linux-gnu",
            "vut-v0.1.0-x86_64-unknown-linux-gnu.tar.gz",
            13,
        );
        write_manifest(
            &dir,
            "x86_64-pc-windows-msvc",
            "vut-v0.1.0-x86_64-pc-windows-msvc.zip",
            13,
        );
        let inputs = Inputs {
            channel: "stable",
            profile: "release",
            cranelift: Some("0.135.2"),
            released_at: Some("2026-01-01T00:00:00Z"),
            repo: "duongonix/vut",
            url_base: None,
        };
        let out = dir.join("releases.json");
        let manifest = generate(&dir, &out, &inputs).unwrap();
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.runtime_abi, 13);
        assert_eq!(manifest.targets.len(), 2);
        // Sorted by triple: windows-msvc sorts before linux-gnu? No: compare
        // strings, `x86_64-pc-...` < `x86_64-unknown-...`.
        assert_eq!(manifest.targets[0].triple, "x86_64-pc-windows-msvc");
        assert_eq!(manifest.targets[1].triple, "x86_64-unknown-linux-gnu");
        assert_eq!(manifest.targets[0].os, "windows");
        assert!(manifest.targets[0].sdk.windows_build_tools);
        assert_eq!(manifest.targets[1].libc.as_deref(), Some("glibc"));
        assert!(manifest.targets[1].sdk.cc);
        assert!(
            manifest.targets[0]
                .url
                .contains("releases/download/v0.1.0/")
        );
        assert_eq!(manifest.targets[0].sha256.len(), 64);
        assert!(out.is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn civil_dates_round_trip_known_values() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }
}
