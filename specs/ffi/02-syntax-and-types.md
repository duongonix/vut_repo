
# Vut FFI — Syntax and Types

## 1. Extern function declaration

Canonical syntax:

```vut
extern "C" fn function_name(
  arg: Type
) -> ReturnType
```

Ví dụ:

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

Không có function body.

Invalid:

```vut
extern "C" fn add(a: i32, b: i32) -> i32:
  a + b
```

Extern declaration chỉ khai báo symbol bên ngoài.

---

## 2. Return type

Không có return value:

```vut
extern "C" fn destroy(value: ptr[void])
```

Có return value:

```vut
extern "C" fn size() -> usize
```

MVP không support multiple return values.

---

## 3. Allowed primitive FFI types

FFI v1 hỗ trợ trực tiếp:

```text
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

usize
isize

ptr[T]
ptr[void]
```

`bool` là FFI-safe trong Native ABI v1 và dùng **target C ABI representation và
calling convention của C `_Bool`** (không hard-code 1 byte). Nếu Vut sau này muốn
khóa ABI 1 byte thì phải định nghĩa đó là ABI `uint8` (C-facing dùng `uint8_t`),
không gọi là `_Bool`. Xem `specs/ffi/06-native-abi-v1.md`.

---

## 4. `int` và `float`

Không dùng generic Vut:

```text
int
float
```

trong raw FFI nếu kích thước ABI của chúng chưa được khóa.

Ví dụ nên viết:

```vut
extern "C" fn native_add(a: i32, b: i32) -> i32
```

không:

```vut
extern "C" fn native_add(a: int, b: int) -> int
```

Type checker phải reject ABI-ambiguous types trong extern declaration.

---

## 5. Raw pointers

Syntax:

```vut
ptr[T]
```

Ví dụ:

```vut
ptr[u8]
ptr[i32]
ptr[NativeEngine]
ptr[void]
```

`ptr[void]` tương đương opaque untyped native pointer.

Không hỗ trợ pointer arithmetic trong safe Vut.

Pointer operations phải thuộc unsafe context hoặc dedicated APIs.

---

## 6. Null pointers

FFI cần representation cho null pointer.

Nếu Vut pointer type có null support chính thức, dùng representation đó.

Không tự biến:

```text
T?
```

thành C pointer semantics.

Optional và pointer nullability là hai vấn đề khác nhau.

Compiler không được invent implicit `ptr[T] ↔ T?` conversion.

---

## 7. Opaque data

Syntax:

```vut
opaque data NativeEngine
```

Opaque data:

* không có fields;
* không thể instantiate từ Vut;
* không thể đọc layout;
* chỉ dùng như native pointee type.

Ví dụ:

```vut
opaque data NativeWindow

extern "C" fn window_create() -> ptr[NativeWindow]
extern "C" fn window_destroy(window: ptr[NativeWindow])
```

---

## 8. C-compatible data

Data truyền by-value qua FFI phải có:

```vut
@repr(C)
```

Ví dụ:

```vut
@repr(C)
data Point:
  x: f32
  y: f32
```

Extern:

```vut
extern "C" fn distance(point: Point) -> f32
```

Không được truyền normal Vut data trực tiếp:

```vut
data Point:
  x: f32
  y: f32

extern "C" fn distance(point: Point) -> f32
```

nếu không có explicit ABI representation.

### FFI v1 restriction: by-value repr(C) is deferred

FFI v1 passes aggregate data through `ptr[T]` only. A `@repr(C)` `data` type is
valid as a pointer pointee and as a nested field of another `@repr(C)` type,
but passing it **by value** across an extern boundary is rejected with
`E8004` until target-specific struct ABI lowering is implemented.

```vut
@repr(C)
data Point:
  x: f32
  y: f32

extern "C" fn distance(point: Point) -> f32        # rejected in v1
extern "C" fn distance(point: ptr[Point]) -> f32   # accepted
```

---

## 9. Transparent wrapper

Support:

```vut
@repr(transparent)
data NativeId:
  value: u64
```

Rules:

* chỉ một representation-carrying field;
* ABI tương đương field đó;
* không có managed layout hidden;
* type checker/layout checker validate.

---

## 10. Strings

`str` không phải `char*`.

Raw FFI dùng `ptr[u8]` (+ `usize` length) cho public C ABI:

```vut
extern "C" fn puts(value: ptr[u8]) -> i32
```

Conversion từ Vut `str` sang C string phải explicit và đảm bảo lifetime.

Trong **Runtime Handle ABI** (chỉ compiler/stdlib/official runtime — xem
`specs/ffi/06-native-abi-v1.md`), `str` đi qua boundary như một runtime-owned
handle pointer: tham số được borrow trong thời gian gọi, giá trị trả về chuyển
ownership cho Vut. Third-party C không được dùng representation này.

---

## 11. Buffers

Pattern chuẩn:

```text
pointer + length
```

Ví dụ:

```vut
extern "C" fn process(
  data: ptr[u8],
  len: usize
) -> i32
```

Không truyền trực tiếp qua **public** C ABI:

```vut
list[u8]
bytes
[u8, N]
```

Trong Runtime Handle ABI, `bytes`/`list[T]` đi qua như một runtime-owned handle
pointer (borrow vào, owned ra); third-party C vẫn dùng `ptr[u8] + len`
(xem `specs/ffi/06-native-abi-v1.md`).

---

## 12. Out parameters

C-style out parameter phải support:

```vut
extern "C" fn engine_create(
  out: ptr[ptr[NativeEngine]]
) -> i32
```

Compiler phải support nested pointer types:

```text
ptr[ptr[T]]
```

---

## 13. Function pointer types

Callback type:

```vut
type NativeCallback = extern "C" fn(i32)
```

Có return:

```vut
type CompareFn = extern "C" fn(
  a: ptr[void],
  b: ptr[void]
) -> i32
```

Function pointer ABI là C ABI.

---

## 14. Callback arguments

Extern functions có thể nhận callback:

```vut
type EventCallback = extern "C" fn(
  event: i32,
  user_data: ptr[void]
)

extern "C" fn set_callback(
  callback: EventCallback,
  user_data: ptr[void]
)
```

MVP callback target chỉ cần support non-capturing Vut functions.

Không cần capture closure trong FFI v1.

---

## 15. Varargs

C variadic functions như:

```c
printf(const char*, ...)
```

không cần support trong FFI MVP.

Nếu gặp:

```vut
extern "C" fn printf(...)
```

parser/type checker phải reject nếu varargs chưa được implement.

Không implement partial unsafe varargs behavior.

---

## 16. Unsupported FFI types

FFI type checker phải reject trực tiếp trong raw extern signatures:

```text
int
float
map[K, V]
dyn
interface
result[T, E]
T?
array[T, N]
enum
by-value data (non-ptr)
```

`str`, `bytes`, `list[T]`, và `resource[T]` **được** phép qua **Runtime Handle
ABI** (compiler/stdlib/official runtime). Third-party C phải dùng
`ptr[u8] + usize` (xem `specs/ffi/06-native-abi-v1.md`).

Diagnostic phải nói rõ type nào không ABI-safe.

---

## 17. Symbol names

Default:

```vut
extern "C" fn native_add(a: i32, b: i32) -> i32
```

symbol:

```text
native_add
```

Override:

```vut
@link_name("library_native_add")
extern "C" fn native_add(a: i32, b: i32) -> i32
```

symbol:

```text
library_native_add
```

---

## 18. Naming visibility

Extern declarations tuân theo visibility hiện tại:

```text
normal name = public
_name = module-private
```

Ví dụ raw FFI package nên dùng:

```vut
@link_name("engine_create")
extern "C" fn _engine_create() -> ptr[NativeEngine]
```

và expose wrapper public riêng.

---

## 19. Owned native resources

Opaque native objects whose lifetime must be deterministic may be exposed as an
owned resource handle:

```vut
opaque data Engine

extern "C" fn engine_create() -> resource[Engine]
extern "C" fn engine_use(engine: resource[Engine])
```

`resource[T]` is a move-only managed handle (see `specs/08-memory-model.md`
§60). `T` must be an `opaque data`, `@repr(C)` data, scalar, pointer, or
`void` pointee.

A `resource[T]` may be passed to an `extern` parameter of type `ptr[T]` (or
`ptr[void]`) as a **borrow**: the native code receives the inner native pointer
and Vut keeps ownership (no move, no retain/release, destructor still runs
exactly once). The borrow is internal; `ptr[T]` is only the native ABI type at
that boundary, not a language-level borrow form. This lets native code observe a
resource repeatedly, e.g. a streaming reader. See `specs/08-memory-model.md`
§30.2.

FFI ownership modes:

```text
parameter (default)   borrowed for the duration of the call
parameter (consumed)  ownership transfers to native   (reserved)
return                ownership transfers to Vut
```

MVP exposes borrowed parameters and owned returns. A future `consumed`
parameter qualifier changes only Vut-side lowering (move instead of borrow); it
does not change the C ABI call shape, so the extension point is preserved.
Native code must not retain a borrowed resource past the call.
