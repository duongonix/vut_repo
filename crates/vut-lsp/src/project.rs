//! Persistent project discovery and canonical compiler-front-end analysis.

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};
use tokio::sync::{Mutex, RwLock};
use vut_ast::File;
use vut_compiler::{CheckedProgram, CompilerConfig, CompilerSession};
use vut_source::SourceId;

#[derive(Clone)]
pub(super) struct SourceSnapshot {
    pub(super) path: Option<PathBuf>,
    pub(super) text: String,
}

pub(super) struct ProjectSnapshot {
    pub(super) checked: CheckedProgram,
    pub(super) file: File,
    pub(super) sources: HashMap<SourceId, SourceSnapshot>,
    pub(super) root: PathBuf,
}

pub(super) struct ProjectAnalysis {
    target: RwLock<String>,
    sessions: Mutex<HashMap<PathBuf, Session>>,
}

struct Session {
    target: String,
    compiler: CompilerSession,
}

impl Default for ProjectAnalysis {
    fn default() -> Self {
        Self {
            target: RwLock::new(CompilerConfig::default().target),
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl ProjectAnalysis {
    pub(super) async fn set_target(&self, target: String) {
        *self.target.write().await = target;
    }

    pub(super) async fn analyze(
        &self,
        path: &Path,
        overlays: &BTreeMap<PathBuf, String>,
    ) -> Result<ProjectSnapshot, String> {
        let path = normalize(path);
        let root = project_root(&path);
        let roots = if root.join("vpm.toml").is_file() {
            vpm::analysis_source_roots(&root).map_err(|error| error.to_string())?
        } else {
            vec![(root.clone(), Vec::new())]
        };
        let overlays = overlays
            .iter()
            .map(|(path, text)| (normalize(path), text.clone()))
            .collect();
        let target = self.target.read().await.clone();
        let mut sessions = self.sessions.lock().await;
        let session = sessions.entry(root.clone()).or_insert_with(|| Session {
            target: target.clone(),
            compiler: CompilerSession::new(CompilerConfig::for_target(
                target.clone(),
                vut_compiler::BuildMode::Debug,
            )),
        });
        if session.target != target {
            *session = Session {
                target: target.clone(),
                compiler: CompilerSession::new(CompilerConfig::for_target(
                    target,
                    vut_compiler::BuildMode::Debug,
                )),
            };
        }
        let checked = session
            .compiler
            .check_source_roots_with_overlays(&roots, &overlays)
            .map_err(|error| error.to_string())?;
        let source = session
            .compiler
            .sources()
            .id_for_path(&path)
            .ok_or_else(|| {
                format!(
                    "source `{}` is outside the discovered project",
                    path.display()
                )
            })?;
        let file = checked
            .resolution
            .modules
            .iter()
            .find(|module| module.source == source)
            .map(|module| module.file.clone())
            .ok_or_else(|| format!("source `{}` is not a Vut module", path.display()))?;
        let sources = checked
            .resolution
            .modules
            .iter()
            .map(|module| {
                session
                    .compiler
                    .sources()
                    .get(module.source)
                    .map(|source| {
                        (
                            module.source,
                            SourceSnapshot {
                                path: source.path().map(Path::to_owned),
                                text: source.text().to_owned(),
                            },
                        )
                    })
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(ProjectSnapshot {
            checked,
            file,
            sources,
            root,
        })
    }
}

fn project_root(path: &Path) -> PathBuf {
    let start = path.parent().unwrap_or(path);
    start
        .ancestors()
        .find(|directory| directory.join("vpm.toml").is_file())
        .map_or_else(|| start.to_owned(), Path::to_owned)
}

fn normalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn assert_semantic_tooling(tooling: &crate::Analysis, text: &str, source: SourceId) {
        let member_cursor = text.find("helper.answer").unwrap() + "helper.".len();
        let completions = crate::completion::items(tooling, text, member_cursor);
        assert!(completions.iter().any(|item| item.label == "answer"));
        let call_cursor = text.find("answer(").unwrap() + "answer(".len();
        let help = crate::signature_help::help(tooling, text, call_cursor).unwrap();
        assert!(help.signatures[0].label.starts_with("fn answer("));
        let rename_at = text.find("answer").unwrap();
        let target = crate::rename::target(tooling, source, rename_at).unwrap();
        assert!(matches!(
            &target.response,
            tower_lsp::lsp_types::PrepareRenameResponse::RangeWithPlaceholder { range, .. }
                if range.start.line == 2
        ));
        let edit = crate::rename::edits(tooling, &target, "renamed").unwrap();
        let changes = edit.changes.unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes.values().map(Vec::len).sum::<usize>(), 2);
        assert!(crate::rename::edits(tooling, &target, "not-valid").is_err());
        assert!(crate::rename::edits(tooling, &target, "fn").is_err());
        assert!(crate::rename::edits(tooling, &target, "taken").is_err());
    }

    #[tokio::test]
    async fn resolves_project_imports_and_preserves_overlay_source_ids() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vut-lsp-project-{nonce}"));
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            root.join("vpm.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n[dependencies]\n",
        )
        .unwrap();
        std::fs::write(root.join("vpm.lock"), "lock-version = 1\npackage = []\n").unwrap();
        let main = src.join("main.vut");
        std::fs::write(
            &main,
            "import helper\nfn main() -> int:\n  helper.answer()\n",
        )
        .unwrap();
        std::fs::write(
            src.join("helper.vut"),
            "fn answer() -> int:\n  41\nfn taken() -> int:\n  0\n",
        )
        .unwrap();
        let analysis = ProjectAnalysis::default();
        let first = analysis.analyze(&main, &BTreeMap::new()).await.unwrap();
        assert!(!first.checked.resolution.diagnostics.has_errors());
        let documents = BTreeMap::from([(
            main.clone(),
            "import helper\nfn main() -> int:\n  helper.answer() + 1\n".to_owned(),
        )]);
        let second = analysis.analyze(&main, &documents).await.unwrap();
        let source_for = |snapshot: &ProjectSnapshot| {
            snapshot
                .sources
                .iter()
                .find(|(_, source)| {
                    source
                        .path
                        .as_ref()
                        .is_some_and(|path| path.ends_with("main.vut"))
                })
                .map(|(id, _)| *id)
                .unwrap()
        };
        assert_eq!(source_for(&first), source_for(&second));
        assert!(!second.checked.resolution.diagnostics.has_errors());
        assert!(!second.checked.semantics.diagnostics.has_errors());
        let overlay_text = documents.get(&main).unwrap();
        let current_source = source_for(&second);
        let tooling = crate::Analysis {
            file: second.file,
            resolution: second.checked.resolution,
            semantics: second.checked.semantics,
            sources: second.sources,
            current_source,
            root: second.root,
        };
        assert_semantic_tooling(&tooling, overlay_text, current_source);
        let broken = BTreeMap::from([(
            src.join("helper.vut"),
            "fn answer() -> int:\n  missing\n".to_owned(),
        )]);
        let third = analysis.analyze(&main, &broken).await.unwrap();
        let helper_source = third
            .sources
            .iter()
            .find(|(_, source)| {
                source
                    .path
                    .as_ref()
                    .is_some_and(|path| path.ends_with("helper.vut"))
            })
            .map(|(id, _)| *id)
            .unwrap();
        assert!(
            third
                .checked
                .resolution
                .diagnostics
                .as_slice()
                .iter()
                .any(|diagnostic| diagnostic
                    .primary
                    .as_ref()
                    .is_some_and(|label| label.span.source() == helper_source))
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
