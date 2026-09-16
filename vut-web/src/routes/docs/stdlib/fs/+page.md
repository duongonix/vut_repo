---
title: Filesystem
description: 'Typed file and directory operations.'
section: Standard library
order: 20
---

## Responsibility

The specified `fs` surface covers files, directories, metadata, permissions, and filesystem operations. `fs.read(path)` returns `result(bytes, FsError)`; `fs.read_str(path)` returns `result(str, FsError)` and must validate UTF-8 rather than silently replacing invalid bytes.

## Error handling

Expected filesystem failures are typed results. Permission errors must not be mistaken for absence where correctness requires distinguishing them. Native errors map into stable Vut errors.

## API availability

These are specified contracts, not verification of an installed stdlib release. Exact imports and release-tested examples will be added when distribution details are confirmed.
