# Vut Memory Model

## 1. Purpose

This document defines the memory model for Vut.

Vut is a statically typed, native compiled language designed for:

* high runtime performance
* low memory overhead
* predictable memory behavior
* deterministic resource destruction
* efficient native data layouts
* minimal unnecessary heap allocation
* minimal unnecessary copying
* simple source-level programming
* strong memory safety in safe Vut

The memory model must be designed for performance from the beginning.

Vut must not depend on a tracing garbage collector for ordinary memory management.

---

# 2. Core Memory Model

Vut uses:

```text
compiler-managed deterministic ownership
+
automatic moves
+
automatic destruction
+
value semantics
```

The compiler determines how values are physically stored, moved, copied and destroyed.

Users should not normally need to manually manage memory.

Example:

```vut
fn create() -> User:
  user = User(
    name = "Nam",
    age = 20
  )

  user
```

The compiler should transfer ownership of `user` into the return value.

It must not unnecessarily:

```text
deep copy user
free old user
allocate another user
```

---

# 3. No Global Garbage Collector

Vut does not use a tracing garbage collector as its default memory-management system.

Do not require:

```text
GC heap
GC pauses
mark-and-sweep
generational collection
runtime object tracing
```

for ordinary Vut programs.

Memory should normally be reclaimed deterministically when its owning value reaches the end of its lifetime.

Benefits:

```text
predictable latency
smaller runtime
lower memory overhead
better cache behavior
deterministic destruction
better FFI compatibility
better systems-level performance
```

The compiler/runtime architecture must not require a global tracing GC.

---

# 4. No Manual Memory Management in Safe Vut

Safe Vut does not require:

```text
malloc
free
new
delete
```

for ordinary values.

Example:

```vut
fn process():
  values = @(1, 2, 3)
  out("done")
```

The compiler automatically determines when `values` can be destroyed.

Conceptually:

```text
allocate values if necessary

use values

drop values
```

The user does not write the final `drop`.

---

# 5. No Source-Level Borrow Checker in MVP

Vut does not expose Rust-style lifetime syntax in normal source code.

Do not require syntax such as:

```text
&T
&mut T
'a
'lifetime
```

for ordinary Vut programming.

Memory safety must be achieved primarily through:

```text
value semantics
compiler ownership analysis
controlled mutation
automatic lifetime management
safe runtime abstractions
```

Ownership is primarily a compiler concept rather than source-level ceremony.

A future explicit reference model may be designed separately if zero-copy use cases require it.

Do not invent one during MVP.

---

# 6. Value Semantics

Ordinary Vut values use observable value semantics.

Example:

```vut
data Point:
  x: int
  y: int

a = Point(
  x = 10,
  y = 20
)

b = a
b.x = 100
```

The observable value of:

```vut
a.x
```

must remain:

```text
10
```

The compiler must not expose accidental mutable aliasing merely because an internal representation shares storage.

---

# 7. Physical Copy Is Not Required

Value semantics does not mean every assignment physically copies all bytes.

The compiler may internally use:

```text
move
copy
copy elision
shared immutable storage
reference counting
copy-on-write
scalar replacement
stack promotion
```

provided observable Vut semantics remain unchanged.

Example:

```vut
b = a
```

does not automatically mean:

```text
memcpy(all bytes of a)
```

The compiler should select the cheapest correct representation.

---

# 8. Automatic Move

Vut automatically moves values when ownership can be transferred safely.

Example:

```vut
fn create() -> list(int):
  values = @(1, 2, 3)
  values
```

`values` is not used after the final expression.

The compiler should conceptually perform:

```text
Move(values → return)
```

rather than:

```text
Copy(values → return)
Drop(values)
```

Move optimization is automatic.

There is no required user-facing:

```text
move
```

keyword in MVP.

---

# 9. Move Must Preserve Source Semantics

Automatic move is primarily a compiler optimization.

The compiler must not unexpectedly make ordinary source code unusable merely because it selected a move internally.

Example:

```vut
b = a
out("$a")
```

Because `a` is used afterwards, the compiler cannot optimize the assignment as a destructive move that invalidates observable source behavior.

It must choose another valid strategy.

Possible strategies include:

```text
direct Copy for cheap Copy types
optimized clone
shared immutable backing
copy-on-write
```

depending on the type.

---

# 10. Copy Types

The compiler must internally distinguish types that are cheap and safe to copy directly.

Examples include:

```text
bool

i8
i16
i32
i64

u8
u16
u32
u64

f32
f64
```

and other primitive values where applicable.

These types should generally be copied through registers/native values.

No heap allocation is required.

---

# 11. Composite Copy Types

A `data` containing only Copy-compatible fields may itself be Copy-compatible.

Example:

```vut
data Vec2:
  x: f64
  y: f64
```

The compiler may classify:

```text
Vec2
```

as trivially copyable.

This allows efficient:

```text
register passing
stack copies
memcpy where appropriate
ABI-native passing
```

without requiring user annotations.

---

# 12. Managed Types

Some values may own external memory or other resources.

Examples:

```text
str
bytes
list(T)
map(K, V)

data containing managed fields

some interface representations
dyn
```

The compiler must know whether a type:

```text
needs_drop
is_copy
contains_managed_memory
has_trivial_move
```

or has other relevant ownership properties.

---

# 13. Type Memory Properties

Compiler type metadata should provide properties conceptually similar to:

```text
TypeInfo
├── size
├── alignment
├── layout
├── is_copy
├── needs_drop
├── contains_managed
└── ABI classification
```

Additional internal properties may be added when required.

Do not repeatedly rediscover these properties throughout codegen.

They should come from centralized type/layout infrastructure.

---

# 14. Stack Allocation Preferred

Values should not automatically be heap allocated.

Example:

```vut
data Vec2:
  x: f64
  y: f64

point = Vec2(
  x = 10.0,
  y = 20.0
)
```

should normally be representable directly through:

```text
registers
or
stack
```

rather than:

```text
heap allocation
+
pointer
```

A `data` declaration does not imply heap allocation.

---

# 15. Native Data Layout

Statically known `data` types should have native compile-time layouts.

Conceptually:

```vut
data Vec2:
  x: f64
  y: f64
```

may become:

```text
Vec2
offset 0  : f64 x
offset 8  : f64 y
```

subject to target ABI/alignment.

Do not represent normal `data` as:

```text
hash maps
dynamic property tables
boxed field collections
```

---

# 16. Field Layout

Compiler codegen must centrally calculate:

```text
size
alignment
field offsets
padding
ABI representation
```

for every concrete data type.

Layout decisions must be deterministic for the same:

```text
compiler version
target
ABI
type definition
```

Do not duplicate layout logic across compiler subsystems.

---

# 17. Escape Analysis

The compiler architecture must support escape analysis.

Example:

```vut
fn calculate() -> int:
  point = Point(
    x = 10,
    y = 20
  )

  point.x + point.y
```

`point` does not escape.

Compiler should be able to keep it:

```text
on stack
in registers
```

or eliminate the aggregate entirely.

---

# 18. Stack Promotion

A value that would otherwise require temporary heap storage may be promoted to stack storage when the compiler proves it does not escape.

Conceptually:

```text
heap candidate
    ↓
escape analysis
    ↓
does not escape
    ↓
stack allocation
```

This optimization should be supported by MIR/optimization architecture.

---

# 19. Scalar Replacement

Small aggregates should be eligible for scalar replacement.

Example:

```vut
point = Point(
  x = 10,
  y = 20
)

result = point.x + point.y
```

may optimize toward:

```text
x = 10
y = 20
result = x + y
```

without materializing `point`.

---

# 20. List Representation

`list(T)` should use an efficient native growable-buffer representation.

Conceptually:

```text
List<T>
├── pointer
├── length
└── capacity
```

The descriptor may live on:

```text
registers
stack
inside another data value
```

while its dynamic buffer may live on heap.

---

# 21. List Elements

For:

```text
list(int)
```

elements must be stored as statically typed integers.

Conceptually:

```text
[10][20][30][40]
```

Do not represent this as:

```text
[box(int)][box(int)][box(int)][box(int)]
```

Do not convert list elements to `dyn`.

---

# 22. List Destruction

When:

```text
list(T)
```

owns its buffer and reaches the end of its lifetime:

If `T` does not require destruction:

```text
free buffer
```

If `T` requires destruction:

```text
drop each live element
free buffer
```

Compiler/runtime should specialize this behavior for concrete `T`.

Do not perform unnecessary per-element runtime type inspection.

---

# 23. String Representation

`str` represents UTF-8 text.

The runtime representation should remain compact.

Conceptually it may contain:

```text
pointer
byte length
ownership metadata if required
```

Exact ABI remains an implementation detail.

Do not require:

```text
per-character objects
UTF-32 storage
NUL termination
```

for ordinary Vut strings.

---

# 24. String Allocation

String literals should not require heap allocation on every execution.

Compiler/runtime should allow immutable static strings to reside in static program data.

Example:

```vut
out("Hello")
```

should be able to reference read-only binary data directly.

---

# 25. String Operations

String operations must avoid unnecessary copies.

Where safe and beneficial, implementation may use:

```text
views internally
shared immutable buffers
copy-on-write
specialized builders
```

provided memory safety and value semantics are preserved.

Do not expose dangling views to safe Vut.

---

# 26. Bytes

`bytes` uses efficient contiguous binary storage.

It must not:

```text
box every byte
store each byte as dyn
```

A byte sequence should use compact native storage.

`bytes` is a managed type:

```text
needs_drop          = true
is_copy             = false
contains_managed    = true
```

Value semantics apply. Assigning a `bytes` value produces an observable copy:
mutating one binding must not change the other. The compiler may use a move,
copy elision, or shared storage as long as the observable value is preserved.
A buffer is released exactly once when its lifetime ends; there is no leak,
double free, or use-after-free. `bytes` storage is contiguous and length is
constant time.

# 27. UTF-8 Validation Error

`bytes.to_str()` returns `result(str, Utf8Error)`. `Utf8Error` is a core
managed data type:

```text
data Utf8Error:
  valid_up_to: int
  error_len: int
```

`valid_up_to` is the number of leading bytes that form valid UTF-8.
`error_len` is the length of the invalid sequence, or `0` when the input ends
prematurely. Conversion never reinterprets, truncates, or lossy-replaces
invalid input.

---

# 27. Maps

`map(K, V)` should use an optimized hash-table implementation.

Keys and values retain their concrete static layouts where practical.

Do not represent every key/value pair as dynamic objects.

Use a mature, benchmarked implementation strategy.

Do not design a custom hash table without a demonstrated Vut-specific reason.

---

# 28. Deterministic Destruction

Managed resources should be released deterministically.

Conceptually:

```vut
fn work():
  values = @(1, 2, 3)
  out("working")
```

becomes logically:

```text
create values

use values

Drop(values)

return
```

The exact physical drop may be optimized away when unnecessary.

---

# 29. Scope Exit

Compiler-generated cleanup must correctly handle every exit path.

Examples:

```text
normal block exit
function return
early return
loop exit
break
error propagation
```

A managed value must not leak merely because control flow exits early.

A `for` loop may own a collection (a temporary or a projected field being
iterated). That collection must be released on every exit path, including an
early `return` from inside the loop body, not only on the normal loop exit.

---

# 30. MIR Ownership Operations

MIR should explicitly represent ownership-relevant operations.

At minimum, the architecture should be capable of representing:

```text
Move
Copy
Drop
Allocate
```

If shared runtime representations are used, MIR/runtime may additionally support:

```text
Retain
Release
```

where necessary.

These must not be emitted indiscriminately.

---

## 30.1 Method Receivers and Field Projection

A method receiver is a **borrow**. The caller keeps ownership of the receiver
value; the callee must not retain or release it, and its cleanup must not drop
it.

MIR lowering enforces the following:

```text
- A method receiver that is a local (including `self`) is passed as a borrow.
- A method receiver that is a projected field is passed as a field borrow
  (`BorrowField`): the address of the field inside the caller's aggregate, with
  no retain and no release.
- A method receiver that is a temporary is passed as a borrow; the temporary is
  released by the caller after the call.
- Reading `self.field` uses a borrowed base: the field is retained into an
  owned reference, but the borrowed base is not released.
- Assigning `self.field = value` (`FieldStore`) releases the previous field
  value when it is managed and stores the new value in place, so the mutation is
  visible to the caller.
```

A value projected from a borrowed base is an independent owned reference: it may
be stored, returned, or dropped normally. Borrowing never transfers ownership of
the receiver or of the field's containing aggregate.

---

## 30.2 Borrowing Native Resources

A `resource(T)` is a move-only owner with exactly-once destruction. When a
resource is used only where a borrow is required, the compiler passes it by
borrow without moving, retaining, or releasing it, and the owner still runs the
destructor exactly once.

The borrow is internal to the compiler and has no source syntax.

The only borrow context is the native ABI boundary: an argument of type
`resource(T)` given to an `extern` parameter of type `ptr(T)` (or `ptr(void)`)
is passed as the resource's inner native pointer, not as an owned handle:

```text
resource(T) value/local/field  →  extern ptr(T) parameter
    borrow: no move, no retain, no release, no drop
```

`ptr(T)` is the native ABI type at that boundary; it is not a language-level
borrow form, and `resource(T) -> ptr(T)` is not an ordinary source coercion.
Passing a resource where ownership is expected still moves it. Borrowing works
for resource locals and for resource fields of a local aggregate (for example
`self.handle`).

---

# 31. Drop Elimination

Optimization passes should remove redundant destruction.

Example:

```text
Move(value → return)
Drop(value)
```

should not result in destruction of the moved resource.

Likewise, trivially destructible types should not receive unnecessary drop operations.

---

# 32. Copy Elision

The compiler should aggressively eliminate unnecessary copies where correctness permits.

Important cases include:

```text
function returns
temporary values
constructor arguments
assignment chains
intermediate expressions
collection construction
```

Example:

```vut
user = User(
  name = "Nam",
  age = 20
)
```

should ideally construct directly into its destination storage.

---

# 33. Return Value Optimization

Large returned `data` values should support efficient return strategies.

Possible implementation:

```text
register return
ABI aggregate return
hidden return pointer
destination passing
```

depending on target ABI and type size.

Do not blindly heap allocate large return values.

---

# 34. Parameter Passing

Function parameter ABI should consider type properties.

Small Copy values:

```text
register/native value
```

Large values:

```text
ABI-appropriate representation
```

Managed values:

```text
ownership-aware passing
```

Do not force all parameters through heap pointers.

---

# 35. Reference Counting Is Not the Global Memory Model

Vut must not implement every value through automatic reference counting.

Avoid architecture where ordinary operations continuously require:

```text
retain
release
retain
release
```

for all values.

Reference counting is permitted only as an internal strategy where actual shared ownership/storage provides a measurable benefit or is necessary.

---

# 36. Selective Reference Counting

Internal reference counting may be appropriate for:

```text
shared immutable buffers
copy-on-write storage
some string representations
some collection representations
interface/dyn backing storage
explicit future shared ownership abstractions
```

Use it selectively.

Compiler should eliminate redundant retain/release operations whenever possible.

---

# 37. Copy-on-Write

Copy-on-write is permitted as an internal optimization.

Example source semantics:

```vut
a = values
b = a
b.set(0, 100)
```

Observable semantics require `a` to remain unchanged.

An implementation may initially share storage:

```text
a ─┐
   ├── shared buffer
b ─┘
```

and copy only when mutation occurs.

Conceptually:

```text
b mutation
    ↓
storage shared?
    ↓ yes
copy buffer
    ↓
mutate b
```

However, COW must not automatically be used everywhere.

Benchmark it against move/deep-copy/unique ownership strategies.

---

# 38. COW Is an Optimization, Not Language Semantics

User code must not be able to observe whether a value uses:

```text
deep copy
COW
move
shared immutable storage
```

except through legitimate performance characteristics.

Compiler/runtime may change these strategies between versions without changing language semantics.

---

# 39. Unique Storage Optimization

When compiler/runtime knows storage is uniquely owned, mutation should happen directly.

Do not perform:

```text
copy
retain
detach
```

when uniqueness is already proven.

This is particularly important for:

```text
list
map
string builders
managed data
```

---

# 40. Interface Values

Structural interfaces may require runtime representation when dynamic dispatch is necessary.

Conceptually:

```text
InterfaceValue
├── data/reference
└── method table
```

Exact representation is runtime/ABI-defined.

---

# 41. Interface Allocation

Converting a concrete value to an interface must not automatically imply a heap allocation when it can be avoided safely.

Compiler should consider:

```text
stack-backed interface value
direct references to existing storage
escape analysis
devirtualization
```

---

# 42. Interface Devirtualization

When the concrete implementation is statically known:

```vut
dog.speak()
```

compiler should use direct static calls.

Even when source uses an interface, optimization may devirtualize calls when concrete type is proven.

Avoid vtable dispatch when unnecessary.

---

# 43. Dynamic Values

`dyn` intentionally carries runtime type information.

Its representation may require:

```text
type metadata
payload
or payload pointer
```

depending on value size/type.

`dyn` is allowed to have runtime overhead because the programmer explicitly requested dynamic behavior.

---

# 44. Dyn Must Be Localized

Use of:

```text
dyn
```

must not cause unrelated statically typed code to become dynamically represented.

Example:

```vut
numbers: list(int)
```

must remain specialized native integer storage even if another part of the program uses `dyn`.

---

# 45. Small Dyn Optimization

The runtime may use small-value/inline storage for `dyn` where benchmarks justify it.

Small primitive values may potentially be stored directly inside the dynamic container instead of requiring heap allocation.

This is an implementation optimization.

---

# 46. Optional Values

Optional:

```text
T?
```

should use efficient layouts.

Compiler may use niche/null optimization where possible.

Example conceptual pointer-like optional:

```text
null pointer
→ absent

non-null pointer
→ present
```

Do not automatically add a separate tag byte if an existing invalid representation can encode absence safely.

---

# 47. Result Values

`result(T, E)` should be represented as an efficient tagged value once finalized.

Compiler may optimize discriminant representation based on available niches.

Result does not require heap allocation by default.

---

# 47a. Enum Values

A payload enum value is a tagged union. Only the active variant's payload is
live, and only it is destroyed when the value is dropped.

```text
enum Shape:
  point
  circle(radius: float)
  rect(width: float, height: float)
```

`circle`'s payload exists only while the tag selects `circle`. Dropping a
`Shape` drops the active payload exactly once and never inspects inactive
payloads.

Matching consumes the enum by value: bound payload fields move into the arm
bindings, and fields matched with `_` are dropped on the matching arm's path.
An enum whose payloads are all trivially copyable is itself copyable and needs
no drop glue.

Recursive enum payloads by value are rejected because they have infinite size;
recursive shapes must use an indirect container such as `list(T)`.

---

# 48. Static Dispatch by Default

Statically known operations should use direct calls.

Example:

```vut
user.login()
```

when `User.login` is known should compile to a direct function call.

Do not route ordinary methods through runtime lookup tables.

---

# 49. No Runtime Reflection for Ordinary Values

Normal `data` does not require:

```text
field-name tables
runtime property lookup
runtime type dictionaries
```

for ordinary access.

Example:

```vut
user.age
```

should compile to a known field offset.

---

# 50. Allocation Strategy

Heap allocation must be centralized through runtime allocation APIs where appropriate.

This allows future:

```text
allocator replacement
profiling
platform abstraction
specialized allocation
```

without changing language semantics.

Do not scatter direct OS allocation calls throughout generated code.

---

# 51. Allocator

Initial runtime should use a mature high-performance allocator strategy appropriate to the target.

Do not implement a general-purpose allocator from scratch during MVP.

The architecture may allow configurable allocators later.

---

# 52. Allocation Minimization

Compiler/runtime should actively minimize allocations.

Important optimization targets:

```text
temporary data
temporary strings
short-lived collections
interface conversions
function returns
format/template strings
iterator-like operations
```

Allocation count should be included in performance benchmarks where possible.

---

# 53. Template Strings

Template strings such as:

```vut
out("Hello $name, result = $(a + b)")
```

must not require runtime parsing or runtime `eval`.

Compiler parses interpolation at compile time.

Formatting should avoid constructing unnecessary temporary strings when output is immediately written.

For example:

```vut
out("x = $x, y = $y")
```

may lower conceptually to buffered output segments rather than:

```text
allocate complete temporary string
format everything
write
free temporary
```

when direct buffered formatting is more efficient.

---

# 54. String Builders

When repeated string construction requires mutable buffering, std/runtime may use an internal or public builder abstraction.

Growth should be amortized.

Avoid quadratic behavior such as repeatedly reallocating the entire string during loops.

---

# 55. Memory Safety

Safe Vut must prevent:

```text
use-after-free
double free
dangling safe references
invalid safe pointer arithmetic
out-of-bounds safe access
uninitialized safe values
```

Compiler/runtime implementation must preserve these properties.

---

# 56. Bounds Safety

Safe collection access must enforce bounds.

Compiler may eliminate bounds checks when proven redundant.

An invalid explicit index (`at`, `set`, `insert`, `remove`) traps via the shared
runtime bounds-panic path instead of returning a fabricated element; accessors
with their own fallback contracts (`first`, `last`, `pop`, `char_at`) keep them.

Example:

```vut
for value in items:
```

should not require unnecessary repeated generic bounds checks when iteration lowering already proves accesses valid.

---

# 57. Uninitialized Memory

Safe Vut variables and fields must not expose uninitialized memory.

Required `data` fields must be initialized during construction unless they have defaults.

Compiler-internal uninitialized storage may be used during optimized construction only when correctness is proven.

---

# 58. Unsafe Memory

Raw memory operations belong to explicit unsafe/FFI facilities.

Potential future operations include:

```text
raw pointers
pointer arithmetic
manual allocation
manual deallocation
foreign memory
```

They must not weaken guarantees of safe Vut.

Follow:

```text
specs/19-ffi-unsafe.md
```

when implemented.

---

# 59. FFI Ownership

Foreign function interfaces must explicitly define ownership behavior.

Compiler/runtime must not guess whether a foreign pointer is:

```text
borrowed
owned
transferred
static
```

FFI memory contracts must be represented explicitly by the finalized unsafe/FFI design.

---

# 60. Resource Management

The same deterministic destruction model supports non-memory resources such as:

```text
files
sockets
locks
native handles
```

Native resources are represented by the generic `resource(T)` type:

```text
resource(T)
```

where `T` is an `opaque data` (or `@repr(C)`) native pointee. A `resource(T)`
value is:

```text
single ownership
move-only
automatic move
deterministic automatic drop
exactly-once native destructor
no retain
no refcount
```

The runtime owns a small header holding the native pointer and the destructor
registered by the producing native library. Releasing the value invokes the
destructor exactly once. Because the type is move-only, the compiler rejects
any read that would create a second owner (use-after-move or duplication)
rather than inserting a reference-count operation.

Ownership transfer rules:

```text
function parameter            borrowed for the call (native must not retain)
function return               ownership transfers to the caller
data field                    owned by the aggregate; dropped with it
result payload                moved out on success, dropped on error
await state slot              moved into the state object; dropped once
```

MVP restrictions:

```text
a resource field cannot be projected out of an aggregate by value yet
duplicating a resource value is a compile error
using a resource inside a conditional branch is a compile error
```

Shared ownership (reference counting) is **not** the default for `resource(T)`.
A future explicit shared-ownership capability may be specified separately if a
real use case requires more than one owner.

Do not invent destructor syntax solely from this requirement.

---

# 61. Cycles

Because Vut does not rely on tracing GC, compiler/runtime designs must avoid introducing invisible strong-reference cycles through ordinary value semantics.

If future explicit shared-reference types permit cycles, their cycle behavior must be specified separately.

Do not silently introduce a tracing GC merely to handle hypothetical future cycles.

---

# 62. Compiler Pipeline

Memory semantics should integrate with the compiler approximately as:

```text
Source
    ↓
Parser
    ↓
AST
    ↓
HIR
    ↓
Type Checking
    ↓
Type/Layout Properties
    ↓
Ownership/Lifetime Analysis
    ↓
MIR
    ↓
Move / Copy / Drop insertion
    ↓
Escape Analysis
    ↓
Optimization
    ↓
Codegen
```

Exact internal ordering may evolve when benchmarks/architecture justify it.

---

# 63. Ownership Analysis

Compiler ownership analysis should determine where possible:

```text
last use
ownership transfer
copy requirement
destruction point
unique storage
escaping values
```

This analysis must be internal.

Do not expose compiler ownership complexity to ordinary Vut source unless a future specification deliberately introduces such features.

---

# 64. Last-Use Analysis

Last-use analysis enables automatic move.

Example:

```vut
a = create_user()
process(a)
```

If `a` is not used afterwards and `process` consumes ownership, compiler may transfer it without copying.

If `a` remains needed, compiler must preserve it.

---

# 65. Destination-Passing Optimization

Compiler architecture should allow values to be constructed directly into their final destination.

Instead of:

```text
allocate temporary
construct temporary
copy temporary → destination
drop temporary
```

prefer:

```text
construct directly → destination
```

when legal.

This is especially valuable for:

```text
large data
strings
collections
function return values
```

---

# 66. Allocation Sinking

Where profitable, allocation may be delayed until it is proven necessary.

If an object never escapes and can be scalarized, heap allocation should disappear entirely.

---

# 67. Allocation Elimination

Compiler should seek opportunities to completely eliminate allocations.

Example:

```vut
fn sum_point() -> int:
  p = Point(
    x = 10,
    y = 20
  )

  p.x + p.y
```

may require no aggregate allocation at runtime.

---

# 68. Dead Store Elimination

Writes that cannot affect observable behavior may be removed.

Memory optimizations must respect:

```text
I/O
FFI
volatile-like future operations
observable mutation
```

Do not remove operations with side effects.

---

# 69. Memory Reuse

Compiler/runtime may reuse storage after a value's lifetime ends.

Example:

```text
stack slots
temporary buffers
temporary aggregate storage
```

may be reused when lifetimes do not overlap.

This can reduce stack and heap pressure.

---

# 70. Monomorphization

Once generics are finalized, concrete generic instantiations should generally allow specialized layouts.

Example:

```text
list(int)
list(Vec2)
```

should have concrete element layouts.

Do not require universal boxed generic representation.

Exact generic compilation strategy is finalized elsewhere.

---

# 71. Cache Locality

Data representation should prioritize locality where practical.

Prefer:

```text
contiguous list storage
inline data fields
compact metadata
```

over unnecessary pointer-heavy object graphs.

Performance evaluation must consider:

```text
CPU cache misses
pointer chasing
allocation count
working-set size
```

not only instruction count.

---

# 72. Metadata Overhead

Static types should not carry runtime type metadata unless required.

For example:

```text
int
Vec2
list(int)
```

should not carry per-value runtime type descriptors merely because the compiler internally knows their TypeId.

Runtime metadata is justified primarily for features such as:

```text
dyn
interface dispatch
FFI/runtime services where necessary
```

---

# 73. Zero-Cost Static Abstractions

Static Vut abstractions should aim to compile away.

Examples include:

```text
small data wrappers
direct methods
static interfaces after devirtualization
optional niche representations
simple Result propagation
```

Abstraction should not automatically imply heap allocation or runtime dispatch.

---

# 74. Release Builds

Release compilation should aggressively apply safe memory optimizations.

Potential passes:

```text
copy elision
drop elimination
retain/release elimination
escape analysis
stack promotion
scalar replacement
allocation elimination
bounds-check elimination
devirtualization
inlining
dead-code elimination
```

Debug compilation may use fewer expensive optimization passes to preserve compiler speed and debugging quality.

---

# 75. Debug Safety

Debug mode may optionally include additional checks useful for detecting runtime/compiler bugs.

Release mode may remove checks proven redundant.

Safe Vut semantics must remain correct in both modes.

---

# 76. Runtime Size

The Vut runtime should remain small.

Do not require large runtime subsystems for features that can be statically compiled.

Prefer compile-time knowledge over runtime metadata when possible.

---

# 77. Threading Preparation

Although concurrency is deferred, the memory model must not make future safe concurrency impossible.

Do not assume all reference counting can always be non-atomic if a representation may later cross threads.

At the same time, do not impose atomic reference counting overhead on all values today merely for hypothetical future sharing.

Thread-sharing semantics will be specified separately.

---

# 78. Performance Benchmarking

Memory-model decisions must be benchmarked.

Required benchmark categories should include:

```text
primitive assignment
small data copy
large data transfer
function arguments
function returns
list creation
list mutation
list iteration
map access
string creation
string concatenation
template strings
interface calls
dyn values
allocation-heavy workloads
short-lived aggregates
```

Measure at least:

```text
execution time
allocation count
peak memory
memory throughput
binary size where relevant
```

---

# 79. Benchmark Against Alternatives

For performance-sensitive representation decisions, compare alternatives.

Examples:

```text
deep copy vs COW
RC vs unique ownership
heap vs stack
inline dyn vs boxed dyn
different list growth strategies
different map implementations
```

Do not adopt a complex strategy because it appears theoretically faster.

Use benchmark evidence.

---

# 80. Memory Profiling

Compiler/runtime development should support profiling of:

```text
allocations
deallocations
bytes allocated
peak heap
retain/release operations
copies
moves
```

where practical.

This information is primarily for compiler/runtime development and benchmarking.

---

# 81. Correctness Before Optimization

Memory optimization must never change observable program behavior.

Priority:

```text
memory safety
↓
correct semantics
↓
predictable behavior
↓
performance
```

Performance is a core requirement, but unsafe incorrect optimization is not acceptable.

---

# 82. No Premature Heap Model Commitment

Internal representations such as:

```text
str
list
map
interface
dyn
```

must not become unnecessarily frozen as public ABI during MVP.

Keep implementation freedom to improve them based on profiling.

FFI-facing layouts must be explicitly specified separately.

---

# 83. Compiler Architecture Rule

Do not implement memory behavior as scattered special cases.

Avoid:

```text
parser knows allocation
type checker emits free
codegen guesses ownership
stdlib manually compensates
```

Instead maintain clear responsibilities:

```text
type system
→ type properties

ownership analysis
→ lifetime/transfer decisions

MIR
→ explicit memory operations

optimizer
→ removes unnecessary operations

codegen
→ emits efficient native implementation

runtime
→ provides required allocation/resource primitives
```

---

# 84. Library Rule

Use mature implementations for general low-level infrastructure when they outperform and simplify custom implementations.

Examples:

```text
allocator integration
hash tables
Unicode
platform memory APIs
backend ABI handling
```

Do not reimplement mature low-level algorithms merely to make Vut self-contained.

Language-specific ownership analysis and Vut semantics remain compiler responsibilities.

---

# 85. MVP Requirements

The MVP memory model must support at minimum:

```text
primitive Copy values
native data layouts
automatic Move
automatic Drop
str ownership
bytes ownership
list ownership
map ownership
nested managed data
function argument ownership
function return ownership
early-return cleanup
interface representation
dyn representation
optional layout
Result-compatible architecture
```

It must not require:

```text
tracing GC
source-level borrow checker
manual memory management
general user-defined destructors
explicit shared-pointer syntax
```

unless a later finalized specification changes these requirements.

---

# 86. Compiler Tests

Add tests covering:

```text
Copy primitive
Copy trivial data
Move return value
Move function argument
value used after assignment
nested managed data
list destruction
map destruction
string destruction
early return
loop break
error propagation cleanup
interface conversion
dyn lifetime
optional values
```

---

# 87. Memory Safety Tests

Test specifically against:

```text
double free
use-after-free
missing drop
incorrect move
incorrect copy
COW mutation aliasing
nested destruction
partial construction failure
early control-flow exit
```

Use sanitizers or equivalent native tooling where appropriate.

---

# 88. Optimization Tests

Ensure optimization preserves semantics.

Compile representative programs under:

```text
debug
release
```

and compare observable results.

Add regression tests for discovered ownership/codegen bugs.

---

# 89. Final Memory Model

The official Vut v1 direction is:

```text
Value semantics
        +
Compiler-managed ownership
        +
Automatic moves
        +
Deterministic automatic destruction
        +
Native static layouts
        +
Stack/register allocation where possible
        +
Heap allocation only where necessary
        +
Escape analysis
        +
Copy elision
        +
Scalar replacement
        +
Selective RC/COW only where beneficial
        +
No tracing GC
        +
No manual free in safe Vut
        +
No source-level borrow checker in MVP
```

---

# 90. Guiding Principle

The user should be able to write simple code:

```vut
fn process():
  user = User(
    name = "Nam",
    age = 20
  )

  values = @(1, 2, 3)

  out("Hello $user.name")
```

without thinking about:

```text
malloc
free
retain
release
lifetimes
borrow syntax
heap ownership
```

while the compiler should be capable of transforming the program into efficient native operations involving:

```text
registers
stack values
direct field offsets
automatic moves
minimal heap allocation
deterministic destruction
specialized collections
direct calls
optimized native code
```

The complexity belongs primarily inside the compiler.

Vut source should remain simple.

Generated programs should remain fast, memory-efficient and predictable.

---

# 91. Async Suspension and Ownership

Async lowering must obey the same ownership model as ordinary code.

A native stack frame does not survive an `await`. Any local live across a
suspension point is moved into the async state object before suspension:

```text
local live across await
    ↓
move into state slot
    ↓
suspend
```

Rules:

```text
state slots own their values exactly once
managed values crossing a suspension retain balanced ownership
a moved-out state slot is not dropped again
a future dropped before completion destroys all live state slots
```

Cleanup runs on every exit path:

```text
normal completion
explicit return
`?` error propagation
cancellation/drop before completion
```

Async must not leak state, double-drop, or create use-after-free. An incomplete
future that is dropped destroys itself and its sub-future recursively.

Detailed requirements belong to:

```text
specs/async/05-memory.md
```

Async does not change Vut's fundamental memory model:

```text
value semantics
automatic moves
deterministic destruction
no tracing GC
no source-level borrow checker
```
