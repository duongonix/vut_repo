//! Control-flow expression lowering (if, match, result propagation).
use vut_ast::Expr;
use vut_hir::TypeId;
use vut_source::Span;
use vut_types::Type;

use super::super::{BinaryOp, BlockId, BuiltinFunction, Instruction, Terminator, ValueId};
use super::Builder;

impl Builder<'_> {
    pub(super) fn lower_result_propagate(&mut self, source: &Expr) -> Option<ValueId> {
        let source_value = self.expr(source)?;
        let source_ty = *self.semantics.expression_types.get(&source.span())?;
        let Type::Result(ok_ty, _) = self.semantics.types[source_ty.0] else {
            return None;
        };
        let ok_block = self.new_block();
        let err_block = self.new_block();
        let condition = self.value();
        self.emit(Instruction::ResultState {
            result: condition,
            value: source_value,
            ok: true,
        });
        self.terminate(Terminator::Branch {
            condition,
            then_block: ok_block,
            else_block: err_block,
        });
        self.switch_to(err_block);
        let err_payload = self.value();
        self.emit(Instruction::ResultPayload {
            value: err_payload,
            source: source_value,
            ok: false,
        });
        let return_ty = self.return_type?;
        let returned = self.value();
        self.emit(Instruction::ConstructResult {
            value: returned,
            ty: return_ty,
            ok: false,
            payload: err_payload,
        });
        self.cleanup_except(Some(returned));
        self.terminate(Terminator::Return(Some(returned)));
        self.switch_to(ok_block);
        let ok = self.value();
        self.emit(Instruction::ResultPayload {
            value: ok,
            source: source_value,
            ok: true,
        });
        let local = self.add_local(format!("$try{}", ok.0), None, source.span());
        self.local_data[local.0].ty = Some(ok_ty);
        self.emit(Instruction::Store { local, value: ok });
        let value = self.value();
        self.emit(Instruction::Copy { value, local });
        // Ownership of the ok payload transfers to the produced value.
        self.moved.insert(local);
        Some(value)
    }
    pub(super) fn lower_if_expression(&mut self, expression: &vut_ast::If) -> Option<ValueId> {
        let narrowing = self.condition_narrowing(&expression.condition);
        let condition = self.expr(&expression.condition)?;
        let then_block = self.new_block();
        let else_block = self.new_block();
        let join = self.new_block();
        let result_local = self.add_local(format!("$if{}", join.0), None, expression.span);
        self.local_data[result_local.0].ty = self
            .semantics
            .expression_types
            .get(&expression.span)
            .copied();
        self.terminate(Terminator::Branch {
            condition,
            then_block,
            else_block,
        });
        self.switch_to(then_block);
        self.conditional_depth += 1;
        let then_narrowed = self.narrowed.len();
        if let Some((name, false)) = &narrowing {
            self.push_narrowing(name);
        }
        if let Some(value) = self.lower_statements(&expression.body.statements) {
            self.emit(Instruction::Store {
                local: result_local,
                value,
            });
        }
        self.narrowed.truncate(then_narrowed);
        if !self.is_terminated() {
            self.terminate(Terminator::Jump(join));
        }
        self.switch_to(else_block);
        if let Some((condition, body)) = expression.elifs.first() {
            let nested = vut_ast::If {
                condition: condition.clone(),
                body: body.clone(),
                elifs: expression.elifs[1..].to_vec(),
                otherwise: expression.otherwise.clone(),
                span: expression.span,
            };
            if let Some(value) = self.lower_if_expression(&nested) {
                self.emit(Instruction::Store {
                    local: result_local,
                    value,
                });
            }
        } else if let Some(body) = &expression.otherwise {
            let else_narrowed = self.narrowed.len();
            if let Some((name, true)) = &narrowing {
                self.push_narrowing(name);
            }
            if let Some(value) = self.lower_statements(&body.statements) {
                self.emit(Instruction::Store {
                    local: result_local,
                    value,
                });
            }
            self.narrowed.truncate(else_narrowed);
        } else {
            let value = self.value();
            let ty = self.local_data[result_local.0].ty;
            self.emit(Instruction::ConstNull { value, ty });
            self.emit(Instruction::Store {
                local: result_local,
                value,
            });
        }
        if !self.is_terminated() {
            self.terminate(Terminator::Jump(join));
        }
        self.conditional_depth -= 1;
        self.switch_to(join);
        let value = self.value();
        self.emit(Instruction::Copy {
            value,
            local: result_local,
        });
        // The produced value aliases the result block; ownership transfers to
        // it, so the block local must not also be dropped.
        self.moved.insert(result_local);
        Some(value)
    }
    /// Lowers an indented block used as an expression. The block's value is its
    /// final expression, moved out into a synthetic local before any enclosing
    /// scope cleanup drops the block's own bindings.
    pub(super) fn lower_block_expression(&mut self, block: &vut_ast::Block) -> ValueId {
        let result_local =
            self.add_local(format!("$block{}", block.span.start()), None, block.span);
        self.local_data[result_local.0].ty =
            self.semantics.expression_types.get(&block.span).copied();
        if let Some(value) = self.lower_statements(&block.statements) {
            self.emit(Instruction::Store {
                local: result_local,
                value,
            });
        }
        let value = self.value();
        self.emit(Instruction::Copy {
            value,
            local: result_local,
        });
        // The produced value aliases the result local; ownership transfers to it
        // so the local must not also be dropped.
        self.moved.insert(result_local);
        value
    }

    pub(super) fn lower_match_expression(
        &mut self,
        source: &Expr,
        arms: &[vut_ast::MatchArm],
        span: Span,
    ) -> Option<ValueId> {
        let source_ty = *self.semantics.expression_types.get(&source.span())?;
        // Bindings extracted from a borrowed receiver are borrows, not owners.
        let borrowed_source = matches!(
            source,
            Expr::Name(name)
                if self
                    .receiver_local
                    .is_some_and(|receiver| self.locals.get(&name.text).copied() == Some(receiver))
        );
        // A borrowed receiver is read by reference: the pattern does not own the
        // scrutinee, so it must neither retain nor release it.
        let source = if borrowed_source {
            let Expr::Name(name) = source else {
                unreachable!("a borrowed source is a name")
            };
            let local = self
                .locals
                .get(&name.text)
                .copied()
                .expect("a borrowed source is a bound local");
            let value = self.value();
            self.emit(Instruction::Borrow { value, local });
            value
        } else {
            self.expr(source)?
        };
        let join = self.new_block();
        let result_local = self.add_local(format!("$match{}", join.0), None, span);
        self.local_data[result_local.0].ty = self.semantics.expression_types.get(&span).copied();
        let mut test_block = self.current;
        self.conditional_depth += 1;
        for arm in arms {
            self.switch_to(test_block);
            let arm_block = self.new_block();
            let next = self.new_block();
            // A present-requiring pattern on an optional narrows to the inner
            // value, guarded by a presence test so absent values fall through.
            let arm_inner = if pattern_requires_presence(&arm.pattern) {
                match self.semantics.types.get(source_ty.0) {
                    Some(Type::Optional(inner)) => Some(*inner),
                    _ => None,
                }
            } else {
                None
            };
            let (arm_value, arm_ty) = if let Some(inner) = arm_inner {
                let present = self.value();
                self.emit(Instruction::OptionalIsPresent {
                    value: present,
                    operand: source,
                    inner,
                });
                let present_block = self.new_block();
                self.terminate(Terminator::Branch {
                    condition: present,
                    then_block: present_block,
                    else_block: next,
                });
                self.switch_to(present_block);
                let unwrapped = self.value();
                self.emit(Instruction::OptionalUnwrap {
                    value: unwrapped,
                    operand: source,
                    inner,
                });
                (unwrapped, inner)
            } else {
                (source, source_ty)
            };
            let arm_mark = self.local_data.len();
            let mut binding_types = Vec::new();
            self.collect_pattern_bindings(&arm.pattern, arm_ty, &mut binding_types);
            let mut previous = Vec::new();
            for (name, ty) in &binding_types {
                let local = self.add_local(name.clone(), None, arm.span);
                self.local_data[local.0].ty = Some(*ty);
                let old = self.locals.insert(name.clone(), local);
                previous.push((name.clone(), old));
            }
            self.pattern_borrowed = borrowed_source;
            self.lower_pattern(&arm.pattern, arm_value, arm_ty, arm_block, next);
            self.switch_to(arm_block);
            self.pattern_borrowed = false;
            if let Some(guard) = &arm.guard {
                let condition = self.expr(guard)?;
                let body = self.new_block();
                self.terminate(Terminator::Branch {
                    condition,
                    then_block: body,
                    else_block: next,
                });
                self.switch_to(body);
            }
            if let Some(value) = self.expr(&arm.value)
                && !self.is_terminated()
            {
                self.emit(Instruction::Store {
                    local: result_local,
                    value,
                });
            }
            if !self.is_terminated() {
                self.close_scope(arm_mark);
                self.terminate(Terminator::Jump(join));
            }
            for (name, old) in previous.into_iter().rev() {
                match old {
                    Some(local) => {
                        self.locals.insert(name, local);
                    }
                    None => {
                        self.locals.remove(&name);
                    }
                }
            }
            test_block = next;
        }
        self.conditional_depth -= 1;
        self.switch_to(test_block);
        self.terminate(Terminator::Unreachable);
        self.switch_to(join);
        let value = self.value();
        self.emit(Instruction::Copy {
            value,
            local: result_local,
        });
        // The produced value aliases the result block; ownership transfers to
        // it, so the block local must not also be dropped.
        self.moved.insert(result_local);
        Some(value)
    }

    /// Emits tests/branches for `pattern` against `value` in the current block,
    /// jumping to `success` when it matches and `fail` otherwise.
    fn lower_pattern(
        &mut self,
        pattern: &vut_ast::MatchPattern,
        value: ValueId,
        ty: TypeId,
        success: BlockId,
        fail: BlockId,
    ) {
        match pattern {
            vut_ast::MatchPattern::Wildcard(span) | vut_ast::MatchPattern::Error(span) => {
                if !self.pattern_borrowed {
                    self.drop_ignored(value, ty, *span);
                }
                self.terminate(Terminator::Jump(success));
            }
            vut_ast::MatchPattern::Binding(name) => {
                if self.bare_enum_variant(ty, &name.text).is_some() {
                    self.lower_variant_pattern(name, &[], value, ty, success, fail);
                    return;
                }
                if let Some(local) = self.locals.get(&name.text).copied() {
                    self.emit(Instruction::Store { local, value });
                    // A binding extracted from a borrowed scrutinee is itself a
                    // borrow: it must not be released by scope cleanup, because
                    // the scrutinee owner still holds the reference.
                    if self.pattern_borrowed {
                        self.moved.insert(local);
                    }
                }
                self.terminate(Terminator::Jump(success));
            }
            vut_ast::MatchPattern::Group { pattern, .. } => {
                self.lower_pattern(pattern, value, ty, success, fail);
            }
            vut_ast::MatchPattern::Literal { value: literal, .. } => {
                let condition = self.lower_literal_test(literal, value, ty);
                self.terminate(Terminator::Branch {
                    condition,
                    then_block: success,
                    else_block: fail,
                });
            }
            vut_ast::MatchPattern::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let condition = self.lower_range_test(start, end, *inclusive, value);
                if let Some(condition) = condition {
                    self.terminate(Terminator::Branch {
                        condition,
                        then_block: success,
                        else_block: fail,
                    });
                } else {
                    self.terminate(Terminator::Jump(fail));
                }
            }
            vut_ast::MatchPattern::Variant { name, fields, .. } => {
                self.lower_variant_pattern(name, fields, value, ty, success, fail);
            }
            vut_ast::MatchPattern::ResultOk { pattern, .. } => {
                self.lower_result_pattern(pattern, value, ty, true, success, fail);
            }
            vut_ast::MatchPattern::ResultErr { pattern, .. } => {
                self.lower_result_pattern(pattern, value, ty, false, success, fail);
            }
            vut_ast::MatchPattern::Or { alternatives, .. } => {
                for (index, alternative) in alternatives.iter().enumerate() {
                    let last = index + 1 == alternatives.len();
                    let alternative_fail = if last { fail } else { self.new_block() };
                    self.lower_pattern(alternative, value, ty, success, alternative_fail);
                    if !last {
                        self.switch_to(alternative_fail);
                    }
                }
            }
            vut_ast::MatchPattern::List { items, rest, .. } => {
                self.lower_list_pattern(items, rest.as_ref(), value, ty, success, fail);
            }
        }
    }

    fn lower_variant_pattern(
        &mut self,
        name: &vut_ast::Name,
        fields: &[vut_ast::FieldPattern],
        value: ValueId,
        ty: TypeId,
        success: BlockId,
        fail: BlockId,
    ) {
        let Type::Enum(symbol) = self.semantics.types[ty.0] else {
            self.terminate(Terminator::Jump(fail));
            return;
        };
        let Some(variant_index) = self
            .semantics
            .enum_variants
            .get(&symbol)
            .and_then(|variants| {
                variants
                    .iter()
                    .position(|variant| variant.name == name.text)
            })
        else {
            self.terminate(Terminator::Jump(fail));
            return;
        };
        let Some(variant) = self
            .semantics
            .enum_variants
            .get(&symbol)
            .and_then(|variants| variants.get(variant_index))
            .cloned()
        else {
            self.terminate(Terminator::Jump(fail));
            return;
        };

        let tag = self.value();
        self.emit(Instruction::EnumTag {
            result: tag,
            value,
            ty,
        });
        let expected = self.value();
        self.emit(Instruction::ConstInt {
            value: expected,
            literal: i64::try_from(variant_index).unwrap_or(0),
        });
        let condition = self.value();
        self.emit(Instruction::Binary {
            value: condition,
            op: BinaryOp::Equal,
            left: tag,
            right: expected,
        });
        let body = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block: body,
            else_block: fail,
        });
        self.switch_to(body);

        let provided = variant_field_patterns(fields, &variant);
        if provided.is_empty() {
            self.terminate(Terminator::Jump(success));
            return;
        }
        for (index, (field_index, sub_pattern)) in provided.iter().enumerate() {
            let last = index + 1 == provided.len();
            let field_value = self.value();
            self.emit(Instruction::EnumPayload {
                value: field_value,
                source: value,
                ty,
                variant_index,
                field_index: *field_index,
            });
            let field_ty = variant.fields[*field_index].ty;
            let field_success = if last { success } else { self.new_block() };
            self.lower_pattern(sub_pattern, field_value, field_ty, field_success, fail);
            if !last {
                self.switch_to(field_success);
            }
        }
    }

    fn lower_result_pattern(
        &mut self,
        pattern: &vut_ast::MatchPattern,
        value: ValueId,
        ty: TypeId,
        ok: bool,
        success: BlockId,
        fail: BlockId,
    ) {
        let Type::Result(ok_ty, err_ty) = self.semantics.types[ty.0] else {
            self.terminate(Terminator::Jump(fail));
            return;
        };
        let condition = self.value();
        self.emit(Instruction::ResultState {
            result: condition,
            value,
            ok,
        });
        let body = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block: body,
            else_block: fail,
        });
        self.switch_to(body);
        let payload = self.value();
        self.emit(Instruction::ResultPayload {
            value: payload,
            source: value,
            ok,
        });
        let payload_ty = if ok { ok_ty } else { err_ty };
        self.lower_pattern(pattern, payload, payload_ty, success, fail);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "list pattern lowering keeps length, element, and rest handling together"
    )]
    fn lower_list_pattern(
        &mut self,
        items: &[vut_ast::MatchPattern],
        rest: Option<&vut_ast::Name>,
        value: ValueId,
        ty: TypeId,
        success: BlockId,
        fail: BlockId,
    ) {
        let (element_ty, length_fn, at_fn) = match self.semantics.types[ty.0] {
            Type::List(element) => (element, BuiltinFunction::ListLen, BuiltinFunction::ListAt),
            Type::Array(element, _) => {
                (element, BuiltinFunction::ArrayLen, BuiltinFunction::ArrayAt)
            }
            _ => {
                self.terminate(Terminator::Jump(fail));
                return;
            }
        };
        // Length check: exact when there is no rest binding, otherwise `>=`.
        let length = self.value();
        self.emit(Instruction::RuntimeCall {
            value: Some(length),
            result_type: Some(
                self.semantics
                    .types
                    .iter()
                    .position(|ty| matches!(ty, Type::Int))
                    .map_or(element_ty, TypeId),
            ),
            function: length_fn,
            arguments: vec![value],
        });
        let expected = self.value();
        self.emit(Instruction::ConstInt {
            value: expected,
            literal: i64::try_from(items.len()).unwrap_or(0),
        });
        let condition = self.value();
        let op = if rest.is_some() {
            BinaryOp::GreaterEqual
        } else {
            BinaryOp::Equal
        };
        self.emit(Instruction::Binary {
            value: condition,
            op,
            left: length,
            right: expected,
        });
        let body = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block: body,
            else_block: fail,
        });
        self.switch_to(body);
        for (index, item) in items.iter().enumerate() {
            let last = index + 1 == items.len() && rest.is_none();
            let element = self.value();
            let index_value = self.value();
            self.emit(Instruction::ConstInt {
                value: index_value,
                literal: i64::try_from(index).unwrap_or(0),
            });
            self.emit(Instruction::RuntimeCall {
                value: Some(element),
                result_type: Some(element_ty),
                function: at_fn,
                arguments: vec![value, index_value],
            });
            let item_success = if last { success } else { self.new_block() };
            self.lower_pattern(item, element, element_ty, item_success, fail);
            if !last {
                self.switch_to(item_success);
            }
        }
        if let Some(rest) = rest {
            if rest.text == "_" {
                self.terminate(Terminator::Jump(success));
                return;
            }
            let list_ty = self
                .semantics
                .types
                .iter()
                .position(
                    |candidate| matches!(candidate, Type::List(inner) if *inner == element_ty),
                )
                .map(TypeId);
            if let Some(list_ty) = list_ty {
                let start = self.value();
                self.emit(Instruction::ConstInt {
                    value: start,
                    literal: i64::try_from(items.len()).unwrap_or(0),
                });
                let tail = self.value();
                self.emit(Instruction::RuntimeCall {
                    value: Some(tail),
                    result_type: Some(list_ty),
                    function: BuiltinFunction::ListSlice,
                    arguments: vec![value, start, length],
                });
                if let Some(local) = self.locals.get(&rest.text).copied() {
                    self.initialized.insert(local);
                    self.emit(Instruction::Store { local, value: tail });
                }
            }
            self.terminate(Terminator::Jump(success));
        } else if items.is_empty() {
            self.terminate(Terminator::Jump(success));
        }
    }

    fn lower_literal_test(
        &mut self,
        literal: &vut_ast::LiteralPattern,
        value: ValueId,
        ty: TypeId,
    ) -> ValueId {
        // A `null` pattern matches absence. For tagged optionals the presence
        // discriminant must be tested rather than comparing the block address
        // to zero; managed optionals also go through the presence test.
        if matches!(literal, vut_ast::LiteralPattern::Null)
            && let Type::Optional(inner) = self.semantics.types[ty.0]
        {
            let present = self.value();
            self.emit(Instruction::OptionalIsPresent {
                value: present,
                operand: value,
                inner,
            });
            let absent = self.value();
            self.emit(Instruction::Unary {
                value: absent,
                op: vut_ast::UnaryOp::Not,
                operand: present,
            });
            return absent;
        }
        if let vut_ast::LiteralPattern::Str(text) = literal {
            let constant = self.value();
            self.emit(Instruction::ConstString {
                value: constant,
                literal: text.clone(),
            });
            let condition = self.value();
            let boolean = self
                .semantics
                .types
                .iter()
                .position(|ty| matches!(ty, Type::Bool))
                .map(TypeId);
            let str_ty = self
                .semantics
                .types
                .iter()
                .position(|ty| matches!(ty, Type::Str))
                .map(TypeId);
            self.emit(Instruction::RuntimeCall {
                value: Some(condition),
                result_type: boolean,
                function: BuiltinFunction::StringEquals,
                arguments: vec![value, constant],
            });
            // The extracted string payload and the literal constant are both
            // owned temporaries; release them after the comparison.
            if let Some(str_ty) = str_ty {
                self.emit(Instruction::Release {
                    value: constant,
                    ty: str_ty,
                });
                self.emit(Instruction::Release { value, ty: str_ty });
            }
            return condition;
        }
        let constant = self.value();
        match literal {
            vut_ast::LiteralPattern::Integer(text) => {
                self.emit(Instruction::ConstInt {
                    value: constant,
                    literal: text.parse().unwrap_or(0),
                });
            }
            vut_ast::LiteralPattern::Float(text) => {
                self.emit(Instruction::ConstFloat {
                    value: constant,
                    literal: text.parse().unwrap_or(0.0),
                });
            }
            vut_ast::LiteralPattern::Bool(value) => {
                self.emit(Instruction::ConstBool {
                    value: constant,
                    literal: *value,
                });
            }
            vut_ast::LiteralPattern::Str(_) | vut_ast::LiteralPattern::Null => {
                // Unsupported patterns are rejected by the type checker.
                self.emit(Instruction::ConstInt {
                    value: constant,
                    literal: 0,
                });
            }
        }
        let condition = self.value();
        self.emit(Instruction::Binary {
            value: condition,
            op: BinaryOp::Equal,
            left: value,
            right: constant,
        });
        condition
    }

    fn lower_range_test(
        &mut self,
        start: &vut_ast::LiteralPattern,
        end: &vut_ast::LiteralPattern,
        inclusive: bool,
        value: ValueId,
    ) -> Option<ValueId> {
        let (vut_ast::LiteralPattern::Integer(start), vut_ast::LiteralPattern::Integer(end)) =
            (start, end)
        else {
            return None;
        };
        let start_value = self.value();
        self.emit(Instruction::ConstInt {
            value: start_value,
            literal: start.parse().unwrap_or(0),
        });
        let lower = self.value();
        self.emit(Instruction::Binary {
            value: lower,
            op: BinaryOp::GreaterEqual,
            left: value,
            right: start_value,
        });
        let end_value = self.value();
        self.emit(Instruction::ConstInt {
            value: end_value,
            literal: end.parse().unwrap_or(0),
        });
        let upper = self.value();
        self.emit(Instruction::Binary {
            value: upper,
            op: if inclusive {
                BinaryOp::LessEqual
            } else {
                BinaryOp::Less
            },
            left: value,
            right: end_value,
        });
        let condition = self.value();
        self.emit(Instruction::Binary {
            value: condition,
            op: BinaryOp::And,
            left: lower,
            right: upper,
        });
        Some(condition)
    }

    /// Stores an ignored value into a temporary and drops it when it owns
    /// managed data, so `_` patterns do not leak.
    fn drop_ignored(&mut self, value: ValueId, ty: TypeId, span: Span) {
        if !self.layouts.types[ty.0].needs_drop {
            return;
        }
        let local = self.add_local(format!("$ignored{}", value.0), None, span);
        self.local_data[local.0].ty = Some(ty);
        self.initialized.insert(local);
        self.emit(Instruction::Store { local, value });
        self.emit(Instruction::Drop(local));
        self.moved.insert(local);
    }

    /// Resolves a bare identifier pattern to an enum variant index, matching
    /// the type checker's rule.
    fn bare_enum_variant(&self, ty: TypeId, name: &str) -> Option<usize> {
        let Type::Enum(symbol) = self.semantics.types[ty.0] else {
            return None;
        };
        self.semantics
            .enum_variants
            .get(&symbol)
            .and_then(|variants| variants.iter().position(|variant| variant.name == name))
    }

    fn collect_pattern_bindings(
        &self,
        pattern: &vut_ast::MatchPattern,
        subject: TypeId,
        out: &mut Vec<(String, TypeId)>,
    ) {
        match pattern {
            vut_ast::MatchPattern::Binding(name) => {
                if self.bare_enum_variant(subject, &name.text).is_none() {
                    out.push((name.text.clone(), subject));
                }
            }
            vut_ast::MatchPattern::Group { pattern, .. } => {
                self.collect_pattern_bindings(pattern, subject, out);
            }
            vut_ast::MatchPattern::Or { alternatives, .. } => {
                if let Some(first) = alternatives.first() {
                    self.collect_pattern_bindings(first, subject, out);
                }
            }
            vut_ast::MatchPattern::Variant { name, fields, .. } => {
                if let Type::Enum(symbol) = self.semantics.types[subject.0]
                    && let Some(variants) = self.semantics.enum_variants.get(&symbol)
                    && let Some(index) = variants
                        .iter()
                        .position(|variant| variant.name == name.text)
                {
                    let variant = variants[index].clone();
                    for (field_index, sub_pattern) in variant_field_patterns(fields, &variant) {
                        self.collect_pattern_bindings(
                            sub_pattern,
                            variant.fields[field_index].ty,
                            out,
                        );
                    }
                }
            }
            vut_ast::MatchPattern::ResultOk { pattern, .. } => {
                if let Type::Result(ok_ty, _) = self.semantics.types[subject.0] {
                    self.collect_pattern_bindings(pattern, ok_ty, out);
                }
            }
            vut_ast::MatchPattern::ResultErr { pattern, .. } => {
                if let Type::Result(_, err_ty) = self.semantics.types[subject.0] {
                    self.collect_pattern_bindings(pattern, err_ty, out);
                }
            }
            vut_ast::MatchPattern::List { items, rest, .. } => {
                let (Type::List(element) | Type::Array(element, _)) =
                    self.semantics.types[subject.0]
                else {
                    return;
                };
                for item in items {
                    self.collect_pattern_bindings(item, element, out);
                }
                if let Some(rest) = rest
                    && rest.text != "_"
                {
                    let list_ty = self
                        .semantics
                        .types
                        .iter()
                        .position(|ty| matches!(ty, Type::List(inner) if *inner == element))
                        .map(TypeId);
                    if let Some(list_ty) = list_ty {
                        out.push((rest.text.clone(), list_ty));
                    }
                }
            }
            vut_ast::MatchPattern::Wildcard(_)
            | vut_ast::MatchPattern::Literal { .. }
            | vut_ast::MatchPattern::Range { .. }
            | vut_ast::MatchPattern::Error(_) => {}
        }
    }
}

/// Maps provided field patterns to declared variant field indices.
fn variant_field_patterns<'a>(
    fields: &'a [vut_ast::FieldPattern],
    variant: &vut_types::VariantInfo,
) -> Vec<(usize, &'a vut_ast::MatchPattern)> {
    if fields.len() == 1 && matches!(fields[0], vut_ast::FieldPattern::Positional(_)) {
        return vec![(0, fields[0].pattern())];
    }
    let mut provided = Vec::new();
    for field in fields {
        if let vut_ast::FieldPattern::Named {
            field: name,
            pattern,
        } = field
            && let Some(index) = variant
                .fields
                .iter()
                .position(|item| item.name == name.text)
        {
            provided.push((index, pattern));
        }
    }
    provided
}

/// True when a match pattern only matches a present value (so an optional
/// subject should be narrowed). `null` and wildcard also match absence.
fn pattern_requires_presence(pattern: &vut_ast::MatchPattern) -> bool {
    match pattern {
        vut_ast::MatchPattern::Wildcard(_) | vut_ast::MatchPattern::Error(_) => false,
        vut_ast::MatchPattern::Literal {
            value: vut_ast::LiteralPattern::Null,
            ..
        } => false,
        vut_ast::MatchPattern::Group { pattern, .. } => pattern_requires_presence(pattern),
        vut_ast::MatchPattern::Or { alternatives, .. } => {
            !alternatives.is_empty() && alternatives.iter().all(pattern_requires_presence)
        }
        _ => true,
    }
}
