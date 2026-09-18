//! Expression lowering (calls, members, literals, operators).
use vut_ast::Expr;
use vut_diagnostics::{Diagnostic, codes};
use vut_hir::TypeId;
use vut_memory::{Duplication, Transfer, transfer_for_use};
use vut_types::Type;

use super::super::interface::method_names;
use super::super::layout::is_unsigned_type;
use super::super::{BinaryOp, BuiltinFunction, Instruction, Terminator, ValueId};
use super::Builder;

impl Builder<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "MIR expression lowering is intentionally exhaustive"
    )]
    pub(super) fn expr(&mut self, expr: &Expr) -> Option<ValueId> {
        match expr {
            Expr::Lambda { span, .. } => {
                let symbol = self.lambda_symbols.get(span).copied()?;
                let value = self.value();
                self.emit(Instruction::MakeFunction { value, symbol });
                Some(value)
            }
            Expr::Integer { text, .. } => {
                let value = self.value();
                self.emit(Instruction::ConstInt {
                    value,
                    literal: text.replace('_', "").parse().ok()?,
                });
                Some(value)
            }
            Expr::Float { text, .. } => {
                let value = self.value();
                self.emit(Instruction::ConstFloat {
                    value,
                    literal: text.replace('_', "").parse().ok()?,
                });
                Some(value)
            }
            Expr::Bool { value: literal, .. } => {
                let value = self.value();
                self.emit(Instruction::ConstBool {
                    value,
                    literal: *literal,
                });
                Some(value)
            }
            Expr::Null(_) => {
                let value = self.value();
                self.emit(Instruction::ConstNull { value });
                Some(value)
            }
            Expr::Unary {
                op, value: operand, ..
            } => {
                let operand = self.expr(operand)?;
                let value = self.value();
                self.emit(Instruction::Unary {
                    value,
                    op: *op,
                    operand,
                });
                Some(value)
            }
            Expr::String { segments, .. } => {
                if segments.is_empty() {
                    // An empty literal has no text/expression segments; it is
                    // still a real empty string value.
                    let value = self.value();
                    self.emit(Instruction::ConstString {
                        value,
                        literal: String::new(),
                    });
                    return Some(value);
                }
                let mut result = None;
                for segment in segments {
                    let part = match segment {
                        vut_ast::TemplateSegment::Text { text, .. } => {
                            let value = self.value();
                            self.emit(Instruction::ConstString {
                                value,
                                literal: text.clone(),
                            });
                            value
                        }
                        vut_ast::TemplateSegment::Expression(expression) => {
                            let operand = self.expr(expression)?;
                            let ty = *self.semantics.expression_types.get(&expression.span())?;
                            if matches!(self.semantics.types[ty.0], Type::Str) {
                                operand
                            } else {
                                let value = self.value();
                                let unsigned = is_unsigned_type(&self.semantics.types[ty.0]);
                                self.emit(Instruction::FormatValue {
                                    value,
                                    operand,
                                    ty,
                                    unsigned,
                                });
                                value
                            }
                        }
                    };
                    result = Some(if let Some(left) = result {
                        let value = self.value();
                        self.emit(Instruction::ConcatString {
                            value,
                            left,
                            right: part,
                        });
                        if let Some(ty) = self
                            .semantics
                            .types
                            .iter()
                            .position(|ty| matches!(ty, Type::Str))
                            .map(TypeId)
                        {
                            self.emit(Instruction::Release { value: left, ty });
                            self.emit(Instruction::Release { value: part, ty });
                        }
                        value
                    } else {
                        part
                    });
                }
                result
            }
            Expr::List { values, span } => {
                let ty = *self.semantics.expression_types.get(span)?;
                let Type::List(element_ty) = self.semantics.types[ty.0] else {
                    return None;
                };
                let element_size = self.value();
                self.emit(Instruction::ConstInt {
                    value: element_size,
                    literal: i64::try_from(self.layouts.element_storage_size(element_ty)).ok()?,
                });
                let element_align = self.value();
                self.emit(Instruction::ConstInt {
                    value: element_align,
                    literal: i64::try_from(self.layouts.element_storage_align(element_ty)).ok()?,
                });
                let element_retain = self.value();
                self.emit(Instruction::TypeRetain {
                    value: element_retain,
                    ty: element_ty,
                });
                let element_release = self.value();
                self.emit(Instruction::TypeRelease {
                    value: element_release,
                    ty: element_ty,
                });
                let capacity = self.value();
                self.emit(Instruction::ConstInt {
                    value: capacity,
                    literal: i64::try_from(values.len()).ok()?,
                });
                let value = self.value();
                self.emit(Instruction::RuntimeCall {
                    value: Some(value),
                    result_type: Some(ty),
                    function: BuiltinFunction::ListNew,
                    arguments: vec![
                        element_size,
                        element_align,
                        element_retain,
                        element_release,
                        capacity,
                    ],
                });
                let mut owned: Vec<(ValueId, TypeId)> = Vec::new();
                for item in values {
                    let mut operand = Vec::new();
                    self.lower_builtin_value(item, &mut operand, &mut owned)?;
                    let element = *operand.first()?;
                    self.emit(Instruction::RuntimeCall {
                        value: None,
                        result_type: None,
                        function: BuiltinFunction::ListPush,
                        arguments: vec![value, element],
                    });
                }
                for (owned_value, owned_ty) in owned {
                    self.emit(Instruction::Release {
                        value: owned_value,
                        ty: owned_ty,
                    });
                }
                Some(value)
            }
            Expr::Map { entries, span } => {
                let ty = *self.semantics.expression_types.get(span)?;
                let Type::Map(key_ty, value_ty) = self.semantics.types[ty.0] else {
                    return None;
                };
                let key_layout = self.layouts.types[key_ty.0];
                let key_size = self.value();
                self.emit(Instruction::ConstInt {
                    value: key_size,
                    literal: i64::try_from(key_layout.size).ok()?,
                });
                let key_align = self.value();
                self.emit(Instruction::ConstInt {
                    value: key_align,
                    literal: i64::try_from(key_layout.alignment).ok()?,
                });
                let value_size = self.value();
                self.emit(Instruction::ConstInt {
                    value: value_size,
                    literal: i64::try_from(self.layouts.element_storage_size(value_ty)).ok()?,
                });
                let value_align = self.value();
                self.emit(Instruction::ConstInt {
                    value: value_align,
                    literal: i64::try_from(self.layouts.element_storage_align(value_ty)).ok()?,
                });
                let key_kind = self.value();
                self.emit(Instruction::ConstInt {
                    value: key_kind,
                    literal: i64::from(u8::from(matches!(
                        self.semantics.types[key_ty.0],
                        Type::Str
                    ))),
                });
                let value_retain = self.value();
                self.emit(Instruction::TypeRetain {
                    value: value_retain,
                    ty: value_ty,
                });
                let value_release = self.value();
                self.emit(Instruction::TypeRelease {
                    value: value_release,
                    ty: value_ty,
                });
                let capacity = self.value();
                self.emit(Instruction::ConstInt {
                    value: capacity,
                    literal: i64::try_from(entries.len()).ok()?,
                });
                let value = self.value();
                self.emit(Instruction::RuntimeCall {
                    value: Some(value),
                    result_type: Some(ty),
                    function: BuiltinFunction::MapNew,
                    arguments: vec![
                        key_size,
                        key_align,
                        value_size,
                        value_align,
                        key_kind,
                        value_retain,
                        value_release,
                        capacity,
                    ],
                });
                let mut owned: Vec<(ValueId, TypeId)> = Vec::new();
                for entry in entries {
                    let mut key_operand = Vec::new();
                    self.lower_builtin_value(&entry.key, &mut key_operand, &mut owned)?;
                    let key = *key_operand.first()?;
                    let mut value_operand = Vec::new();
                    self.lower_builtin_value(&entry.value, &mut value_operand, &mut owned)?;
                    let item = *value_operand.first()?;
                    self.emit(Instruction::RuntimeCall {
                        value: None,
                        result_type: None,
                        function: BuiltinFunction::MapSet,
                        arguments: vec![value, key, item],
                    });
                }
                for (owned_value, owned_ty) in owned {
                    self.emit(Instruction::Release {
                        value: owned_value,
                        ty: owned_ty,
                    });
                }
                Some(value)
            }
            Expr::Array { values, span } => {
                let elements = values.iter().filter_map(|item| self.expr(item)).collect();
                let ty = *self.semantics.expression_types.get(span)?;
                let value = self.value();
                self.emit(Instruction::Allocate { value, ty });
                self.emit(Instruction::ConstructArray {
                    value,
                    ty,
                    elements,
                });
                Some(value)
            }
            Expr::Name(name) => {
                let Some(local) = self.locals.get(&name.text).copied() else {
                    let ty = self.semantics.expression_types.get(&name.span).copied();
                    if let Some(ty) = ty
                        && let Type::Function(symbol) = self.semantics.types[ty.0]
                    {
                        let value = self.value();
                        self.emit(Instruction::MakeFunction { value, symbol });
                        return Some(value);
                    }
                    return None;
                };
                let value = self.value();
                let last_use = self.remaining_uses.consume(&name.text);
                let ty = self.local_data[local.0].ty;
                let is_copy = ty.is_none_or(|ty| self.layouts.types[ty.0].is_copy);
                let duplication = ty.map_or(Duplication::Opaque, |ty| {
                    let info = &self.layouts.types[ty.0];
                    let duplication = info.ownership.duplication(info.is_copy);
                    if info.contains_linear && duplication == Duplication::Structural {
                        Duplication::Linear
                    } else {
                        duplication
                    }
                });
                let already_moved = self.moved.contains(&local);
                let is_receiver = self.receiver_local == Some(local);
                let transfer = if duplication == Duplication::Linear {
                    if already_moved {
                        self.diagnostics.push(Diagnostic::error(
                            codes::E8009,
                            "value used after move",
                            name.span,
                            "this move-only value was already moved or awaited",
                        ));
                        Transfer::Move
                    } else if self.conditional_depth > 0 {
                        self.diagnostics.push(Diagnostic::error(
                            codes::E8011,
                            "move-only value used inside a branch",
                            name.span,
                            "move the `resource` before the branch; consuming it inside a conditional is not supported",
                        ));
                        Transfer::Move
                    } else if last_use {
                        Transfer::Move
                    } else {
                        self.diagnostics.push(Diagnostic::error(
                            codes::E8010,
                            "cannot duplicate move-only value",
                            name.span,
                            "a `resource` has a single owner and may only be moved once",
                        ));
                        Transfer::Move
                    }
                } else if is_receiver {
                    // `self` is borrowed by the caller: a read produces an owned
                    // reference (Copy + Retain) that its consumer releases, so
                    // the callee never consumes or drops the borrow itself.
                    Transfer::RetainedCopy
                } else if self.conditional_depth > 0 && !is_copy {
                    Transfer::RetainedCopy
                } else {
                    transfer_for_use(duplication, last_use).unwrap_or(Transfer::Copy)
                };
                if transfer != Transfer::Move
                    && self.local_data[local.0]
                        .ty
                        .is_some_and(|ty| matches!(self.semantics.types[ty.0], Type::Bytes))
                {
                    let borrowed = self.value();
                    self.emit(Instruction::Borrow {
                        value: borrowed,
                        local,
                    });
                    self.emit(Instruction::RuntimeCall {
                        value: Some(value),
                        result_type: self.local_data[local.0].ty,
                        function: BuiltinFunction::BytesClone,
                        arguments: vec![borrowed],
                    });
                    return Some(value);
                }
                let instruction = match transfer {
                    Transfer::Copy | Transfer::RetainedCopy => Instruction::Copy { value, local },
                    Transfer::Move => {
                        self.moved.insert(local);
                        Instruction::Move { value, local }
                    }
                };
                self.emit(instruction);
                // A non-moving read produces its own owned reference: consumers
                // such as string concatenation and field access release their
                // operands, so the reference must be retained here.
                if transfer != Transfer::Move
                    && let Some(ty) = self.local_data[local.0].ty
                {
                    self.emit(Instruction::Retain { value, ty });
                }
                Some(value)
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let left_ty = self.semantics.expression_types.get(&left.span()).copied();
                let right_ty = self.semantics.expression_types.get(&right.span()).copied();
                let left = self.expr(left)?;
                let right = self.expr(right)?;
                let value = self.value();
                let ty = self.semantics.expression_types.get(span).copied();
                let operands_are_str = left_ty
                    .is_some_and(|ty| matches!(self.semantics.types[ty.0], Type::Str))
                    && right_ty.is_some_and(|ty| matches!(self.semantics.types[ty.0], Type::Str));
                if operands_are_str && matches!(*op, BinaryOp::Equal | BinaryOp::NotEqual) {
                    // `str` equality is content equality, not pointer identity.
                    let bool_ty = self
                        .semantics
                        .types
                        .iter()
                        .position(|ty| matches!(ty, Type::Bool))
                        .map(TypeId);
                    self.emit(Instruction::RuntimeCall {
                        value: Some(value),
                        result_type: bool_ty,
                        function: BuiltinFunction::StringEquals,
                        arguments: vec![left, right],
                    });
                    let result = if *op == BinaryOp::NotEqual {
                        let inverted = self.value();
                        self.emit(Instruction::Unary {
                            value: inverted,
                            op: vut_ast::UnaryOp::Not,
                            operand: value,
                        });
                        inverted
                    } else {
                        value
                    };
                    if let Some(ty) = self
                        .semantics
                        .types
                        .iter()
                        .position(|ty| matches!(ty, Type::Str))
                        .map(TypeId)
                    {
                        self.emit(Instruction::Release { value: left, ty });
                        self.emit(Instruction::Release { value: right, ty });
                    }
                    return Some(result);
                }
                if *op == BinaryOp::Add
                    && ty.is_some_and(|ty| matches!(self.semantics.types[ty.0], Type::Str))
                {
                    self.emit(Instruction::ConcatString { value, left, right });
                    if let Some(str_ty) = ty {
                        self.emit(Instruction::Release {
                            value: left,
                            ty: str_ty,
                        });
                        self.emit(Instruction::Release {
                            value: right,
                            ty: str_ty,
                        });
                    }
                } else {
                    self.emit(Instruction::Binary {
                        value,
                        op: *op,
                        left,
                        right,
                    });
                    // A comparison consumes nothing; owned managed operands are
                    // temporaries and must be released here (str equality and
                    // concatenation already release theirs above).
                    if matches!(*op, BinaryOp::Equal | BinaryOp::NotEqual) {
                        for (operand, operand_ty) in [(left, left_ty), (right, right_ty)] {
                            if let Some(operand_ty) = operand_ty
                                && self.layouts.types[operand_ty.0].needs_drop
                            {
                                self.emit(Instruction::Release {
                                    value: operand,
                                    ty: operand_ty,
                                });
                            }
                        }
                    }
                }
                Some(value)
            }
            Expr::Call {
                callee,
                arguments,
                span,
            } => {
                if let Some(construction) = self.semantics.variant_constructions.get(span).cloned()
                {
                    let ty = *self.semantics.expression_types.get(span)?;
                    let field_count = self
                        .semantics
                        .enum_variants
                        .get(&construction.enum_symbol)
                        .and_then(|variants| variants.get(construction.variant_index))
                        .map_or(0, |variant| variant.fields.len());
                    let mut payload: Vec<Option<ValueId>> = vec![None; field_count];
                    for (argument, field_index) in arguments.iter().zip(&construction.fields) {
                        let value = self.expr(&argument.value)?;
                        if let Some(slot) = payload.get_mut(*field_index) {
                            *slot = Some(value);
                        }
                    }
                    let payload: Vec<ValueId> = payload.into_iter().flatten().collect();
                    let value = self.value();
                    self.emit(Instruction::ConstructEnum {
                        value,
                        ty,
                        variant_index: construction.variant_index,
                        payload,
                    });
                    return Some(value);
                }
                if let Expr::Member { object, member, .. } = callee.as_ref()
                    && let Some(object_ty) =
                        self.semantics.expression_types.get(&object.span()).copied()
                    && let Type::Interface(interface) = self.semantics.types[object_ty.0]
                {
                    let callee_value = if let Expr::Name(name) = object.as_ref()
                        && let Some(local) = self.locals.get(&name.text).copied()
                    {
                        let value = self.value();
                        self.emit(Instruction::Borrow { value, local });
                        value
                    } else {
                        self.expr(object)?
                    };
                    let method_index = method_names(self.semantics, interface)
                        .iter()
                        .position(|name| name == &member.text)
                        .unwrap_or(0);
                    let mut argument_values = Vec::new();
                    for argument in arguments {
                        argument_values.push(self.expr(&argument.value)?);
                    }
                    let result_type = self.semantics.expression_types.get(span).copied();
                    let returns_value = result_type
                        .is_none_or(|ty| !matches!(self.semantics.types[ty.0], Type::Void));
                    let value = returns_value.then(|| self.value());
                    self.emit(Instruction::InterfaceCall {
                        value,
                        result_type,
                        callee: callee_value,
                        method_index,
                        arguments: argument_values,
                    });
                    return value;
                }
                let resolved_target = self.semantics.call_targets.get(span).copied();
                if self.semantics.receiver_calls.contains(span) {
                    let Expr::Member { object, .. } = callee.as_ref() else {
                        return None;
                    };
                    let callee_value = self.expr(object)?;
                    let mut argument_values = Vec::new();
                    // The receiver is borrowed, exactly like a method receiver:
                    // it is not moved into the receiver function.
                    let mut receiver_owned: Option<(ValueId, vut_hir::TypeId)> = None;
                    if let Some(receiver) = arguments.first() {
                        match &receiver.value {
                            Expr::Name(name) if self.locals.contains_key(&name.text) => {
                                let local = self.locals[&name.text];
                                let value = self.value();
                                self.emit(Instruction::Borrow { value, local });
                                argument_values.push(value);
                            }
                            Expr::Member {
                                object: inner,
                                member: inner_member,
                                ..
                            } if matches!(
                                inner.as_ref(),
                                Expr::Name(base) if self.locals.contains_key(&base.text)
                            ) =>
                            {
                                let Expr::Name(base_name) = inner.as_ref() else {
                                    unreachable!("guard requires a name base")
                                };
                                let base = self.locals[&base_name.text];
                                let value = self.value();
                                self.emit(Instruction::BorrowField {
                                    value,
                                    base,
                                    name: inner_member.text.clone(),
                                });
                                argument_values.push(value);
                            }
                            _ => {
                                let value = self.expr(&receiver.value)?;
                                if let Some(ty) = self
                                    .semantics
                                    .expression_types
                                    .get(&receiver.value.span())
                                    .copied()
                                    && self.layouts.types[ty.0].needs_drop
                                {
                                    receiver_owned = Some((value, ty));
                                }
                                argument_values.push(value);
                            }
                        }
                    }
                    for argument in arguments.iter().skip(1) {
                        argument_values.push(self.expr(&argument.value)?);
                    }
                    let callable_ty = self.semantics.expression_types.get(&object.span()).copied();
                    let result_type = self.semantics.expression_types.get(span).copied();
                    let returns_value = result_type
                        .is_none_or(|ty| !matches!(self.semantics.types[ty.0], Type::Void));
                    let value = returns_value.then(|| self.value());
                    self.emit(Instruction::CallIndirect {
                        value,
                        result_type,
                        callable_ty,
                        callee: callee_value,
                        arguments: argument_values,
                    });
                    if let Some((owned_value, owned_ty)) = receiver_owned {
                        self.emit(Instruction::Release {
                            value: owned_value,
                            ty: owned_ty,
                        });
                    }
                    return value;
                }
                let builtin_target = self.semantics.builtin_calls.get(span).copied();
                if let Some(function) = builtin_target {
                    if super::collections::is_higher_order(function) {
                        return self.lower_higher_order_builtin(
                            function,
                            callee.as_ref(),
                            arguments,
                            *span,
                        );
                    }
                    if matches!(
                        function,
                        BuiltinFunction::VariadicLen | BuiltinFunction::VariadicAt
                    ) {
                        return self.lower_variadic_builtin(callee, arguments, function);
                    }
                    let mut argument_values = Vec::new();
                    let mut owned: Vec<(ValueId, TypeId)> = Vec::new();
                    if let Expr::Member { object, .. } = callee.as_ref() {
                        let static_builtin = matches!(
                            function,
                            BuiltinFunction::BytesFromList | BuiltinFunction::BytesFromHex
                        ) && matches!(object.as_ref(), Expr::Name(name) if name.text == "bytes");
                        if !static_builtin {
                            self.lower_builtin_receiver(object, &mut argument_values, &mut owned)?;
                        }
                    }
                    for argument in arguments {
                        self.lower_builtin_value(
                            &argument.value,
                            &mut argument_values,
                            &mut owned,
                        )?;
                    }
                    // Derive the receiver's element type for sort kind / array
                    // layout arguments.
                    let receiver_element = match callee.as_ref() {
                        Expr::Member { object, .. } => self
                            .semantics
                            .expression_types
                            .get(&object.span())
                            .copied()
                            .and_then(|ty| match self.semantics.types[ty.0] {
                                Type::List(element) | Type::Array(element, _) => Some(element),
                                _ => None,
                            }),
                        _ => None,
                    };
                    // `list.sort` / `array.sort` carry an element-comparison
                    // kind the runtime uses to pick its ordering.
                    if matches!(
                        function,
                        BuiltinFunction::ListSort | BuiltinFunction::ArraySort
                    ) {
                        let kind = receiver_element.map_or(0, |element| {
                            match self.semantics.types[element.0] {
                                Type::Float => 1,
                                Type::Str => 2,
                                Type::Bool => 3,
                                _ => 0,
                            }
                        });
                        let value = self.value();
                        self.emit(Instruction::ConstInt {
                            value,
                            literal: kind,
                        });
                        argument_values.push(value);
                    }
                    // `array.to_list` passes the fixed length, element layout,
                    // and retain/release callbacks to the runtime.
                    if function == BuiltinFunction::ArrayToList {
                        let array = match callee.as_ref() {
                            Expr::Member { object, .. } => self
                                .semantics
                                .expression_types
                                .get(&object.span())
                                .copied()
                                .and_then(|ty| match self.semantics.types[ty.0] {
                                    Type::Array(element, length) => Some((element, length)),
                                    _ => None,
                                }),
                            _ => None,
                        };
                        if let Some((element_ty, length)) = array {
                            let length_value = self.value();
                            self.emit(Instruction::ConstInt {
                                value: length_value,
                                literal: i64::try_from(length).unwrap_or(i64::MAX),
                            });
                            argument_values.push(length_value);
                            let size = self.value();
                            self.emit(Instruction::ConstInt {
                                value: size,
                                literal: i64::try_from(
                                    self.layouts.element_storage_size(element_ty),
                                )
                                .unwrap_or(i64::MAX),
                            });
                            argument_values.push(size);
                            let align = self.value();
                            self.emit(Instruction::ConstInt {
                                value: align,
                                literal: i64::try_from(
                                    self.layouts.element_storage_align(element_ty),
                                )
                                .unwrap_or(i64::MAX),
                            });
                            argument_values.push(align);
                            let retain = self.value();
                            self.emit(Instruction::TypeRetain {
                                value: retain,
                                ty: element_ty,
                            });
                            argument_values.push(retain);
                            let release = self.value();
                            self.emit(Instruction::TypeRelease {
                                value: release,
                                ty: element_ty,
                            });
                            argument_values.push(release);
                        }
                    }
                    // `result.is_ok` / `result.is_err` read the tag directly.
                    if matches!(
                        function,
                        BuiltinFunction::ResultIsOk | BuiltinFunction::ResultIsErr
                    ) {
                        let receiver = argument_values[0];
                        let value = self.value();
                        self.emit(Instruction::ResultState {
                            result: value,
                            value: receiver,
                            ok: function == BuiltinFunction::ResultIsOk,
                        });
                        return Some(value);
                    }
                    // `result.unwrap_or(default)` returns the payload on `ok`,
                    // and the default otherwise.
                    if function == BuiltinFunction::ResultUnwrapOr {
                        let receiver = argument_values[0];
                        let default = argument_values[1];
                        let result_ty = self.semantics.expression_types.get(span).copied();
                        let local = self.add_local("$unwrap_or".into(), None, *span);
                        self.local_data[local.0].ty = result_ty;
                        let ok_block = self.new_block();
                        let err_block = self.new_block();
                        let join = self.new_block();
                        let condition = self.value();
                        self.emit(Instruction::ResultState {
                            result: condition,
                            value: receiver,
                            ok: true,
                        });
                        self.terminate(Terminator::Branch {
                            condition,
                            then_block: ok_block,
                            else_block: err_block,
                        });
                        self.switch_to(ok_block);
                        let payload = self.value();
                        self.emit(Instruction::ResultPayload {
                            value: payload,
                            source: receiver,
                            ok: true,
                        });
                        self.emit(Instruction::Store {
                            local,
                            value: payload,
                        });
                        self.terminate(Terminator::Jump(join));
                        self.switch_to(err_block);
                        self.emit(Instruction::Store {
                            local,
                            value: default,
                        });
                        self.terminate(Terminator::Jump(join));
                        self.switch_to(join);
                        let out = self.value();
                        self.emit(Instruction::Copy { value: out, local });
                        return Some(out);
                    }
                    let result_type = self.semantics.expression_types.get(span).copied();
                    let returns_value = result_type
                        .is_none_or(|ty| !matches!(self.semantics.types[ty.0], Type::Void));
                    let value = returns_value.then(|| self.value());
                    self.emit(Instruction::RuntimeCall {
                        value,
                        result_type,
                        function,
                        arguments: argument_values,
                    });
                    for (owned_value, owned_ty) in owned {
                        self.emit(Instruction::Release {
                            value: owned_value,
                            ty: owned_ty,
                        });
                    }
                    return value;
                }
                let callee_ty = self.semantics.expression_types.get(&callee.span()).copied();
                let is_value_call = resolved_target.is_none()
                    && callee_ty.is_some_and(|ty| {
                        matches!(
                            self.semantics.types[ty.0],
                            Type::Function(_)
                                | Type::Callable { .. }
                                | Type::FunctionPointer { .. }
                        )
                    });
                if is_value_call {
                    let callee_value = self.expr(callee)?;
                    let mut argument_values = Vec::new();
                    for argument in arguments {
                        argument_values.push(self.expr(&argument.value)?);
                    }
                    let result_type = self.semantics.expression_types.get(span).copied();
                    let returns_value = result_type
                        .is_none_or(|ty| !matches!(self.semantics.types[ty.0], Type::Void));
                    let value = returns_value.then(|| self.value());
                    self.emit(Instruction::CallIndirect {
                        value,
                        result_type,
                        callable_ty: callee_ty,
                        callee: callee_value,
                        arguments: argument_values,
                    });
                    return value;
                }
                let mut lowered = Vec::new();
                let mut receiver_owned: Option<(ValueId, vut_hir::TypeId)> = None;
                let target_span = match callee.as_ref() {
                    Expr::Member { object, member, .. } => {
                        // A static/associated call (`Type.name(...)`) has no
                        // receiver, even though its receiver type names a
                        // data/enum.
                        let static_call = resolved_target
                            .is_some_and(|symbol| self.semantics.static_methods.contains(&symbol));
                        let has_runtime_receiver = !static_call
                            && self
                                .semantics
                                .expression_types
                                .get(&object.span())
                                .is_some_and(|ty| {
                                    matches!(
                                        self.semantics.types[ty.0],
                                        Type::Data(_) | Type::Enum(_) | Type::Interface(_)
                                    )
                                });
                        if has_runtime_receiver {
                            if let Expr::Name(name) = object.as_ref()
                                && let Some(local) = self.locals.get(&name.text).copied()
                            {
                                let value = self.value();
                                self.emit(Instruction::Borrow { value, local });
                                lowered.push(value);
                            } else if let Expr::Member {
                                object: inner,
                                member: inner_member,
                                ..
                            } = object.as_ref()
                                && let Expr::Name(base_name) = inner.as_ref()
                                && let Some(base) = self.locals.get(&base_name.text).copied()
                            {
                                // A projected field receiver is a borrow, so a
                                // mutating method acts on the caller's storage
                                // and no ownership is transferred.
                                let value = self.value();
                                self.emit(Instruction::BorrowField {
                                    value,
                                    base,
                                    name: inner_member.text.clone(),
                                });
                                lowered.push(value);
                            } else {
                                // The callee borrows its receiver, so an owned
                                // temporary receiver is released by the caller
                                // after the call.
                                let value = self.expr(object)?;
                                if let Some(ty) =
                                    self.semantics.expression_types.get(&object.span()).copied()
                                    && self.layouts.types[ty.0].needs_drop
                                {
                                    receiver_owned = Some((value, ty));
                                }
                                lowered.push(value);
                            }
                        }
                        member.span
                    }
                    Expr::Name(name) => name.span,
                    _ => callee.span(),
                };
                // An implicit receiver call (`Button("A")` inside a receiver
                // body) passes the current `self` as the leading argument.
                if self
                    .semantics
                    .implicit_receiver_calls
                    .contains(&target_span)
                    && let Some(local) = self.locals.get("self").copied()
                {
                    let value = self.value();
                    self.emit(Instruction::Borrow { value, local });
                    lowered.push(value);
                }
                let target = self
                    .call_instances
                    .get(span)
                    .copied()
                    .or(resolved_target)
                    .or_else(|| self.references.get(&target_span).copied())?;
                // Native (extern) callees borrow their arguments, so managed
                // arguments must stay owned by the caller.
                let is_extern = self.semantics.extern_symbols.contains(&target);
                let extern_parameters: Vec<TypeId> = self
                    .semantics
                    .function_signatures
                    .get(&target)
                    .map(|signature| signature.parameters.clone())
                    .unwrap_or_default();
                let mut owned: Vec<(ValueId, TypeId)> = Vec::new();
                let variadic_target =
                    self.semantics
                        .function_signatures
                        .get(&target)
                        .and_then(|signature| {
                            signature
                                .variadic
                                .map(|element| (signature.parameters.len(), element))
                        });
                for (index, argument) in arguments.iter().enumerate() {
                    if is_extern {
                        // A `resource(T)` argument to a `ptr(T)` native
                        // parameter is borrowed at the ABI boundary: the owner
                        // is not moved, retained, or released.
                        let pointer_parameter = extern_parameters.get(index).is_some_and(|ty| {
                            matches!(self.semantics.types[ty.0], Type::Pointer(_))
                        });
                        if argument.name.is_none()
                            && pointer_parameter
                            && self
                                .lower_resource_borrow(&argument.value, &mut lowered)
                                .is_some()
                        {
                            continue;
                        }
                        self.lower_builtin_value(&argument.value, &mut lowered, &mut owned)?;
                    } else if variadic_target.is_some_and(|(fixed, _)| index >= fixed) {
                        // Variadic arguments are collected after the loop.
                    } else {
                        lowered.push(self.expr(&argument.value)?);
                    }
                }
                if let Some((fixed, element)) = variadic_target {
                    let trailing = arguments.get(fixed.min(arguments.len())..).unwrap_or(&[]);
                    let (data, len) =
                        self.lower_variadic_arguments(trailing, element, &mut owned)?;
                    lowered.push(data);
                    lowered.push(len);
                }
                if !is_extern && variadic_target.is_none() {
                    let expected_parameters = self
                        .semantics
                        .function_signatures
                        .get(&target)
                        .map(|signature| signature.parameters.clone())
                        .unwrap_or_default();
                    let receiver_offset = lowered.len().saturating_sub(arguments.len());
                    for (index, argument) in arguments.iter().enumerate() {
                        let Some(expected) = expected_parameters.get(index).copied() else {
                            break;
                        };
                        let Some(actual) = self
                            .semantics
                            .expression_types
                            .get(&argument.value.span())
                            .copied()
                        else {
                            continue;
                        };
                        let position = index + receiver_offset;
                        if let Some(slot) = lowered.get_mut(position) {
                            *slot = self.coerce_interface(*slot, actual, expected);
                        }
                    }
                }
                if let Some(type_index) = self
                    .semantics
                    .types
                    .iter()
                    .position(|ty| matches!(ty, Type::Data(symbol) if *symbol == target))
                {
                    let ty = TypeId(type_index);
                    let value = self.value();
                    let mut fields: Vec<(String, ValueId)> = arguments
                        .iter()
                        .zip(lowered)
                        .enumerate()
                        .map(|(index, (argument, value))| {
                            (
                                argument
                                    .name
                                    .as_ref()
                                    .map_or_else(|| index.to_string(), |name| name.text.clone()),
                                value,
                            )
                        })
                        .collect();
                    // Fill omitted fields from their resolved defaults; the
                    // semantic layer already checked them against the field type.
                    let positional = arguments
                        .iter()
                        .filter(|argument| argument.name.is_none())
                        .count();
                    let mut pending: Vec<(String, vut_ast::Expr)> = Vec::new();
                    {
                        let provided: std::collections::HashSet<&str> =
                            fields.iter().map(|(name, _)| name.as_str()).collect();
                        if let Some(data_fields) = self.semantics.data_fields.get(&target)
                            && let Some(defaults) = self.field_defaults.get(&target)
                        {
                            for (index, field) in data_fields.iter().enumerate() {
                                if index < positional || provided.contains(field.name.as_str()) {
                                    continue;
                                }
                                if let Some(Some(default)) = defaults.get(index) {
                                    pending.push((field.name.clone(), default.clone()));
                                }
                            }
                        }
                    }
                    for (name, default) in pending {
                        if let Some(field_value) = self.expr(&default) {
                            fields.push((name, field_value));
                        }
                    }
                    self.emit(Instruction::Allocate { value, ty });
                    self.emit(Instruction::Construct { value, ty, fields });
                    return Some(value);
                }
                let result_type = self.semantics.expression_types.get(span).copied();
                // A Vut `async fn` call always yields a future handle, even when
                // its logical result is `void`; the caller must be able to await
                // (and therefore release) that handle.
                let is_async_call = self.semantics.async_symbols.contains(&target);
                let returns_value = is_async_call
                    || result_type
                        .is_none_or(|ty| !matches!(self.semantics.types[ty.0], Type::Void));
                let value = returns_value.then(|| self.value());
                if let Some(function) = self.semantics.builtin_functions.get(&target).copied() {
                    self.emit(Instruction::RuntimeCall {
                        value,
                        result_type,
                        function,
                        arguments: lowered.clone(),
                    });
                    for (argument, lowered) in arguments.iter().zip(lowered) {
                        if let Some(ty) = self
                            .semantics
                            .expression_types
                            .get(&argument.value.span())
                            .copied()
                            && self.layouts.types[ty.0].needs_drop
                        {
                            self.emit(Instruction::Release { value: lowered, ty });
                        }
                    }
                } else {
                    if self.semantics.async_symbols.contains(&target) {
                        self.emit(Instruction::StartFuture {
                            value,
                            result_type,
                            target,
                            arguments: lowered,
                        });
                    } else {
                        self.emit(Instruction::Call {
                            value,
                            result_type,
                            target,
                            arguments: lowered,
                        });
                    }
                    for (owned_value, owned_ty) in owned {
                        self.emit(Instruction::Release {
                            value: owned_value,
                            ty: owned_ty,
                        });
                    }
                    if let Some((receiver, ty)) = receiver_owned {
                        self.emit(Instruction::Release {
                            value: receiver,
                            ty,
                        });
                    }
                }
                value
            }
            Expr::Member {
                object,
                member,
                span,
            } => {
                if let Some(ty) = self.semantics.expression_types.get(span).copied()
                    && let Type::Enum(symbol) = self.semantics.types[ty.0]
                    && let Some(variant_index) =
                        self.semantics
                            .enum_variants
                            .get(&symbol)
                            .and_then(|variants| {
                                variants
                                    .iter()
                                    .position(|variant| variant.name == member.text)
                            })
                {
                    let value = self.value();
                    self.emit(Instruction::ConstructEnum {
                        value,
                        ty,
                        variant_index,
                        payload: Vec::new(),
                    });
                    return Some(value);
                }
                // `self.field` reads the borrowed receiver by reference: the
                // base must not be released, and the projected field becomes an
                // owned reference when its value escapes.
                // A field read borrows its base. When the base is a local (or
                // the receiver) it must stay owned by the caller: aggregate
                // fields project an address into the base's storage, so
                // releasing the base here would leave that address dangling.
                let local_base = match object.as_ref() {
                    Expr::Name(name) => self.locals.get(&name.text).copied(),
                    _ => None,
                };
                let base = if let Some(local) = local_base {
                    let value = self.value();
                    self.emit(Instruction::Borrow { value, local });
                    value
                } else {
                    let Some(base) = self.expr(object) else {
                        let value = self.value();
                        self.emit(Instruction::ConstInt {
                            value,
                            literal: member.text.bytes().fold(0_i64, |hash, byte| {
                                hash.wrapping_mul(31).wrapping_add(i64::from(byte))
                            }),
                        });
                        return Some(value);
                    };
                    base
                };
                let value = self.value();
                self.emit(Instruction::Field {
                    value,
                    base,
                    name: member.text.clone(),
                });
                let mut result = value;
                if let Some(field_ty) = self.semantics.expression_types.get(span).copied() {
                    let info = self.layouts.types[field_ty.0];
                    let field_duplication = info.ownership.duplication(info.is_copy);
                    if field_duplication == Duplication::Linear
                        || (info.contains_linear && field_duplication == Duplication::Structural)
                    {
                        self.diagnostics.push(Diagnostic::error(
                            codes::E8012,
                            "cannot read move-only field by value",
                            *span,
                            "native resource handles cannot be projected out of an aggregate in this version",
                        ));
                    } else if self.layouts.is_aggregate(field_ty) {
                        // An inline aggregate field is projected by address, but
                        // the produced value must be independent of the base, so
                        // it is copied into fresh storage that owns its managed
                        // fields.
                        let copy = self.value();
                        self.emit(Instruction::CopyAggregate {
                            value: copy,
                            source: value,
                            ty: field_ty,
                        });
                        result = copy;
                    } else if info.needs_drop {
                        self.emit(Instruction::Retain {
                            value,
                            ty: field_ty,
                        });
                    }
                }
                if local_base.is_none()
                    && let Some(base_ty) =
                        self.semantics.expression_types.get(&object.span()).copied()
                    && self.layouts.types[base_ty.0].needs_drop
                {
                    self.emit(Instruction::Release {
                        value: base,
                        ty: base_ty,
                    });
                }
                Some(result)
            }
            Expr::If(value) => self.lower_if_expression(value),
            Expr::Block(block) => Some(self.lower_block_expression(block)),
            Expr::Match { value, arms, span } => self.lower_match_expression(value, arms, *span),
            Expr::ResultOk {
                value: payload,
                span,
            }
            | Expr::ResultErr {
                value: payload,
                span,
            } => {
                let ty = *self.semantics.expression_types.get(span)?;
                let payload = self.expr(payload)?;
                let value = self.value();
                self.emit(Instruction::ConstructResult {
                    value,
                    ty,
                    ok: matches!(expr, Expr::ResultOk { .. }),
                    payload,
                });
                Some(value)
            }
            Expr::ResultPropagate { value, .. } => self.lower_result_propagate(value),
            Expr::Await { value, .. } => {
                let operand_ty = self.semantics.expression_types.get(&value.span()).copied();
                // A `vutcon(T)`, a native `future(T)`, and a direct `async fn`
                // call are all poll tasks awaited the same way.
                if let Some(operand_ty) = operand_ty
                    && matches!(
                        self.semantics.types[operand_ty.0],
                        Type::Vutcon(_) | Type::Future(_)
                    )
                {
                    let result_type = match self.semantics.types[operand_ty.0] {
                        Type::Vutcon(inner) | Type::Future(inner) => inner,
                        _ => operand_ty,
                    };
                    let handle = self.expr(value)?;
                    let result = self.value();
                    self.emit(Instruction::AwaitFuture {
                        value: result,
                        handle,
                        result_type,
                    });
                    return Some(result);
                }
                // A Vut `async fn` call is awaited directly: start its future,
                // then drive it to completion.
                if let Some(target) = self.semantics.call_targets.get(&value.span()).copied()
                    && self.semantics.async_symbols.contains(&target)
                    && let Some(result_type) = operand_ty
                {
                    let handle = self.expr(value)?;
                    let result = self.value();
                    self.emit(Instruction::AwaitFuture {
                        value: result,
                        handle,
                        result_type,
                    });
                    return Some(result);
                }
                self.expr(value)
            }
            Expr::Spawn { callable, span } => {
                let result_ty = *self.semantics.expression_types.get(span)?;
                let Type::Vutcon(inner) = self.semantics.types[result_ty.0] else {
                    return None;
                };
                let callable_symbol = match callable.as_ref() {
                    Expr::Lambda { span, .. } => self.lambda_symbols.get(span).copied(),
                    _ => None,
                };
                let start = self.expr(callable)?;
                let value = self.value();
                self.emit(Instruction::Spawn {
                    value,
                    start,
                    callable: callable_symbol?,
                    result_type: inner,
                });
                Some(value)
            }
            Expr::Group { value, .. } => self.expr(value),
            Expr::Error(_) => None,
        }
    }

    /// Lowers `values.len()` / `values.at(i)` for the current variadic parameter.
    fn lower_variadic_builtin(
        &mut self,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        function: BuiltinFunction,
    ) -> Option<ValueId> {
        let Expr::Member { object, .. } = callee else {
            return None;
        };
        let Expr::Name(name) = object.as_ref() else {
            return None;
        };
        let (element, data_local, len_local) = {
            let pair = self.variadic_param.as_ref()?;
            if pair.name != name.text {
                return None;
            }
            (pair.element, pair.data, pair.len)
        };
        if function == BuiltinFunction::VariadicLen {
            let value = self.value();
            self.emit(Instruction::Copy {
                value,
                local: len_local,
            });
            return Some(value);
        }
        let index = arguments
            .first()
            .and_then(|argument| self.expr(&argument.value))?;
        let data = self.value();
        self.emit(Instruction::Copy {
            value: data,
            local: data_local,
        });
        let len = self.value();
        self.emit(Instruction::Copy {
            value: len,
            local: len_local,
        });
        let value = self.value();
        self.emit(Instruction::VariadicAt {
            value,
            data,
            len,
            index,
            element,
        });
        // The view borrows its elements; an extracted managed element becomes an
        // owned reference.
        if self.layouts.types[element.0].needs_drop {
            self.emit(Instruction::Retain { value, ty: element });
        }
        Some(value)
    }

    /// Builds the `(data, len)` ABI pair for a variadic call: either a spread of
    /// a matching sequence, or a contiguous buffer of the trailing arguments.
    fn lower_variadic_arguments(
        &mut self,
        trailing: &[vut_ast::Argument],
        element: TypeId,
        owned: &mut Vec<(ValueId, TypeId)>,
    ) -> Option<(ValueId, ValueId)> {
        if trailing.len() == 1 && trailing[0].spread {
            let spread = &trailing[0].value;
            let actual = *self.semantics.expression_types.get(&spread.span())?;
            return match self.semantics.types[actual.0] {
                Type::List(_) => {
                    // A named list is borrowed for the duration of the call; a
                    // temporary list is released after it.
                    let list = if let Expr::Name(name) = spread
                        && let Some(local) = self.locals.get(&name.text).copied()
                    {
                        let value = self.value();
                        self.emit(Instruction::Borrow { value, local });
                        value
                    } else {
                        let value = self.expr(spread)?;
                        owned.push((value, actual));
                        value
                    };
                    let data = self.value();
                    self.emit(Instruction::RuntimeCall {
                        value: Some(data),
                        result_type: None,
                        function: BuiltinFunction::ListData,
                        arguments: vec![list],
                    });
                    let len = self.value();
                    self.emit(Instruction::RuntimeCall {
                        value: Some(len),
                        result_type: None,
                        function: BuiltinFunction::ListLen,
                        arguments: vec![list],
                    });
                    Some((data, len))
                }
                Type::Array(_, length) => {
                    let data = if let Expr::Name(name) = spread
                        && let Some(local) = self.locals.get(&name.text).copied()
                    {
                        let value = self.value();
                        self.emit(Instruction::Borrow { value, local });
                        value
                    } else {
                        self.expr(spread)?
                    };
                    let len = self.value();
                    self.emit(Instruction::ConstInt {
                        value: len,
                        literal: i64::try_from(length).ok()?,
                    });
                    Some((data, len))
                }
                Type::Variadic(_) => {
                    let Expr::Name(name) = spread else {
                        return None;
                    };
                    let (data_local, len_local) = {
                        let pair = self.variadic_param.as_ref()?;
                        if pair.name != name.text {
                            return None;
                        }
                        (pair.data, pair.len)
                    };
                    let data = self.value();
                    self.emit(Instruction::Copy {
                        value: data,
                        local: data_local,
                    });
                    let len = self.value();
                    self.emit(Instruction::Copy {
                        value: len,
                        local: len_local,
                    });
                    Some((data, len))
                }
                _ => None,
            };
        }
        let mut elements = Vec::new();
        for argument in trailing {
            // Variadic arguments are borrowed into the caller's buffer; managed
            // arguments stay owned by the caller and are released after the call.
            self.lower_builtin_value(&argument.value, &mut elements, owned)?;
        }
        let len = self.value();
        self.emit(Instruction::ConstInt {
            value: len,
            literal: i64::try_from(elements.len()).ok()?,
        });
        if elements.is_empty() {
            let data = self.value();
            self.emit(Instruction::ConstInt {
                value: data,
                literal: 0,
            });
            return Some((data, len));
        }
        let data = self.value();
        self.emit(Instruction::ConstructVariadicBuffer {
            value: data,
            element,
            elements,
        });
        Some((data, len))
    }
}
