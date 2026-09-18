//! Constant-binding name rules.
//!
//! `specs/02-type-system.md` §28 and `specs/03-data-model.md` §26 define
//! ALL-CAPS bindings as constants: the compiler still infers a normal static
//! type, but the binding cannot be reassigned (`E1104`).

/// Returns true when `name` is an ALL-CAPS constant binding name: after any
/// leading private `_` prefix it consists only of ASCII uppercase letters,
/// digits, and underscores, and contains at least one uppercase letter.
///
/// This only classifies the *binding*; it does not imply deep immutability of
/// the value (mutation through a constant is `E1105`, deferred until the
/// memory model defines const-depth semantics).
#[must_use]
pub(super) fn is_constant_name(name: &str) -> bool {
    let core = name.trim_start_matches('_');
    !core.is_empty()
        && core
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        && core.chars().any(|ch| ch.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::is_constant_name;

    #[test]
    fn classifies_constant_binding_names() {
        for name in ["MAX_SIZE", "X", "A1", "_PRIVATE", "__LIMIT", "ABC_123"] {
            assert!(is_constant_name(name), "`{name}` must be a constant");
        }
        for name in [
            "max_size", "Max", "MAXSize", "value", "_", "__", "A_b", "aBC", "x1",
        ] {
            assert!(!is_constant_name(name), "`{name}` must not be a constant");
        }
    }
}
