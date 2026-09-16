---
title: vpm.toml
description: 'The project and package manifest.'
section: Packages
order: 14
---

## Package metadata

```toml title="vpm.toml"
[package]
name = "hello"
version = "0.1.0"

[dependencies]
```

The package name and a valid semantic version are required. Manifest versions omit a leading `v`, even if remote directories use one.

## Dependencies

A compact dependency maps its import namespace to an exact version. The following is a format illustration, not a claim that this example package is available:

```toml
[dependencies]
math = "1.2.0"
```

## Aliases

The dependency table key is the import namespace. The explicit `package` field preserves package identity when an alias is used:

```toml
[dependencies.webhttp]
package = "http"
version = "1.2.0"
```

The import is `import webhttp`. See [Dependencies](/docs/packages/dependencies/) before editing resolved versions.
