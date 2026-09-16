---
title: Linter
description: 'Find suspicious legal code without weakening compiler checks.'
section: Tooling
order: 11
---

## Usage

```bash
vpm lint
```

The linter reuses the compiler frontend. Its job is to report suspicious, redundant, unclear, or error-prone code, not to replace mandatory language checks.

## Typical findings

Unused variables and imports, unreachable branches, and redundant conditions are lint concerns. Invalid types, unknown symbols, and invalid imports remain compiler errors.

## Diagnostics and fixes

Warnings use stable `W####` codes where practical and should preserve source locations. Only apply automatic fixes that reliably preserve semantics. Check your installed VPM help before relying on a particular fix flag or lint policy.
