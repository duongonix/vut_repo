//! Receiver discovery and call context for builtin member tooling.

use tower_lsp::lsp_types::{ParameterInformation, ParameterLabel};
use vut_ast::{Block, Expr, File, ForKind, Item, Stmt, TemplateSegment};
use vut_hir::TypeId;
use vut_source::Span;
use vut_types::{BuiltinKind, builtin_kind};

use super::{Analysis, contains};

/// A member access (`object.member`) discovered in the syntax tree.
pub(super) struct MemberUse {
    pub(super) receiver: Span,
    pub(super) member: String,
    pub(super) member_span: Span,
}

/// Finds the member access whose name contains `at`, if any.
pub(super) fn find_member(file: &File, at: usize) -> Option<MemberUse> {
    let mut uses = Vec::new();
    for item in &file.items {
        item_members(item, &mut uses);
    }
    uses.into_iter()
        .find(|usage| contains(usage.member_span, at))
}

fn item_members(item: &Item, uses: &mut Vec<MemberUse>) {
    match item {
        Item::Function(function) => block_members(&function.body, uses),
        Item::Method(method) => block_members(&method.body, uses),
        Item::Data(data) => {
            for field in &data.fields {
                if let Some(default) = &field.default {
                    expr_members(default, uses);
                }
            }
        }
        Item::Statement(statement) => stmt_members(statement, uses),
        Item::Import(_)
        | Item::ExternFunction(_)
        | Item::Interface(_)
        | Item::Enum(_)
        | Item::TypeAlias(_) => {}
    }
}

fn block_members(block: &Block, uses: &mut Vec<MemberUse>) {
    for statement in &block.statements {
        stmt_members(statement, uses);
    }
}

fn stmt_members(statement: &Stmt, uses: &mut Vec<MemberUse>) {
    match statement {
        Stmt::Binding { target, value, .. } => {
            expr_members(target, uses);
            expr_members(value, uses);
        }
        Stmt::Expression(expression)
        | Stmt::Return {
            value: Some(expression),
            ..
        } => expr_members(expression, uses),
        Stmt::Return { value: None, .. } | Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        Stmt::If(value) => if_members(value, uses),
        Stmt::For(value) => {
            match &value.kind {
                ForKind::Infinite => {}
                ForKind::Conditional(condition) => expr_members(condition, uses),
                ForKind::Iterable { iterable, .. } => expr_members(iterable, uses),
            }
            block_members(&value.body, uses);
        }
        Stmt::Unsafe(block) => block_members(block, uses),
    }
}

fn if_members(value: &vut_ast::If, uses: &mut Vec<MemberUse>) {
    expr_members(&value.condition, uses);
    block_members(&value.body, uses);
    for (condition, block) in &value.elifs {
        expr_members(condition, uses);
        block_members(block, uses);
    }
    if let Some(block) = &value.otherwise {
        block_members(block, uses);
    }
}

fn expr_members(expression: &Expr, uses: &mut Vec<MemberUse>) {
    match expression {
        Expr::Name(_)
        | Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Null(_)
        | Expr::Error(_) => {}
        Expr::String { segments, .. } => {
            for segment in segments {
                if let TemplateSegment::Expression(value) = segment {
                    expr_members(value, uses);
                }
            }
        }
        Expr::List { values, .. } | Expr::Array { values, .. } => {
            for value in values {
                expr_members(value, uses);
            }
        }
        Expr::Map { entries, .. } => {
            for entry in entries {
                expr_members(&entry.key, uses);
                expr_members(&entry.value, uses);
            }
        }
        Expr::Unary { value, .. }
        | Expr::Await { value, .. }
        | Expr::Spawn {
            callable: value, ..
        }
        | Expr::Group { value, .. }
        | Expr::ResultOk { value, .. }
        | Expr::ResultErr { value, .. }
        | Expr::ResultPropagate { value, .. } => expr_members(value, uses),
        Expr::Binary { left, right, .. } => {
            expr_members(left, uses);
            expr_members(right, uses);
        }
        Expr::Member { object, member, .. } => {
            expr_members(object, uses);
            uses.push(MemberUse {
                receiver: object.span(),
                member: member.text.clone(),
                member_span: member.span,
            });
        }
        Expr::Subscript { object, index, .. } => {
            expr_members(object, uses);
            if let Some(index) = index {
                expr_members(index, uses);
            }
        }
        Expr::Lambda { body, .. } => match body {
            vut_ast::LambdaBody::Expression(value) => expr_members(value, uses),
            vut_ast::LambdaBody::Block(block) => block_members(block, uses),
        },
        Expr::Call {
            callee, arguments, ..
        } => {
            expr_members(callee, uses);
            for argument in arguments {
                expr_members(&argument.value, uses);
            }
        }
        Expr::If(value) => if_members(value, uses),
        Expr::Block(block) => block_members(block, uses),
        Expr::Match { value, arms, .. } => {
            expr_members(value, uses);
            for arm in arms {
                expr_members(&arm.value, uses);
            }
        }
    }
}

/// Resolves the builtin method family of a receiver expression span.
pub(super) fn receiver_kind(a: &Analysis, receiver: Span) -> Option<BuiltinKind> {
    a.semantics
        .expression_types
        .get(&receiver)
        .and_then(|ty| builtin_kind(&a.semantics.types[ty.0]))
}

/// Resolves the builtin method family of the expression ending at `dot`.
pub(super) fn receiver_kind_at(a: &Analysis, dot: usize) -> Option<BuiltinKind> {
    receiver_type_at(a, dot).and_then(|ty| builtin_kind(&a.semantics.types[ty.0]))
}

/// Resolves the semantic type of the expression ending immediately before a dot.
pub(super) fn receiver_type_at(a: &Analysis, dot: usize) -> Option<TypeId> {
    a.semantics
        .expression_types
        .iter()
        .filter(|(span, _)| span.start() <= dot && span.end() <= dot)
        .max_by_key(|(span, _)| span.end())
        .map(|(_, ty)| *ty)
}

/// Finds the `.member` context at the cursor, returning the dot offset and the
/// partial member text typed after it.
pub(super) fn member_context(text: &str, at: usize) -> Option<(usize, String)> {
    let end = at.min(text.len());
    let prefix = &text[..end];
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let line = &text[line_start..end];
    let dot = line.rfind('.')?;
    let partial = &line[dot + 1..];
    if partial.chars().all(|c| c == '_' || c.is_alphanumeric()) {
        Some((line_start + dot, partial.to_owned()))
    } else {
        None
    }
}

/// The receiver dot, member name, and opening paren of a call at the cursor.
pub(super) struct CallContext {
    pub(super) dot: usize,
    pub(super) member: String,
}

/// Locates the innermost unclosed call before the cursor and its member callee.
pub(super) fn call_context(text: &str, at: usize) -> Option<CallContext> {
    let open = open_call(text, at)?;
    let before = text[..open].trim_end();
    let name_start = before
        .rfind(|character: char| !(character == '_' || character.is_alphanumeric()))
        .map_or(0, |index| index + 1);
    let member = &before[name_start..];
    if member.is_empty() {
        return None;
    }
    let dot = before[..name_start].trim_end().rfind('.')?;
    Some(CallContext {
        dot,
        member: member.to_owned(),
    })
}

/// Locates the opening parenthesis of the innermost unclosed call.
pub(super) fn open_call(text: &str, at: usize) -> Option<usize> {
    let end = at.min(text.len());
    let mut depth = 0_i32;
    for (index, character) in text[..end].char_indices().rev() {
        match character {
            ')' => depth += 1,
            '(' => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Counts top-level arguments consumed before the cursor.
pub(super) fn active_parameter(text: &str, open: usize, at: usize) -> u32 {
    let mut depth = 0_i32;
    let mut count = 0_u32;
    for character in text[open + 1..at.min(text.len())].chars() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}

/// Splits a rendered signature's parameter list into editor parameters.
pub(super) fn parameter_infos(rendered: &str) -> Vec<ParameterInformation> {
    let Some(start) = rendered.find('(') else {
        return Vec::new();
    };
    let Some(end) = matching_close(rendered, start) else {
        return Vec::new();
    };
    let inner = &rendered[start + 1..end];
    if inner.trim().is_empty() {
        return Vec::new();
    }
    split_top_level(inner)
        .into_iter()
        .map(|parameter| ParameterInformation {
            label: ParameterLabel::Simple(parameter.trim().to_owned()),
            documentation: None,
        })
        .collect()
}

fn matching_close(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0_u32;
    for (relative, character) in text[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + relative);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, character) in text.char_indices() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    result.push(&text[start..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_preserve_nested_generic_commas() {
        let values = parameter_infos("fn f(values: map[str, list[int]], callback: fn(int, str))");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn open_call_finds_innermost_unclosed_call() {
        let source = "outer(1, inner(2, ";
        assert_eq!(open_call(source, source.len()), Some(14));
        assert_eq!(active_parameter(source, 14, source.len()), 1);
    }
}
