---
title: Introduction
description: 'Meet Vut: a concise, statically typed language built for real-world development.'
section: Getting started
---

## A little less ceremony

Vut is a compiled programming language with indentation-based blocks, static typing, and type inference. Source files use the `.vut` extension.

The goal is straightforward: keep programs readable while making types and behavior predictable.

```vut title="main.vut"
fn main():
  name: str = "World"
  out("Hello, $name!")
```

## Designed to be readable

- **Concise syntax.** Assign a value to declare a variable.
- **Static types.** Inferred types stay fixed after the first assignment.
- **Explicit structure.** Indentation defines blocks; a colon begins each block.
- **Native compilation.** The compiler builds native executables.

## Your first steps

Start with [Installation](/docs/getting-started/installation/), then write your [first Vut program](/docs/getting-started/hello-world/).

If you are exploring the language, read about [variables](/docs/language/variables/), [types](/docs/language/types/), and [functions](/docs/language/functions/).

## The toolchain

The `vut` compiler focuses on two commands: `run` and `build`. [VPM](/docs/tooling/vpm/) manages projects and packages.

> **Note:** Vut is under active development. These guides describe the language specification; platform availability and individual tooling features may evolve.
