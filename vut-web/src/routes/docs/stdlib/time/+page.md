---
title: Time
description: 'Separate wall-clock timestamps from elapsed-time measurement.'
section: Standard library
order: 23
---

## Responsibility

The specified time module covers timestamps, durations, monotonic measurement, and sleeping. Wall-clock time can change independently of elapsed time; use a monotonic source for measuring duration.

## Duration

The design calls for explicit duration units and checked conversions when a representation could overflow. Proposed constructors must follow the language features actually supported by the toolchain.

## Reference status

Exact release-verified constructors, imports, and async sleep behavior are pending. No browser playground timing output is presented as a Vut runtime result.
