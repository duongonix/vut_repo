---
title: Publishing
description: 'Current status of publishing and registry workflows.'
section: Packages
order: 16
---

## Status

Publishing automation, authentication workflows, and yanking semantics are not finalized in the VPM command specification. No package-upload server or working publishing endpoint is implied by this website.

## Package model

Packages are represented by repository directories and concrete versions. That model differs from a centralized upload registry; an eventual publishing command must respect it.

## What to prepare

Keep package metadata accurate, use a valid semantic version, document the public API, and test supported platforms. Do not put credentials in `vpm.toml` or `vpm.lock`.

Return to [Packages](/docs/packages/overview/) for the specified manifest and dependency workflow.
