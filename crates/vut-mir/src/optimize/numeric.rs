//! Constant evaluation obeying the same target width/signedness as codegen.
use crate::{BinaryOp, LayoutTable, ValueRepr};
use vut_hir::TypeId;

#[derive(Clone, Copy, Debug)]
pub(super) enum Folded {
    Integer(i64),
    Boolean(bool),
}

/// Result of an algebraic identity that does not need both operands constant.
#[derive(Clone, Copy, Debug)]
pub(super) enum Simplified {
    /// The result is the left operand.
    Left,
    /// The result is the right operand.
    Right,
    /// The result is a constant.
    Constant(Folded),
}

/// Algebraic identities with exactly one constant operand.
///
/// Restricted to integer and boolean operations: floating-point identities such
/// as `x + 0.0` or `x * 0.0` are not value-preserving (signed zero, NaN).
pub(super) fn simplify(
    op: BinaryOp,
    left: Option<i64>,
    right: Option<i64>,
    operand_type: Option<TypeId>,
    layouts: &LayoutTable,
) -> Option<Simplified> {
    let integer = operand_type.is_none_or(|ty| {
        let info = layouts.types[ty.0];
        info.repr == ValueRepr::Integer
    });
    if !integer {
        return None;
    }
    match op {
        BinaryOp::Add => {
            if right == Some(0) {
                Some(Simplified::Left)
            } else if left == Some(0) {
                Some(Simplified::Right)
            } else {
                None
            }
        }
        BinaryOp::Subtract => (right == Some(0)).then_some(Simplified::Left),
        BinaryOp::Multiply => {
            if left == Some(0) || right == Some(0) {
                Some(Simplified::Constant(Folded::Integer(0)))
            } else if right == Some(1) {
                Some(Simplified::Left)
            } else if left == Some(1) {
                Some(Simplified::Right)
            } else {
                None
            }
        }
        BinaryOp::Divide => (right == Some(1)).then_some(Simplified::Left),
        BinaryOp::And => {
            if right == Some(1) {
                Some(Simplified::Left)
            } else if left == Some(1) {
                Some(Simplified::Right)
            } else if left == Some(0) || right == Some(0) {
                Some(Simplified::Constant(Folded::Boolean(false)))
            } else {
                None
            }
        }
        BinaryOp::Or => {
            if right == Some(0) {
                Some(Simplified::Left)
            } else if left == Some(0) {
                Some(Simplified::Right)
            } else if left == Some(1) || right == Some(1) {
                Some(Simplified::Constant(Folded::Boolean(true)))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn fold(
    op: BinaryOp,
    a: i64,
    b: i64,
    ty: Option<TypeId>,
    layouts: &LayoutTable,
) -> Option<Folded> {
    let (bits, unsigned) = if let Some(ty) = ty {
        let info = layouts.types[ty.0];
        if info.repr != ValueRepr::Integer || !(1..=8).contains(&info.size) {
            return None;
        }
        (info.size * 8, layouts.unsigned.contains(&ty))
    } else {
        (64, false)
    };
    let mask = (1_u128 << bits) - 1;
    let a = u128::from(u64::from_ne_bytes(a.to_ne_bytes())) & mask;
    let b = u128::from(u64::from_ne_bytes(b.to_ne_bytes())) & mask;
    if unsigned {
        fold_unsigned(op, a, b, mask)
    } else {
        let signed = |value: u128| {
            let value = i128::try_from(value).ok()?;
            Some(if value >= (1_i128 << (bits - 1)) {
                value - (1_i128 << bits)
            } else {
                value
            })
        };
        fold_signed(op, signed(a)?, signed(b)?, bits)
    }
}

fn compare<T: Copy + Ord>(op: BinaryOp, a: T, b: T) -> Option<Folded> {
    Some(Folded::Boolean(match op {
        BinaryOp::Equal => a == b,
        BinaryOp::NotEqual => a != b,
        BinaryOp::Less => a < b,
        BinaryOp::LessEqual => a <= b,
        BinaryOp::Greater => a > b,
        BinaryOp::GreaterEqual => a >= b,
        _ => return None,
    }))
}

fn fold_unsigned(op: BinaryOp, a: u128, b: u128, maximum: u128) -> Option<Folded> {
    let value = match op {
        BinaryOp::Add => a.checked_add(b)?,
        BinaryOp::Subtract => a.checked_sub(b)?,
        BinaryOp::Multiply => a.checked_mul(b)?,
        BinaryOp::Divide => a.checked_div(b)?,
        BinaryOp::Modulo => a.checked_rem(b)?,
        _ => return compare(op, a, b),
    };
    if value > maximum {
        return None;
    }
    let bits = u64::try_from(value).ok()?;
    Some(Folded::Integer(i64::from_ne_bytes(bits.to_ne_bytes())))
}

fn fold_signed(op: BinaryOp, a: i128, b: i128, bits: usize) -> Option<Folded> {
    let value = match op {
        BinaryOp::Add => a.checked_add(b)?,
        BinaryOp::Subtract => a.checked_sub(b)?,
        BinaryOp::Multiply => a.checked_mul(b)?,
        BinaryOp::Divide => a.checked_div(b)?,
        BinaryOp::Modulo => a.checked_rem(b)?,
        _ => return compare(op, a, b),
    };
    let limit = 1_i128 << (bits - 1);
    if !(-limit..limit).contains(&value) {
        return None;
    }
    Some(Folded::Integer(i64::try_from(value).ok()?))
}
