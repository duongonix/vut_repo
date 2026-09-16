//! Platform helpers for emitted executables.
use std::path::Path;

#[cfg(unix)]
pub(super) fn restore_executable_permissions(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
}
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
pub(super) fn restore_executable_permissions(_path: &Path) -> Result<(), std::io::Error> {
    Ok(())
}
