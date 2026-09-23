# Vut Error Code Registry

## 1. Purpose

This document defines the stable diagnostic-code namespace used by the Vut compiler.

Compiler errors use:

```text
E####
```

Warnings use:

```text
W####
```

The goals are:

* stable documentation references
* searchable diagnostics
* consistent compiler behavior
* easier regression testing
* IDE/tooling integration

---

## 2. Code Structure

Error code ranges are grouped by compiler domain.

```text
E0xxx  syntax / lexer / parser
E1xxx  type system
E2xxx  names / symbols
E3xxx  modules / imports
E4xxx  interfaces
E5xxx  control flow
E6xxx  functions / calls
E7xxx  data / enums
E8xxx  memory / safety / FFI
E9xxx  internal compiler errors
```

Warnings use corresponding `W` namespaces.

---

## 3. Stability

Once a diagnostic code is publicly assigned to a specific error meaning, it should not be reused for an unrelated meaning.

Diagnostic wording may improve over time.

The semantic identity of the code should remain stable.

---

# E0xxx — Syntax, Lexer and Parser

## E0001 — Invalid Token

Used when the lexer encounters a character/token sequence that cannot form valid Vut syntax.

Example:

```text
error[E0001]: invalid token
```

---

## E0002 — Unterminated String

Example:

```vut
name = "Vut
```

---

## E0003 — Invalid Escape Sequence

Example concept:

```text
"\q"
```

when `\q` is not a defined escape.

---

## E0004 — Invalid Numeric Literal

Used for lexically malformed numbers.

---

## E0005 — Invalid Template Interpolation

Used for an invalid identifier after `$` or malformed interpolation syntax.

---

## E0006 — Unclosed Template Interpolation

Used when `$(expression)` has no matching closing parenthesis before the end of
its string literal.

---

## E0007 — Invalid Template Expression

Used when the expression inside `$(...)` is syntactically invalid. The primary
span must point inside the string at the expression.

---

## E0101 — Unexpected Token

Used when no more specific parser diagnostic applies.

Prefer specialized diagnostics whenever possible.

---

## E0102 — Expected `:`

Example:

```vut
if active
  run()
```

---

## E0103 — Expected Expression

Example:

```vut
value =
```

---

## E0104 — Expected Declaration

Used when syntax appears where a top-level declaration is required.

---

## E0105 — Unexpected Indentation

Example:

```text
error[E0105]: unexpected indentation
```

---

## E0106 — Expected Indented Block

Example:

```vut
if active:
print("yes")
```

---

## E0107 — Invalid Dedent

Used when indentation does not return to a valid previous indentation level.

---

## E0108 — Invalid Import Syntax

Example:

```vut
import math as m at add
```

---

## E0109 — Invalid Parenthesized Expression

May be used when unsupported tuple-like syntax appears.

Example:

```vut
value = (1, 2)
```

---

## E0110 — Invalid Declaration Syntax

Generic declaration-level syntax failure when a more specific diagnostic is unavailable.

---

## E0112 — Invalid `async` Modifier

Used when `async` is not directly followed by `fn`, or is placed on a
non-function declaration.

Example:

```vut
async data Point:
  x: int
```

`async` is only valid as a function/method modifier. See
`specs/async/06-diagnostics.md`.

---

# E1xxx — Type System

## E1001 — Unknown Type

Example:

```vut
value: UnkownType = ...
```

---

## E1002 — Invalid Type Expression

Used for syntactically valid but semantically invalid type application.

---

## E1003 — Type Mismatch

Canonical general type mismatch.

Example:

```vut
age: int = "20"
```

---

## E1004 — Cannot Infer Type

Example:

```vut
items = @[]
```

when no context exists to infer the list element type.

---

## E1005 — Heterogeneous List

Example:

```vut
@[1, "hello", true]
```

without explicit `list[dyn]`.

---

## E1006 — Null Assigned to Non-Optional Type

Example:

```vut
name: str = null
```

---

## E1007 — Invalid Optional Operation

An optional value was used where its present type is required without
narrowing, or an otherwise invalid optional operation was attempted.

Example:

```vut
nickname: str? = null
out(nickname.byte_len())   # E1007: narrow with `if nickname != null`
```

See `specs/optional/00-overview.md`.

---

## E1008 — Invalid Numeric Conversion

Used when a compile-time numeric value cannot fit the requested type.

Example:

```vut
value: u8 = 300
```

---

## E1009 — Invalid Operator Types

Example:

```vut
"hello" - 10
```

---

## E1010 — Incompatible Expression Branch Types

Used when expression branches cannot satisfy the required result type.

---

## E1011 — Invalid Dynamic Conversion

Reserved for invalid `dyn` conversion/use cases.

---

## E1012 — Cannot Infer Lambda Parameter Type

Used when an arrow lambda parameter has no annotation and no contextual
function type is available.

```vut
handler = x => x + 1
```

Parameter and return types are inferred from the expected `fn(...) -> ...`
type; add annotations when no context exists.

---

## E1013 — Closure Capture Not Supported (Removed)

Removed. Capturing closures are supported; see `specs/04` §46a. The code is
retired and must not be emitted.

---

## E1014 — Value Is Not Callable

Used when a call target is a value whose static type is not a function type.

```vut
fn main():
  value = 1
  value(2)
```

---

## E1027 — Invalid Closure Capture

Used when a closure capture is syntactically or semantically invalid: assigning
to a captured binding, or capturing a binding that cannot be captured.

```vut
fn main():
  base = 10
  add = fn(x):
    base = x
    x + base
```

Captured bindings are read-only inside the closure (`specs/04` §46a).

---

## E1029 — Unsupported Closure Capture (Removed)

Removed. Managed and move-only captures are supported (`specs/04` §46a). The
code is retired and must not be emitted.

---

## E1020 — Trailing Body Requires a Receiver Function Parameter

Used when trailing `Call():` syntax is applied to a callee whose last parameter
is not a receiver function type (specs/receiver/01-syntax-semantics.md). The `:`
body form is only valid when the final parameter has a receiver, e.g.
`fn(Scope)() -> void`.

```vut
fn consume(callback: fn() -> void) -> void:
  callback()

fn main():
  consume():
    out("x")
```

---

## E1021 — Receiver Type Mismatch

Used when the receiver passed to a receiver function does not match the declared
receiver type (specs/receiver/02-type-system-resolution.md).

```vut
body.call(column_scope)   # body expects fn(UserCardScope)(Event) -> void
```

---

## E1023 — Unknown Implicit Receiver Method

Used when a bare call inside a receiver body is not a local, not a method of any
enclosing receiver, and not a module symbol. Resolution is static; there is no
runtime receiver lookup (specs/receiver/02-type-system-resolution.md §11).

```vut
Column():
  Missing()   # no enclosing receiver declares `Missing`
```

---

## E1024 — Ambiguous Receiver Resolution

Used when more than one enclosing receiver declares the same method name and the
compiler cannot choose by lexical nesting. Candidates must not be selected by
declaration order.

---

## E1025 — `.call()` Requires a Receiver Function

Used when `.call(receiver, ...)` is applied to a value that is not a receiver
function. Plain function values are invoked directly as `callback(args...)`.

```vut
fn double(x: int) -> int:
  x * 2

fn main():
  f = double
  f.call(1)
```

---

## E1026 — Cannot Infer Receiver Function Type

Used when a trailing `Call():` body's receiver function type cannot be
determined statically, e.g. the callee is a function value whose callable type
is not known at resolution time.

---

## E1028 — Invalid Collection Method Element Type

Used when a `list[T]` builtin method is applied to an unsupported element type.
Currently this covers `join`, which is only defined for `list[str]`.

```vut
nums = @[1, 2, 3]
nums.join("-")   # E1028: join is only available for list[str]
```

Map the elements to `str` first (for example with `map`) when joining is needed.

---

## E1101 — Invalid Assignment

General assignment error not covered by a more specific code.

---

## E1102 — Assignment to Read-Only Value

Used for values that cannot be mutated through the current binding.

---

## E1103 — Invalid Field Assignment Type

Example:

```vut
user.age = "twenty"
```

---

## E1104 — Constant Reassignment

Example:

```vut
MAX_SIZE = 100
MAX_SIZE = 200
```

---

## E1105 — Mutation Through Constant Binding

Example:

```vut
DEFAULT_POINT.x = 10
```

when the constant value cannot be mutated through that binding.

---

# E2xxx — Names and Symbols

## E2001 — Unknown Symbol

Example:

```vut
pritn("hello")
```

---

## E2002 — Duplicate Symbol

Example:

```vut
name = "A"
name declaration duplicated in same declaration scope where prohibited
```

Exact scope behavior depends on declaration rules.

---

## E2003 — Name Collision

Used when two declarations/imports introduce incompatible identical bindings.

---

## E2004 — Unknown Field

Example:

```vut
user.naem
```

---

## E2005 — Unknown Method

Example:

```vut
user.saev()
```

---

## E2006 — Private Symbol Access

Used for non-import private access where module visibility is violated.

---

## E2007 — Invalid `self`

Example:

```vut
fn test():
  print(self.value)
```

---

## E2008 — Duplicate Method

Used when duplicate methods are declared for one type.

---

## E2009 — Duplicate Function

Used when unsupported overloading creates duplicate free-function definitions.

---

# E3xxx — Modules and Imports

## E3001 — Module Not Found

Example:

```vut
import missing.module
```

---

## E3002 — Imported Symbol Not Found

Example:

```vut
import math at unknown
```

---

## E3003 — Import Name Collision

Example:

```vut
import a.math
import b.math
```

without aliases.

---

## E3004 — Private Imported Symbol

Example:

```vut
import math at _internal
```

---

## E3005 — Import Outside Module Scope

Example:

```vut
fn run():
  import math
```

---

## E3006 — Relative Import Escapes Root

Used when leading dots attempt to move above the package/project source root.

---

## E3007 — Invalid Module Path

Used when a resolved path violates module naming/path rules.

---

## E3008 — Module Namespace Conflict

Used when a local top-level module namespace collides with a dependency import
namespace. Neither side may silently shadow the other; alias the dependency
instead.

---

## E3010 — Circular Module Dependency

Example:

```text
a -> b -> c -> a
```

---

# E4xxx — Interfaces

## E4001 — Unknown Interface

Used when a referenced interface cannot be resolved.

---

## E4002 — Invalid Interface Declaration

Generic structural interface declaration failure.

---

## E4003 — Interface Contains Field

Interfaces define behavior only.

Example invalid concept:

```vut
interface User:
  name: str
```

---

## E4004 — Invalid Interface Parent

Used when a composed parent is not a valid interface.

---

## E4010 — Interface Composition Conflict

Example:

```text
A requires read() -> str
B requires read() -> bytes
```

---

## E4011 — Circular Interface Composition

Example:

```text
A -> B -> A
```

---

## E4101 — Interface Requirement Missing

General missing requirement code.

---

## E4102 — Type Does Not Satisfy Interface

Canonical use-site interface mismatch.

Example:

```text
`Dog` does not satisfy interface `Animal`
```

---

## E4103 — Interface Method Signature Mismatch

Example:

```text
required: speak() -> str
found:    speak(int) -> str
```

---

## E4104 — Interface Method Not Public

Used when the required behavior exists but cannot satisfy interface visibility requirements.

---

# E5xxx — Control Flow

## E5001 — Invalid `for` Syntax

Generic loop syntax failure when parser recovery produces enough structure for semantic reporting.

---

## E5002 — Invalid Loop Binding

Example:

```vut
for value, index, other in items:
```

---

## E5003 — Loop Condition Must Be Bool

Example:

```vut
for 10:
```

---

## E5004 — Non-Iterable Value

Example:

```vut
for item in 10:
```

---

## E5005 — `break` Outside Loop

---

## E5006 — `continue` Outside Loop

---

## E5007 — Invalid `if` Condition

Used when an `if` condition is not `bool`.

---

## E5008 — Invalid Match Target

Reserved for unsupported/unmatchable target types.

---

## E5009 — Non-Exhaustive Match

Example:

```text
missing variant: done
```

---

## E5010 — Unreachable Control-Flow State

Reserved for semantic errors where unreachable structure violates a required rule.

Ordinary unreachable code may instead be a warning.

---

# E6xxx — Functions and Calls

## E6001 — Function Not Found

May be used when call syntax specifically resolves to an absent function, while E2001 remains the general unknown-symbol code.

Implementation should choose consistently.

---

## E6002 — Invalid Argument Count

Example:

```vut
add(1)
```

for:

```vut
fn add(a: int, b: int):
```

---

## E6003 — Invalid Named Argument

Example:

```vut
User(
  unknown: 10
)
```

when applied to callable argument binding rather than data-field construction.

---

## E6004 — Duplicate Named Argument

---

## E6005 — Invalid Return Type

Example:

```vut
fn value() -> int:
  "hello"
```

---

## E6006 — Missing Required Return

Used when a function can fall through despite requiring a value.

---

## E6007 — Return Outside Function

---

## E6008 — Explicit `self` Parameter

Example:

```vut
fn Counter.add(self: Counter, amount: int):
```

---

## E6009 — Invalid Method Receiver

Used when a method cannot be applied to the receiver's static type.

---

## E6010 — Unsupported Function Overload

Used when declarations attempt unsupported overload behavior.

---

## E6011 — `await` Outside `async fn`

Used when `await` appears outside an `async fn` body, including inside a plain
`fn()` lambda or an arrow (`() => ...`) lambda.

An anonymous `async fn(): ...` is an async body, so `await` inside it is valid
(see `specs/async/08-vutcon.md`).

Example:

```vut
fn main():
  value = await load()
```

---

## E6012 — Cannot Await Non-Awaitable Value

Used when the operand of `await` is not an async computation.

Example:

```vut
async fn main():
  value = await 123
```

---

## E6013 — Invalid Async Return

Used when an async function returns an awaitable value where its logical return
type is required.

Example:

```vut
async fn load() -> int:
  read()
```

where `read` is an `async fn`.

---

## E6014 — Awaitable Value Used As Ordinary Value

Used when an awaitable value is used without `await`.

Example:

```vut
async fn load() -> int:
  value = read()
  value + 1
```

---

## E6015 — Invalid Default Parameter

Used when a default parameter value is declared in an invalid position or on an
unsupported parameter:

* a required parameter follows a parameter with a default;
* a variadic parameter declares a default;
* an `extern` parameter declares a default.

Example:

```vut
fn f(a: int = 1, b: int):    // required parameter after a default
  a + b
```

---

## E6020 — Invalid `vut(...)` Callable

Used when `vut(...)` does not receive an anonymous callable.

```vut
vut(5)
vut calculate()
```

---

## E6021 — Vutcon Callback Has Parameters

Used when the `vut(...)` callback declares parameters. Callbacks take no
parameters in this phase.

---

## E6022 — `vut(...)` Outside `async fn`

Used when `vut(...)` appears where no async execution context exists.

---

## E6023 — Invalid Vutcon Result Handling

Used for unsupported Vutcon result usage (for example an invalid `await ...?`
when the handle's result is not `result[_, E]`).

---

## E6024 — Anonymous `async fn` Outside `vut(...)`

Used when an anonymous `async fn(): ...` appears anywhere other than as the
argument of `vut(...)`. Anonymous async callables are supported only directly in
`vut(...)` in this phase; asynchronous functions are not yet first-class values.

```vut
handler = async fn():
  await load()
```

---

# E7xxx — Data and Enums

## E7001 — Unknown Data Type

May be used for construction-specific unresolved data declarations.

---

## E7002 — Missing Required Field

Example:

```vut
User(
  name: "Ha"
)
```

when `age` is required.

---

## E7003 — Unknown Data Field

Example:

```vut
User(
  name: "Ha",
  unknown: true
)
```

---

## E7004 — Duplicate Field Initialization

Example:

```vut
User(
  name: "Ha",
  name: "Nam"
)
```

---

## E7005 — Invalid Default Field Value

Example:

```vut
data Counter:
  value: int = "zero"
```

---

## E7006 — Invalid Data Construction

General construction error not covered by more specific codes.

---

## E7101 — Unknown Enum Variant

---

## E7102 — Duplicate Enum Variant

---

## E7103 — Invalid Enum Construction

Reserved for payload-enum behavior.

---

## E7104 — Invalid Enum Pattern

Used for an enum pattern that does not match the scrutinee.

---

## E7105 — Recursive Enum Payload

Used when an enum variant contains the enum by value, which would have infinite
size. Use an indirect container such as `list[T]`.

---

## E7106 — Variant Payload Mismatch

Used for missing, unknown, or duplicate variant payload fields, or for a
payloadless/positional mismatch in a construction or pattern.

---

## E7107 — Pattern Type Mismatch

Used when a pattern cannot match the scrutinee type (for example a variant
pattern on a non-enum, or a literal pattern on an incompatible type).

---

## E7108 — Duplicate Pattern Binding

Used when the same name is bound more than once in a single pattern.

---

## E7109 — Or-Pattern Binding Mismatch

Used when `or` alternatives bind different names or incompatible types.

---

## E7110 — Guard Must Be Bool

Used when a match guard expression is not `bool`.

---

## E7111 — Invalid Range Pattern

Used for a range pattern on a non-integer scrutinee or invalid bounds.

---

## E7112 — Invalid/Unsupported List Pattern

Used when a list pattern is invalid, or for the reserved list-pattern syntax
that is not yet implemented.

---

# E8xxx — Memory, Safety and FFI

## E8001 — Unsafe Operation Outside Unsafe Context

Example:

```text
raw pointer operation requires `unsafe`
```

---

## E8002 — Invalid Raw Pointer Operation

Reserved for pointer semantics once finalized.

---

## E8003 — Invalid Pointer Conversion

---

## E8004 — Invalid FFI Type

Used when a non-ABI-safe Vut type is passed directly across a raw FFI boundary.

---

## E8005 — Unknown ABI

Example:

```text
extern "unknown"
```

---

## E8006 — Invalid External Declaration

Used for malformed or semantically invalid foreign declarations.

---

## E8007 — External Function Has Body

Used if a raw foreign declaration attempts to define a Vut body.

---

## E8008 — Unsafe Memory Mutation

Reserved for invalid unsafe/raw memory operations.

---

## E8009 — Value Used After Move

Raised when a move-only value (`resource[T]` or a `vutcon[T]` handle) is read
after it was moved or awaited.

---

## E8010 — Cannot Duplicate Move-Only Value

Raised when a move-only value would need a second owner on the same path.

---

## E8011 — Move-Only Value Used In Branch

Raised when a move-only value owned outside a conditional branch or loop is
consumed inside it. Move the value before the branch instead.

---

## E8012 — Cannot Read Move-Only Field By Value

Raised when a native resource field would be projected out of an aggregate.
Resource handles cannot be extracted from an aggregate in this version.

---

# E9xxx — Internal Compiler Errors

## E9001 — Unexpected Compiler State

Generic internal compiler invariant failure.

---

## E9002 — Backend Failure

Used when native backend code generation fails unexpectedly.

---

## E9003 — Runtime ABI Mismatch

Compiler and runtime are incompatible.

---

## E9004 — Corrupted Compiler Cache

Used when an internal cache cannot be validated/recovered automatically.

---

## E9005 — Internal Module Graph Failure

Reserved for impossible resolver graph states.

---

## E9006 — Async Lowering Failure

Used when async state-machine lowering or its internal verification fails.

This is a compiler bug, not a user source error. See
`specs/async/03-lowering.md`.

---

## E9999 — Unknown Internal Compiler Error

Fallback only.

Prefer a more specific internal code when available.

---

# Warning Codes

## W1001 — Suspicious Type Conversion

Reserved for legal but potentially unintended conversions.

---

## W2001 — Unused Variable

Example:

```vut
value = calculate()
```

when `value` is unused.

---

## W2002 — Unused Import

---

## W2003 — Unused Function

May be applied where appropriate without warning on intentionally public library APIs.

---

## W2004 — Unused Private Symbol

Private unused declarations may be easier to detect reliably than public API declarations.

---

## W5001 — Unreachable Code

Example:

```vut
return
print("never")
```

---

## W5002 — Constant Condition

Example:

```vut
if true:
```

when likely accidental.

---

## W5003 — Redundant Boolean Comparison

Example:

```vut
if active == true:
```

Suggestion:

```vut
if active:
```

---

## W5004 — Unreachable Match Arm

Used when a match arm is already covered by earlier unguarded arms, or follows
an unconditional wildcard/binding arm.

---

## W6001 — Inferred Public Return Type

Potential future style warning encouraging explicit return types for stable public APIs.

This warning should not be enabled by default unless intentionally adopted.

---

## W6002 — Awaitable Value Dropped Without Await

Used when an async computation value is created and discarded without being
awaited, so the lazy computation never runs.

Example:

```vut
async fn main():
  load()
```

---

# Diagnostic Code Selection

When several codes might apply, choose the most specific error that explains the root cause.

Example:

```vut
import missing at foo
```

If module `missing` does not exist, report:

```text
E3001
```

Do not additionally report:

```text
E3002
```

for `foo`.

---

## Cascading Diagnostics

Once a root error has invalidated a syntax or semantic node, downstream stages should avoid emitting derivative errors.

Example:

```text
E0102 missing colon
```

should not lead to numerous unrelated name/type errors caused only by parser recovery.

---

## One Primary Code per Diagnostic

Each primary diagnostic has one stable code.

Related notes do not need separate codes.

Example:

```text
error[E1003]: type mismatch
...
= note: `age` was inferred as `int` here
```

The note is part of E1003.

---

## Error Documentation

Each public compiler code should eventually have documentation containing:

* meaning
* example
* why it occurs
* typical fixes
* related rules

This enables future tooling such as:

```text
vut explain E1003
```

However, `vut explain` is not currently an approved CLI command and must not be implemented without revising `specs/13-vut-cli.md`.

Documentation may instead be available through external docs/tooling.

---

## Reserved Codes

Unused numbers inside a category may remain reserved.

Do not densely reuse every available number merely to avoid gaps.

Gaps allow related diagnostics to be grouped logically later.

---

## Removal

When an error condition disappears from the language, its code should be retired rather than immediately reused for another meaning.

---

## Error Code Principles

1. Compiler errors use `E####`.
2. Warnings use `W####`.
3. E0xxx is parser/lexer syntax.
4. E1xxx is type system.
5. E2xxx is names/symbols.
6. E3xxx is module/import.
7. E4xxx is interface.
8. E5xxx is control flow.
9. E6xxx is functions/calls.
10. E7xxx is data/enums.
11. E8xxx is memory/safety/FFI.
12. E9xxx is internal compiler failures.
13. Codes are stable once published.
14. Retired codes are not casually reused.
15. Prefer one precise root diagnostic over cascading errors.
16. Error wording may improve without changing code identity.

This document is the canonical error-code registry for the Vut compiler.
