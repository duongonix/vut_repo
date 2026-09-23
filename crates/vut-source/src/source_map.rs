//! Source storage and byte-accurate location mapping for the Vut compiler.

use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(usize);

impl SourceId {
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    source: SourceId,
    start: usize,
    end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self { source, start, end }
    }
    #[must_use]
    pub const fn source(self) -> SourceId {
        self.source
    }
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceError {
    InvalidUtf8 { name: String },
    Io { path: PathBuf, message: String },
    UnknownSource(SourceId),
    InvalidOffset { source: SourceId, offset: usize },
    InvalidSpan(Span),
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 { name } => write!(formatter, "source `{name}` is not valid UTF-8"),
            Self::Io { path, message } => {
                write!(formatter, "could not read `{}`: {message}", path.display())
            }
            Self::UnknownSource(id) => write!(formatter, "unknown source id {}", id.index()),
            Self::InvalidOffset { source, offset } => write!(
                formatter,
                "invalid offset {offset} in source {}",
                source.index()
            ),
            Self::InvalidSpan(span) => write!(
                formatter,
                "invalid span {}:{}..{}",
                span.source().index(),
                span.start(),
                span.end()
            ),
        }
    }
}
impl std::error::Error for SourceError {}

#[derive(Debug)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    path: Option<PathBuf>,
    text: Arc<str>,
    line_starts: Box<[usize]>,
}
impl SourceFile {
    fn new(id: SourceId, name: String, path: Option<PathBuf>, text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );
        Self {
            id,
            name,
            path,
            text: Arc::from(text),
            line_starts: line_starts.into_boxed_slice(),
        }
    }
    fn replace_text(&mut self, text: String) {
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );
        self.text = Arc::from(text);
        self.line_starts = line_starts.into_boxed_slice();
    }
    #[must_use]
    pub const fn id(&self) -> SourceId {
        self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    #[must_use]
    pub fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }
    /// Returns the one-based line and Unicode-scalar column for a byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::InvalidOffset`] when the offset is outside this source
    /// or is not on a UTF-8 boundary.
    pub fn location(&self, offset: usize) -> Result<SourceLocation, SourceError> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return Err(SourceError::InvalidOffset {
                source: self.id,
                offset,
            });
        }
        let line_index = self
            .line_starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = self.text[line_start..offset].chars().count() + 1;
        Ok(SourceLocation {
            line: line_index + 1,
            column,
        })
    }

    /// Returns the exact source text covered by a span.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::InvalidSpan`] for a foreign, reversed, out-of-range,
    /// or non-UTF-8-boundary span.
    pub fn slice(&self, span: Span) -> Result<&str, SourceError> {
        if span.source() != self.id
            || span.start() > span.end()
            || span.end() > self.text.len()
            || !self.text.is_char_boundary(span.start())
            || !self.text.is_char_boundary(span.end())
        {
            return Err(SourceError::InvalidSpan(span));
        }
        Ok(&self.text[span.start()..span.end()])
    }

    #[must_use]
    pub fn line(&self, one_based_line: usize) -> Option<&str> {
        let index = one_based_line.checked_sub(1)?;
        let start = *self.line_starts.get(index)?;
        let end = self
            .line_starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.text.len());
        Some(self.text[start..end].trim_end_matches(['\r', '\n']))
    }
}

#[derive(Debug, Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
    paths: HashMap<PathBuf, SourceId>,
}
impl SourceManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            paths: HashMap::new(),
        }
    }
    pub fn add_text(&mut self, name: impl Into<String>, text: String) -> SourceId {
        let id = SourceId::from_index(self.files.len());
        self.files
            .push(SourceFile::new(id, name.into(), None, text));
        id
    }
    /// Loads a UTF-8 file into the manager.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the file cannot be read or invalid-UTF-8 when it
    /// cannot represent Vut source text.
    pub fn load_path(&mut self, path: impl AsRef<Path>) -> Result<SourceId, SourceError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| SourceError::Io {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
        let text = String::from_utf8(bytes).map_err(|_| SourceError::InvalidUtf8 {
            name: path.display().to_string(),
        })?;
        Ok(self.set_path_text(path, text))
    }
    /// Inserts or replaces a path-backed source while preserving its source ID.
    pub fn set_path_text(&mut self, path: impl AsRef<Path>, text: String) -> SourceId {
        let path = normalized_path(path.as_ref());
        if let Some(id) = self.paths.get(&path).copied() {
            self.files[id.index()].replace_text(text);
            return id;
        }
        let id = SourceId::from_index(self.files.len());
        self.files.push(SourceFile::new(
            id,
            path.display().to_string(),
            Some(path.clone()),
            text,
        ));
        self.paths.insert(path, id);
        id
    }
    #[must_use]
    pub fn id_for_path(&self, path: impl AsRef<Path>) -> Option<SourceId> {
        self.paths.get(&normalized_path(path.as_ref())).copied()
    }
    /// Returns a source file by ID.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::UnknownSource`] when the ID is not owned by this manager.
    pub fn get(&self, id: SourceId) -> Result<&SourceFile, SourceError> {
        self.files
            .get(id.index())
            .ok_or(SourceError::UnknownSource(id))
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

fn normalized_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn ids_are_deterministic() {
        let mut sources = SourceManager::new();
        assert_eq!(sources.add_text("a", String::new()).index(), 0);
        assert_eq!(sources.add_text("b", String::new()).index(), 1);
    }
    #[test]
    fn path_overlay_reuses_source_id_and_rebuilds_line_index() {
        let path = std::env::temp_dir().join("vut-source-overlay-stable.vut");
        let mut sources = SourceManager::new();
        let first = sources.set_path_text(&path, "one\n".into());
        let second = sources.set_path_text(&path, "one\ntwo\n".into());
        assert_eq!(first, second);
        assert_eq!(sources.id_for_path(&path), Some(first));
        assert_eq!(sources.get(first).unwrap().line_starts(), &[0, 4, 8]);
    }
    #[test]
    fn maps_utf8_offsets() {
        let mut sources = SourceManager::new();
        let id = sources.add_text("test", "aé\n猫".into());
        let file = sources.get(id).unwrap();
        assert_eq!(
            file.location(3).unwrap(),
            SourceLocation { line: 1, column: 3 }
        );
        assert_eq!(
            file.location(4).unwrap(),
            SourceLocation { line: 2, column: 1 }
        );
    }
    #[test]
    fn rejects_non_boundary_offsets() {
        let mut sources = SourceManager::new();
        let id = sources.add_text("test", "é".into());
        assert!(matches!(
            sources.get(id).unwrap().location(1),
            Err(SourceError::InvalidOffset { .. })
        ));
    }
    #[test]
    fn validates_and_slices_spans() {
        let mut sources = SourceManager::new();
        let id = sources.add_text("test", "xin chào".into());
        let file = sources.get(id).unwrap();
        assert_eq!(file.slice(Span::new(id, 0, 3)).unwrap(), "xin");
        assert!(matches!(
            file.slice(Span::new(id, 4, 2)),
            Err(SourceError::InvalidSpan(_))
        ));
    }
    #[test]
    fn loads_multiple_files_and_rejects_invalid_utf8() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("vut-source-{nonce}"));
        fs::create_dir(&directory).unwrap();
        let first = directory.join("first.vut");
        let second = directory.join("second.vut");
        let invalid = directory.join("invalid.vut");
        fs::write(&first, "name = \"Nam\"\n").unwrap();
        fs::write(&second, "out(\"xin chào\")\n").unwrap();
        fs::write(&invalid, [0xff, 0xfe]).unwrap();
        let mut sources = SourceManager::new();
        assert_eq!(sources.load_path(&first).unwrap().index(), 0);
        assert_eq!(sources.load_path(&second).unwrap().index(), 1);
        assert!(matches!(
            sources.load_path(&invalid),
            Err(SourceError::InvalidUtf8 { .. })
        ));
        assert_eq!(sources.len(), 2);
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
        fs::remove_file(invalid).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
