---
title: Filesystem
description: 'Typed file and directory operations.'
section: Standard library
order: 20
---

## Responsibility

The specified `fs` surface covers files, directories, metadata, permissions, and filesystem operations. `fs.read(path)` returns `result[bytes, FsError]`; `fs.read_str(path)` returns `result[str, FsError]` and must validate UTF-8 rather than silently replacing invalid bytes.

## Error handling

Expected filesystem failures are typed results. Permission errors must not be mistaken for absence where correctness requires distinguishing them. Native errors map into stable Vut errors.

## Read text

```vut
import fs

fn main():
  match fs.read_str("notes.txt"):
    ok(text): out(text)
    err(error): out(error.message)
```

This example reads a local file and reports a typed error if it cannot be read.

## File and directory functions

| Operations                                                      | Successful payload        |
| --------------------------------------------------------------- | ------------------------- |
| `read(path)`, `read_str(path)`                                  | `bytes`, `str`            |
| `write(path, bytes)`, `write_str(path, text)`                   | `unit`                    |
| `append(path, bytes)`, `append_str(path, text)`                 | `unit`                    |
| `create_dir(path)`, `create_dir_all(path)`                      | `unit`                    |
| `remove_file(path)`, `remove_dir(path)`, `remove_dir_all(path)` | `unit`                    |
| `copy(from, to)`, `rename(from, to)`                            | `u64` byte count, `unit`  |
| `read_dir(path)`                                                | `list[DirectoryEntry]`    |
| `metadata(path)`, `permissions(path)`                           | `Metadata`, `Permissions` |
| `set_permissions(path, permissions)`                            | `unit`                    |

These operations return `result[payload, FsError]`. `exists`, `is_file`, and `is_dir` return Boolean convenience checks; use a fallible operation when error distinctions matter. Recursive removal is destructive: validate its target explicitly.

## Open files

Import `File` with `import fs at File`. `File.open(location)` and `File.create(location)` return `result[File, FsError]`. `OpenOptions.open(location)` provides configurable opening.

File methods include `path()`, `metadata()`, `read()`, `write(blob)`, `flush()`, and `sync()`. Stream methods use `io.IoError`; writes return the number of bytes written. Use I/O helpers for full-buffer writes. Directory entries expose `name`, `path`, `is_file`, `is_dir`, and fallible `metadata`.
