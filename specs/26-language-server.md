# Vut Language Server

`vut-lsp` is the official long-running LSP process. Its analysis pipeline is the compiler frontend: lexer, parser, resolver, and type checker. VS Code clients must not duplicate semantic rules.

Open documents are kept in memory and analyzed after LSP document notifications. Diagnostics retain compiler error codes and byte spans are converted to LSP UTF-16 ranges. Formatting delegates to `vut_tooling::format_source`.

The initial server publishes compiler diagnostics and provides formatting, document symbols, definitions, references, safe symbol rename, hover, completion, and semantic tokens. Multi-file project analysis is the next extension point: the server process owns document state so it can cache a source map and module graph without changing the VS Code protocol surface.

Syntax highlighting covers the accepted surface syntax, including nested template
interpolation, using standard TextMate scopes without requiring an LSP connection.
Semantic tokens refine identifiers whose roles are known to the resolver. They
must not blanket-classify unresolved identifiers as properties, or cover lexical
escapes/interpolation with a single string token. Token ranges are non-overlapping,
single-line UTF-16 ranges; synthetic declarations must not color unrelated source.
