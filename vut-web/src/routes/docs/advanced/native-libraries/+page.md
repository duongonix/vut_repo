---
title: Native libraries
description: "Package native artifacts without compiling foreign source on the user's machine."
section: Advanced
order: 9
---

## Prebuilt artifacts

The FFI design consumes native libraries supplied as prebuilt artifacts through VPM. It does not compile `.c`, `.cpp`, `.rs`, `.m`, or `.mm` files on the application user's machine.

Static libraries use platform-appropriate formats such as `.lib` and `.a`. Do not assume dynamic loading is supported by every toolchain phase.

## Safe wrapper boundary

Keep raw extern declarations in a private module, for example `_native.vut`, and expose an ordinary safe Vut API from `mod.vut`. Match the native ABI, target, symbol names, layout, and allocation/deallocation contract.

## Before distributing

Verify the supported target triples and the exact artifact manifest schema against the selected VPM release. This page does not provide an unverified download endpoint or fabricate a native build hook.

See [FFI](/docs/advanced/ffi/) for language declarations and ABI restrictions.
