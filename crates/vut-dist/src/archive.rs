//! Archive writers for distribution trees.
//!
//! Archives contain the distribution contents at the archive root
//! (`manifest.json`, `bin/`, `lib/`, `std/`), which keeps installer extraction
//! simple.
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

/// Writes a `.zip` archive of `root`.
///
/// # Errors
/// Returns I/O or zip encoder errors.
pub fn zip_tree(root: &Path, out: &Path) -> io::Result<()> {
    let file = File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut files = Vec::new();
    collect_files(root, Path::new(""), &mut files)?;
    for (relative, absolute) in files {
        let name = relative.to_string_lossy().replace('\\', "/");
        zip.start_file(name, options)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let mut source = File::open(&absolute)?;
        io::copy(&mut source, &mut zip)?;
    }
    zip.finish()
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}

/// Writes a `.tar.gz` archive of `root`.
///
/// # Errors
/// Returns I/O or tar/gzip encoder errors.
pub fn tar_gz_tree(root: &Path, out: &Path) -> io::Result<()> {
    let file = File::create(out)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        builder.append_dir_all(entry.file_name(), entry.path())?;
    }
    builder.finish()?;
    builder.into_inner()?.finish()?;
    Ok(())
}

fn collect_files(root: &Path, prefix: &Path, out: &mut Vec<(PathBuf, PathBuf)>) -> io::Result<()> {
    let mut entries = std::fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let relative = prefix.join(entry.file_name());
        if path.is_dir() {
            collect_files(&path, &relative, out)?;
        } else {
            out.push((relative, path));
        }
    }
    Ok(())
}
