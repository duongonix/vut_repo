---
title: Language server
description: 'Editor intelligence backed by the compiler frontend.'
section: Tooling
order: 12
---

## The server

`vut-lsp` is the official long-running language-server process. Open documents are analyzed in memory after document notifications, using the lexer, parser, resolver, and type checker.

## Editor features

The initial server provides compiler diagnostics, formatting, document symbols, definitions, references, safe symbol rename, hover, completion, and semantic tokens. Multi-file project analysis is an extension point rather than a blanket promise of complete workspace analysis.

## Highlighting

TextMate highlighting works without an LSP connection, including nested string interpolation. Semantic tokens refine identifiers whose roles are known to the resolver; they must not hide lexical escapes or interpolation inside a single string token.

## Setup

Use an editor client configured to launch the `vut-lsp` executable supplied with your toolchain. A verified marketplace installation link has not been configured for this website.
