---
title: Modules
description: 'Files are modules. Directories organize them into namespaces.'
section: Language
---

## Files as modules

Every `.vut` file is automatically a module. The file `src/math.vut` defines the module `math`; no declaration is required.

## Directory namespaces

Directories form namespaces. A file at `src/app/user.vut` corresponds to `app.user`.

The module path is relative to the source root, normally `src/`.

## Module resolution

For a local import segment named `name`, resolution checks:

1. `name.vut`
2. `name/mod.vut`

If both exist, `name.vut` wins. A directory becomes an importable module object when it contains `mod.vut`.

## Visibility

Identifiers beginning with `_` are private to their module. Other identifiers are public by default.

Vut does not require `pub`, `public`, `private`, or `export` keywords.
