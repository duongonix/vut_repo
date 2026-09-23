
# Vut FFI — Safety, Diagnostics and Testing

## 1. FFI is unsafe by default

Raw native calls có thể:

* dereference invalid pointer;
* use-after-free;
* buffer overflow;
* violate ownership;
* call incompatible ABI;
* invoke undefined behavior.

Do đó raw FFI phải được đánh dấu unsafe ở language level.

---

## 2. `unsafe` context

Canonical syntax đề xuất:

```vut
unsafe:
  native_call()
```

Ví dụ:

```vut
extern "C" fn native_add(a: i32, b: i32) -> i32

fn add(a: i32, b: i32) -> i32:
  unsafe:
    native_add(a, b)
```

Nếu Vut chưa có `unsafe`, task FFI phải thêm syntax/semantic support phù hợp hoặc tạo spec dependency rõ ràng.

Không silently cho raw FFI call trong safe code.

---

## 3. Safe wrapper pattern

Raw layer:

```vut
extern "C" fn _engine_create() -> ptr[NativeEngine]
```

Public wrapper:

```vut
fn create_engine() -> Engine:
  unsafe:
    handle = _engine_create()

  Engine(_handle: handle)
```

User bình thường không cần làm việc trực tiếp với raw pointer.

---

## 4. Unsafe operations

Ít nhất các operation sau phải yêu cầu unsafe:

```text
calling extern function
raw pointer dereference
raw pointer arithmetic, nếu sau này support
constructing raw pointer from integer
FFI memory mutation through pointer
```

Có thể relax một số extern function thành safe sau này nếu compiler có annotation/contracts, nhưng không trong MVP.

---

## 5. FFI-safe type validation

Compiler phải validate mọi extern parameter/return type.

Ví dụ invalid:

```vut
extern "C" fn foo(value: str)
```

Diagnostic:

```text
error[E....]: type `str` is not FFI-safe

help: pass a native pointer and explicit length instead
```

---

## 6. Non-repr data diagnostic

Invalid:

```vut
data Point:
  x: f32
  y: f32

extern "C" fn process(point: Point)
```

Diagnostic phải gợi ý:

```vut
@repr(C)
data Point:
```

nếu fields tương thích FFI.

---

## 7. Invalid ABI

```vut
extern "Rust" fn foo()
```

Diagnostic:

```text
error[E....]: unsupported FFI ABI `Rust`

supported ABIs:
  C
```

---

## 8. Extern body

Invalid:

```vut
extern "C" fn foo():
  123
```

Diagnostic:

```text
extern functions cannot have a Vut function body
```

---

## 9. Invalid `@link_name`

Invalid:

```vut
@link_name(foo)
extern "C" fn bar()
```

must report:

```text
link_name requires a string literal
```

Invalid target:

```vut
@link_name("foo")
fn bar():
  ...
```

must report:

```text
link_name is only valid on external/native declarations
```

---

## 10. Invalid `@repr(C)`

Invalid:

```vut
@repr(C)
data Bad:
  items: list[i32]
```

must report exact incompatible field.

Example:

```text
error: field `items` is not FFI-safe

type:
  list[i32]
```

---

## 11. Opaque type restrictions

Invalid:

```vut
opaque data Engine

e = Engine()
```

must fail.

Invalid field access:

```vut
engine.value
```

must fail because opaque data has no visible layout.

---

## 12. Pointer type safety

Compiler must distinguish:

```text
ptr[Window]
ptr[Device]
ptr[Engine]
```

No implicit conversion between unrelated pointer pointee types.

Explicit `ptr[void]` conversion may be allowed only under well-defined rules.

Do not add unrestricted pointer casts by default.

---

## 13. Null pointer safety

If native function may return null:

```vut
extern "C" fn create() -> ptr[NativeObject]
```

FFI layer cannot pretend result is guaranteed valid.

Exact null-check API must follow pointer semantics defined elsewhere.

Do not invent Optional conversion automatically.

---

## 14. Native callback safety

Callback signature must exactly match expected C ABI signature.

Example mismatch must fail compile-time:

```text
expected:
extern "C" fn(i32) -> i32

found:
fn(str) -> str
```

---

## 15. Panic/exception rule

Document clearly:

```text
FFI boundary is non-unwinding.
```

Native package author must catch:

```text
Rust panic
C++ exception
```

before returning through C ABI.

Vut compiler/runtime is not required to recover foreign unwind.

---

## 16. Tests — parser

Add parser tests:

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

```vut
@link_name("native_add")
extern "C" fn add(a: i32, b: i32) -> i32
```

```vut
opaque data Engine
```

```vut
type Callback = extern "C" fn(i32)
```

---

## 17. Tests — type checking

Valid:

```text
fixed integers
floats
usize/isize
pointers
repr(C) structs
transparent structs
opaque pointers
function pointers
```

Invalid:

```text
str
bytes
list
map
dyn
interface
result
optional
normal non-repr data
```

---

## 18. Tests — layout

Verify:

```vut
@repr(C)
data Point:
  x: f32
  y: f32
```

matches C equivalent for:

```text
size
alignment
field offsets
```

Test nested repr(C) types.

Test padding.

Test transparent wrapper.

---

## 19. Tests — codegen

Compile native C fixture:

```c
int add(int a, int b) {
    return a + b;
}
```

Vut:

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

Build and execute.

Expected:

```text
add(2, 3) == 5
```

---

## 20. Tests — `@link_name`

Native:

```c
int native_add(int a, int b) {
    return a + b;
}
```

Vut:

```vut
@link_name("native_add")
extern "C" fn add(a: i32, b: i32) -> i32
```

Build and execute successfully.

---

## 21. Tests — repr(C)

C:

```c
typedef struct {
    float x;
    float y;
} Point;

float point_sum(Point p) {
    return p.x + p.y;
}
```

Vut:

```vut
@repr(C)
data Point:
  x: f32
  y: f32

extern "C" fn point_sum(point: Point) -> f32
```

Test by-value struct calling convention.

---

## 22. Tests — opaque handle

Native fixture:

```c
typedef struct Counter Counter;

Counter* counter_create();
void counter_destroy(Counter*);
void counter_inc(Counter*);
int counter_get(Counter*);
```

Vut raw bindings:

```vut
opaque data Counter

extern "C" fn counter_create() -> ptr[Counter]
extern "C" fn counter_destroy(counter: ptr[Counter])
extern "C" fn counter_inc(counter: ptr[Counter])
extern "C" fn counter_get(counter: ptr[Counter]) -> i32
```

Build end-to-end.

---

## 23. Tests — callbacks

C fixture:

```c
typedef int (*callback_t)(int);

int call_callback(callback_t callback, int value) {
    return callback(value);
}
```

Vut:

```vut
type Callback = extern "C" fn(i32) -> i32

extern "C" fn call_callback(
  callback: Callback,
  value: i32
) -> i32

fn double(value: i32) -> i32:
  value * 2
```

Expected:

```text
call_callback(double, 10) == 20
```

---

## 24. Tests — Rust staticlib

Create integration fixture Rust crate:

```toml
[lib]
crate-type = ["staticlib"]
```

Rust exports:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}
```

Vut must call it exactly the same as C:

```vut
extern "C" fn rust_add(a: i32, b: i32) -> i32
```

This proves Vut does not require a Rust-specific ABI.

---

## 25. Tests — C++ bridge

Optional integration fixture if CI toolchain supports C++.

C++ implementation should expose:

```cpp
extern "C"
```

Vut uses normal C ABI declaration.

No C++ ABI-specific compiler code is allowed.

---

## 26. VPM integration test

Create package fixture:

```text
app
 └─ package
     ├─ Vut wrapper
     └─ native static library
```

Run:

```text
vpm install
vut build
```

Verify native artifact is automatically linked.

User must not manually pass linker flags.

---

## 27. Regression tests

FFI implementation must not break:

```text
normal fn
data
methods
list literal @[...]
attributes
module resolution
native package download
existing linker flow
```

Specially ensure:

```vut
@[1, 2, 3]
```

continues to parse as list literal.

---

## 28. Compiler architecture

Implement across proper crates/layers.

Do not put all FFI logic in parser or codegen.

Expected responsibilities:

```text
lexer/parser:
syntax

AST:
extern declarations

resolver:
symbols

type checker:
FFI-safe validation

layout/memory:
repr(C), transparent

HIR/MIR:
typed native call representation

codegen:
C ABI lowering

linker:
native artifacts

runtime:
callback/thread entry where necessary
```

---

## 29. No placeholder implementation

Feature is not complete if compiler merely parses:

```vut
extern "C"
```

but does not actually emit native call ABI.

Likewise `@repr(C)` cannot be silently stored and ignored.

Likewise `@link_name` must change linked symbol.

---

## 30. Definition of Done

FFI v1 is complete only when all of these work:

```text
C native function
      ↓
static library
      ↓
VPM/compiler linker
      ↓
extern "C" declaration
      ↓
typed MIR/native call
      ↓
C ABI codegen
      ↓
working executable
```

and:

```text
@repr(C)
opaque data
ptr[T]
@link_name
callbacks
unsafe
```

are implemented according to specs.

At minimum prove end-to-end interoperability with:

```text
C
Rust staticlib
```

C++ bridge should require no Vut-specific ABI implementation beyond C ABI.

After implementation:

1. run Rust formatter;
2. run Clippy;
3. run compiler tests;
4. run FFI integration tests;
5. run existing project tests;
6. update roadmap/features/phases;
7. document any intentionally deferred functionality such as:

   * varargs;
   * dynamic libraries;
   * foreign-thread callbacks;
   * richer native enum ABI;
   * automatic header generation.

---

## 31. Deferred in FFI v1

The following are intentionally not implemented in FFI v1 and must not be
silently substituted:

```text
by-value repr(C) struct arguments/returns   (E8004; pass ptr[T])
raw pointer dereference                     (no source syntax yet)
raw pointer arithmetic                      (no source syntax yet)
pointer-from-integer construction           (no cast syntax yet)
varargs                                     (rejected)
dynamic libraries (.dll/.so/.dylib)         (static only)
remote native artifact download/checksum    (local [native] paths only)
foreign-thread callbacks                    (runtime attach not implemented)
rich typed enum ABI                         (use explicit integer types)
automatic header generation                 (not planned for v1)
```

Diagnostics codes used by FFI v1:

```text
E8001  unsafe operation outside unsafe context
E8004  invalid / non-FFI-safe type across the boundary
E8005  unknown ABI
E8006  invalid external declaration or attribute
E8007  external function has a body
E8013  capturing closure used as an FFI callback
E8014  invalid @repr(C) field (managed handle / resource / non-FFI-safe)
```

The normative Public Native ABI v1 and the Vut-internal Runtime Handle ABI are
specified in `specs/ffi/06-native-abi-v1.md`.
