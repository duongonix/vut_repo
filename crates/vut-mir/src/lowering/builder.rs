//! Expression/statement lowering builder.
use std::collections::{HashMap, HashSet};

use vut_ast::{Expr, Stmt};
use vut_hir::TypeId;
use vut_memory::LastUse;
use vut_resolver::SymbolId;
use vut_source::Span;
use vut_types::{SemanticResult, Type};

use super::{
    BasicBlock, BlockId, Instruction, LayoutTable, Local, LocalId, LocalStorage, Terminator,
    ValueId,
};

mod collections;
mod control;
mod expression;
mod interface;

/// The current function's variadic parameter: its source name and the two MIR
/// locals holding the argument buffer pointer and element count.
pub(super) struct VariadicParam {
    pub(super) name: String,
    pub(super) element: TypeId,
    pub(super) data: LocalId,
    pub(super) len: LocalId,
}

/// An active loop's targets plus any collection it owns (a `for` over a
/// temporary or a projected field), which must be released when control leaves
/// the loop through an early `return`.
#[derive(Clone, Copy)]
pub(super) struct LoopTarget {
    pub(super) continue_target: BlockId,
    pub(super) exit: BlockId,
    pub(super) mark: usize,
    pub(super) iterable: Option<(ValueId, vut_hir::TypeId)>,
}

pub(super) struct Builder<'a> {
    pub(super) semantics: &'a SemanticResult,
    pub(super) layouts: &'a LayoutTable,
    pub(super) locals: HashMap<String, LocalId>,
    pub(super) local_data: Vec<Local>,
    pub(super) blocks: Vec<BasicBlock>,
    pub(super) current: BlockId,
    pub(super) next_value: usize,
    pub(super) remaining_uses: LastUse,
    pub(super) moved: HashSet<LocalId>,
    pub(super) initialized: HashSet<LocalId>,
    pub(super) references: &'a HashMap<Span, SymbolId>,
    pub(super) lambda_symbols: &'a HashMap<Span, SymbolId>,
    /// Default field expressions per data symbol (resolved and type-checked by
    /// the semantic layer), aligned with the data type's field order.
    pub(super) field_defaults: &'a HashMap<SymbolId, Vec<Option<vut_ast::Expr>>>,
    /// Call-site retargeting for monomorphized generic specializations.
    pub(super) call_instances: &'a HashMap<Span, SymbolId>,
    pub(super) loop_targets: Vec<LoopTarget>,
    pub(super) terminated: HashSet<BlockId>,
    pub(super) return_type: Option<TypeId>,
    /// Nesting depth of conditional branches. Values referenced inside a
    /// branch are never moved (they are retained and dropped at cleanup) so
    /// that path-insensitive move tracking cannot leak or double-free.
    pub(super) conditional_depth: usize,
    /// The receiver local (`self`) of the current function, if any. Receivers
    /// are borrowed by callers, so their bindings must not be released by the
    /// callee's cleanup.
    pub(super) receiver_local: Option<LocalId>,
    /// Whether the pattern currently being lowered binds payloads of a
    /// borrowed scrutinee; such bindings are borrows and are not dropped.
    pub(super) pattern_borrowed: bool,
    /// The current function's variadic parameter, if any.
    pub(super) variadic_param: Option<VariadicParam>,
    /// Ownership diagnostics discovered while lowering this function.
    pub(super) diagnostics: vut_diagnostics::DiagnosticSink,
}
impl Builder<'_> {
    fn emit(&mut self, instruction: Instruction) {
        self.blocks[self.current.0].instructions.push(instruction);
    }
    pub(super) fn terminate(&mut self, terminator: Terminator) {
        self.blocks[self.current.0].terminator = terminator;
        self.terminated.insert(self.current);
    }
    pub(super) fn is_terminated(&self) -> bool {
        self.terminated.contains(&self.current)
    }
    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(BasicBlock {
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
        });
        id
    }
    fn switch_to(&mut self, block: BlockId) {
        self.current = block;
    }
    pub(super) fn lower_statements(&mut self, statements: &[Stmt]) -> Option<ValueId> {
        let mut value = None;
        for statement in statements {
            if self.is_terminated() {
                break;
            }
            value = self.statement(statement).or(value);
        }
        value
    }
    fn value(&mut self) -> ValueId {
        let value = ValueId(self.next_value);
        self.next_value += 1;
        value
    }
    pub(super) fn add_local(
        &mut self,
        name: String,
        type_span: Option<Span>,
        span: Span,
    ) -> LocalId {
        let id = LocalId(self.local_data.len());
        let ty = type_span.and_then(|span| self.semantics.expression_types.get(&span).copied());
        self.local_data.push(Local {
            name: name.clone(),
            ty,
            span,
            storage: LocalStorage::Stack,
        });
        self.locals.insert(name, id);
        id
    }

    /// Adds a synthesized local for a loop-carried value. Unlike a user
    /// binding it is not entered into the name map, so repeated lowering of the
    /// same builtin cannot collide on a name.
    pub(super) fn add_temp_local(&mut self, ty: TypeId, span: Span) -> LocalId {
        let id = LocalId(self.local_data.len());
        self.local_data.push(Local {
            name: String::new(),
            ty: Some(ty),
            span,
            storage: LocalStorage::Stack,
        });
        id
    }
    #[expect(
        clippy::too_many_lines,
        reason = "one exhaustive statement visitor keeps binding and control-flow cleanup together"
    )]
    fn statement(&mut self, statement: &Stmt) -> Option<ValueId> {
        match statement {
            Stmt::Binding {
                target: Expr::Name(name),
                value,
                ..
            } => {
                let value_ty = self.semantics.expression_types.get(&value.span()).copied();
                let declared = self.semantics.expression_types.get(&name.span).copied();
                let local = self
                    .locals
                    .get(&name.text)
                    .copied()
                    .unwrap_or_else(|| self.add_local(name.text.clone(), None, name.span));
                if self.local_data[local.0].ty.is_none() {
                    self.local_data[local.0].ty = declared.or(value_ty);
                }
                let target_ty = self.local_data[local.0].ty;
                let raw_value = self.expr(value)?;
                let value_id = self.coerce_optional(raw_value, value_ty, target_ty);
                if self.initialized.contains(&local)
                    && !self.moved.contains(&local)
                    && self.local_data[local.0]
                        .ty
                        .is_some_and(|ty| self.layouts.types[ty.0].needs_drop)
                {
                    self.emit(Instruction::Drop(local));
                }
                self.moved.remove(&local);
                self.initialized.insert(local);
                self.emit(Instruction::Store {
                    local,
                    value: value_id,
                });
                Some(value_id)
            }
            Stmt::Binding {
                target: Expr::Member { object, member, .. },
                value,
                ..
            } => {
                // Assigning to a field of an aggregate held in a local (the
                // receiver or another local) releases the previous value and
                // stores the new one.
                let value_id = self.expr(value)?;
                if let Expr::Name(base_name) = object.as_ref()
                    && let Some(base) = self.locals.get(&base_name.text).copied()
                {
                    self.emit(Instruction::FieldStore {
                        base,
                        name: member.text.clone(),
                        value: value_id,
                    });
                }
                Some(value_id)
            }
            Stmt::Expression(expr) => self.expr(expr),
            Stmt::Return {
                value: expression, ..
            } => {
                let mut value = expression.as_ref().and_then(|expr| self.expr(expr));
                if let (Some(current), Some(expression), Some(expected)) =
                    (value, expression.as_ref(), self.return_type)
                    && let Some(actual) = self
                        .semantics
                        .expression_types
                        .get(&expression.span())
                        .copied()
                {
                    let current = self.coerce_interface(current, actual, expected);
                    if let Type::Optional(inner) = self.semantics.types[expected.0]
                        && !matches!(
                            self.semantics.types[inner.0],
                            Type::Str
                                | Type::Bytes
                                | Type::List(_)
                                | Type::Map(_, _)
                                | Type::Vutcon(_)
                                | Type::Future(_)
                                | Type::Resource(_)
                                | Type::Interface(_)
                                | Type::Dyn
                        )
                    {
                        // Scalar optionals are currently boxed in the callee's
                        // frame; returning one would dangle. Reject it cleanly
                        // until the indirect/by-value ABI lands.
                        self.diagnostics.push(vut_diagnostics::Diagnostic::error(
                            vut_diagnostics::codes::E1007,
                            "unsupported optional return",
                            expression.span(),
                            "returning a scalar optional across functions is not yet supported",
                        ));
                    }
                    value = Some(self.coerce_optional(current, Some(actual), Some(expected)));
                }
                // Exit every active loop without running its normal exit edge,
                // releasing any collection the loop owns (a `for` over a
                // temporary or a projected field).
                let iterables: Vec<(ValueId, vut_hir::TypeId)> = self
                    .loop_targets
                    .iter()
                    .rev()
                    .filter_map(|target| target.iterable)
                    .collect();
                for (iterable, ty) in iterables {
                    self.emit(Instruction::Release {
                        value: iterable,
                        ty,
                    });
                }
                self.cleanup_except(value);
                self.terminate(Terminator::Return(value));
                value
            }
            Stmt::If(value) => {
                self.lower_if(value);
                None
            }
            Stmt::For(value) => {
                self.lower_for(value);
                None
            }
            Stmt::Unsafe(body) => self.lower_statements(&body.statements),
            Stmt::Break(_) => {
                if let Some(target) = self.loop_targets.last().copied() {
                    self.cleanup_from(target.mark);
                    self.terminate(Terminator::Jump(target.exit));
                }
                None
            }
            Stmt::Continue(_) => {
                if let Some(target) = self.loop_targets.last().copied() {
                    self.cleanup_from(target.mark);
                    self.terminate(Terminator::Jump(target.continue_target));
                }
                None
            }
            Stmt::Error(_) => {
                self.cleanup_from(0);
                self.terminate(Terminator::Unreachable);
                None
            }
            Stmt::Binding { .. } => None,
        }
    }
    fn lower_if(&mut self, statement: &vut_ast::If) {
        let Some(condition) = self.expr(&statement.condition) else {
            return;
        };
        let then_block = self.new_block();
        let else_block = self.new_block();
        let join = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block,
            else_block,
        });
        self.switch_to(then_block);
        self.conditional_depth += 1;
        let then_mark = self.local_data.len();
        self.lower_statements(&statement.body.statements);
        if !self.is_terminated() {
            self.cleanup_from(then_mark);
            self.terminate(Terminator::Jump(join));
        }
        self.switch_to(else_block);
        if let Some((condition, body)) = statement.elifs.first() {
            let nested = vut_ast::If {
                condition: condition.clone(),
                body: body.clone(),
                elifs: statement.elifs[1..].to_vec(),
                otherwise: statement.otherwise.clone(),
                span: statement.span,
            };
            self.lower_if(&nested);
        } else if let Some(body) = &statement.otherwise {
            let else_mark = self.local_data.len();
            self.lower_statements(&body.statements);
            if !self.is_terminated() {
                self.cleanup_from(else_mark);
            }
        }
        self.conditional_depth -= 1;
        if !self.is_terminated() {
            self.terminate(Terminator::Jump(join));
        }
        self.switch_to(join);
    }
    /// Lowers a builtin-call receiver as a borrow. Named locals and field
    /// projections of a local must not be moved or copied: a mutating builtin
    /// (`set`, `insert`, `push`) acts on the caller's storage, and an aggregate
    /// field receiver must be its address, not an owned copy. Only a true
    /// temporary receiver is owned and released after the call.
    fn lower_builtin_receiver(
        &mut self,
        expression: &Expr,
        lowered: &mut Vec<ValueId>,
        owned: &mut Vec<(ValueId, TypeId)>,
    ) -> Option<()> {
        match expression {
            Expr::Name(name) => {
                if let Some(local) = self.locals.get(&name.text).copied() {
                    self.remaining_uses.consume(&name.text);
                    let value = self.value();
                    self.emit(Instruction::Borrow { value, local });
                    lowered.push(value);
                    return Some(());
                }
            }
            Expr::Member { object, member, .. } => {
                if let Expr::Name(base_name) = object.as_ref()
                    && let Some(base) = self.locals.get(&base_name.text).copied()
                {
                    self.remaining_uses.consume(&base_name.text);
                    // Mirror field access: an aggregate field yields its
                    // address (mutated in place); a managed handle field is
                    // loaded. The base stays owned either way.
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
                    lowered.push(value);
                    return Some(());
                }
            }
            _ => {}
        }
        self.lower_builtin_value(expression, lowered, owned)
    }
    /// Lowers one builtin-call operand. Named locals are borrowed so the
    /// runtime can retain what it stores and read-only operands stay owned by
    /// the caller; only temporaries are recorded for release after the call.
    fn lower_builtin_value(
        &mut self,
        expression: &Expr,
        lowered: &mut Vec<ValueId>,
        owned: &mut Vec<(ValueId, TypeId)>,
    ) -> Option<()> {
        if let Expr::Name(name) = expression
            && let Some(local) = self.locals.get(&name.text).copied()
        {
            self.remaining_uses.consume(&name.text);
            let value = self.value();
            self.emit(Instruction::Borrow { value, local });
            lowered.push(value);
            return Some(());
        }
        let ty = self
            .semantics
            .expression_types
            .get(&expression.span())
            .copied();
        let value = self.expr(expression)?;
        lowered.push(value);
        if let Some(ty) = ty
            && self.layouts.types[ty.0].needs_drop
        {
            owned.push((value, ty));
        }
        Some(())
    }
    /// Lowers a `resource(T)` used where only a borrow is required (an `extern`
    /// `ptr(T)` parameter). The handle value is passed without moving,
    /// retaining, or releasing it. Supports resource locals and resource fields
    /// of a local aggregate (`self.handle`).
    fn lower_resource_borrow(
        &mut self,
        expression: &Expr,
        lowered: &mut Vec<ValueId>,
    ) -> Option<()> {
        match expression {
            Expr::Name(name) => {
                let local = self.locals.get(&name.text).copied()?;
                if !self.local_data[local.0]
                    .ty
                    .is_some_and(|ty| matches!(self.semantics.types[ty.0], Type::Resource(_)))
                {
                    return None;
                }
                self.remaining_uses.consume(&name.text);
                let handle = self.value();
                self.emit(Instruction::Borrow {
                    value: handle,
                    local,
                });
                let value = self.value();
                self.emit(Instruction::ResourceDeref { value, handle });
                lowered.push(value);
                Some(())
            }
            Expr::Member {
                object,
                member,
                span,
            } => {
                let Expr::Name(base_name) = object.as_ref() else {
                    return None;
                };
                let base = self.locals.get(&base_name.text).copied()?;
                let field_ty = self.semantics.expression_types.get(span).copied()?;
                if !matches!(self.semantics.types[field_ty.0], Type::Resource(_)) {
                    return None;
                }
                self.remaining_uses.consume(&base_name.text);
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
                let handle = value;
                let value = self.value();
                self.emit(Instruction::ResourceDeref { value, handle });
                lowered.push(value);
                Some(())
            }
            _ => None,
        }
    }
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
    fn lower_for(&mut self, statement: &vut_ast::For) {
        let head = self.new_block();
        let body = self.new_block();
        let exit = self.new_block();
        let mut iteration_bindings = None;
        let mut owned_iterable: Option<(ValueId, vut_hir::TypeId)> = None;
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
                let element_type = iterable_type.and_then(|ty| match self.semantics.types[ty.0] {
                    Type::Array(element, _) | Type::List(element) | Type::Variadic(element) => {
                        Some(element)
                    }
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
                if let Some((iterable_value, length, length_value, stride)) = source {
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
                        element_type: element_type.expect("checked iterable has an element type"),
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
        let continue_target = if matches!(statement.kind, vut_ast::ForKind::Infinite) {
            body
        } else {
            head
        };
        self.loop_targets.push(LoopTarget {
            continue_target,
            exit,
            mark,
            iterable: owned_iterable,
        });
        self.lower_statements(&statement.body.statements);
        self.loop_targets.pop();
        if !self.is_terminated() {
            self.close_scope(mark);
            self.terminate(Terminator::Jump(continue_target));
        }
        self.conditional_depth -= 1;
        self.switch_to(exit);
        if let Some((value, ty)) = owned_iterable {
            self.emit(Instruction::Release { value, ty });
        }
    }
    /// Wraps `value` into an optional when `expected` is `T?` and `actual` is
    /// `T`. Managed handles wrap by identity; scalars are boxed.
    pub(super) fn coerce_optional(
        &mut self,
        value: ValueId,
        actual: Option<TypeId>,
        expected: Option<TypeId>,
    ) -> ValueId {
        let (Some(actual), Some(expected)) = (actual, expected) else {
            return value;
        };
        let Type::Optional(inner) = self.semantics.types[expected.0] else {
            return value;
        };
        if actual != inner {
            return value;
        }
        let wrapped = self.value();
        self.emit(Instruction::OptionalWrap {
            value: wrapped,
            operand: value,
            ty: expected,
            inner,
        });
        wrapped
    }
    pub(super) fn cleanup_except(&mut self, returned: Option<ValueId>) {
        let _ = returned;
        self.cleanup_from(0);
    }
    fn cleanup_from(&mut self, start: usize) {
        let drops: Vec<_> = self
            .local_data
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .filter_map(|(index, local)| {
                let id = LocalId(index);
                // A receiver is borrowed by the caller and must never be
                // released by the callee's cleanup.
                if self.receiver_local == Some(id) {
                    return None;
                }
                (!self.moved.contains(&id)
                    && local
                        .ty
                        .is_some_and(|ty| self.layouts.types[ty.0].needs_drop))
                .then_some(id)
            })
            .collect();
        for local in drops {
            self.emit(Instruction::Drop(local));
        }
    }
    /// Drops the locals declared at or after `start` on the current path and
    /// marks them consumed. Used to close a scoped arm/branch so its bindings
    /// cannot be dropped again by an outer cleanup (path-insensitive move
    /// tracking otherwise treats them as still live).
    pub(super) fn close_scope(&mut self, start: usize) {
        self.cleanup_from(start);
        for index in start..self.local_data.len() {
            self.moved.insert(LocalId(index));
        }
    }
}
