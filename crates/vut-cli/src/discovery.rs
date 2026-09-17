//! CLI-local process discovery. Runtime/stdlib/startup resolution lives in
//! `vut-paths` so every component shares one implementation.
use std::path::{Path, PathBuf};

/// Finds a program on `PATH`, honoring `PATHEXT` on Windows.
///
/// When `program` already has an extension (for example `link.exe`), `PATHEXT`
/// is not appended again.
#[must_use]
pub fn find_on_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let has_extension = Path::new(program).extension().is_some();
    let extensions: Vec<String> = if cfg!(windows) && !has_extension {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .map(str::to_ascii_lowercase)
            .collect()
    } else {
        vec![String::new()]
    };
    for directory in std::env::split_paths(&path) {
        for extension in &extensions {
            let candidate = if extension.is_empty() {
                directory.join(program)
            } else {
                directory.join(format!("{program}{extension}"))
            };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
