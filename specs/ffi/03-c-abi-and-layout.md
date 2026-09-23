
# Vut FFI — C ABI and Native Layout

## 1. ABI model

FFI v1 chỉ support:

```text
C ABI
```

Xem `specs/ffi/06-native-abi-v1.md` cho normative Public Native ABI v1 và
Vut-internal Runtime Handle ABI.

Compiler phải sử dụng calling convention tương ứng target.

Không hardcode ABI chỉ cho một OS.

Target triple quyết định ABI details.

---

## 2. Function calling convention

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

phải lower thành native function declaration với C calling convention của target.

Codegen không được treat extern function như normal Vut function ABI.

---

## 3. C integer mapping

Canonical mapping:

```text
Vut     Native semantic
-----------------------
i8      signed 8-bit
i16     signed 16-bit
i32     signed 32-bit
i64     signed 64-bit

u8      unsigned 8-bit
u16     unsigned 16-bit
u32     unsigned 32-bit
u64     unsigned 64-bit
```

Không map dựa vào C `int`, `long` một cách mơ hồ.

Native wrappers nên dùng `<stdint.h>` equivalents.

`bool` dùng target C ABI representation và calling convention của C `_Bool`
(không hard-code 1 byte). `int`/`float` không dùng trong FFI.

---

## 4. Pointer layout

```text
ptr[T]
```

có native pointer size/alignment của target.

Ví dụ:

```text
x86_64 → 64-bit pointer
aarch64 → 64-bit pointer
```

Compiler phải lấy từ target layout, không hardcode 64-bit globally.

---

## 5. `usize` / `isize`

`usize` và `isize` có pointer width của target.

Dùng cho:

```text
size
length
native indexing
buffer size
```

---

## 6. `@repr(C)`

Example:

```vut
@repr(C)
data Point:
  x: f32
  y: f32
```

Requirements:

* preserve declaration field order;
* use C-compatible alignment rules;
* insert padding tương ứng target ABI;
* no hidden managed metadata;
* no field reordering;
* layout metadata phải canonical trong type system.

---

## 7. Nested repr types

Allowed:

```vut
@repr(C)
data Vec2:
  x: f32
  y: f32

@repr(C)
data Rect:
  min: Vec2
  max: Vec2
```

chỉ khi mọi nested field đều FFI-safe.

---

## 8. Invalid fields in repr(C)

Compiler phải reject:

```vut
@repr(C)
data Bad:
  values: list[i32]
```

nếu `list[i32]` chưa có C ABI representation.

Tương tự:

```text
str
bytes
map
dyn
interface
managed values
```

---

## 9. `@repr(transparent)`

Example:

```vut
@repr(transparent)
data Handle:
  raw: ptr[void]
```

Requirements:

* exactly one ABI-carrying field;
* same size;
* same alignment;
* same ABI classification as wrapped field.

Reject invalid transparent representations.

---

## 10. ABI classification

Compiler type/layout system phải có query tương đương:

```text
is_ffi_safe(type)
```

và metadata:

```text
size
alignment
layout
ABI class
needs_drop
managed?
```

Không duplicate FFI safety logic trong parser/codegen.

---

## 11. Return structs

Nếu target C ABI cho phép struct return, codegen phải tuân theo target calling convention.

Không tự assume struct luôn return trong register.

Dùng ABI lowering của backend/target implementation.

---

## 12. Struct arguments

By-value repr(C) struct phải được pass theo ABI rules target.

Ví dụ:

```vut
extern "C" fn process(point: Point)
```

Không lower thành pointer trừ khi target ABI yêu cầu indirect passing.

---

## 13. Alignment

Nếu sau này có:

```vut
@align(16)
```

thì alignment phải integrate với layout system.

FFI v1 không cần implement `@align` nếu chưa có Attribute này.

Không invent behavior.

---

## 14. Native enums

Vut enum không được coi mặc định là C enum.

FFI v1 nên dùng integer representation explicit.

Ví dụ native:

```c
enum Status {
    STATUS_OK = 0,
    STATUS_ERROR = 1
};
```

Vut raw FFI:

```vut
type NativeStatus = i32
```

Typed enum ABI có thể bổ sung sau bằng explicit repr.

---

## 15. Function pointers

Function pointer ABI phải chứa:

```text
C calling convention
native function address
```

Callback signature phải được ABI-validated như extern function signature.

---

## 16. No unwind across boundary

Compiler/runtime không đảm bảo unwind interoperability.

Contract:

```text
native exceptions/panic must not cross C ABI boundary
```

Package author chịu trách nhiệm catch/convert failure.

---

## 17. ABI target consistency

Native artifact target phải match compiler target.

Ví dụ:

```text
x86_64-pc-windows-msvc
```

không được link artifact:

```text
x86_64-unknown-linux-gnu
```

Link plan phải giữ target identity.

---

## 18. Backend independence

FFI semantics không được phụ thuộc Cranelift implementation detail.

Nếu codegen backend thay đổi, C ABI behavior vẫn phải giữ nguyên.

---

