//! Numeric primitive metadata and canonical explicit conversion destinations.
use super::Type;

/// A concrete numeric primitive, used by explicit conversions to describe the
/// source and destination without depending on layout/pointer width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericKind {
    Int,
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
    Float,
    F32,
    F64,
}
impl NumericKind {
    /// Whether the type is unsigned (its comparisons and division are unsigned).
    #[must_use]
    pub const fn is_unsigned(self) -> bool {
        matches!(
            self,
            Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Usize
        )
    }
    /// Whether the type is a floating-point type.
    #[must_use]
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float | Self::F32 | Self::F64)
    }
    /// Bit width, or `0` for pointer-sized `isize`/`usize`.
    #[must_use]
    pub const fn bits(self) -> u32 {
        match self {
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 | Self::F32 => 32,
            Self::I64 | Self::U64 | Self::Int | Self::Float | Self::F64 => 64,
            Self::Isize | Self::Usize => 0,
        }
    }
}
/// Maps a semantic numeric type to its [`NumericKind`], or `None` for
/// non-numeric types.
pub(super) fn numeric_kind(ty: &Type) -> Option<NumericKind> {
    Some(match ty {
        Type::Int => NumericKind::Int,
        Type::Float => NumericKind::Float,
        Type::Numeric(name) => match name.as_str() {
            "i8" => NumericKind::I8,
            "i16" => NumericKind::I16,
            "i32" => NumericKind::I32,
            "i64" => NumericKind::I64,
            "isize" => NumericKind::Isize,
            "u8" => NumericKind::U8,
            "u16" => NumericKind::U16,
            "u32" => NumericKind::U32,
            "u64" => NumericKind::U64,
            "usize" => NumericKind::Usize,
            "f32" => NumericKind::F32,
            "f64" => NumericKind::F64,
            _ => return None,
        },
        _ => return None,
    })
}

/// Maps a `to_<name>()` method to its numeric destination kind and type.
///
/// Conversion methods are compiler-known (no stdlib import) and checked at
/// runtime by codegen; they never silently wrap or truncate.
pub(super) fn numeric_cast_target(name: &str) -> Option<(NumericKind, Type)> {
    let target = name.strip_prefix("to_")?;
    Some(match target {
        "int" => (NumericKind::Int, Type::Int),
        "float" => (NumericKind::Float, Type::Float),
        _ => {
            let ty = Type::Numeric(target.to_owned());
            let kind = numeric_kind(&ty)?;
            (kind, ty)
        }
    })
}
