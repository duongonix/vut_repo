# Vut Standard Library — Roadmap

## 1. Goal

Build the Vut standard library incrementally from foundational APIs toward higher-level system functionality.

Do not attempt to implement the entire stdlib simultaneously.

Each phase must leave the repository buildable and in a usable state.

---

# Phase 1 — Stdlib Foundation

Establish:

```text
vut-stdlib repository structure
std/ module discovery
_internal convention
local stdlib resolution
VUT_STDLIB_PATH
basic stdlib compilation
```

Verify a minimal Vut stdlib function can be imported and executed.

---

# Phase 2 — Native Runtime Foundation

Establish:

```text
native/vut-runtime
Rust staticlib
C ABI exports
vut_rt_* symbol convention
automatic runtime linking
VUT_RUNTIME_PATH
target resolution
```

Create a trivial native test symbol and verify:

```text
Vut
→ C ABI
→ Rust
→ return to Vut
```

---

# Phase 3 — Native Data Bridge

Establish reusable ABI rules for:

```text
str input
bytes input/output
native buffers
status/error codes
ownership transfer
cleanup
```

Do not implement large native modules until buffer ownership is clear.

---

# Phase 4 — `path`

Implement foundational path manipulation.

Target areas:

```text
join
parent
file name
extension
stem
absolute/relative helpers
normalization where appropriate
```

Keep pure path logic in Vut whenever practical.

---

# Phase 5 — Basic `fs`

Start with simple native operations:

```text
exists
create_dir
create_dir_all
remove_file
remove_dir
```

Validate the full:

```text
Vut API
→ native bridge
→ Rust
→ OS
```

pipeline.

---

# Phase 6 — Complete `fs`

After buffer and error ABI is stable, implement:

```text
read
write
copy
rename
remove_dir_all
metadata
read_dir
```

Then implement useful abstractions as appropriate:

```text
File
OpenOptions
Metadata
DirectoryEntry
Permissions
```

Keep high-level behavior in Vut where possible.

---

# Phase 7 — `os`

Implement operating-system information.

Target functionality:

```text
OS name
architecture
home directory
temporary directory
other essential platform information
```

Use stable canonical values such as:

```text
windows
linux
macos

x86_64
aarch64
```

---

# Phase 8 — `env`

Implement environment support:

```text
get
exists
set
remove
```

Also support access to process environment where appropriate.

---

# Phase 9 — `io`

Establish common I/O abstractions needed by other stdlib modules.

Avoid unnecessary abstraction.

Only introduce stream abstractions when they provide real reuse between:

```text
files
processes
terminal
network
```

---

# Phase 10 — `time`

Implement:

```text
Duration
Instant
system time
sleep
```

Keep platform clock details hidden behind the native layer.

---

# Phase 11 — `process`

Implement:

```text
spawn
arguments
environment
working directory
exit status
stdin
stdout
stderr
```

Use explicit Result-based errors.

---

# Phase 12 — Utility Modules

Implement foundational utility modules:

```text
math
random
ffi
```

Then add other utility modules only when needed:

```text
cmp
hash
fmt
unicode
ascii
encoding
collections
iter
```

Do not add modules merely to make the stdlib appear larger.

---

# Phase 13 — Networking Foundation

When the core system stdlib is stable, implement low-level networking:

```text
net
TCP
UDP
address resolution
basic socket abstractions
```

WebSocket belongs in a VPM package.

`http` is an official stdlib module (`import http`) specified by
`specs/std/http.md`; it is not part of this networking phase and is not a VPM
package.

---

# Phase 14 — Concurrency Integration

Only begin this phase when Vut's corresponding language/runtime specs are ready.

Possible modules:

```text
thread
sync
channel
task
```

Do not assume `async` requires multithreading.

If current Vut async/await is single-threaded, preserve that model until concurrency specs explicitly introduce additional behavior.

---

# Phase 15 — Terminal and System Integration

Implement where appropriate:

```text
terminal
signal
```

Keep platform-specific details isolated.

---

# Phase 16 — Full Stdlib Hardening

After implementation phases are complete, perform comprehensive testing and cleanup.

This is where exhaustive testing belongs.

Cover:

```text
unit tests
integration tests
E2E tests
regression tests
error paths
edge cases
ownership/drop
memory leaks
invalid UTF-8
large buffers
empty buffers
platform differences
sanitizers where available
fuzzing where useful
Windows
Linux
macOS
```

Review public API consistency across all modules.

Remove:

```text
temporary workarounds
dead code
duplicate implementations
obsolete APIs
unnecessary native code
```

---

# Testing Between Phases

During Phases 1–15, do not spend excessive time writing exhaustive tests.

Use lightweight verification:

```text
implement phase
↓
compile
↓
basic functional test
↓
verify no obvious regression
↓
continue
```

At minimum verify:

* modified code compiles;
* relevant workspace components build;
* basic functionality works;
* compiler/runtime does not obviously crash;
* important existing tests remain functional.

Do not attempt complete edge-case coverage after every intermediate phase.

**Comprehensive testing belongs to Phase 16.**

---

# Completion Rule

A phase is complete when:

1. its required functionality exists;
2. architecture follows the stdlib specs;
3. code is appropriately modular;
4. basic verification passes;
5. no known critical issue prevents the next phase.

Update roadmap progress after completing each phase.

Do not skip foundational phases merely because a later feature is more interesting.

The intended progression is:

```text
foundation
→ ABI
→ data bridge
→ path
→ fs
→ os/env/io
→ time/process
→ utilities
→ networking
→ concurrency
→ system integration
→ full hardening
```
