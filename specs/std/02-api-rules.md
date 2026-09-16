# Vut Standard Library — API Rules

## 1. General API Philosophy

Stdlib APIs must be:

* predictable;
* concise;
* strongly typed;
* consistent between modules;
* explicit about failure;
* cross-platform where practical.

Prefer obvious APIs over clever APIs.

---

## 2. Naming

Use lowercase module names:

```vut
import fs
import path
import os
import time
```

Functions and methods use descriptive lowercase names:

```vut
fs.exists(path)
fs.read(path)
fs.write(path, data)
fs.remove_file(path)
```

Types use the normal Vut type naming convention.

Error types should clearly identify their domain:

```text
FsError
PathError
ProcessError
```

Avoid unnecessary abbreviations.

---

## 3. Failure

Expected runtime failures must use `result(T, E)`.

Example:

```vut
fs.read(path) -> result(bytes, FsError)
```

Do not represent normal errors using:

```text
panic
magic integer
empty string
empty bytes
null
false when the caller needs the reason
```

Boolean APIs are appropriate when the API genuinely represents a boolean query.

Example:

```vut
fs.exists(path) -> bool
```

---

## 4. Optional Values

Use optional values only when absence is a normal state rather than an error.

Conceptually:

```vut
value: T?
```

Do not use optional values to hide real I/O or OS errors.

---

## 5. Error Types

Each major subsystem may define a domain-specific error type.

Conceptually:

```vut
enum FsErrorKind:
  not_found
  permission_denied
  already_exists
  invalid_input
  io
  unknown

data FsError:
  kind: FsErrorKind
  message: str
```

Exact error types may evolve as implementation progresses, but errors must remain typed and deterministic.

Native OS errors must be translated into stable Vut-level errors.

Do not expose Rust error objects directly.

---

## 6. Result and `?`

Stdlib Result APIs must work naturally with Vut's `?` operator.

Example:

```vut
fn load() -> result(bytes, FsError):
  data = fs.read("config.bin")?
  ok(data)
```

Do not create a separate error propagation mechanism specifically for stdlib.

---

## 7. Ownership

Stdlib APIs must follow Vut's normal value semantics.

Users should not need to manually free:

```text
str
bytes
list(T)
map(K, V)
stdlib data
```

Native implementations must respect the compiler/runtime ownership model.

No public safe stdlib API may require manual `free`.

---

## 8. Managed Values

Managed Vut values must not be passed directly through raw C ABI unless a stable ABI representation has explicitly been defined.

Examples include:

```text
str
bytes
list(T)
map(K, V)
result(T, E)
```

Use appropriate raw representations at native boundaries.

---

## 9. Path APIs

Path manipulation should normally belong to `path`.

Filesystem access belongs to `fs`.

For example:

```text
path.join(...)
path.extension(...)
path.parent(...)

fs.exists(...)
fs.read(...)
fs.write(...)
```

Do not mix pure path manipulation with filesystem I/O unnecessarily.

---

## 10. Filesystem Direction

The `fs` module should eventually cover at least:

```text
exists
read
write
create_dir
create_dir_all
remove_file
remove_dir
remove_dir_all
copy
rename
metadata
read_dir
```

More advanced abstractions may include:

```text
File
OpenOptions
Metadata
DirectoryEntry
Permissions
```

Do not implement all advanced APIs before basic filesystem operations are stable.

---

## 11. OS and Environment

`os` should represent operating-system information.

Examples:

```text
OS name
architecture
home directory
temporary directory
```

Canonical OS values should be stable, for example:

```text
windows
linux
macos
```

Canonical architecture values should be stable, for example:

```text
x86_64
aarch64
```

Environment-variable operations may live in `env` rather than overloading `os`.

---

## 12. Time

`time` should provide stable concepts such as:

```text
Duration
Instant
system time
sleep
```

Do not couple the API directly to platform-specific clock representations.

---

## 13. Process

`process` should eventually support:

```text
process creation
arguments
environment
working directory
exit status
stdin
stdout
stderr
```

High-level process APIs must not expose raw platform handles unless explicitly using a low-level API.

---

## 14. Async

Async stdlib APIs must follow the official Vut `async` / `await` specs.

Do not assume:

```text
async = multithreading
```

Do not introduce thread pools, workers, channels, or parallel execution simply to implement an async API.

Concurrency features must be implemented only when their own specs require them.

---

## 15. API Stability

Once an API is documented and implemented as canonical, do not silently change:

```text
function name
argument order
return type
error behavior
ownership semantics
platform semantics
```

If implementation reveals a design problem, update the relevant specs intentionally before changing canonical behavior.

---

## 16. Avoid API Duplication

Do not provide many aliases for the same operation.

Prefer:

```vut
fs.read(path)
```

over multiple equivalent names such as:

```text
fs.read
fs.read_file
fs.load
fs.load_file
```

unless the functions genuinely have different semantics.

---

## 17. Platform Consistency

A public stdlib API should behave as consistently as reasonably possible across:

```text
Windows
Linux
macOS
```

When unavoidable platform differences exist:

* document them;
* isolate them;
* return appropriate typed errors.

Do not silently behave completely differently on different operating systems.
