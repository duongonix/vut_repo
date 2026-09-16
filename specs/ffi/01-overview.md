
# Vut FFI — Overview

## 1. Mục tiêu

FFI của Vut cho phép code Vut giao tiếp với native libraries bên ngoài.

FFI v1 chỉ định nghĩa trực tiếp:

```text
C ABI
```

Vut không có:

```text
Rust ABI
C++ ABI
Swift ABI
```

riêng.

Các ngôn ngữ khác phải expose API thông qua C-compatible ABI.

Ví dụ:

```text
Vut
 ↓
C ABI
 ├─ C
 ├─ C++
 ├─ Rust
 ├─ Objective-C
 └─ Zig
```

Điều này giúp Vut có ABI boundary ổn định và không phụ thuộc implementation details của ngôn ngữ khác.

---

## 2. Use cases

FFI phải hỗ trợ các use case chính:

* gọi C library;
* viết Vut wrapper cho Rust crate;
* viết Vut wrapper cho C++ library;
* dùng Vulkan/OpenGL/SDL;
* dùng native OS APIs;
* dùng static libraries từ VPM;
* callback từ native code về Vut;
* opaque native handles;
* C-compatible structs.

---

## 3. Design principles

FFI phải tuân theo:

```text
explicit
predictable
ABI-stable
unsafe where required
zero hidden conversion
```

Không tự convert managed Vut type thành native ABI type nếu không có rule rõ ràng.

Không cho phép backend tự suy đoán layout.

---

## 4. FFI layers

Architecture:

```text
Vut safe API
     ↓
Vut wrapper
     ↓
raw FFI declarations
     ↓
C ABI
     ↓
native library
```

Ví dụ package:

```text
hello/
├── src/
│   ├── mod.vut
│   └── _native.vut
└── vpm.toml
```

`_native.vut` chứa raw extern declarations.

`mod.vut` expose safe Vut API.

---

## 5. FFI declaration

Canonical syntax:

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

`"C"` là ABI identifier.

FFI v1 chỉ support:

```text
"C"
```

Các ABI khác phải bị reject.

Ví dụ invalid:

```vut
extern "Rust" fn foo()
extern "C++" fn foo()
```

---

## 6. Native symbol remapping

Dùng Attribute:

```vut
@link_name("native_add")
extern "C" fn add(a: i32, b: i32) -> i32
```

Tên trong Vut:

```text
add
```

Native symbol:

```text
native_add
```

`@link_name` chỉ thay đổi symbol name, không thay đổi ABI.

---

## 7. Struct layout

C-compatible data:

```vut
@repr(C)
data Point:
  x: f32
  y: f32
```

Không được giả định mọi `data` của Vut có C layout.

Chỉ type có ABI representation rõ ràng mới được truyền trực tiếp qua extern boundary.

---

## 8. Opaque native types

Native object pointer nên dùng opaque type:

```vut
opaque data NativeEngine
```

FFI:

```vut
extern "C" fn engine_create() -> ptr(NativeEngine)
extern "C" fn engine_destroy(engine: ptr(NativeEngine))
```

Không expose internal layout.

---

## 9. Ownership boundary

FFI không tự suy đoán ownership.

Ví dụ:

```vut
extern "C" fn engine_create() -> ptr(NativeEngine)
```

Compiler không tự biết pointer này phải free như thế nào.

Wrapper package chịu trách nhiệm gọi:

```vut
engine_destroy(...)
```

Nếu API native trả memory cần giải phóng, raw FFI wrapper phải expose corresponding destroy/free function.

---

## 10. Managed Vut types

Các type sau không được truyền trực tiếp qua C ABI trong FFI v1:

```text
str
bytes
list(T)
map(K, V)
dyn
result(T, E)
T?
interface values
managed data
```

trừ khi type đó sau này có explicit stable ABI representation được spec bổ sung.

Raw FFI phải dùng ABI-safe types.

---

## 11. No automatic native build

FFI không compile:

```text
.c
.cpp
.rs
.m
.mm
```

trên máy user.

Native artifacts phải được VPM cung cấp dưới dạng prebuilt:

```text
.lib
.a
```

hoặc dynamic libraries nếu feature đó được hỗ trợ sau này.

---

## 12. Native library source

Native implementation có thể viết bằng bất kỳ ngôn ngữ nào miễn expose C ABI.

Rust:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

C++:

```cpp
extern "C" int add(int a, int b) {
    return a + b;
}
```

Vut:

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

---

## 13. Exceptions and panics

Không được unwind qua FFI boundary.

C++ exception:

```text
must be caught before returning to Vut
```

Rust panic:

```text
must not unwind into Vut
```

Native wrappers phải convert failure thành:

```text
error code
out parameter
nullable pointer
ABI-safe result struct
```

---

## 14. Compiler pipeline

FFI phải đi qua architecture bình thường:

```text
Parser
 ↓
AST
 ↓
Resolver
 ↓
HIR
 ↓
Type checker
 ↓
MIR
 ↓
Codegen
 ↓
Linker
```

Không bypass type checker bằng raw backend hacks.

---

## 15. Source of truth

Các FFI specs trong folder này là source of truth cho:

* extern declarations;
* ABI-safe types;
* native layout;
* callbacks;
* linker integration;
* FFI diagnostics;
* unsafe behavior.

Attribute rules chung vẫn thuộc Attribute spec.

VPM artifact download vẫn thuộc VPM specs.

---


