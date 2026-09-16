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

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_lsp::{Client, LanguageServer, LspService, Server, jsonrpc::Result, lsp_types::*};
use vut_ast::File;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_resolver::{ModuleInput, ModulePath, Resolver, SymbolKind as ResolverSymbolKind};
use vut_source::{SourceId, Span};
use vut_types::{Analyzer, builtin_method_signature, builtin_methods};

mod members;
mod semantic_tokens;
mod symbols;

use members::{
    active_parameter, call_context, find_member, member_context, parameter_infos, receiver_kind,
    receiver_kind_at,
};
use symbols::symbols;

#[derive(Default)]
struct Documents(RwLock<HashMap<Url, String>>);

struct Backend {
    client: Client,
    documents: Arc<Documents>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Documents::default()),
        }
    }

    async fn analyze(&self, uri: &Url, text: &str) -> Analysis {
        let source = SourceId::from_index(0);
        let (tokens, lexical) = Lexer::new(source, text).lex();
        let (file, syntax) = Parser::new(source, text, tokens).parse();
        let resolution = Resolver::new(vec![ModuleInput {
            logical_path: ModulePath(vec!["main".into()]),
            filesystem_path: uri.to_file_path().ok(),
            file: file.clone(),
        }])
        .resolve();
        let semantics = Analyzer::new(&resolution).analyze();
        let diagnostics = lexical
            .as_slice()
            .iter()
            .chain(syntax.as_slice())
            .chain(resolution.diagnostics.as_slice())
            .chain(semantics.diagnostics.as_slice())
            .filter_map(|d| {
                let label = d.primary.as_ref()?;
                Some(Diagnostic {
                    range: range(text, label.span),
                    severity: Some(match d.severity {
                        vut_diagnostics::Severity::Error => DiagnosticSeverity::ERROR,
                        vut_diagnostics::Severity::Warning => DiagnosticSeverity::WARNING,
                        vut_diagnostics::Severity::Note => DiagnosticSeverity::INFORMATION,
                        vut_diagnostics::Severity::Help => DiagnosticSeverity::HINT,
                    }),
                    code: d
                        .code
                        .map(|value| NumberOrString::String(value.as_str().into())),
                    code_description: None,
                    source: Some("vut".into()),
                    message: format!("{}: {}", d.title, label.message),
                    related_information: None,
                    tags: None,
                    data: None,
                })
            })
            .collect();
        Analysis {
            file,
            resolution,
            semantics,
            diagnostics,
        }
    }

    async fn publish(&self, uri: Url) {
        let text = self
            .documents
            .0
            .read()
            .await
            .get(&uri)
            .cloned()
            .unwrap_or_default();
        let analysis = self.analyze(&uri, &text).await;
        self.client
            .publish_diagnostics(uri, analysis.diagnostics, None)
            .await;
    }
}

struct Analysis {
    file: File,
    resolution: vut_resolver::Resolution,
    semantics: vut_types::SemanticResult,
    diagnostics: Vec<Diagnostic>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
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
            .0
            .write()
            .await
            .insert(doc.uri.clone(), doc.text);
        self.publish(doc.uri).await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.last() {
            self.documents
                .0
                .write()
                .await
                .insert(params.text_document.uri.clone(), change.text.clone());
            self.publish(params.text_document.uri).await;
        }
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .0
            .write()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
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
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        Ok(Some(DocumentSymbolResponse::Nested(symbols(
            &a.file, &text,
        ))))
    }
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        let offset = offset(&text, params.text_document_position_params.position);
        let target = a
            .resolution
            .references
            .iter()
            .find(|r| contains(r.span, offset))
            .map(|r| r.symbol)
            .or_else(|| {
                a.resolution
                    .symbols
                    .iter()
                    .find(|s| contains(s.span, offset))
                    .map(|s| s.id)
            });
        Ok(target.map(|id| {
            GotoDefinitionResponse::Scalar(Location {
                uri,
                range: range(&text, a.resolution.symbols[id.0].span),
            })
        }))
    }
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        let at = offset(&text, params.text_document_position.position);
        let id = a
            .resolution
            .references
            .iter()
            .find(|r| contains(r.span, at))
            .map(|r| r.symbol)
            .or_else(|| {
                a.resolution
                    .symbols
                    .iter()
                    .find(|s| contains(s.span, at))
                    .map(|s| s.id)
            });
        Ok(id.map(|symbol| {
            a.resolution
                .references
                .iter()
                .filter(|r| r.symbol == symbol)
                .map(|r| Location {
                    uri: uri.clone(),
                    range: range(&text, r.span),
                })
                .chain(
                    a.resolution
                        .symbols
                        .iter()
                        .filter(|s| s.id == symbol)
                        .map(|s| Location {
                            uri: uri.clone(),
                            range: range(&text, s.span),
                        }),
                )
                .collect()
        }))
    }
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        let at = offset(&text, params.text_document_position.position);
        let id = a
            .resolution
            .references
            .iter()
            .find(|r| contains(r.span, at))
            .map(|r| r.symbol)
            .or_else(|| {
                a.resolution
                    .symbols
                    .iter()
                    .find(|s| contains(s.span, at))
                    .map(|s| s.id)
            });
        Ok(id.map(|symbol| WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri,
                a.resolution
                    .references
                    .iter()
                    .filter(|r| r.symbol == symbol)
                    .map(|r| TextEdit {
                        range: range(&text, r.span),
                        new_text: params.new_name.clone(),
                    })
                    .chain(
                        a.resolution
                            .symbols
                            .iter()
                            .filter(|s| s.id == symbol)
                            .map(|s| TextEdit {
                                range: range(&text, s.span),
                                new_text: params.new_name.clone(),
                            }),
                    )
                    .collect(),
            )])),
            document_changes: None,
            change_annotations: None,
        }))
    }
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        let at = offset(&text, params.text_document_position_params.position);
        let symbol = a
            .resolution
            .references
            .iter()
            .find(|r| contains(r.span, at))
            .map(|r| r.symbol)
            .or_else(|| {
                a.resolution
                    .symbols
                    .iter()
                    .find(|s| contains(s.span, at))
                    .map(|s| s.id)
            });
        if let Some(id) = symbol {
            let s = &a.resolution.symbols[id.0];
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(signature(&a, id))),
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
        Ok(None)
    }
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        let at = offset(&text, params.text_document_position.position);
        let mut items: Vec<CompletionItem> = a
            .resolution
            .symbols
            .iter()
            .filter(|s| s.public || s.module.0 == 0)
            .map(|s| CompletionItem {
                label: s.name.clone(),
                kind: Some(completion_kind(s.kind)),
                detail: Some(signature(&a, s.id)),
                ..CompletionItem::default()
            })
            .collect();
        if let Some((dot, partial)) = member_context(&text, at)
            && let Some(kind) = receiver_kind_at(&a, dot)
        {
            items.extend(
                builtin_methods(kind)
                    .iter()
                    .filter(|(name, _)| name.starts_with(&partial))
                    .map(|(name, rendered)| CompletionItem {
                        label: (*name).to_owned(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some((*rendered).to_owned()),
                        ..CompletionItem::default()
                    }),
            );
        }
        Ok(Some(CompletionResponse::Array(items)))
    }
    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let a = self.analyze(&uri, &text).await;
        let at = offset(&text, params.text_document_position_params.position);
        let Some(call) = call_context(&text, at) else {
            return Ok(None);
        };
        let Some(kind) = receiver_kind_at(&a, call.dot) else {
            return Ok(None);
        };
        let Some(rendered) = builtin_method_signature(kind, &call.member) else {
            return Ok(None);
        };
        let active = active_parameter(&text, call.open, at);
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: rendered.to_owned(),
                documentation: None,
                parameters: Some(parameter_infos(rendered)),
                active_parameter: Some(active),
            }],
            active_signature: Some(0),
            active_parameter: Some(active),
        }))
    }
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.0.read().await.get(&uri).cloned() else {
            return Ok(None);
        };
        let analysis = self.analyze(&uri, &text).await;
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
fn signature(a: &Analysis, id: vut_resolver::SymbolId) -> String {
    let s = &a.resolution.symbols[id.0];
    let kind = match s.kind {
        ResolverSymbolKind::Function => "fn",
        ResolverSymbolKind::Method => "fn",
        ResolverSymbolKind::Data => "data",
        ResolverSymbolKind::Interface => "interface",
        ResolverSymbolKind::Enum => "enum",
        ResolverSymbolKind::TypeAlias => "type",
        _ => "let",
    };
    if let Some(sig) = a.semantics.function_signatures.get(&id) {
        format!(
            "{kind} {}({}) -> {:?}",
            s.name,
            sig.parameters
                .iter()
                .map(|p| format!("{:?}", a.semantics.types[p.0]))
                .collect::<Vec<_>>()
                .join(", "),
            a.semantics.types[sig.result.0]
        )
    } else {
        format!("{kind} {}", s.name)
    }
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

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
