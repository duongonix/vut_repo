---
title: Dependencies
description: 'Resolve deliberately and keep manifests and lockfiles consistent.'
section: Packages
order: 15
---

## Exact versions

Normal manifests store concrete versions. Do not assume npm- or Cargo-style version ranges. VPM resolves provider metadata and records exact state in `vpm.lock`.

## Install versus update

`vpm install` reuses valid locked versions and locally available packages. It must not silently upgrade dependencies. `vpm update` intentionally resolves newer versions according to the package model.

## Repository sources

A GitHub package source uses `owner/repository/package-path`; a GitLab source prefixes it with `gitlab:`. Package paths may be nested. The dependency key supplies the import namespace.

## Safe changes

`add` and `remove` must keep the manifest and lockfile consistent. Failed resolution must not leave half-applied project state. Use `vpm tree` to inspect the resulting dependency graph.
