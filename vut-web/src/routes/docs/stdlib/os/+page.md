---
title: Operating system
description: 'Platform integration without pretending every OS is identical.'
section: Standard library
order: 22
---

## Responsibility

The standard-library architecture assigns OS integration to `os`, while file access belongs to `fs`, lexical path manipulation to `path`, and environment access to `env`.

## Cross-platform behavior

Platform-specific capabilities must be exposed honestly. A cross-platform surface must not silently equate Windows and Unix semantics.

## Reference status

Release-verified signatures and examples for this module are pending. This page does not fabricate environment queries or promise APIs that have not been confirmed against a distributed standard library.
