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
    let mut files = Vec::new();
    collect_files(root, Path::new(""), &mut files)?;
    for (relative, absolute) in files {
        let name = relative.to_string_lossy().replace('\\', "/");
        let mut source = File::open(&absolute)?;
        builder.append_file(name, &mut source)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("vut-dist-archive-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tree/bin")).unwrap();
        std::fs::create_dir_all(root.join("tree/std/os")).unwrap();
        std::fs::write(root.join("tree/manifest.json"), b"{}").unwrap();
        std::fs::write(root.join("tree/bin/vut"), b"binary").unwrap();
        std::fs::write(root.join("tree/std/os/mod.vut"), b"module").unwrap();
        root
    }

    #[test]
    fn writes_a_tar_gz_tree() {
        let root = scratch("targz");
        let out = root.join("out.tar.gz");
        tar_gz_tree(&root.join("tree"), &out).unwrap();
        assert!(std::fs::metadata(&out).unwrap().len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn writes_a_zip_tree() {
        let root = scratch("zip");
        let out = root.join("out.zip");
        zip_tree(&root.join("tree"), &out).unwrap();
        assert!(std::fs::metadata(&out).unwrap().len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
