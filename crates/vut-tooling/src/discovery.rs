use std::path::{Path, PathBuf};

pub fn discover(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    for directory in [root.join("src"), root.join("tests")] {
        if directory.is_dir() {
            visit(&directory, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn visit(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            visit(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "vut") {
            files.push(path);
        }
    }
    Ok(())
}
