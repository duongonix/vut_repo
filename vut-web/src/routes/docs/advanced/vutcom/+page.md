---
title: Vutcom
description: 'Typed declarative composition, with meaning supplied by libraries.'
section: Advanced
order: 3
---

## Composition domains

Vutcom provides language-level composition and domain safety. It is not a UI framework, renderer, virtual DOM, router, or workflow engine.

```vut
composition UI
composition Build
```

These declarations introduce distinct domains: `vutcom(UI)` and `vutcom(Build)` are different types.

## Ordinary functions

A composition-producing function is an ordinary function with a `vutcom(D)` return type. Libraries supply primitives and consumers; the compiler does not hardcode names such as `Column`, `Text`, or `Pipeline`.

The following is a conceptual library example, not a standalone program: it requires a library defining the UI domain and its primitives.

```vut
fn Home() -> vutcom(UI):
  Column():
    Text(value = "Hello")
```

## Opaque values

Do not depend on `.nodes`, `.children`, `.props`, or a particular tree representation. The runtime representation is intentionally opaque.

## Domain safety

There is no implicit conversion between domains. A consumer of `vutcom(UI)` cannot receive `vutcom(Build)`. Cross-domain composition requires an explicit ordinary typed parameter.
