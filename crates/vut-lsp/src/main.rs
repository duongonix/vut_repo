//! Official Vut language server. Semantic answers are derived from the Vut frontend.

#![allow(
    deprecated,
    clippy::cast_possible_truncation,
    clippy::enum_glob_use,
    clippy::match_same_arms,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::wildcard_imports
)]

use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::{Error, Result},
    lsp_types::*,
};
use vut_ast::File;
use vut_resolver::SymbolKind as ResolverSymbolKind;
use vut_source::Span;
use vut_types::builtin_method_signature;

mod completion;
mod diagnostics;
mod documents;
mod members;
mod presentation;
mod project;
mod query;
mod rename;
mod semantic_tokens;
mod signature_help;
mod symbols;

use documents::{Document, Documents};
use members::{find_member, receiver_kind};
use symbols::symbols;

struct Backend {
    client: Client,
    documents: Arc<Documents>,
    projects: project::ProjectAnalysis,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Documents::default()),
            projects: project::ProjectAnalysis::default(),
        }
    }

    async fn snapshot(&self, uri: &Url) -> std::result::Result<project::ProjectSnapshot, String> {
        let documents = self.documents.snapshot().await;
        self.snapshot_with_documents(uri, &documents).await
    }

    async fn snapshot_with_documents(
        &self,
        uri: &Url,
        documents: &HashMap<Url, Document>,
    ) -> std::result::Result<project::ProjectSnapshot, String> {
        let path = uri
            .to_file_path()
            .map_err(|()| "Vut LSP only supports file URIs".to_owned())?;
        let overlays = Documents::overlays(documents);
        self.projects.analyze(&path, &overlays).await
    }

    async fn analyze(&self, uri: &Url) -> std::result::Result<Analysis, String> {
        let snapshot = self.snapshot(uri).await?;
        let current_source = source_for_uri(&snapshot, uri)
            .ok_or_else(|| "document is not part of the analyzed project".to_owned())?;
        let checked = snapshot.checked;
        Ok(Analysis {
            file: snapshot.file,
            resolution: checked.resolution,
            semantics: checked.semantics,
            sources: snapshot.sources,
            current_source,
            root: snapshot.root,
        })
    }

    async fn publish(&self, uri: Url) {
        let documents = self.documents.snapshot().await;
        match self.snapshot_with_documents(&uri, &documents).await {
            Ok(snapshot) => {
                let mut reports = diagnostics::collect(&snapshot);
                for (document_uri, document) in documents {
                    let Some(source) = source_for_uri(&snapshot, &document_uri) else {
                        continue;
                    };
                    self.client
                        .publish_diagnostics(
                            document_uri,
                            reports.remove(&source).unwrap_or_default(),
                            Some(document.version),
                        )
                        .await;
                }
            }
            Err(message) => {
                let version = documents.get(&uri).map(|document| document.version);
                self.client
                    .publish_diagnostics(
                        uri,
                        vec![Diagnostic::new_simple(
                            Range::new(Position::new(0, 0), Position::new(0, 0)),
                            message,
                        )],
                        version,
                    )
                    .await;
            }
        }
    }
}

struct Analysis {
    file: File,
    resolution: vut_resolver::Resolution,
    semantics: vut_types::SemanticResult,
    sources: HashMap<vut_source::SourceId, project::SourceSnapshot>,
    current_source: vut_source::SourceId,
    root: PathBuf,
}

fn location(a: &Analysis, span: Span) -> Option<Location> {
    let source = a.sources.get(&span.source())?;
    Some(Location::new(
        Url::from_file_path(source.path.as_ref()?).ok()?,
        range(&source.text, span),
    ))
}

fn source_for_uri(snapshot: &project::ProjectSnapshot, uri: &Url) -> Option<vut_source::SourceId> {
    source_for_uri_parts(&snapshot.sources, uri)
}

fn source_for_uri_parts(
    sources: &HashMap<vut_source::SourceId, project::SourceSnapshot>,
    uri: &Url,
) -> Option<vut_source::SourceId> {
    let path = normalize_path(uri.to_file_path().ok()?);
    sources.iter().find_map(|(id, source)| {
        source
            .path
            .as_ref()
            .is_some_and(|candidate| normalize_path(candidate.clone()) == path)
            .then_some(*id)
    })
}

fn normalize_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(target) = params
            .initialization_options
            .as_ref()
            .and_then(|value| value.get("target"))
            .and_then(serde_json::Value::as_str)
        {
            self.projects.set_target(target.to_owned()).await;
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into()]),
                    ..CompletionOptions::default()
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    retrigger_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "vut-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Vut language server ready")
            .await;
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.documents
            .open(doc.uri.clone(), doc.text, doc.version)
            .await;
        self.publish(doc.uri).await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.last() {
            self.documents
                .open(
                    params.text_document.uri.clone(),
                    change.text.clone(),
                    params.text_document.version,
                )
                .await;
            self.publish(params.text_document.uri).await;
        }
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.close(&params.text_document.uri).await;
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        match vut_tooling::format_source(&text) {
            Ok(formatted) => Ok(Some(vec![TextEdit {
                range: whole_range(&text),
                new_text: formatted,
            }])),
            Err(_) => Ok(None),
        }
    }
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Nested(symbols(
            &a.file, &text,
        ))))
    }
    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let documents = self.documents.snapshot().await;
        let Some(uri) = documents.keys().min_by_key(|uri| uri.as_str()).cloned() else {
            return Ok(Some(Vec::new()));
        };
        let Ok(snapshot) = self.snapshot_with_documents(&uri, &documents).await else {
            return Ok(None);
        };
        let checked = snapshot.checked;
        let a = Analysis {
            file: snapshot.file,
            current_source: source_for_uri_parts(&snapshot.sources, &uri)
                .unwrap_or(vut_source::SourceId::from_index(0)),
            sources: snapshot.sources,
            semantics: checked.semantics,
            resolution: checked.resolution,
            root: snapshot.root,
        };
        let queries = query::SemanticQueries::new(&a.resolution);
        Ok(Some(
            queries
                .workspace_symbols(&params.query)
                .filter_map(|symbol| {
                    Some(SymbolInformation {
                        name: symbol.name.clone(),
                        kind: symbol_kind(symbol.kind),
                        tags: None,
                        deprecated: None,
                        location: location(&a, symbol.span)?,
                        container_name: Some(
                            a.resolution.modules[symbol.module.0].logical_path.display(),
                        ),
                    })
                })
                .collect(),
        ))
    }
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let offset = offset(&text, params.text_document_position_params.position);
        let queries = query::SemanticQueries::new(&a.resolution);
        let response = queries
            .symbol_at(a.current_source, offset)
            .and_then(|id| location(&a, queries.definition_span(id)))
            .map(GotoDefinitionResponse::Scalar);
        Ok(response)
    }
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let at = offset(&text, params.text_document_position.position);
        let queries = query::SemanticQueries::new(&a.resolution);
        let Some(symbol) = queries.symbol_at(a.current_source, at) else {
            return Ok(None);
        };
        let mut locations: Vec<_> = queries
            .references(symbol)
            .filter_map(|reference| location(&a, reference.span))
            .collect();
        if params.context.include_declaration
            && let Some(declaration) = location(&a, queries.declaration(symbol).span)
        {
            locations.push(declaration);
        }
        Ok(Some(locations))
    }
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let at = offset(&text, params.text_document_position.position);
        let Some(target) = rename::target(&a, a.current_source, at) else {
            return Ok(None);
        };
        rename::edits(&a, &target, &params.new_name)
            .map(Some)
            .map_err(Error::invalid_params)
    }
    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let at = offset(&text, params.position);
        Ok(rename::target(&a, a.current_source, at).map(|target| target.response))
    }
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let at = offset(&text, params.text_document_position_params.position);
        let symbol = query::SemanticQueries::new(&a.resolution).symbol_at(a.current_source, at);
        if let Some(id) = symbol {
            let s = &a.resolution.symbols[id.0];
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(
                    presentation::symbol_signature(&a, id),
                )),
                range: Some(range(&text, s.span)),
            }));
        }
        if let Some(member) = find_member(&a.file, at)
            && let Some(kind) = receiver_kind(&a, member.receiver)
            && let Some(rendered) = builtin_method_signature(kind, &member.member)
        {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(rendered.to_owned())),
                range: Some(range(&text, member.member_span)),
            }));
        }
        if let Some((span, ty)) = a
            .semantics
            .expression_types
            .iter()
            .filter(|(span, _)| span.source() == a.current_source && contains(**span, at))
            .min_by_key(|(span, _)| span.end().saturating_sub(span.start()))
        {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(format!(
                    "type: {}",
                    presentation::render_type(&a, *ty)
                ))),
                range: Some(range(&text, *span)),
            }));
        }
        Ok(None)
    }
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let at = offset(&text, params.text_document_position.position);
        Ok(Some(CompletionResponse::Array(completion::items(
            &a, &text, at,
        ))))
    }
    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(a) = self.analyze(&uri).await else {
            return Ok(None);
        };
        let at = offset(&text, params.text_document_position_params.position);
        Ok(signature_help::help(&a, &text, at))
    }
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.text(&uri).await else {
            return Ok(None);
        };
        let Ok(analysis) = self.analyze(&uri).await else {
            return Ok(None);
        };
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: semantic_tokens::tokens(&text, &analysis.resolution),
        })))
    }
}

fn range(text: &str, span: Span) -> Range {
    Range::new(position(text, span.start()), position(text, span.end()))
}
fn whole_range(text: &str) -> Range {
    Range::new(Position::new(0, 0), position(text, text.len()))
}
fn position(text: &str, offset: usize) -> Position {
    let prefix = &text[..offset.min(text.len())];
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let start = prefix.rfind('\n').map_or(0, |n| n + 1);
    Position::new(
        line as u32,
        text[start..offset.min(text.len())].encode_utf16().count() as u32,
    )
}
fn offset(text: &str, position: Position) -> usize {
    let mut line = 0;
    let mut start = 0;
    for (i, b) in text.bytes().enumerate() {
        if line == position.line as usize {
            break;
        }
        if b == b'\n' {
            line += 1;
            start = i + 1;
        }
    }
    text[start..]
        .char_indices()
        .scan(0u32, |units, (i, c)| {
            let here = *units;
            *units += c.len_utf16() as u32;
            Some((i, here))
        })
        .find(|(_, units)| *units >= position.character)
        .map_or(text.len(), |(i, _)| start + i)
}
fn contains(span: Span, value: usize) -> bool {
    span.start() <= value && value <= span.end()
}
fn completion_kind(kind: ResolverSymbolKind) -> CompletionItemKind {
    match kind {
        ResolverSymbolKind::Function => CompletionItemKind::FUNCTION,
        ResolverSymbolKind::Method => CompletionItemKind::METHOD,
        ResolverSymbolKind::Data
        | ResolverSymbolKind::Interface
        | ResolverSymbolKind::Enum
        | ResolverSymbolKind::TypeAlias => CompletionItemKind::CLASS,
        ResolverSymbolKind::Module => CompletionItemKind::MODULE,
        ResolverSymbolKind::Parameter => CompletionItemKind::VARIABLE,
        _ => CompletionItemKind::VARIABLE,
    }
}

fn symbol_kind(kind: ResolverSymbolKind) -> tower_lsp::lsp_types::SymbolKind {
    match kind {
        ResolverSymbolKind::Module => tower_lsp::lsp_types::SymbolKind::MODULE,
        ResolverSymbolKind::Function => tower_lsp::lsp_types::SymbolKind::FUNCTION,
        ResolverSymbolKind::Method => tower_lsp::lsp_types::SymbolKind::METHOD,
        ResolverSymbolKind::Data => tower_lsp::lsp_types::SymbolKind::STRUCT,
        ResolverSymbolKind::Interface => tower_lsp::lsp_types::SymbolKind::INTERFACE,
        ResolverSymbolKind::Enum => tower_lsp::lsp_types::SymbolKind::ENUM,
        ResolverSymbolKind::TypeAlias | ResolverSymbolKind::TypeParameter => {
            tower_lsp::lsp_types::SymbolKind::TYPE_PARAMETER
        }
        ResolverSymbolKind::Parameter | ResolverSymbolKind::Binding | ResolverSymbolKind::Local => {
            tower_lsp::lsp_types::SymbolKind::VARIABLE
        }
        ResolverSymbolKind::AnonymousFunction => tower_lsp::lsp_types::SymbolKind::FUNCTION,
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
