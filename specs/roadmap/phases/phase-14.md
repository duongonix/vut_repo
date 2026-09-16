# Phase 14 — Runtime and Memory Management

## Status

Complete as a modular runtime library; generated-code integration remains tracked by Phase 13.

The safe Rust runtime supplies a versioned ABI registry, startup bridge, owned
UTF-8 strings/bytes, typed lists/maps, heap blocks, deterministic Rust drops,
bounds failures, panic, interface tables, explicit dynamic values, and buffered
console primitives without a VM, tracing GC, or global ARC.

The I/O bridge has independently testable writer/reader entry points and preserves
the language distinction that `print` has no newline while `out` appends one.

Generated strings use versioned runtime-managed handles with atomic retain and
release operations and observable live-allocation accounting for native memory
safety tests. `VutString` shares immutable UTF-8 storage, while typed lists and
maps use `Arc::make_mut` to provide value-semantic copy-on-write mutation.
Runtime ABI version 2 records this ownership contract.

The collection runtime now covers the complete bounded list surface
(`len/capacity/push/pop/at/set/slice/insert/remove/clear/contains`) and map
surface (`get/set/remove/contains_key/len/clear`, plus capacity reservation and
key/value views). The string runtime is Unicode-aware and covers byte/character
length, matching, trimming, case conversion, replacement, splitting, lines,
numeric conversion, and UTF-8 byte conversion. Native string query and transform
entry points validate null handles and document their raw-handle lifetime
contract. Map literal syntax remains intentionally unspecified by the language
grammar and is not invented by the runtime.

The `bytes` buffer lives in a dedicated runtime module and exposes a contiguous,
reference-counted `u8` buffer with bounds-safe access (`at`/`first`/`last`
return the zeroed fallback on an out-of-range index, and `slice` yields an empty
buffer for an invalid range). UTF-8 validation is centralized and drives a
native `Utf8Error` data type (`valid_up_to`/`error_len`) for `bytes.to_str()`.
The runtime tracks live bytes, list, and map allocations so generated programs
fail with exit code 254 when any managed allocation leaks.
