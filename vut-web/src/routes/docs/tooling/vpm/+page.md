---
title: VPM
description: 'The project and package workflow around the Vut compiler.'
section: Tooling
order: 6
---

## Create a project

```bash
vpm new hello
```

The specified layout contains `src/main.vut`, `tests/`, `vpm.toml`, and `.gitignore`. Use `vpm init` to initialize an existing directory without replacing user files.

## Project commands

| Command     | Responsibility                                       |
| ----------- | ---------------------------------------------------- |
| `vpm run`   | Resolve project dependencies, compile, and execute.  |
| `vpm build` | Prepare dependencies and produce native output.      |
| `vpm check` | Check the program without a normal final executable. |
| `vpm test`  | Run project tests.                                   |
| `vpm fmt`   | Format Vut source.                                   |
| `vpm lint`  | Analyze source quality.                              |

Use the help provided by your installed VPM build for available options. These commands document the specified workflow, not the availability of a public downloadable release.

## Dependencies

`add` and `remove` change direct dependencies. `install` uses locked exact versions; `update` intentionally resolves newer versions. `tree` displays the dependency graph and `outdated` reports newer versions without modifying the project.

See [Packages](/docs/packages/overview/) and the [manifest](/docs/packages/vpm-toml/).

## Publishing status

The current source implements `vpm publish [registry] [--dry-run]` as a review-gated source submission. See [Publishing](/docs/packages/publishing/) for credentials, dry-run, and safety requirements. No registry uptime or yanking API is implied.

## CLI packages

`vpm install package[@version]` installs a published CLI globally; bare `vpm install` resolves project dependencies. `vpm uninstall package` removes a global package. `vpm exec package --bin name -- arguments` executes a package binary without permanently installing it. These operations can fetch and run package code; use trusted sources.

`vpm doc`, `clean`, `search query`, and `info package` round out the command surface. `vpm test [filter] --release` selects tests and release mode; `vpm fmt --check` checks formatting without rewriting source. Global `--quiet` and `--color` control output.
