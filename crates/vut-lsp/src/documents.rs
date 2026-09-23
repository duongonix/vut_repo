//! Versioned in-memory text document overlay.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

#[derive(Clone)]
pub(super) struct Document {
    pub(super) text: String,
    pub(super) version: i32,
}

#[derive(Default)]
pub(super) struct Documents(RwLock<HashMap<Url, Document>>);

impl Documents {
    pub(super) async fn open(&self, uri: Url, text: String, version: i32) {
        self.0.write().await.insert(uri, Document { text, version });
    }

    pub(super) async fn close(&self, uri: &Url) {
        self.0.write().await.remove(uri);
    }

    pub(super) async fn text(&self, uri: &Url) -> Option<String> {
        self.0
            .read()
            .await
            .get(uri)
            .map(|document| document.text.clone())
    }

    pub(super) async fn snapshot(&self) -> HashMap<Url, Document> {
        self.0.read().await.clone()
    }

    pub(super) fn overlays(documents: &HashMap<Url, Document>) -> BTreeMap<PathBuf, String> {
        documents
            .iter()
            .filter_map(|(uri, document)| {
                uri.to_file_path()
                    .ok()
                    .map(|path| (path, document.text.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn overlays_track_versions_and_close_deterministically() {
        let documents = Documents::default();
        let uri = Url::parse("file:///C:/project/main.vut").unwrap();
        documents.open(uri.clone(), "first".into(), 3).await;
        documents.open(uri.clone(), "second".into(), 4).await;
        let snapshot = documents.snapshot().await;
        assert_eq!(snapshot[&uri].text, "second");
        assert_eq!(snapshot[&uri].version, 4);
        documents.close(&uri).await;
        assert!(documents.snapshot().await.is_empty());
    }
}
