---
title: Vutcom
description: 'Migration from the historical composition proposal to receiver functions.'
section: Advanced
order: 3
---

## Superseded design

Vutcom is a historical proposal, not the current language surface. Its overview explicitly delegates trailing-colon blocks to the receiver-function specification. Do not write `composition UI`, `vutcom[UI]`, or `children` based on older examples.

## Use receiver functions

Declare a callback type such as `fn(ColumnScope)() -> void`, invoke it with `body.call(scope)`, and supply it with a trailing colon block. Scope types and methods belong to ordinary Vut libraries; UI, router, build, and testing DSLs do not require separate compiler domains.

See [Receiver functions](/docs/advanced/receivers/) for a complete example, and [Source compatibility](/docs/reference/source-compatibility/) for the source reconciliation policy.
