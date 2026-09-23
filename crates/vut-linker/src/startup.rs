//! Resolution and development build of the `vut-startup` object.
//!
//! In a release distribution the startup object is prebuilt per target and
//! shipped next to the runtime archives. When it is absent (development), this
//! module builds it on demand with `rustc`. Production linking must always use
//! the prebuilt object so no Rust toolchain is required at link time.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::{LinkError, LinkFailure};
use crate::target::TargetProfile;

/// Serializes on-demand startup builds within a process.
static BUILD_LOCK: Mutex<()> = Mutex::new(());

/// Namespace for startup object helpers.
pub struct StartupObject;

impl StartupObject {
    /// Bundled startup source (single source of truth across targets).
    pub const SOURCE: &'static str = include_str!("../startup/vut-startup.rs");

    /// Resolves the startup object for a target.
    ///
    /// Order: explicit `plan.startup` path, then `<search_dir>/<name>`, then an
    /// on-demand development build in `cache_dir`.
    ///
    /// # Errors
    /// Returns an error when an explicit path is missing or the development
    /// build fails.
    pub fn resolve(
        profile: &TargetProfile,
        explicit: Option<&Path>,
        search_dir: Option<&Path>,
        cache_dir: &Path,
    ) -> Result<PathBuf, LinkError> {
        if let Some(path) = explicit {
            if path.is_file() {
                return Ok(path.to_path_buf());
            }
            return Err(LinkError::classified(
                format!("startup object `{}` was not found", path.display()),
                LinkFailure::MissingLibrary,
            ));
        }
        if let Some(directory) = search_dir {
            let candidate = directory.join(profile.startup_object_name());
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        Self::build(profile, cache_dir)
    }

    /// Builds the startup object for `profile` into `cache_dir` using `rustc`.
    ///
    /// The object is built to a unique temporary path and atomically installed,
    /// so concurrent links never observe a partially written object.
    ///
    /// # Errors
    /// Returns [`LinkFailure::ToolNotFound`] when `rustc` is unavailable, or
    /// [`LinkFailure::Other`] when the source fails to compile.
    pub fn build(profile: &TargetProfile, cache_dir: &Path) -> Result<PathBuf, LinkError> {
        let _guard = BUILD_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let object = cache_dir.join(profile.startup_object_name());
        if object.is_file() {
            return Ok(object);
        }
        std::fs::create_dir_all(cache_dir).map_err(|error| {
            LinkError::new(format!(
                "failed to create startup cache `{}`: {error}",
                cache_dir.display()
            ))
        })?;
        let sequence = crate::process::next_sequence();
        let source = cache_dir.join(format!("vut-startup-{}-{sequence}.rs", std::process::id()));
        std::fs::write(&source, Self::SOURCE).map_err(|error| {
            LinkError::new(format!(
                "failed to write startup source `{}`: {error}",
                source.display()
            ))
        })?;
        let staged = cache_dir.join(format!("vut-startup-{}-{sequence}.obj", std::process::id()));
        let result = Self::compile(profile, &source, &staged);
        let _ = std::fs::remove_file(&source);
        result?;
        // Install atomically; if another process won the race, use its object.
        if std::fs::rename(&staged, &object).is_ok() {
            Ok(object)
        } else {
            let _ = std::fs::remove_file(&staged);
            if object.is_file() {
                Ok(object)
            } else {
                Err(LinkError::new(format!(
                    "failed to install startup object `{}`",
                    object.display()
                )))
            }
        }
    }

    /// Compiles the startup source to `output` with `rustc`.
    fn compile(profile: &TargetProfile, source: &Path, output: &Path) -> Result<(), LinkError> {
        let mut command = std::process::Command::new("rustc");
        command.args([
            "--edition",
            "2024",
            "--crate-type=lib",
            "--emit=obj",
            "-C",
            "panic=abort",
            "-C",
            "opt-level=2",
        ]);
        if profile.triple() != target_lexicon::HOST.to_string() {
            command.arg("--target").arg(profile.triple());
        }
        command.arg("-o").arg(output).arg(source);
        let compiled = command.output().map_err(|error| {
            let failure = if error.kind() == std::io::ErrorKind::NotFound {
                LinkFailure::ToolNotFound
            } else {
                LinkFailure::Other
            };
            LinkError::classified(
                format!("failed to start rustc to build the Vut startup object: {error}"),
                failure,
            )
        })?;
        if !compiled.status.success() {
            return Err(LinkError::classified(
                format!(
                    "failed to build Vut startup object: {}",
                    String::from_utf8_lossy(&compiled.stderr).trim()
                ),
                LinkFailure::Other,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_profile() -> TargetProfile {
        TargetProfile::parse(&target_lexicon::HOST.to_string()).expect("host triple")
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("vut-startup-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn explicit_missing_path_is_reported() {
        let profile = host_profile();
        let error = StartupObject::resolve(
            &profile,
            Some(Path::new("definitely-missing-startup.obj")),
            None,
            &scratch("explicit"),
        )
        .expect_err("missing explicit path must fail");
        assert_eq!(error.failure(), LinkFailure::MissingLibrary);
    }

    #[test]
    fn builds_a_startup_object_for_the_host() {
        let profile = host_profile();
        let cache = scratch("build");
        let _ = std::fs::remove_dir_all(&cache);
        let object = StartupObject::build(&profile, &cache).expect("build startup object");
        assert!(object.is_file(), "{}", object.display());
        assert!(std::fs::metadata(&object).expect("metadata").len() > 0);
        assert_eq!(
            object.file_name().and_then(|name| name.to_str()),
            Some(profile.startup_object_name())
        );
    }
}
