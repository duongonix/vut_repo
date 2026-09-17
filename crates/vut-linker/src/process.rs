//! Shared process execution for native linkers.
use std::{
    path::Path,
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::error::{LinkError, LinkFailure};

static LINK_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Serializes native link invocations within a process. Linkers are external,
/// resource-heavy tools that share platform temporary files; running several
/// concurrently can race on those shared paths.
static LINK_LOCK: Mutex<()> = Mutex::new(());

/// Monotonic counter used to build unique linker crate/shim names.
pub(crate) fn next_sequence() -> u64 {
    LINK_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

/// Runs a linker command, returning a classified error on failure and flushing
/// the produced artifact on success.
pub(crate) fn run(program: &Path, args: &[String], output: &Path) -> Result<(), LinkError> {
    let _guard = LINK_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let result = Command::new(program).args(args).output().map_err(|error| {
        let failure = if error.kind() == std::io::ErrorKind::NotFound {
            LinkFailure::ToolNotFound
        } else {
            LinkFailure::Other
        };
        LinkError::classified(
            format!("failed to start linker {}: {error}", program.display()),
            failure,
        )
    })?;
    if result.status.success() {
        flush_artifact(output)
    } else {
        Err(LinkError::from_linker_output(program, &result.stderr))
    }
}

/// Ensures a just-written native artifact is fully materialized on the target
/// filesystem before a caller attempts to execute it. On Windows the linker
/// writes the executable through memory-mapped sections, so an immediate
/// `CreateProcess` can observe zero pages until the file is flushed.
pub(crate) fn flush_artifact(output: &Path) -> Result<(), LinkError> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(output)
        .map_err(|error| {
            LinkError::new(format!(
                "failed to open linked artifact {}: {error}",
                output.display()
            ))
        })?;
    file.sync_all().map_err(|error| {
        LinkError::new(format!(
            "failed to flush linked artifact {}: {error}",
            output.display()
        ))
    })
}
