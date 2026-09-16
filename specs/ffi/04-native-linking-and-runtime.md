
# Vut FFI — Native Linking and Runtime

## 1. Scope

File này định nghĩa cách compiler/VPM liên kết native artifacts với extern declarations.

FFI declaration không tự chứa path library.

---

## 2. Separation

Source code:

```vut
extern "C" fn hello() -> i32
```

Manifest (FFI v1, local static libraries):

```toml
[native]
libraries = ["native/hello.lib"]
```

Vut source định nghĩa ABI.

VPM định nghĩa native artifact.

Remote artifact download with URL and checksum is deferred. The future shape
is reserved as:

```toml
[[native.x86_64-pc-windows-msvc.static]]
url = "https://..."
sha256 = "..."
```

When implemented, VPM must download the correct target artifact, verify its
checksum, cache it, and pass it transitively to the link plan.

---

## 3. No hardcoded library paths

Không support:

```vut
@link("C:/libs/hello.lib")
```

hoặc:

```vut
extern "C" "hello.lib" fn hello()
```

Library resolution thuộc build/package system.

---

## 4. Build flow

```text
vpm add/install
   ↓
download correct native artifact
   ↓
verify checksum
   ↓
cache artifact
   ↓
dependency graph
   ↓
Vut compiler generates object files
   ↓
link plan collects native artifacts
   ↓
linker
   ↓
final executable
```

---

## 5. Static libraries

FFI v1 ưu tiên:

```text
Windows → .lib
Linux   → .a
macOS   → .a
```

Compiler/linker phải nhận artifact path từ VPM dependency resolution.

---

## 6. Transitive dependencies

Nếu:

```text
app
 └─ package-a
     └─ package-b
         └─ native-b.lib
```

build app phải link:

```text
native-b.lib
```

Native artifacts phải được thu thập transitively từ dependency graph.

---

## 7. Link plan

Nên có typed internal representation tương đương:

```text
LinkPlan
```

chứa:

```text
object files
static libraries
system libraries
frameworks
output
target
link flags
```

Không truyền native library paths bằng ad-hoc string list qua nhiều layer.

---

## 8. Duplicate libraries

Nếu cùng artifact xuất hiện qua nhiều dependency path, linker input phải deduplicate hợp lý.

Không link cùng exact artifact nhiều lần chỉ vì dependency graph có diamond shape.

---

## 9. Link order

Trên platform/linker cần order-sensitive static libs, compiler/linker integration phải preserve dependency-safe ordering.

Không sort alphabetically nếu làm sai semantics.

---

## 10. Missing symbol

Nếu extern declaration không có symbol tương ứng trong native libraries, linker phải fail bình thường.

Diagnostic nên enrich nếu có thể:

```text
error: unresolved native symbol `engine_create`
```

Nếu symbol đến từ:

```vut
@link_name("engine_create")
```

diagnostic nên hiển thị native symbol.

---

## 11. Runtime ownership

Vut runtime không tự destroy native handles.

Ví dụ:

```vut
ptr(NativeEngine)
```

không có automatic destructor semantics trong FFI v1.

Safe wrapper phải quản lý lifecycle.

---

## 12. Native memory

Memory allocated bởi native library nên được free bởi đúng native API.

Không mặc định dùng Vut allocator để free memory tạo từ Rust/C/C++.

Ví dụ:

```text
native_create()
native_destroy()
```

hoặc:

```text
native_alloc()
native_free()
```

phải paired đúng phía native.

---

## 13. Vut buffers passed to native

Khi Vut truyền pointer tới buffer cho native function:

```text
pointer validity
lifetime
mutation rules
```

phải được giữ trong suốt call.

Native code không được giữ pointer sau call trừ khi API contract và wrapper đảm bảo ownership/lifetime riêng.

Compiler không tự assume escaping pointer là safe.

---

## 14. Callbacks

Callback native → Vut cần trampoline/codegen tương thích C ABI.

Example:

```vut
type Callback = extern "C" fn(
  value: i32,
  user_data: ptr(void)
)
```

Vut callback:

```vut
fn on_value(value: i32, user_data: ptr(void)):
  ...
```

Nếu signature compatible, có thể pass function pointer.

---

## 15. Capturing closures

MVP không support direct C callback từ capturing closure.

Ví dụ không bắt buộc support:

```vut
value = 10

callback = fn x:
  x + value
```

Thay vào đó dùng:

```text
callback function
+
user_data pointer
```

pattern.

---

## 16. Thread callbacks

Native library có thể callback từ thread không do Vut tạo.

Nếu Vut runtime chưa support foreign-thread entry, phải document/reject hoặc provide runtime attach mechanism.

Không silently assume mọi callback thread-safe.

Nếu chưa triển khai trong MVP, callback contract có thể giới hạn:

```text
callbacks must occur on a Vut-compatible/runtime-attached thread
```

và diagnostic/docs phải rõ.

---

## 17. Dynamic libraries

Dynamic linking:

```text
.dll
.so
.dylib
```

không bắt buộc trong FFI v1 nếu VPM hiện chỉ hỗ trợ static artifacts.

Architecture không được khóa khả năng thêm sau.

---

## 18. System libraries

Build model nên có khả năng sau này link:

```text
Windows:
d3d12
dxgi

Linux:
pthread
dl

macOS:
Metal.framework
QuartzCore.framework
```

Nếu VPM/linker hiện đã support, integrate.

Nếu chưa, giữ typed extension point.

---

## 19. Rust-backed package example

Rust:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn image_width(handle: *const Image) -> u32 {
    ...
}
```

build:

```text
crate-type = ["staticlib"]
```

outputs:

```text
Windows → image_core.lib
Linux   → libimage_core.a
macOS   → libimage_core.a
```

Vut:

```vut
opaque data NativeImage

extern "C" fn image_width(
  handle: ptr(NativeImage)
) -> u32
```

Package user không cần Rust hoặc Cargo.

---

## 20. C++-backed package example

C++:

```cpp
class Engine {
public:
    int run();
};
```

bridge:

```cpp
extern "C" Engine* engine_create();
extern "C" void engine_destroy(Engine*);
extern "C" int engine_run(Engine*);
```

Vut chỉ giao tiếp với bridge này.

Không parse C++ mangled symbol/classes/vtable ABI trực tiếp.
