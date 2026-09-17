# Vut Standard Library — Overview

## 1. Purpose

The Vut standard library provides the common APIs required by normal Vut applications while keeping the compiler core small.

The architecture is:

```text
Core
= functionality the compiler/runtime must understand

Standard Library
= official libraries distributed with Vut

VPM Package
= independently distributed libraries installed through VPM
```

The compiler must not hardcode high-level standard-library behavior unless compiler support is fundamentally required.

---

## 2. Core vs Stdlib vs Package

### Core

Core language/runtime functionality includes concepts such as:

```text
bool
integers
floats
str
bytes
list(T)
array(T, N)
map(K, V)
result(T, E)
optional
dyn
ptr(T)
memory/drop primitives
basic compiler/runtime intrinsics
```

These are language/runtime concepts rather than normal stdlib modules.

### Standard Library

The official stdlib should provide system and general-purpose functionality such as:

```text
io
path
fs
os
env
time
process
math
random

thread
sync
channel
task

net
terminal
signal

collections
iter
cmp
hash
fmt
unicode
ascii
encoding
ffi
```

Not every module must be implemented in the first version.

### VPM Packages

Higher-level or independently evolving functionality should normally remain outside the standard library.

Examples:

```text
json
http
websocket
regex
crypto
sqlite
postgres
mysql
xml
yaml
image
audio
video
GUI frameworks
web frameworks
```

These should normally be distributed through VPM.

---

## 3. Stdlib Repository

The standard library lives in the separate repository:

```text
vut-stdlib/
├── native/
│   └── vut-runtime/
│       └── src/
│
├── std/
│   ├── io/
│   ├── path/
│   ├── fs/
│   ├── os/
│   ├── env/
│   ├── time/
│   ├── process/
│   ├── math/
│   ├── random/
│   └── ...
│
├── tests/
├── scripts/
└── dist/
```

The repository contains both:

```text
std/
    high-level Vut implementation

native/vut-runtime/
    low-level native support implemented in Rust
```

---

## 4. Fundamental Design Rule

The primary rule is:

> Implement high-level behavior in Vut whenever practical. Use Rust only for native, OS, ABI, or runtime primitives that Vut cannot reasonably implement itself.

Example:

```text
Vut public API
↓
Vut validation / conversion / high-level logic
↓
_internal/native.vut
↓
stable C ABI
↓
Rust vut-runtime
↓
Operating System
```

Do not move ordinary high-level stdlib logic into Rust merely because Rust implementation is easier.

---

## 5. Native Runtime

All official native stdlib support is compiled into two static libraries plus a
small startup object:

```text
vut-core      language/runtime primitives (memory/ownership, str/bytes/list/map,
              dyn/interface, panic, bounds, async/Vutcon, core ABI)
vut-stdlib    native implementation of the official stdlib
              (fs/io/os/env/time/process/http/...)
vut-startup   platform C entry `main` forwarding to `vut_entry`
```

Windows:

```text
vut-core.lib
vut-stdlib.lib
vut-startup.obj
```

Linux/macOS:

```text
libvut-core.a
libvut-stdlib.a
vut-startup.o
```

Do not create a separate native library per stdlib module:

```text
fs.lib
os.lib
time.lib
```

The two archives are internal build units; the distribution names above are the
stable contract. The startup object is prebuilt per target and shipped so that
linking never requires a Rust toolchain.

Native functionality is distinguished by exported symbols.

Examples:

```text
vut_rt_fs_exists
vut_rt_fs_read
vut_rt_os_home_dir
vut_rt_time_now
```

---

## 6. Installed Layout

The canonical installation layout is:

```text
~/.vut/
├── bin/
│   ├── vut
│   ├── vpm
│   └── vut-lsp
│
├── lib/
│   └── runtime/
│       └── <target>/
│           ├── vut-core.lib / libvut-core.a
│           ├── vut-stdlib.lib / libvut-stdlib.a
│           └── vut-startup.obj / vut-startup.o
│
├── std/
│   ├── fs/
│   ├── path/
│   ├── os/
│   └── ...
│
├── packages/
├── cache/
└── config/
```

There is no top-level `toolchains/` or global native `artifacts/` directory in the current MVP design.

---

## 7. Initial Implementation Priority

Implement the foundational stdlib first:

```text
io
path
fs
os
env
time
process
math
random
ffi
```

After the foundation is stable, additional modules may be implemented:

```text
thread
sync
channel
task
net
signal
terminal
collections
iter
cmp
hash
fmt
unicode
ascii
encoding
```

Concurrency modules must follow the language's concurrency/async specs when those features are ready.

Do not introduce multithreading merely because async/await exists.

---

## 8. Design Goals

The Vut stdlib should be:

* small and predictable;
* cross-platform;
* type-safe;
* memory-safe in safe Vut;
* consistent across modules;
* efficient;
* deterministic where practical;
* easy to use;
* easy for the compiler and LSP to understand;
* independent from unnecessary third-party dependencies.

Prefer a small coherent API over a very large convenience API.
