---
title: Project structure
description: 'Understand source files, modules, and the role of VPM.'
section: Getting started
---

## A standard project

The VPM specification defines this starting structure:

```text
hello/
  src/
    main.vut
  tests/
  vpm.toml
  vpm.lock
  .gitignore
```

## Source files

Application source lives under `src/`. Each `.vut` file is automatically a module. No module declaration is needed.

The default compiler entry point is `src/main.vut`.

## Project metadata

`vpm.toml` holds project configuration. `vpm.lock` records the resolved dependency state.

VPM owns package workflows, while the compiler consumes the resolved sources.

## Build output

The compiler specification separates debug and release output under `build/`.

```bash
vut build
vut build --release
```

Read [Modules](/docs/language/modules/) to learn how files and directories become import paths.
