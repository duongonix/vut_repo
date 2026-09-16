---
title: Packages
description: 'Project metadata, exact versions, and VPM-managed dependencies.'
section: Packages
order: 13
---

## Project files

VPM manages the `vpm.toml` manifest and `vpm.lock` resolution state. The compiler consumes already-resolved dependencies; it does not download packages itself.

## Sources

The package model supports a default registry source and repository-based GitHub and GitLab sources. Source identity and import namespace are separate concerns.

## Start here

Read the [manifest format](/docs/packages/vpm-toml/) before adding dependencies. [Dependency resolution](/docs/packages/dependencies/) explains exact versions and lockfile behavior.

Publishing automation and yanking are not finalized services; see [Publishing](/docs/packages/publishing/).
