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

Authentication, publishing automation, and yanking require finalized semantics. This site does not present them as working upload services. See [Publishing](/docs/packages/publishing/).
