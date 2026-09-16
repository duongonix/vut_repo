# Result Overview

`result(T, E)` is a core Vut language type for expected success-or-error values.

Result is lowercase and canonical:

```vut
fn load() -> result(User, LoadError):
  ...
```

A result value has exactly one active state:

- `success`, written and matched as `ok(value)`
- `error`, written and matched as `err(error)`

Result is not a standard-library enum. The compiler represents it directly in
AST, semantic types, MIR, and native codegen so propagation and ownership are
well-defined without macro expansion or heap boxing.

Result does not replace panic. Expected application failures use
`result(T, E)`. Programmer errors and violated invariants may still panic.

