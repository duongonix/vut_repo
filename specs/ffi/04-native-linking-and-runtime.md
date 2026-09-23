
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

Local development may link prebuilt static libraries with `[native] libraries`.
Registry packages instead publish native **source** and describe how it is built:

```toml
[native]
build = "native/build.toml"
```

The registry/trusted-CI side resolves that source into a per-target artifact
(absolute URL + trusted SHA-256 + size) recorded in `native-artifacts.toml`.
VPM downloads the correct target artifact, verifies its trusted checksum, caches
it, and passes it transitively to the link plan. See `## 21` below.

`[native] libraries` and `[native] build` are mutually exclusive, and a registry
package must not ship prebuilt binaries.

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

VPM collects the resolved per-target artifacts, system libraries, and library
search paths across the whole graph, de-duplicates them, and orders them
deterministically before building the link plan. The user must never pass
`--native-lib` or otherwise list native libraries by hand.

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

De-duplication is by the resolved local artifact path (which is
content-addressed by the trusted SHA-256), so two dependency paths that resolve
to the same artifact contribute one linker input.

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
ptr[NativeEngine]
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
  user_data: ptr[void]
)
```

Vut callback:

```vut
fn on_value(value: i32, user_data: ptr[void]):
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
  handle: ptr[NativeImage]
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

---

## 21. Registry native build metadata

A registry package publishes native source together with `native/build.toml`:

```text
package/
├── vpm.toml            # [native] build = "native/build.toml"
└── native/
    ├── build.toml
    └── src/
```

`native/build.toml` selects one structured backend:

```toml
backend = "cmake"          # cmake | cargo | make | cc | custom

[output]
name = "math"

[cmake]
source = "native/src"
build_dir = "native/.build"
config = "Release"

[target.x86_64-pc-windows-msvc]
artifact = "native/.build/Release/math.lib"
system_libraries = ["user32"]

[target.aarch64-apple-darwin]
backend = "cargo"
artifact = "native/target/release/libmath.a"
```

Rules:

* `cmake`, `cargo`, `make`, and `cc` are first-class; `custom` is a single
  command escape hatch, not the primary abstraction.
* Per-target overrides may select another backend and/or `artifact` and
  `system_libraries`; otherwise they inherit the top-level backend.
* All paths are relative to the package root and must stay inside it.
* The Vut↔native boundary is always the stable C ABI; the build backend only
  produces the static library.

The two metadata layers are distinct:

```text
native/build.toml        (publisher source; author-owned)
native-artifacts.toml    (registry/CI resolved; trusted, excluded from the
                          source BLAKE3 checksum)
```

Publishing source → review/merge → trusted CI builds per target → upload →
compute SHA-256 → generate `native-artifacts.toml`. A publisher must not submit
arbitrary prebuilt binaries as official artifacts.

### `native-artifacts.toml`

```toml
format = 1

[target.x86_64-pc-windows-msvc]
url = "https://cdn.example.com/math/1.0.0/math.lib"   # absolute URL
checksum = "sha256:<hex>"                              # trusted expectation
size = 123456
system_libraries = ["user32"]

[target.x86_64-unknown-linux-gnu]
url = "https://cdn.example.com/math/1.0.0/libmath.a"
checksum = "sha256:<hex>"
size = 120000
```

Rules:

* `url` is absolute; a local filesystem path is not a URL.
* `checksum` is the trusted expected SHA-256; a checksum computed after download
  is not trust.
* The selected artifact is cached content-addressed under
  `~/.vut/cache/artifacts/<sha256>` and verified against both the expected size
  and checksum.
* The lockfile pins the resolved `target`/`url`/`checksum`/`size`; `vpm install`
  reuses the pinned entry and never searches for a replacement artifact. A dead
  URL with a cold cache is an explicit error.
* `native-artifacts.toml` is excluded from the package source BLAKE3 checksum.

Không parse C++ mangled symbol/classes/vtable ABI trực tiếp.
