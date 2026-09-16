//! Shared metadata for core builtin types and their methods.
//!
//! The type checker owns the authoritative method signatures; this module
//! exposes a tooling-facing view (names and rendered signatures) so editors can
//! offer completion and hover for builtin receivers without duplicating
//! semantic rules. Keep this list aligned with `checker::builtin_*_method`.

use crate::Type;

/// Builtin receiver families that expose methods.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinKind {
    Str,
    Bytes,
    List,
    Map,
}

/// Names that resolve to builtin types (the resolver prelude plus `Utf8Error`).
pub const BUILTIN_TYPE_NAMES: &[&str] = &[
    "void",
    "bool",
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "usize",
    "isize",
    "f32",
    "f64",
    "int",
    "float",
    "str",
    "bytes",
    "dyn",
    "null",
    "list",
    "map",
    "result",
    "ptr",
    "Utf8Error",
    "vutcon",
    "resource",
    "future",
];

/// Returns `true` when `name` is a reserved builtin type name.
#[must_use]
pub fn is_builtin_type_name(name: &str) -> bool {
    BUILTIN_TYPE_NAMES.contains(&name)
}

/// Maps a semantic type to a builtin method owner, if any.
#[must_use]
pub fn builtin_kind(ty: &Type) -> Option<BuiltinKind> {
    match ty {
        Type::Str => Some(BuiltinKind::Str),
        Type::Bytes => Some(BuiltinKind::Bytes),
        Type::List(_) => Some(BuiltinKind::List),
        Type::Map(_, _) => Some(BuiltinKind::Map),
        _ => None,
    }
}

/// Method `(name, rendered signature)` pairs exposed by a builtin receiver.
#[must_use]
pub fn builtin_methods(kind: BuiltinKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        BuiltinKind::Str => &[
            ("byte_len", "byte_len() -> int"),
            ("char_len", "char_len() -> int"),
            ("is_empty", "is_empty() -> bool"),
            ("contains", "contains(needle: str) -> bool"),
            ("starts_with", "starts_with(prefix: str) -> bool"),
            ("ends_with", "ends_with(suffix: str) -> bool"),
            ("trim", "trim() -> str"),
            ("trim_start", "trim_start() -> str"),
            ("trim_end", "trim_end() -> str"),
            ("to_lower", "to_lower() -> str"),
            ("to_upper", "to_upper() -> str"),
            ("replace", "replace(from: str, to: str) -> str"),
            ("to_bytes", "to_bytes() -> bytes"),
        ],
        BuiltinKind::Bytes => &[
            ("len", "len() -> int"),
            ("is_empty", "is_empty() -> bool"),
            ("capacity", "capacity() -> int"),
            ("reserve", "reserve(capacity: int) -> void"),
            ("at", "at(index: int) -> u8"),
            ("set", "set(index: int, value: u8) -> void"),
            ("first", "first() -> u8"),
            ("last", "last() -> u8"),
            ("slice", "slice(start: int, end: int) -> bytes"),
            ("clear", "clear() -> void"),
            ("to_list", "to_list() -> list(u8)"),
            ("to_str", "to_str() -> result(str, Utf8Error)"),
        ],
        BuiltinKind::List => &[
            ("len", "len() -> int"),
            ("is_empty", "is_empty() -> bool"),
            ("capacity", "capacity() -> int"),
            ("reserve", "reserve(capacity: int) -> void"),
            ("push", "push(value: T) -> void"),
            ("at", "at(index: int) -> T"),
            ("set", "set(index: int, value: T) -> void"),
            ("insert", "insert(index: int, value: T) -> void"),
            ("remove", "remove(index: int) -> T"),
            ("clear", "clear() -> void"),
            ("slice", "slice(start: int, end: int) -> list(T)"),
            ("contains", "contains(value: T) -> bool"),
            ("to_bytes", "to_bytes() -> bytes  # list(u8) only"),
        ],
        BuiltinKind::Map => &[
            ("len", "len() -> int"),
            ("is_empty", "is_empty() -> bool"),
            ("capacity", "capacity() -> int"),
            ("reserve", "reserve(capacity: int) -> void"),
            ("get", "get(key: K) -> V"),
            ("set", "set(key: K, value: V) -> void"),
            ("contains_key", "contains_key(key: K) -> bool"),
            ("remove", "remove(key: K) -> V"),
            ("clear", "clear() -> void"),
        ],
    }
}

/// Looks up the rendered signature of a builtin method.
#[must_use]
pub fn builtin_method_signature(kind: BuiltinKind, name: &str) -> Option<&'static str> {
    builtin_methods(kind)
        .iter()
        .find(|(method, _)| *method == name)
        .map(|(_, signature)| *signature)
}
