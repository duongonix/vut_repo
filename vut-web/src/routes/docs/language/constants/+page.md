---
title: Constants
description: 'Declare immutable bindings through ALL-CAPS names.'
section: Language
order: 2
---

## Declare a constant

```vut
MAX_SIZE = 100
APP_NAME: str = "Vut"
```

ALL-CAPS bindings are constants. Vut does not require a `const` keyword. Type inference and explicit annotations work as they do for ordinary variables.

## Reassignment is invalid

After initialization, a constant cannot be reassigned. The compiler reports the reassignment and can point to the original declaration.

```text
MAX_SIZE = 100
MAX_SIZE = 200  # invalid: constant reassignment
```

## Naming and privacy

Case is meaningful: `max_size` and `MAX_SIZE` are distinct identifiers. A leading underscore is the module privacy convention, not a replacement for type checking.
