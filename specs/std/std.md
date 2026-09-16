# Vut Standard Library — Canonical Feature Specification

This document defines the canonical feature surface for the initial Vut standard library.

Codex/agents implementing the standard library MUST read together:

```text
specs/std/std.md
specs/std/00-overview.md
specs/std/01-architecture.md
specs/std/02-api-rules.md
specs/std/03-native-runtime.md
specs/std/04-roadmap.md
```

This file defines **what must exist**.

The other stdlib specs define **how it must be designed and implemented**.

The goal is that an agent can read `specs/std/` and implement the initial standard library end-to-end without requiring a separate specification for every module.

---

# 1. Initial Standard Library Scope

The initial required stdlib consists of:

```text
io
path
fs
os
env
time
process
```

These modules are the required first complete stdlib surface.

`http` is an additional approved stdlib module, specified separately by
`specs/std/http.md`. It is not part of the initial seven modules above but it is
an official stdlib module (`import http`), not a VPM package.

Do not expand this task into unrelated modules such as:

```text
json
regex
crypto
database
GUI
image
audio
video
websocket
```

Those belong outside this initial stdlib scope.

---

# 2. General Rules

All modules must follow Vut conventions.

Expected failures use:

```vut
result(T, E)
```

Normal absence uses optional values where appropriate:

```vut
T?
```

Do not use:

```text
exceptions
sentinel values
magic error integers in public APIs
manual memory management in safe APIs
```

High-level APIs should be written in Vut whenever practical.

Native Rust should provide platform/runtime primitives only.

---

# 3. `io`

`io` provides the common synchronous input/output abstractions used by files, processes, terminals, and future I/O-capable modules.

This initial specification covers **synchronous I/O only**.

Async I/O is not part of this module implementation unless separately specified by the async stdlib design.

---

## 3.1 Goals

`io` should provide common concepts for:

```text
reading bytes
writing bytes
reading text
writing text
buffering
flushing
copying streams
standard input
standard output
standard error
```

Avoid overengineering a large stream hierarchy.

---

# 3.2 Core Interfaces

Provide a basic readable interface.

Conceptually:

```vut
interface Reader:
  read(buffer: bytes) -> result(usize, IoError)
```

However, if mutable caller-provided `bytes` is incompatible with the final Vut bytes API, design an equivalent API consistent with the actual ownership model.

The important semantic contract is:

```text
read some bytes
return number of bytes read
0 means EOF
expected failures use IoError
```

Provide a writer interface:

```vut
interface Writer:
  write(data: bytes) -> result(usize, IoError)

  flush() -> result(null, IoError)
```

`write()` may write fewer bytes than requested.

Provide helpers for completing operations.

Conceptually:

```vut
reader.read_all() -> result(bytes, IoError)

reader.read_exact(size: usize) -> result(bytes, IoError)

writer.write_all(data: bytes) -> result(null, IoError)
```

Exact method signatures may adapt to the actual Vut ownership model, but semantics must remain clear.

---

# 3.3 Text Reading

Provide text-oriented helpers.

Recommended API:

```text
read_to_str
read_line
lines
```

Conceptually:

```vut
reader.read_to_str() -> result(str, IoError)

reader.read_line() -> result(str?, IoError)
```

For `read_line()`:

```text
line exists
→ str

EOF before any additional character
→ null
```

Invalid UTF-8 must not silently produce corrupted text.

It must result in a typed error or UTF-8 conversion error consistent with existing Vut `bytes -> str` semantics.

Do not perform unchecked UTF-8 conversion.

---

# 3.4 Copy

Provide a generic stream copy helper.

Conceptually:

```vut
io.copy(reader, writer) -> result(u64, IoError)
```

Return the number of bytes copied.

Implementation should use bounded buffers and must not load the entire stream into memory unnecessarily.

---

# 3.5 Buffered I/O

Provide basic buffering:

```text
BufferedReader
BufferedWriter
```

Recommended behavior:

```vut
reader = io.BufferedReader(source)

writer = io.BufferedWriter(target)
```

`BufferedReader` reduces native reads.

`BufferedWriter` accumulates writes and exposes:

```text
write
write_all
flush
```

Dropping a buffered writer must not silently hide an important flush error.

Do not rely solely on destructor-time flushing for correctness.

---

# 3.6 Standard Streams

Provide:

```text
stdin
stdout
stderr
```

Recommended API:

```vut
io.stdin()
io.stdout()
io.stderr()
```

These return stream objects compatible with the standard Reader/Writer abstractions.

Existing core:

```vut
print(...)
out(...)
input(...)
```

may remain simple language/runtime convenience functions.

Do not duplicate or break them unnecessarily.

`io` provides lower-level reusable stream access.

---

# 3.7 `IoError`

Provide an I/O error model.

Conceptually:

```vut
enum IoErrorKind:
  interrupted
  unexpected_eof
  invalid_data
  invalid_input
  permission_denied
  broken_pipe
  timed_out
  unsupported
  other

data IoError:
  kind: IoErrorKind
  message: str
```

Exact internal representation may evolve.

Do not expose Rust `std::io::Error` directly.

---

# 3.8 IO Requirements

The initial `io` implementation must support:

```text
Reader
Writer
read
write
flush
read_all
read_exact
write_all
read_to_str
read_line
stream copy
basic buffering
stdin
stdout
stderr
typed errors
```

---

# 4. `path`

`path` performs **pure path manipulation**.

It should not access the filesystem for operations that can be answered purely from the path value.

Example:

```vut
path.extension("assets/image.png")
```

must not touch the filesystem.

---

# 4.1 Path Representation

The initial API may accept and return `str`.

A dedicated `Path` type may be introduced if it materially improves correctness and architecture.

Do not introduce a complex path type merely to imitate another language.

Path semantics must respect the target platform where necessary.

---

# 4.2 Required Path Operations

Provide:

```text
join
parent
file_name
stem
extension
with_extension
is_absolute
is_relative
normalize
components
separator
```

Recommended conceptual signatures:

```vut
path.join(base: str, child: str) -> str

path.parent(value: str) -> str?

path.file_name(value: str) -> str?

path.stem(value: str) -> str?

path.extension(value: str) -> str?

path.with_extension(value: str, extension: str) -> str

path.is_absolute(value: str) -> bool

path.is_relative(value: str) -> bool

path.normalize(value: str) -> str

path.components(value: str) -> list(str)

path.separator() -> str
```

---

# 4.3 Join

`path.join()` should:

```text
use target-platform path rules
avoid malformed duplicate separators
preserve meaningful roots
handle absolute child paths according to documented semantics
```

Support multiple components if Vut's API design permits it cleanly.

Possible direction:

```vut
path.join("home", "user", "file.txt")
```

Do not force this if Vut currently lacks variadic function support.

A list-based alternative may be used if necessary.

---

# 4.4 Normalize

`normalize()` should perform lexical normalization where safe.

Examples:

```text
a/./b
→ a/b

a/x/../b
→ a/b
```

Do not require filesystem access.

Do not resolve symbolic links.

Do not pretend lexical normalization is equivalent to canonical filesystem resolution.

---

# 4.5 Absolute Path Resolution

If an API is provided to convert a relative path to absolute form, distinguish it from canonicalization.

Possible API:

```vut
path.absolute(value: str) -> result(str, PathError)
```

This may require current working directory information.

Do not mix:

```text
absolute path conversion
filesystem canonicalization
lexical normalization
```

as if they were the same operation.

---

# 4.6 Path Platform Behavior

Support at least:

```text
Windows
Linux
macOS
```

Windows concerns include:

```text
drive letters
UNC paths
backslash separator
absolute/rooted distinctions
```

Unix-like systems primarily use:

```text
/
```

Path APIs must not assume Unix-only path syntax.

---

# 4.7 Required Path Surface

Initial `path` must provide at least:

```text
join
parent
file_name
stem
extension
with_extension
is_absolute
is_relative
normalize
components
separator
```

Optional additions are allowed only when clearly useful and architecturally consistent.

---

# 5. `fs`

`fs` provides filesystem access.

It covers:

```text
files
directories
metadata
permissions
filesystem operations
```

Filesystem failures use typed Results.

---

# 5.1 Basic Queries

Provide:

```vut
fs.exists(path: str) -> bool

fs.is_file(path: str) -> bool

fs.is_dir(path: str) -> bool
```

If checking these conditions can encounter errors that materially matter, provide additional Result-based APIs or clearly document the behavior of the boolean convenience functions.

Do not allow permission errors to masquerade as nonexistent paths in APIs where correctness requires distinguishing them.

---

# 5.2 File Reading

Provide binary reading:

```vut
fs.read(path: str) -> result(bytes, FsError)
```

Provide text reading:

```vut
fs.read_str(path: str) -> result(str, FsError)
```

`read_str` must validate UTF-8.

Invalid UTF-8 must produce a typed failure.

Do not silently replace invalid bytes.

---

# 5.3 File Writing

Provide:

```vut
fs.write(path: str, data: bytes) -> result(null, FsError)

fs.write_str(path: str, data: str) -> result(null, FsError)
```

Define whether `write`:

```text
creates missing files
truncates existing files
```

Recommended default:

```text
create if absent
truncate if present
```

Document this explicitly.

---

# 5.4 Append

Provide:

```vut
fs.append(path: str, data: bytes) -> result(null, FsError)

fs.append_str(path: str, data: str) -> result(null, FsError)
```

Append must create or fail according to a clearly documented policy.

Prefer predictable cross-platform behavior.

---

# 5.5 Directory Operations

Provide:

```vut
fs.create_dir(path: str) -> result(null, FsError)

fs.create_dir_all(path: str) -> result(null, FsError)

fs.remove_dir(path: str) -> result(null, FsError)

fs.remove_dir_all(path: str) -> result(null, FsError)

fs.read_dir(path: str) -> result(list(DirectoryEntry), FsError)
```

`create_dir`:

```text
creates exactly the requested directory
parent must already exist
```

`create_dir_all`:

```text
creates missing parent directories
```

`remove_dir`:

```text
removes empty directory
```

`remove_dir_all`:

```text
recursive removal
must be implemented carefully
```

---

# 5.6 File Operations

Provide:

```vut
fs.remove_file(path: str) -> result(null, FsError)

fs.copy(from: str, to: str) -> result(u64, FsError)

fs.rename(from: str, to: str) -> result(null, FsError)
```

`copy` should return bytes copied when meaningful.

Document overwrite behavior.

---

# 5.7 Directory Entry

Provide:

```vut
data DirectoryEntry:
  ...
```

Required information/methods should include:

```text
name
path
file type
is_file
is_dir
metadata
```

Do not eagerly perform unnecessary metadata syscalls for every directory entry unless required.

Lazy metadata retrieval is acceptable.

---

# 5.8 Metadata

Provide:

```vut
fs.metadata(path: str) -> result(Metadata, FsError)
```

`Metadata` should expose useful portable information:

```text
file type
size
read-only status
created time when available
modified time when available
accessed time when available
```

Suggested conceptual surface:

```vut
metadata.len() -> u64

metadata.is_file() -> bool

metadata.is_dir() -> bool

metadata.readonly() -> bool

metadata.created() -> result(Timestamp, FsError)

metadata.modified() -> result(Timestamp, FsError)

metadata.accessed() -> result(Timestamp, FsError)
```

Platform limitations must produce an error or optional result rather than fabricated values.

---

# 5.9 Permissions

Provide a portable minimum permissions model.

At minimum:

```text
readonly
```

Conceptual:

```vut
permissions = metadata.permissions()

permissions.readonly() -> bool

permissions.set_readonly(value: bool)
```

And:

```vut
fs.set_permissions(path, permissions) -> result(null, FsError)
```

Do not attempt to force Unix permission bits into Windows as if they have identical semantics.

Platform-specific advanced permissions can be added later.

---

# 5.10 File Object

Provide a reusable `File` abstraction.

Conceptually:

```vut
File.open(path)
File.create(path)
```

`File` should integrate with `io.Reader` / `io.Writer` where applicable.

Required capabilities:

```text
open
create
read
write
flush
metadata
sync
close/drop
```

The API should use deterministic automatic resource cleanup.

Users should not be required to manually close a file for memory/resource safety.

An explicit `close()` may exist when useful for deterministic error handling.

---

# 5.11 Open Options

Provide an `OpenOptions` abstraction for controlled file opening.

Support options such as:

```text
read
write
append
truncate
create
create_new
```

Conceptually:

```vut
options = OpenOptions(
  read = true,
  write = true,
  create = true
)

file = options.open(path)?
```

Follow normal Vut data/named-argument conventions.

---

# 5.12 Filesystem Errors

Provide:

```text
FsError
FsErrorKind
```

At minimum distinguish:

```text
not_found
permission_denied
already_exists
invalid_input
not_a_directory
is_a_directory
directory_not_empty
unsupported
io
unknown
```

Map native OS errors into stable Vut errors.

Preserve useful human-readable diagnostic messages.

---

# 5.13 Required FS Surface

Initial complete `fs` should include:

```text
exists
is_file
is_dir

read
read_str
write
write_str
append
append_str

create_dir
create_dir_all
remove_dir
remove_dir_all
read_dir

remove_file
copy
rename

metadata
permissions
set_permissions

File
OpenOptions
DirectoryEntry
Metadata
FsError
```

---

# 6. `os`

`os` exposes stable operating-system and machine information.

Do not turn `os` into a dumping ground for unrelated system functionality.

---

# 6.1 Platform

Provide canonical OS identification.

Recommended:

```vut
os.name() -> str
```

Canonical values:

```text
windows
linux
macos
```

Do not return inconsistent marketing strings.

---

# 6.2 Architecture

Provide:

```vut
os.arch() -> str
```

Canonical values should align with Vut target naming.

Examples:

```text
x86_64
aarch64
x86
arm
```

Unsupported architectures should still use deterministic canonical names.

---

# 6.3 Family

Optional but recommended:

```vut
os.family() -> str
```

Possible values:

```text
windows
unix
```

This should be used only when useful for broad platform behavior.

---

# 6.4 Home Directory

Provide:

```vut
os.home_dir() -> result(str, OsError)
```

Do not assume a home directory always exists.

Do not silently return an empty string.

---

# 6.5 Temporary Directory

Provide:

```vut
os.temp_dir() -> result(str, OsError)
```

Return the platform's appropriate temporary directory.

---

# 6.6 Current Directory

Current working directory logically belongs either to `os` or `fs`.

For consistency, choose one canonical location and document it.

Recommended:

```vut
os.current_dir() -> result(str, OsError)

os.set_current_dir(path: str) -> result(null, OsError)
```

Do not expose duplicate canonical APIs in both `fs` and `os`.

---

# 6.7 Executable

Provide:

```vut
os.current_exe() -> result(str, OsError)
```

Return the current executable path when supported.

---

# 6.8 CPU Information

Initial stdlib only needs simple machine information.

Potential APIs:

```vut
os.cpu_count() -> usize
```

Do not add hardware inventory or detailed CPU topology in the initial stdlib.

---

# 6.9 OS Error

Provide:

```text
OsError
OsErrorKind
```

Use typed errors for operations such as:

```text
home_dir
temp_dir
current_dir
current_exe
```

---

# 6.10 Required OS Surface

Implement at least:

```text
name
arch
family
home_dir
temp_dir
current_dir
set_current_dir
current_exe
cpu_count
OsError
```

---

# 7. `env`

`env` manages environment variables and process arguments.

Environment APIs should be separate from generic OS information.

---

# 7.1 Get Environment Variable

Provide:

```vut
env.get(name: str) -> result(str?, EnvError)
```

Distinguish:

```text
variable does not exist
→ null

environment access/conversion failure
→ err(...)
```

Do not collapse both into the same result.

If the platform environment representation cannot be represented as valid Vut UTF-8 strings, return a typed error.

---

# 7.2 Check Existence

Provide:

```vut
env.has(name: str) -> bool
```

or equivalent.

If this cannot reliably distinguish malformed platform values, document the behavior.

---

# 7.3 Set Variable

Provide:

```vut
env.set(name: str, value: str) -> result(null, EnvError)
```

---

# 7.4 Remove Variable

Provide:

```vut
env.remove(name: str) -> result(null, EnvError)
```

---

# 7.5 List Environment

Provide:

```vut
env.all() -> result(map(str, str), EnvError)
```

If `map(str, str)` API is not yet mature enough, use the nearest stable collection representation rather than inventing a new collection.

---

# 7.6 Process Arguments

Provide:

```vut
env.args() -> list(str)
```

This contains program arguments in a documented form.

Decide explicitly whether argument 0 is the executable path.

Recommended behavior:

```text
env.args()
includes all arguments supplied to the Vut program,
with the executable/program name as the first element when available.
```

Keep behavior deterministic.

---

# 7.7 Executable Argument Helpers

Optional convenience:

```vut
env.arg(index: usize) -> str?
```

Do not duplicate unnecessary APIs if `env.args()` is sufficient.

---

# 7.8 Env Errors

Provide:

```text
EnvError
EnvErrorKind
```

Likely kinds:

```text
invalid_name
invalid_value
invalid_encoding
unsupported
other
```

---

# 7.9 Required Env Surface

Implement:

```text
get
has
set
remove
all
args
EnvError
```

---

# 8. `time`

`time` provides:

```text
timestamps
durations
monotonic measurement
sleep
```

Keep wall-clock time distinct from monotonic elapsed-time measurement.

---

# 8.1 Duration

Provide a `Duration` value type.

Conceptually:

```vut
data Duration:
  ...
```

Provide constructors:

```text
nanoseconds
microseconds
milliseconds
seconds
minutes
hours
```

Possible API:

```vut
Duration.nanoseconds(value)
Duration.microseconds(value)
Duration.milliseconds(value)
Duration.seconds(value)
Duration.minutes(value)
Duration.hours(value)
```

If static methods are not yet supported by Vut, provide module functions instead:

```vut
time.nanoseconds(...)
time.milliseconds(...)
time.seconds(...)
```

Follow actual Vut language capabilities.

Do not invent unsupported static-method syntax merely for this spec.

---

# 8.2 Duration Accessors

Provide:

```text
as_nanoseconds
as_microseconds
as_milliseconds
as_seconds
```

Where integer conversion could overflow, use sufficiently wide types or checked conversion.

Duration arithmetic should support sensible operations if Vut operator overloading or built-in support permits it.

Otherwise use explicit methods/functions.

---

# 8.3 Timestamp

Provide wall-clock timestamp representation.

`Timestamp` should represent an absolute point in wall-clock/system time.

Provide:

```vut
time.now() -> Timestamp
```

Recommended internal representation:

```text
integer duration relative to Unix epoch
```

but do not expose implementation details unnecessarily.

---

# 8.4 Unix Timestamp Conversion

Provide useful conversions:

```text
Unix seconds
Unix milliseconds
```

Conceptually:

```vut
timestamp.unix_seconds() -> i64

timestamp.unix_milliseconds() -> i64
```

Provide creation from Unix timestamps where useful.

Invalid/overflowing values must be handled safely.

---

# 8.5 Instant

Provide a monotonic time source for measuring elapsed time.

Conceptually:

```vut
start = time.instant()

# work

elapsed = start.elapsed()
```

`Instant` must not represent calendar/wall-clock time.

System clock adjustments must not invalidate elapsed-time measurement.

---

# 8.6 Sleep

Provide synchronous sleep:

```vut
time.sleep(duration: Duration)
```

This blocks the current OS thread.

Do not label it async.

If Vut later adds asynchronous sleep:

```text
async sleep
```

must be specified separately and integrate with the async runtime.

---

# 8.7 Time Arithmetic

Support or provide helpers for:

```text
timestamp + duration
timestamp - duration
timestamp - timestamp → duration
duration + duration
duration - duration
```

Only implement operator forms if supported cleanly by the language.

Otherwise expose equivalent functions/methods.

Handle underflow/overflow safely.

---

# 8.8 Calendar Formatting

Full timezone/calendar/date formatting is NOT required in this initial stdlib phase.

Do not expand initial `time` into a large timezone database implementation.

The initial module focuses on:

```text
Timestamp
Duration
Instant
now
sleep
elapsed measurement
Unix timestamp conversions
```

Higher-level date/time formatting may be added later.

---

# 8.9 Required Time Surface

Implement:

```text
Duration
Timestamp
Instant

now
instant
sleep

duration constructors
duration accessors
Unix timestamp conversion
elapsed measurement
basic time arithmetic where supported
```

---

# 9. `process`

`process` provides child-process creation and control.

This module is synchronous-first.

Do not implement a multithreaded process runtime merely for this module.

---

# 9.1 Command

Provide a command builder.

Recommended conceptual design:

```vut
command = Command("git")
```

or equivalent Vut-compatible constructor.

Required configuration:

```text
program
arguments
environment
working directory
stdin mode
stdout mode
stderr mode
```

---

# 9.2 Arguments

Provide:

```text
arg
args
```

Conceptually:

```vut
command.arg("--version")

command.args(@("status", "--short"))
```

Use Vut's canonical list syntax:

```vut
@(...)
```

Do not use `[]`.

---

# 9.3 Environment

Allow child-process environment configuration:

```text
set env variable
remove env variable
clear inherited environment if needed
```

Conceptually:

```vut
command.env("MODE", "release")
command.env_remove("DEBUG")
```

Exact chaining style should follow Vut method/value semantics.

Do not force fluent mutable APIs if they conflict with the language design.

---

# 9.4 Working Directory

Provide:

```vut
command.current_dir(path)
```

or equivalent.

---

# 9.5 Spawn

Provide:

```vut
command.spawn() -> result(Child, ProcessError)
```

`spawn()` starts the process and returns a child handle.

It must not automatically wait for completion.

---

# 9.6 Status

Provide simple run-and-wait behavior.

Conceptually:

```vut
command.status() -> result(ExitStatus, ProcessError)
```

This:

```text
starts process
waits
returns exit status
```

---

# 9.7 Captured Output

Provide:

```vut
command.output() -> result(Output, ProcessError)
```

`Output` contains:

```text
status
stdout bytes
stderr bytes
```

Conceptually:

```vut
data Output:
  status: ExitStatus
  stdout: bytes
  stderr: bytes
```

Do not assume process output is UTF-8.

Provide user-level conversion through normal `bytes.to_str()` semantics when desired.

---

# 9.8 Exit Status

Provide:

```text
success
exit code
```

Conceptually:

```vut
status.success() -> bool

status.code() -> i32?
```

Exit code may be absent when a process is terminated by signals or platform-specific mechanisms.

Do not invent an exit code when none exists.

---

# 9.9 Child

`Child` should provide:

```text
id
wait
try_wait
kill
stdin
stdout
stderr
```

Conceptually:

```vut
child.id() -> u32

child.wait() -> result(ExitStatus, ProcessError)

child.try_wait() -> result(ExitStatus?, ProcessError)

child.kill() -> result(null, ProcessError)
```

---

# 9.10 Child Standard Streams

Support configurable:

```text
inherit
null
pipe
```

for:

```text
stdin
stdout
stderr
```

Recommended public concept:

```text
Stdio
```

with modes equivalent to:

```text
inherit
null
piped
```

Do not expose raw native handles as the primary safe API.

When piped:

```text
ChildStdin implements Writer
ChildStdout implements Reader
ChildStderr implements Reader
```

or equivalent structural interfaces.

Reuse `io`.

Do not duplicate stream logic inside `process`.

---

# 9.11 Process ID

Provide the current process ID where useful:

```vut
process.id() -> u32
```

Child process IDs come through:

```vut
child.id()
```

---

# 9.12 Exit Current Process

Provide:

```vut
process.exit(code: i32)
```

This terminates the current process immediately.

Document that normal deterministic cleanup may not run after forced process termination.

Use carefully.

---

# 9.13 Process Errors

Provide:

```text
ProcessError
ProcessErrorKind
```

Distinguish useful cases such as:

```text
not_found
permission_denied
invalid_input
spawn_failed
io
unsupported
other
```

Do not expose raw Rust process errors.

---

# 9.14 Shell Execution

Do NOT make implicit shell execution the default.

This:

```vut
Command("program")
```

must execute the program directly.

Do not interpret:

```text
|
>
&&
*
$VAR
```

through a shell automatically.

If shell execution is ever provided, it must be explicit and documented separately.

This avoids:

```text
command injection
cross-platform shell inconsistency
unexpected quoting behavior
```

---

# 9.15 Required Process Surface

Implement at least:

```text
Command
Child
ExitStatus
Output
Stdio

arg
args
env
env_remove
current_dir

spawn
status
output

child.id
child.wait
child.try_wait
child.kill

piped stdin
piped stdout
piped stderr

process.id
process.exit

ProcessError
```

---

# 10. Cross-Module Integration

The modules must integrate rather than duplicate functionality.

Expected relationships:

```text
io
↑
├── fs.File
├── process.ChildStdin
├── process.ChildStdout
└── process.ChildStderr
```

And:

```text
path
↑
└── used by fs where useful
```

And:

```text
time
↑
└── fs.Metadata timestamps
```

Do not define separate incompatible stream, timestamp, path, or error concepts inside every module.

---

# 11. Native Runtime Requirements

Native symbols required by these modules should use:

```text
vut_rt_<module>_<operation>
```

Examples:

```text
vut_rt_fs_exists
vut_rt_fs_read
vut_rt_fs_write

vut_rt_os_home_dir
vut_rt_os_current_dir

vut_rt_env_get
vut_rt_env_set

vut_rt_time_now
vut_rt_time_sleep

vut_rt_process_spawn
vut_rt_process_wait

vut_rt_io_stdin_read
vut_rt_io_stdout_write
```

Do not hardcode this exact symbol list before implementation proves the required boundary.

The naming convention is mandatory.

The exact primitive split should be chosen to avoid excessive FFI calls and duplicated native logic.

---

# 12. Resource Management

Resources such as:

```text
files
process handles
pipes
native buffers
```

must use deterministic automatic cleanup.

Safe Vut users must not need:

```text
free
delete
manual deallocation
```

Explicit close/wait/flush APIs may exist for semantic control and error reporting.

Dropping a resource must never cause:

```text
double close
double free
use-after-free
memory leak
handle leak
```

---

# 13. Platform Support

The initial target platforms are:

```text
Windows
Linux
macOS
```

Public behavior should remain as consistent as possible.

Platform-specific behavior belongs behind internal/native abstractions.

Do not write public APIs that unnecessarily expose:

```text
Win32 HANDLE
POSIX fd
errno
DWORD
Rust std types
```

Low-level FFI APIs are separate from normal stdlib APIs.

---

# 14. Performance Requirements

Do not intentionally introduce unnecessary:

```text
heap allocations
full-buffer copies
UTF-8 conversions
syscalls
temporary strings
runtime indirection
```

Important examples:

* `io.copy` should stream data.
* `fs.read` may allocate according to known file size where appropriate.
* `process.output` should efficiently capture output.
* `path` operations should not access the filesystem.
* native read/write paths should minimize redundant copies where ownership permits.

Correctness and ownership safety come before micro-optimization.

---

# 15. Documentation Requirements

When implementing this specification, update `specs/std/` when implementation reveals a necessary clarification.

Do not silently invent semantics.

Document at minimum:

```text
public signature
error behavior
ownership behavior
platform differences
important edge cases
```

Keep the stdlib specs concise.

Do not create dozens of tiny spec files unless complexity genuinely requires it.

---

# 16. Testing Requirements

During implementation phases, perform basic verification after each major module:

```text
compile
basic happy path
basic error path
no obvious crash
existing tests remain working
```

Do not stop after every function to build a massive exhaustive test suite.

After all seven modules are implemented, perform comprehensive stdlib testing covering:

```text
io
path
fs
os
env
time
process
```

Include:

```text
unit tests
integration tests
E2E tests
platform differences
invalid input
error propagation
resource cleanup
large files
empty files
invalid UTF-8
process failures
pipe behavior
memory/handle leaks
```

---

# 17. One-Shot Implementation Instruction

An agent tasked with implementing the initial Vut standard library should:

1. Read `AGENTS.md`.
2. Read all files under `specs/std/`.
3. Inspect existing compiler/runtime/std infrastructure.
4. Reuse existing abstractions rather than creating parallel systems.
5. Resolve stdlib architecture and native ABI foundation first.
6. Implement modules in dependency-aware order:

```text
io
→ path
→ time foundation
→ os
→ env
→ fs
→ process
```

The exact order may change when existing architecture makes another order clearly better.

7. Implement necessary native runtime primitives.
8. Implement high-level APIs in Vut.
9. Integrate types/errors/resources across modules.
10. Keep the workspace buildable throughout.
11. Perform basic verification between phases.
12. After all modules are implemented, run comprehensive stdlib tests and hardening.
13. Update specs when final implementation requires clarified semantics.
14. Review the final architecture for duplicate abstractions, unnecessary native code, resource leaks, and inconsistent APIs.

The task is complete only when:

```text
io
path
fs
os
env
time
process
```

form a coherent, usable, documented standard-library foundation rather than seven isolated partial modules.

---

# 18. Scope Boundary

Completing this specification does NOT require implementing:

```text
thread
sync
channel
multithread runtime
HTTP
JSON
regex
crypto
database
GUI
web framework
async networking
```

Do not expand scope unless another canonical spec explicitly requires it.

`async` / `await`, if already part of the language, must remain compatible with this stdlib architecture, but synchronous stdlib functionality should not be delayed merely to build async versions of every API.
