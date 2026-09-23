//! Iterable/conditional loops and per-iteration cleanup.
use super::super::BinaryOp;
use super::{Builder, Expr, Instruction, LoopTarget, Terminator, Type, ValueId};

impl Builder<'_> {
    /// Lowers the value of a `for ... in` iterable. Managed collections are
    /// borrowed rather than moved: the iterator only reads the collection, so
    /// the local must remain owned and be destroyed by normal cleanup.
    fn lower_iterable(&mut self, iterable: &Expr) -> Option<ValueId> {
        if let Expr::Name(name) = iterable
            && let Some(local) = self.locals.get(&name.text).copied()
            && self.local_data[local.0].ty.is_some_and(|ty| {
                matches!(
                    self.semantics.types[ty.0],
                    Type::List(_) | Type::Array(_, _)
                )
            })
        {
            let value = self.value();
            self.emit(Instruction::Borrow { value, local });
            return Some(value);
        }
        // A collection field is read without taking ownership: the iterator
        // only reads it, so the owning aggregate must keep its reference. An
        // array field projects inline storage (its address); a list field loads
        // the handle; neither is retained here.
        if let Expr::Member { object, member, .. } = iterable
            && let Expr::Name(base_name) = object.as_ref()
            && let Some(base) = self.locals.get(&base_name.text).copied()
            && self
                .semantics
                .expression_types
                .get(&iterable.span())
                .is_some_and(|ty| {
                    matches!(
                        self.semantics.types[ty.0],
                        Type::List(_) | Type::Array(_, _)
                    )
                })
        {
            let base_value = self.value();
            self.emit(Instruction::Borrow {
                value: base_value,
                local: base,
            });
            let value = self.value();
            self.emit(Instruction::Field {
                value,
                base: base_value,
                name: member.text.clone(),
            });
            return Some(value);
        }
        self.expr(iterable)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "loop lowering keeps CFG construction and binding types together"
    )]
    pub(super) fn lower_for(&mut self, statement: &vut_ast::For) {
        let head = self.new_block();
        let body = self.new_block();
        let exit = self.new_block();
        let mut iteration_bindings = None;
        let mut owned_iterable: Option<(ValueId, vut_hir::TypeId)> = None;
        let mut custom_continue: Option<super::BlockId> = None;
        match &statement.kind {
            vut_ast::ForKind::Infinite => {
                self.terminate(Terminator::Jump(body));
            }
            vut_ast::ForKind::Conditional(condition) => {
                self.terminate(Terminator::Jump(head));
                self.switch_to(head);
                if let Some(value) = self.expr(condition) {
                    self.terminate(Terminator::Branch {
                        condition: value,
                        then_block: body,
                        else_block: exit,
                    });
                }
            }
            vut_ast::ForKind::Iterable {
                value: name,
                index: index_name,
                iterable,
            } => {
                let iterable_type = self
                    .semantics
                    .expression_types
                    .get(&iterable.span())
                    .copied();
                let range_element = iterable_type.and_then(|ty| match self.semantics.types[ty.0] {
                    Type::Range(element) => Some(element),
                    _ => None,
                });
                if let Some(element) = range_element {
                    // A range `start..end` / `start..=end` lowers to a counter
                    // loop; both bounds are evaluated exactly once, before the
                    // loop, and never re-evaluated per iteration.
                    let Expr::Binary {
                        left, op, right, ..
                    } = iterable
                    else {
                        return;
                    };
                    let inclusive = *op == BinaryOp::RangeInclusive;
                    let span = iterable.span();
                    let (Some(start), Some(limit)) = (self.expr(left), self.expr(right)) else {
                        return;
                    };
                    let counter = self.add_local("$range_counter".into(), None, span);
                    self.local_data[counter.0].ty = Some(element);
                    self.initialized.insert(counter);
                    self.emit(Instruction::Store {
                        local: counter,
                        value: start,
                    });
                    let bound = self.add_local("$range_limit".into(), None, span);
                    self.local_data[bound.0].ty = Some(element);
                    self.initialized.insert(bound);
                    self.emit(Instruction::Store {
                        local: bound,
                        value: limit,
                    });

                    self.terminate(Terminator::Jump(head));
                    self.switch_to(head);
                    let current = self.value();
                    self.emit(Instruction::Copy {
                        value: current,
                        local: counter,
                    });
                    let end = self.value();
                    self.emit(Instruction::Copy {
                        value: end,
                        local: bound,
                    });
                    let condition = self.value();
                    self.emit(Instruction::Binary {
                        operand_type: Some(element),
                        value: condition,
                        op: if inclusive {
                            BinaryOp::LessEqual
                        } else {
                            BinaryOp::Less
                        },
                        left: current,
                        right: end,
                    });
                    self.terminate(Terminator::Branch {
                        condition,
                        then_block: body,
                        else_block: exit,
                    });

                    // `continue` increments the counter before re-testing.
                    let step = self.new_block();
                    self.switch_to(step);
                    let value = self.value();
                    self.emit(Instruction::Copy {
                        value,
                        local: counter,
                    });
                    let one = self.value();
                    self.emit(Instruction::ConstInt {
                        value: one,
                        literal: 1,
                    });
                    let next = self.value();
                    self.emit(Instruction::Binary {
                        operand_type: Some(element),
                        value: next,
                        op: BinaryOp::Add,
                        left: value,
                        right: one,
                    });
                    self.emit(Instruction::Store {
                        local: counter,
                        value: next,
                    });
                    self.terminate(Terminator::Jump(head));

                    iteration_bindings = Some((
                        name.clone(),
                        index_name.clone(),
                        current,
                        current,
                        Some(element),
                    ));
                    custom_continue = Some(step);
                } else {
                    let element_type =
                        iterable_type.and_then(|ty| match self.semantics.types[ty.0] {
                            Type::Array(element, _)
                            | Type::List(element)
                            | Type::Variadic(element) => Some(element),
                            _ => None,
                        });
                    let array = iterable_type.and_then(|ty| match self.semantics.types[ty.0] {
                        Type::Array(element, length) => {
                            Some((length, self.layouts.element_storage_size(element)))
                        }
                        _ => None,
                    });
                    let element_stride =
                        element_type.map(|element| self.layouts.element_storage_size(element));
                    let variadic = iterable_type.and_then(|ty| match self.semantics.types[ty.0] {
                        Type::Variadic(element) => Some(element),
                        _ => None,
                    });
                    // `(data, literal length, runtime length, stride)`.
                    let mut source = None;
                    if let Some(element) = variadic {
                        let pair = self.variadic_param.as_ref().and_then(|pair| {
                            matches!(iterable, Expr::Name(name) if name.text == pair.name)
                                .then_some((pair.data, pair.len))
                        });
                        if let Some((data_local, len_local)) = pair {
                            let data_value = self.value();
                            self.emit(Instruction::Copy {
                                value: data_value,
                                local: data_local,
                            });
                            let len_value = self.value();
                            self.emit(Instruction::Copy {
                                value: len_value,
                                local: len_local,
                            });
                            source = Some((
                                data_value,
                                None,
                                Some(len_value),
                                Some(self.layouts.element_storage_size(element)),
                            ));
                        }
                    } else if let Some(iterable_value) = self.lower_iterable(iterable) {
                        // A local or field-projected collection stays owned by its
                        // owner; any other collection is a temporary this loop must
                        // release after the exit edge.
                        let borrowed = matches!(iterable, Expr::Name(_))
                            || matches!(iterable, Expr::Member { object, .. }
                            if matches!(object.as_ref(), Expr::Name(_)));
                        if !borrowed
                            && let Some(ty) = iterable_type.filter(|ty| {
                                matches!(
                                    self.semantics.types[ty.0],
                                    Type::List(_) | Type::Array(_, _)
                                )
                            })
                        {
                            owned_iterable = Some((iterable_value, ty));
                        }
                        source = Some((
                            iterable_value,
                            array.map(|value| value.0),
                            None,
                            array.map(|value| value.1).or(element_stride),
                        ));
                    }
                    if let Some((iterable_value, length, length_value, stride)) = source
                        && let Some(element_type_id) = element_type
                    {
                        let iterator = self.value();
                        self.emit(Instruction::IteratorInit {
                            iterator,
                            iterable: iterable_value,
                            length,
                            length_value,
                            stride,
                            slot: None,
                        });
                        self.terminate(Terminator::Jump(head));
                        self.switch_to(head);
                        let has_value = self.value();
                        let value = self.value();
                        let index = self.value();
                        self.emit(Instruction::IteratorNext {
                            has_value,
                            value,
                            index,
                            iterator,
                            element_type: element_type_id,
                        });
                        iteration_bindings =
                            Some((name.clone(), index_name.clone(), value, index, element_type));
                        self.terminate(Terminator::Branch {
                            condition: has_value,
                            then_block: body,
                            else_block: exit,
                        });
                    }
                }
            }
        }
        self.switch_to(body);
        self.conditional_depth += 1;
        // The loop bindings live inside the per-iteration cleanup scope so a
        // managed element is released at the end of each iteration.
        let mark = self.local_data.len();
        if let Some((name, index_name, value, index, element_type)) = iteration_bindings {
            let local = self.add_local(name.text, None, name.span);
            self.local_data[local.0].ty = element_type;
            self.emit(Instruction::Store { local, value });
            // The iterator yields borrowed elements; retain managed elements so
            // the binding owns a reference and using it by value is safe.
            if let Some(ty) = element_type
                && self.layouts.types[ty.0].needs_drop
            {
                self.emit(Instruction::Retain { value, ty });
            }
            if let Some(name) = index_name {
                let local = self.add_local(name.text, None, name.span);
                self.local_data[local.0].ty = self
                    .semantics
                    .types
                    .iter()
                    .position(|ty| matches!(ty, Type::Int))
                    .map(vut_hir::TypeId);
                self.emit(Instruction::Store {
                    local,
                    value: index,
                });
            }
        }
        let continue_target =
            custom_continue.unwrap_or(if matches!(statement.kind, vut_ast::ForKind::Infinite) {
                body
            } else {
                head
            });
        self.loop_targets.push(LoopTarget {
            continue_target,
            exit,
            mark,
            iterable: owned_iterable,
        });
        self.lower_statements(&statement.body.statements);
        self.loop_targets.pop();
        if self.is_terminated() {
            self.forget_scope(mark);
        } else {
            self.close_scope(mark);
            self.terminate(Terminator::Jump(continue_target));
        }
        self.conditional_depth -= 1;
        self.switch_to(exit);
        if let Some((value, ty)) = owned_iterable {
            self.emit(Instruction::Release { value, ty });
        }
    }
}
