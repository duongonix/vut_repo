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

## API

```vut
import path

fn main():
  file = path.join("docs", "guide.vut")
  out(path.normalize(file))
  suffix = path.extension(file)
  if suffix != null:
    out(suffix)
```

| Operation                                                              | Return      |
| ---------------------------------------------------------------------- | ----------- |
| `separator()`                                                          | `str`       |
| `is_absolute(value)`, `is_relative(value)`                             | `bool`      |
| `components(value)`                                                    | `list[str]` |
| `join(base, child)`, `normalize(value)`                                | `str`       |
| `parent(value)`, `file_name(value)`, `extension(value)`, `stem(value)` | `str?`      |
| `with_extension(value, suffix)`                                        | `str`       |

All path parameters are strings. Handle `null` when no requested component exists. Lexical normalization is not a security boundary: validate filesystem access separately, particularly with untrusted paths and symlinks.
