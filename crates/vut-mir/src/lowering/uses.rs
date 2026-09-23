//! Use-count analysis that drives move/cleanup decisions.
use std::collections::HashMap;

use vut_ast::{Expr, Stmt};

#[expect(
    clippy::semicolon_if_nothing_returned,
    clippy::match_same_arms,
    clippy::too_many_lines,
    reason = "recursive AST visitor mirrors syntax structure"
)]
pub(super) fn count_uses(block: &vut_ast::Block) -> HashMap<String, usize> {
    fn expression(expr: &Expr, uses: &mut HashMap<String, usize>) {
        match expr {
            Expr::Lambda { .. } | Expr::Spawn { .. } => {}
            Expr::Name(name) => *uses.entry(name.text.clone()).or_default() += 1,
            Expr::Unary { value, .. }
            | Expr::Group { value, .. }
            | Expr::ResultOk { value, .. }
            | Expr::ResultErr { value, .. }
            | Expr::ResultPropagate { value, .. }
            | Expr::Await { value, .. } => expression(value, uses),
            Expr::Binary { left, right, .. } => {
                expression(left, uses);
                expression(right, uses);
            }
            Expr::Member { object, .. } => expression(object, uses),
            Expr::Subscript { object, index, .. } => {
                expression(object, uses);
                if let Some(index) = index {
                    expression(index, uses);
                }
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                expression(callee, uses);
                for argument in arguments {
                    expression(&argument.value, uses);
                }
            }
            Expr::List { values, .. } | Expr::Array { values, .. } => {
                for value in values {
                    expression(value, uses);
                }
            }
            Expr::Map { entries, .. } => {
                for entry in entries {
                    expression(&entry.key, uses);
                    expression(&entry.value, uses);
                }
            }
            Expr::String { segments, .. } => {
                for segment in segments {
                    if let vut_ast::TemplateSegment::Expression(value) = segment {
                        expression(value, uses);
                    }
                }
            }
            Expr::If(value) => {
                expression(&value.condition, uses);
                statements(&value.body, uses);
                for (condition, body) in &value.elifs {
                    expression(condition, uses);
                    statements(body, uses);
                }
                if let Some(body) = &value.otherwise {
                    statements(body, uses);
                }
            }
            Expr::Match { value, arms, .. } => {
                expression(value, uses);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        expression(guard, uses);
                    }
                    expression(&arm.value, uses);
                }
            }
            Expr::Block(block) => statements(block, uses),
            Expr::Integer { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Null(_)
            | Expr::Error(_) => {}
        }
    }
    fn statements(block: &vut_ast::Block, uses: &mut HashMap<String, usize>) {
        for statement in &block.statements {
            match statement {
                Stmt::Binding { target, value, .. } => {
                    if !matches!(target, Expr::Name(_)) {
                        expression(target, uses)
                    }
                    expression(value, uses)
                }
                Stmt::Expression(value) => expression(value, uses),
                Stmt::If(value) => {
                    expression(&value.condition, uses);
                    statements(&value.body, uses);
                    for (condition, body) in &value.elifs {
                        expression(condition, uses);
                        statements(body, uses)
                    }
                    if let Some(body) = &value.otherwise {
                        statements(body, uses)
                    }
                }
                Stmt::For(value) => {
                    match &value.kind {
                        vut_ast::ForKind::Conditional(condition) => expression(condition, uses),
                        vut_ast::ForKind::Iterable { iterable, .. } => expression(iterable, uses),
                        vut_ast::ForKind::Infinite => {}
                    }
                    statements(&value.body, uses)
                }
                Stmt::Unsafe(body) => statements(body, uses),
                Stmt::Return {
                    value: Some(value), ..
                } => expression(value, uses),
                Stmt::Return { value: None, .. }
                | Stmt::Break(_)
                | Stmt::Continue(_)
                | Stmt::Error(_) => {}
            }
        }
    }
    let mut uses = HashMap::new();
    statements(block, &mut uses);
    uses
}
