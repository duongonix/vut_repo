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

```text
vpm new
vpm init

vpm add <package>[@version]
vpm remove <package>
vpm install [<package>[@version]]
vpm uninstall <package>
vpm exec <package> [--bin <name>] [-- <args>]
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

vpm publish [<registry>] [--dry-run]
```

`vpm login` and `vpm yank` remain reserved until their semantics are finalized.

`vpm install` is context-sensitive:

```text
vpm install            install the current project's dependencies
vpm install <package>  install a published CLI package globally
```

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
│   └── bin/
│       └── hello.vut
├── tests/
├── vpm.toml
└── .gitignore
```

`vpm.toml` declares the entry point:

```toml
[package]
name = "hello"
version = "0.1.0"

[dependencies]

[[bin]]
name = "hello"
path = "src/bin/hello.vut"
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

Do not necessarily delete the package from global `~/.vut/packages`.

---

# 8. `vpm install`

Without a package argument, installs dependencies described by project state.

With a valid lockfile:

```text
use locked exact versions
```

Do not upgrade dependencies unexpectedly.

Use pinned lockfile data (source, version, revision, checksums, native
artifacts); do not re-resolve. If packages already exist locally and validate
correctly, reuse them.

With a package argument, installs a published CLI package globally:

```text
vpm install math
vpm install math@1.0.0
```

The package's `[[bin]]` entries are built for the host target and installed into
`~/.vut/bin`. Global binary-name collisions are detected and reported.

---

# 8a. `vpm uninstall`

Removes a globally installed CLI package:

```text
vpm uninstall math
```

It removes the installed binary/binaries and their install metadata. It does not
remove the project manifest entry for the same name.

---

# 8b. `vpm exec`

Runs a published CLI package without pre-installing it:

```text
vpm exec math
vpm exec math --bin math-tool -- arg1 arg2
```

Bin selection:

```text
1. [[bin]] whose name equals the package name
2. otherwise, if the package declares exactly one [[bin]], run it
3. otherwise, error and list available bin names
```

Never silently run the first bin. `--bin <name>` overrides the selection.

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

```text
vpm publish                          publish to the default registry (duongonix/vpm)
vpm publish alice/vut-packages       publish to a self-hosted registry
vpm publish --dry-run                validate + report without submitting
```

The package name is taken from `[package].name`; it is never supplied again on
the command line.

Publication is a review-gated, source-only flow:

```text
validate manifest + layout + version + native build source
→ submit a change/PR to the target registry for review
→ trusted CI builds native artifacts per target
→ trusted CI uploads artifacts and generates native-artifacts.toml
```

Publishers must not submit arbitrary prebuilt binaries as official artifacts.

Provider and authentication must be abstracted (GitHub is not privileged), and
credentials must never be written to the manifest, lockfile, or package source.

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
├── uninstall.rs
├── exec.rs
├── publish.rs
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

Handlers may be grouped by responsibility (for example one module for global
package install/uninstall/exec and one for publishing) rather than one file per
command.

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
8. `install` without an argument respects pinned lockfile data.
9. `install <package>` installs published CLI packages into `~/.vut/bin`.
10. `exec` never silently selects a bin; ambiguous selection is an error.
11. `outdated` does not modify project state.
12. `build/run/check` orchestrate compiler APIs rather than duplicate compiler logic.
13. Project file modifications should be atomic.
14. Commands avoid unnecessary network access.
15. `publish` is source-only, review-gated, `--dry-run`-capable, and provider-abstract.
16. `login`/`yank` semantics must not be invented.
17. Errors are structured and actionable.
