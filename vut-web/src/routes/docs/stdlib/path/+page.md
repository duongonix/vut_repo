---
title: Paths
description: 'Pure, target-aware path manipulation.'
section: Standard library
order: 21
---

## Responsibility

The `path` module owns lexical operations such as joining paths, finding a parent or extension, testing absolute paths, and normalizing components. Pure operations do not touch the filesystem.

## Normalization is not canonicalization

Lexical normalization does not resolve symbolic links and is not equivalent to canonical filesystem resolution. Target-platform roots and separator rules must remain meaningful.

## API availability

The specification gives conceptual signatures such as `path.join(base: str, child: str) -> str`. Confirm the installed stdlib's exact API before relying on optional or proposed forms.
