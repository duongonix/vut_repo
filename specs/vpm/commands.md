# VPM Commands

## 1. Purpose

This document defines the VPM command surface and command responsibilities.

VPM is the project/package workflow tool.

The compiler CLI:

```text
vut
```

remains intentionally minimal.

---

# 2. CLI Architecture

Use:

```text
clap
```

for command/argument parsing.

The CLI layer must remain thin.

Conceptually:

```text
CLI
 ↓
Command Handler
 ↓
VPM Core
```

Do not implement package-manager logic directly inside `main.rs`.

---

# 3. Command Set

Current command direction:

```text
vpm new
vpm init

vpm add
vpm remove
vpm install
vpm update

vpm tree
vpm outdated

vpm run
vpm build
vpm check
vpm test

vpm fmt
vpm lint
vpm doc
vpm clean

vpm search
vpm info

vpm login
vpm publish
vpm yank
```

Publishing-related commands remain dependent on finalized publishing semantics.

Do not fake unsupported behavior.

---

# 4. `vpm new`

Creates a new Vut project.

Example:

```text
vpm new hello
```

Recommended initial structure:

```text
hello/
├── src/
│   └── main.vut
├── tests/
├── vpm.toml
└── .gitignore
```

The command should fail safely if the destination already contains conflicting data.

Do not overwrite user files silently.

---

# 5. `vpm init`

Initializes the current directory as a Vut project.

Example:

```text
vpm init
```

It creates missing project metadata/files without destructively replacing existing user content.

---

# 6. `vpm add`

Registry:

```text
vpm add math
vpm add math@latest
vpm add math@1.2.0
```

GitHub:

```text
vpm add nam/abc/math
vpm add nam/abc/math@1.2.0
```

GitLab:

```text
vpm add gitlab:nam/abc/math
vpm add gitlab:nam/abc/math@1.2.0
```

Responsibilities:

```text
parse package reference
resolve version
validate namespace
resolve dependencies
download/install if needed
update vpm.toml
update vpm.lock
```

Manifest/lockfile changes should happen only after successful resolution.

Avoid leaving half-applied project state.

---

# 7. `vpm remove`

Example:

```text
vpm remove math
```

Responsibilities:

```text
remove direct dependency from manifest
re-resolve graph
remove obsolete lock entries
preserve unrelated dependencies
```

Do not necessarily delete the package from global `~/.vpm/packages`.

---

# 8. `vpm install`

Installs dependencies described by project state.

With a valid lockfile:

```text
use locked exact versions
```

Do not upgrade dependencies unexpectedly.

If packages already exist locally and validate correctly, reuse them.

---

# 9. `vpm update`

Checks/resolves newer package versions according to VPM's version semantics.

Because general version ranges are currently deferred, update behavior must remain consistent with the exact-version manifest model.

Do not invent Cargo/npm range semantics.

When a dependency is intentionally updated:

```text
manifest
lockfile
local package state
```

must remain consistent.

---

# 10. `vpm tree`

Displays the resolved dependency graph.

Conceptual output:

```text
hello 0.1.0
├── math 1.2.0
│   └── core 1.0.0
└── json 2.1.0
```

Output ordering must be deterministic.

---

# 11. `vpm outdated`

Checks whether newer stable versions exist.

Conceptual output:

```text
Package   Current   Latest
math      1.2.0     1.4.0
json      2.1.0     2.1.0
```

This command may require fresh provider metadata.

It must not modify the project.

---

# 12. `vpm run`

Project-level run workflow.

Responsibilities:

```text
discover project
resolve dependencies
ensure dependencies available
prepare compiler source roots
invoke Vut compiler
run resulting program
forward program arguments
```

VPM orchestrates.

The compiler performs compilation.

---

# 13. `vpm build`

Project-level build workflow.

Responsibilities:

```text
load project
load lockfile
resolve/validate dependencies
prepare dependency source roots
invoke compiler
manage build configuration
```

Do not duplicate compiler internals inside VPM.

---

# 14. `vpm check`

Runs semantic/compiler checking without producing the normal final executable where compiler architecture supports it.

Although `vut` exposes only:

```text
run
build
```

VPM may invoke compiler-library APIs directly.

Do not require adding a public:

```text
vut check
```

command.

---

# 15. `vpm test`

Discovers and runs Vut project tests according to:

```text
specs/17-testing.md
```

Testing semantics belong to the test system, not command parsing.

---

# 16. `vpm fmt`

Formats Vut source according to:

```text
specs/18-formatter-linter.md
```

The command should use formatter library APIs.

Do not embed formatter implementation in VPM.

---

# 17. `vpm lint`

Runs Vut lint analysis.

Linter implementation remains separate from command handling.

---

# 18. `vpm doc`

Generates project/package documentation when the documentation subsystem exists.

Do not invent undocumented language documentation syntax.

---

# 19. `vpm clean`

Removes generated project build artifacts/caches according to defined scope.

It must not delete:

```text
source files
vpm.toml
vpm.lock
```

It must not indiscriminately erase the entire global VPM package store.

---

# 20. `vpm search`

Searches the default VPM registry/package index model.

Because the default registry is GitHub repository-based rather than a dedicated registry server, implementation should use available repository metadata/provider capabilities.

Exact search quality/indexing may evolve.

Do not introduce a central VPM web service merely to implement this command unless architecture is explicitly changed.

---

# 21. `vpm info`

Displays package information.

Examples:

```text
vpm info math
vpm info nam/abc/math
```

Potential output:

```text
name
available versions
latest stable version
source
selected package metadata
```

Only display metadata actually known from package/provider data.

---

# 22. `vpm login`

Reserved for authentication workflows if required by:

```text
GitHub
GitLab
future publishing
private packages
```

Authentication design must be finalized before this command becomes stable.

Secrets must not be written to project manifests or lockfiles.

---

# 23. `vpm publish`

Publishing is unusual under VPM's Git repository/folder model because packages are represented directly in repository directories.

Exact publishing automation must be specified before implementation.

Do not assume a central package-upload server exists.

---

# 24. `vpm yank`

Yanking semantics are not currently finalized.

A Git-folder package registry has different constraints from centralized registries.

Do not implement `yank` until metadata and resolver behavior are explicitly defined.

---

# 25. Project Discovery

Commands requiring a project should locate:

```text
vpm.toml
```

using one shared project-discovery implementation.

Do not duplicate directory traversal in every command.

---

# 26. Exit Codes

Commands should return consistent process exit status.

Conceptually:

```text
0 success
non-zero failure
```

More detailed exit-code policy may be defined later.

Deep library code must not call:

```text
std::process::exit
```

Command/CLI boundary owns process termination.

---

# 27. Diagnostics

VPM uses the same visual philosophy as the Vut compiler:

```text
clear
structured
actionable
```

But package/network errors must not fabricate source-code frames.

Example:

```text
error: package version not found

package:
  math

requested:
  2.0.0

source:
  default registry

available:
  1.8.0
  1.9.0
```

---

# 28. Atomic Project Modification

Commands modifying:

```text
vpm.toml
vpm.lock
```

must avoid partial state.

For `vpm add`, conceptually:

```text
resolve
↓
validate
↓
download/verify
↓
prepare manifest changes
↓
prepare lockfile
↓
commit project changes
```

Failure before commit should leave the previous valid project state intact where practical.

---

# 29. Network Behavior

Commands should only access the network when required.

Examples:

Usually local:

```text
vpm build
vpm run
```

when all locked dependencies are available.

Usually remote-aware:

```text
vpm add
vpm update
vpm outdated
vpm search
vpm info
```

depending on cached metadata and requested operation.

---

# 30. Command Modules

Recommended internal structure:

```text
commands/
├── new.rs
├── init.rs
├── add.rs
├── remove.rs
├── install.rs
├── update.rs
├── tree.rs
├── outdated.rs
├── run.rs
├── build.rs
├── check.rs
├── test.rs
├── fmt.rs
├── lint.rs
├── doc.rs
├── clean.rs
├── search.rs
└── info.rs
```

Publishing commands may be added only when implemented.

---

# 31. Command Context

Shared state may be represented through a structured context.

Conceptually:

```text
VpmContext
├── configuration
├── project
├── local store
├── cache
├── providers
└── diagnostics
```

Do not use uncontrolled global mutable state.

---

# 32. Performance

Commands should avoid repeatedly:

```text
loading the same manifest
scanning the same project
querying the same provider
hashing unchanged packages
discovering the same compiler
```

Load once and pass structured state.

---

# 33. Rules

1. `vpm` owns project/package workflows.
2. `vut` remains compiler-focused.
3. CLI parsing uses a proven library.
4. `main.rs` remains thin.
5. Each command has a focused handler.
6. Project discovery is shared.
7. `add` supports registry/GitHub/GitLab package syntax.
8. `install` respects lockfile resolution.
9. `outdated` does not modify project state.
10. `build/run/check` orchestrate compiler APIs rather than duplicate compiler logic.
11. Project file modifications should be atomic.
12. Commands avoid unnecessary network access.
13. Publishing/yanking semantics must not be invented.
14. Errors are structured and actionable.
