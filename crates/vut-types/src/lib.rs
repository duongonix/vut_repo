//! Vut type representation, inference and checking.
mod attributes;
mod builtins;
mod checker;
pub use attributes::{AttributeSemantics, AttributeTarget, Repr};
pub use builtins::{
    BUILTIN_TYPE_NAMES, BuiltinKind, builtin_kind, builtin_method_signature, builtin_methods,
    is_builtin_type_name,
};
pub use checker::*;
