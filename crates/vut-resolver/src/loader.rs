use crate::{ModuleInput, ModulePath};
use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
};
use vut_diagnostics::DiagnosticSink;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_source::SourceManager;

#[derive(Debug)]
pub enum DiscoverError {
    Io { path: PathBuf, message: String },
    InvalidModulePath(PathBuf),
    Source(vut_source::SourceError),
}
impl fmt::Display for DiscoverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(formatter, "could not scan `{}`: {message}", path.display())
            }
            Self::InvalidModulePath(path) => {
                write!(formatter, "invalid module path `{}`", path.display())
            }
            Self::Source(error) => error.fmt(formatter),
        }
    }
}
impl std::error::Error for DiscoverError {}
#[derive(Debug)]
pub struct LoadedRoot {
    pub modules: Vec<ModuleInput>,
    pub diagnostics: DiagnosticSink,
}

/// Loads one standalone `.vut` file without discovering unrelated files in
/// its containing directory.
///
/// # Errors
/// Returns filesystem, UTF-8, or invalid module-name errors.
pub fn load_source_file(
    path: &Path,
    sources: &mut SourceManager,
) -> Result<LoadedRoot, DiscoverError> {
    let canonical = fs::canonicalize(path).map_err(|error| DiscoverError::Io {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    let stem = canonical
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| DiscoverError::InvalidModulePath(path.to_owned()))?;
    let source = sources
        .load_path(&canonical)
        .map_err(DiscoverError::Source)?;
    let text = sources.get(source).map_err(DiscoverError::Source)?.text();
    let (tokens, lexical) = Lexer::new(source, text).lex();
    let (file, syntax) = Parser::new(source, text, tokens).parse();
    let mut diagnostics = DiagnosticSink::new();
    diagnostics.extend(lexical);
    diagnostics.extend(syntax);
    Ok(LoadedRoot {
        modules: vec![ModuleInput {
            logical_path: ModulePath(vec![stem.to_owned()]),
            filesystem_path: Some(canonical),
            file,
        }],
        diagnostics,
    })
}

/// Loads and parses every `.vut` module below one explicit source root.
///
/// # Errors
///
/// Returns filesystem, UTF-8, or non-Unicode module-path errors.
pub fn load_source_root(
    root: &Path,
    prefix: &[String],
    sources: &mut SourceManager,
) -> Result<LoadedRoot, DiscoverError> {
    let canonical_root = fs::canonicalize(root).map_err(|error| DiscoverError::Io {
        path: root.to_owned(),
        message: error.to_string(),
    })?;
    let mut paths = Vec::new();
    visit(&canonical_root, &mut paths)?;
    paths.sort();
    let mut module_paths = BTreeMap::new();
    let mut diagnostics = DiagnosticSink::new();
    for path in paths {
        let (segments, is_mod) = logical_segments(&canonical_root, prefix, &path)?;
        let source = sources.load_path(&path).map_err(DiscoverError::Source)?;
        let text = sources.get(source).map_err(DiscoverError::Source)?.text();
        let (tokens, lexical) = Lexer::new(source, text).lex();
        diagnostics.extend(lexical);
        let (file, syntax) = Parser::new(source, text, tokens).parse();
        diagnostics.extend(syntax);
        let input = ModuleInput {
            logical_path: ModulePath(segments),
            filesystem_path: Some(path),
            file,
        };
        match module_paths.get(&input.logical_path) {
            Some((true, _)) if !is_mod => {
                module_paths.insert(input.logical_path.clone(), (is_mod, input));
            }
            Some(_) => {}
            None => {
                module_paths.insert(input.logical_path.clone(), (is_mod, input));
            }
        }
    }
    Ok(LoadedRoot {
        modules: module_paths.into_values().map(|(_, input)| input).collect(),
        diagnostics,
    })
}

fn logical_segments(
    root: &Path,
    prefix: &[String],
    path: &Path,
) -> Result<(Vec<String>, bool), DiscoverError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| DiscoverError::InvalidModulePath(path.to_owned()))?;
    let mut segments = prefix.to_vec();
    let components: Vec<_> = relative.components().collect();
    let mut is_mod = false;
    for (index, component) in components.iter().enumerate() {
        let text = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| DiscoverError::InvalidModulePath(path.to_owned()))?;
        if index + 1 == components.len() {
            let stem = text.strip_suffix(".vut").unwrap_or(text);
            is_mod = stem == "mod";
            if is_mod {
                continue;
            }
            if stem != "lib" || prefix.is_empty() {
                segments.push(stem.to_owned());
            }
        } else {
            segments.push(text.to_owned());
        }
    }
    Ok((segments, is_mod))
}

fn visit(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), DiscoverError> {
    let entries = fs::read_dir(directory).map_err(|error| DiscoverError::Io {
        path: directory.to_owned(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| DiscoverError::Io {
            path: directory.to_owned(),
            message: error.to_string(),
        })?;
        let file_type = entry.file_type().map_err(|error| DiscoverError::Io {
            path: entry.path(),
            message: error.to_string(),
        })?;
        if file_type.is_dir() {
            visit(&entry.path(), output)?;
        } else if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "vut")
        {
            output.push(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_loader_does_not_scan_sibling_sources() {
        let root = std::env::temp_dir().join(format!(
            "vut-single-file-loader-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let source = root.join("a.vut");
        fs::write(&source, "fn main():\n  out(\"hello\")\n").unwrap();
        fs::write(root.join("src/main.vut"), "import missing\n").unwrap();

        let mut sources = SourceManager::new();
        let loaded = load_source_file(&source, &mut sources).unwrap();

        assert_eq!(loaded.modules.len(), 1);
        assert_eq!(loaded.modules[0].logical_path, ModulePath(vec!["a".into()]));
        assert!(!loaded.diagnostics.has_errors());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mod_file_is_folder_module_and_file_wins_same_name() {
        let root = std::env::temp_dir().join(format!(
            "vut-mod-loader-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src/http")).unwrap();
        fs::write(root.join("src/http.vut"), "SOURCE = \"file\"\n").unwrap();
        fs::write(root.join("src/http/mod.vut"), "SOURCE = \"folder\"\n").unwrap();
        fs::write(root.join("src/http/client.vut"), "fn get() -> int:\n  1\n").unwrap();
        fs::write(root.join("src/app.vut"), "import http\n").unwrap();

        let mut sources = SourceManager::new();
        let loaded = load_source_root(&root.join("src"), &[], &mut sources).unwrap();
        let modules = loaded
            .modules
            .iter()
            .map(|module| {
                (
                    module.logical_path.display(),
                    module
                        .filesystem_path
                        .as_ref()
                        .unwrap()
                        .strip_prefix(fs::canonicalize(root.join("src")).unwrap())
                        .unwrap()
                        .to_owned(),
                )
            })
            .collect::<Vec<_>>();

        assert!(
            modules
                .iter()
                .any(|(path, file)| { path == "http" && file == &PathBuf::from("http.vut") })
        );
        assert!(modules.iter().any(|(path, file)| {
            path == "http.client" && file == &PathBuf::from("http/client.vut")
        }));
        assert!(
            !modules
                .iter()
                .any(|(_, file)| file == &PathBuf::from("http/mod.vut"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_mod_is_importable_when_no_file_candidate_exists() {
        let root = std::env::temp_dir().join(format!(
            "vut-folder-mod-loader-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src/app")).unwrap();
        fs::write(root.join("src/app/mod.vut"), "fn boot() -> int:\n  1\n").unwrap();

        let mut sources = SourceManager::new();
        let loaded = load_source_root(&root.join("src"), &[], &mut sources).unwrap();

        assert_eq!(loaded.modules.len(), 1);
        assert_eq!(
            loaded.modules[0].logical_path,
            ModulePath(vec!["app".into()])
        );
        fs::remove_dir_all(root).unwrap();
    }
}
