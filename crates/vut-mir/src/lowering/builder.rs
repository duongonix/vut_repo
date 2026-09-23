//! Expression/statement lowering builder.
use std::collections::{HashMap, HashSet};

use vut_ast::{Expr, Stmt};
use vut_hir::TypeId;
use vut_memory::LastUse;
use vut_resolver::SymbolId;
use vut_source::Span;
use vut_types::{SemanticResult, SubscriptKind, Type};

use super::{
    BasicBlock, BlockId, BuiltinFunction, ClosureLayout, Instruction, LayoutTable, Local, LocalId,
    LocalStorage, Terminator, ValueId,
};

mod branches;
mod cleanup;
mod coercion;
mod collections;
mod control;
mod expression;
mod interface;
mod loops;

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
    /// Default parameter expressions per function/method symbol (resolved and
    /// type-checked by the semantic layer), aligned with the callee's resolved
    /// parameter order. A call that omits a trailing defaulted parameter
    /// materializes the default at the call site.
    pub(super) parameter_defaults: &'a HashMap<SymbolId, Vec<Option<vut_ast::Expr>>>,
    /// Declared parameter names per function/method symbol, used to map named
    /// call arguments to their declared position.
    pub(super) parameter_names: &'a HashMap<SymbolId, Vec<String>>,
    /// Maps a concrete specialization symbol to its generic template, whose
    /// parameter names and defaults describe the call.
    pub(super) instance_templates: &'a HashMap<SymbolId, SymbolId>,
    /// Call-site retargeting for monomorphized generic specializations.
    pub(super) call_instances: &'a HashMap<Span, SymbolId>,
    pub(super) loop_targets: Vec<LoopTarget>,
    pub(super) terminated: HashSet<BlockId>,
    pub(super) return_type: Option<TypeId>,
    /// The type of the most recent expression statement, used to coerce an
    /// implicit return of an optional function result.
    pub(super) tail_type: Option<TypeId>,
    /// Names currently narrowed to their optional's present (inner) type by an
    /// enclosing `if value != null` / `if value == null` guard or branch.
    pub(super) narrowed: Vec<(String, TypeId)>,
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
    /// Closure capture locals. They alias a value owned by the closure
    /// environment, so reads must retain a copy and cleanup must not release
    /// them.
    pub(super) borrowed_locals: HashSet<LocalId>,
    /// The current function's variadic parameter, if any.
    pub(super) variadic_param: Option<VariadicParam>,
    /// Write-backs queued by a mutating subscript receiver (e.g. `a[0].push`).
    /// Applied after the builtin call so the mutation reaches the parent.
    pub(super) pending_writeback: Vec<WriteBack>,
    /// Ownership diagnostics discovered while lowering this function.
    pub(super) diagnostics: vut_diagnostics::DiagnosticSink,
    /// Recursion depth while inlining a module-level constant reference.
    pub(super) constant_depth: usize,
}

/// A queued write of a mutated collection element back into its parent.
pub(super) struct WriteBack {
    pub(super) function: BuiltinFunction,
    pub(super) parent: ValueId,
    pub(super) index: ValueId,
    pub(super) element: ValueId,
    pub(super) element_ty: TypeId,
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
        let narrowed_mark = self.narrowed.len();
        let mut value = None;
        for (index, statement) in statements.iter().enumerate() {
            if self.is_terminated() {
                break;
            }
            let mut produced = self.statement(statement);
            if index + 1 < statements.len()
                && let Stmt::Expression(expr) = statement
                && let Some(current) = produced
                && let Some(ty) = self.semantics.expression_types.get(&expr.span()).copied()
            {
                self.drop_ignored(current, ty, expr.span());
                produced = None;
            }
            if produced.is_some()
                && let Stmt::Expression(expr) = statement
            {
                self.tail_type = self.semantics.expression_types.get(&expr.span()).copied();
            }
            value = produced.or(value);
        }
        self.narrowed.truncate(narrowed_mark);
        value
    }
    /// Returns true when a guard clause has narrowed `name` to its present type
    /// in the current scope.
    pub(super) fn is_narrowed(&self, name: &str) -> bool {
        self.narrowed.iter().any(|(narrowed, _)| narrowed == name)
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
    /// Adds one local per capture of a capturing closure body and loads each
    /// capture from the hidden environment parameter (always local 0). Called
    /// after the body's ordinary parameters so local indices stay aligned with
    /// the ABI parameter list.
    pub(super) fn setup_closure_captures(
        &mut self,
        layout: &ClosureLayout,
        names: &[(String, TypeId)],
        span: Span,
    ) {
        let env_value = self.value();
        self.emit(Instruction::Copy {
            value: env_value,
            local: LocalId(0),
        });
        for (index, capture) in layout.captures.iter().enumerate() {
            let capture_value = self.value();
            self.emit(Instruction::LoadRaw {
                value: capture_value,
                pointer: env_value,
                offset: i64::try_from(capture.offset).unwrap_or(0),
                ty: capture.ty,
            });
            let name = names
                .get(index)
                .map_or_else(|| format!("$capture{index}"), |(name, _)| name.clone());
            let capture_local = self.add_local(name, None, span);
            self.initialized.insert(capture_local);
            self.local_data[capture_local.0].ty = Some(capture.ty);
            if self.layouts.types[capture.ty.0].needs_drop {
                // The environment owns the value; the local only aliases it.
                self.borrowed_locals.insert(capture_local);
            }
            self.emit(Instruction::Store {
                local: capture_local,
                value: capture_value,
            });
        }
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
                let value_id = self.coerce_value(raw_value, value_ty, target_ty);
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
                        release: true,
                    });
                }
                Some(value_id)
            }
            Stmt::Binding {
                target:
                    Expr::Subscript {
                        object,
                        index,
                        span,
                        ..
                    },
                value,
                ..
            } => {
                if self.semantics.subscripts.get(span) == Some(&SubscriptKind::Generic) {
                    return None;
                }
                let index = index.as_deref()?;
                let element_ty = *self.semantics.expression_types.get(span)?;
                let actual = self.semantics.expression_types.get(&value.span()).copied();
                let raw = self.expr(value)?;
                let stored = self.coerce_value(raw, actual, Some(element_ty));
                self.lower_index_store(object, index, stored)?;
                if self.layouts.types[element_ty.0].needs_drop {
                    self.emit(Instruction::Release {
                        value: stored,
                        ty: element_ty,
                    });
                }
                Some(stored)
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
                    value = Some(self.coerce_value(current, Some(actual), Some(expected)));
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

    /// Stores through an index and recursively writes modified aggregate
    /// elements back through their parent index. This preserves value
    /// semantics for chains such as `matrix[0][1] = value`.
    fn lower_index_store(&mut self, object: &Expr, index: &Expr, stored: ValueId) -> Option<()> {
        let collection_ty = *self.semantics.expression_types.get(&object.span())?;
        let function = match self.semantics.types[collection_ty.0] {
            Type::List(_) => BuiltinFunction::ListSet,
            Type::Array(_, _) => BuiltinFunction::ArraySet,
            Type::Map(_, _) => BuiltinFunction::MapSet,
            _ => return None,
        };
        let mut arguments = Vec::new();
        let mut owned = Vec::new();
        let nested = matches!(object, Expr::Subscript { .. });
        if nested {
            let collection = self.expr(object)?;
            arguments.push(collection);
            owned.push((collection, collection_ty));
        } else {
            self.lower_builtin_receiver(object, true, &mut arguments, &mut owned)?;
        }
        self.lower_builtin_value(index, &mut arguments, &mut owned)?;
        arguments.push(stored);
        let receiver = arguments[0];
        self.emit(Instruction::RuntimeCall {
            value: None,
            result_type: None,
            function,
            arguments,
        });
        if let Expr::Subscript {
            object: parent,
            index: parent_index,
            ..
        } = object
        {
            let parent_index = parent_index.as_deref()?;
            self.lower_index_store(parent, parent_index, receiver)?;
        }
        for (owned_value, owned_ty) in owned {
            self.emit(Instruction::Release {
                value: owned_value,
                ty: owned_ty,
            });
        }
        Some(())
    }
    /// Lowers a builtin-call receiver as a borrow. Named locals and field
    /// projections of a local must not be moved or copied: a mutating builtin
    /// (`set`, `insert`, `push`) acts on the caller's storage, and an aggregate
    /// field receiver must be its address, not an owned copy. Only a true
    /// temporary receiver is owned and released after the call.
    ///
    /// When `mutating` is set and the receiver is a managed collection, the
    /// handle is made unique first (copy-on-write) and written back, so a
    /// mutation after a copy detaches from the shared storage.
    #[expect(
        clippy::too_many_lines,
        reason = "receiver lowering enumerates local, field, and subscript forms"
    )]
    fn lower_builtin_receiver(
        &mut self,
        expression: &Expr,
        mutating: bool,
        lowered: &mut Vec<ValueId>,
        owned: &mut Vec<(ValueId, TypeId)>,
    ) -> Option<()> {
        match expression {
            Expr::Name(name) => {
                if let Some(local) = self.locals.get(&name.text).copied() {
                    self.remaining_uses.consume(&name.text);
                    let receiver_ty = self
                        .semantics
                        .expression_types
                        .get(&expression.span())
                        .copied();
                    if mutating
                        && !self.is_narrowed(&name.text)
                        && self.receiver_local != Some(local)
                        && self.is_cow_collection(receiver_ty)
                    {
                        let receiver_ty = receiver_ty?;
                        let handle = self.value();
                        self.emit(Instruction::Copy {
                            value: handle,
                            local,
                        });
                        // A capture local borrows the environment's reference;
                        // retain it so the local can own the detached copy.
                        if self.borrowed_locals.contains(&local) {
                            self.emit(Instruction::Retain {
                                value: handle,
                                ty: receiver_ty,
                            });
                            self.borrowed_locals.remove(&local);
                        }
                        let unique = self.value();
                        self.emit(Instruction::MakeUnique {
                            value: unique,
                            operand: handle,
                            ty: receiver_ty,
                        });
                        self.emit(Instruction::Store {
                            local,
                            value: unique,
                        });
                        lowered.push(unique);
                        return Some(());
                    }
                    let value = self.value();
                    self.emit(Instruction::Borrow { value, local });
                    // A narrowed optional receiver borrows the optional storage
                    // and then unwraps to the present inner value (its address
                    // for aggregates, the handle/payload for others).
                    let mut receiver = value;
                    if self.is_narrowed(&name.text)
                        && let Some(ty) = self.local_data[local.0].ty
                        && let Type::Optional(inner) = self.semantics.types[ty.0]
                    {
                        let unwrapped = self.value();
                        self.emit(Instruction::OptionalUnwrap {
                            value: unwrapped,
                            operand: value,
                            inner,
                        });
                        receiver = unwrapped;
                    }
                    lowered.push(receiver);
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
                    let field_ty = self
                        .semantics
                        .expression_types
                        .get(&expression.span())
                        .copied();
                    if mutating && self.is_cow_collection(field_ty) {
                        let field_ty = field_ty?;
                        let unique = self.value();
                        self.emit(Instruction::MakeUnique {
                            value: unique,
                            operand: value,
                            ty: field_ty,
                        });
                        self.emit(Instruction::FieldStore {
                            base,
                            name: member.text.clone(),
                            value: unique,
                            release: false,
                        });
                        lowered.push(unique);
                        return Some(());
                    }
                    lowered.push(value);
                    return Some(());
                }
            }
            Expr::Subscript {
                object,
                index,
                span,
                ..
            } if mutating => {
                if self.semantics.subscripts.get(span) == Some(&SubscriptKind::Generic) {
                    return self.lower_builtin_value(expression, lowered, owned);
                }
                let index = index.as_deref()?;
                let collection_ty = *self.semantics.expression_types.get(&object.span())?;
                let element_ty = *self.semantics.expression_types.get(span)?;
                let (get, set) = match self.semantics.types[collection_ty.0] {
                    Type::List(_) => (BuiltinFunction::ListAt, BuiltinFunction::ListSet),
                    Type::Map(_, _) => (BuiltinFunction::MapGet, BuiltinFunction::MapSet),
                    // Inline arrays have no shared storage; mutate a temporary.
                    _ => return self.lower_builtin_value(expression, lowered, owned),
                };
                // The parent is an lvalue (made unique); the element is read as
                // an owned reference, made unique, and written back after the
                // call so the mutation reaches the parent.
                let mut parent_args = Vec::new();
                self.lower_builtin_receiver(object, true, &mut parent_args, owned)?;
                let parent = *parent_args.first()?;
                let mut index_args = Vec::new();
                self.lower_builtin_value(index, &mut index_args, owned)?;
                let index_value = *index_args.first()?;
                let element = self.value();
                self.emit(Instruction::RuntimeCall {
                    value: Some(element),
                    result_type: Some(element_ty),
                    function: get,
                    arguments: vec![parent, index_value],
                });
                let unique = self.value();
                self.emit(Instruction::MakeUnique {
                    value: unique,
                    operand: element,
                    ty: element_ty,
                });
                self.pending_writeback.push(WriteBack {
                    function: set,
                    parent,
                    index: index_value,
                    element: unique,
                    element_ty,
                });
                lowered.push(unique);
                return Some(());
            }
            _ => {}
        }
        self.lower_builtin_value(expression, lowered, owned)
    }

    /// Applies queued subscript write-backs (innermost first) after a mutating
    /// builtin call, transferring each mutated element back into its parent.
    pub(super) fn apply_pending_writeback(&mut self) {
        let writes = std::mem::take(&mut self.pending_writeback);
        for write in writes.into_iter().rev() {
            self.emit(Instruction::RuntimeCall {
                value: None,
                result_type: None,
                function: write.function,
                arguments: vec![write.parent, write.index, write.element],
            });
            if self.layouts.types[write.element_ty.0].needs_drop {
                self.emit(Instruction::Release {
                    value: write.element,
                    ty: write.element_ty,
                });
            }
        }
    }

    /// Whether a mutating builtin must detach shared storage for this type.
    fn is_cow_collection(&self, ty: Option<TypeId>) -> bool {
        ty.is_some_and(|ty| {
            matches!(
                self.layouts.types[ty.0].ownership,
                super::OwnershipKind::RcList
                    | super::OwnershipKind::RcMap
                    | super::OwnershipKind::RcBytes
            )
        })
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
    /// Lowers a `resource[T]` used where only a borrow is required (an `extern`
    /// `ptr[T]` parameter). The handle value is passed without moving,
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
}

/// Builtins that mutate a collection in place and therefore require
/// copy-on-write detachment when the receiver's storage is shared.
pub(super) fn is_mutating_builtin(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::ListPush
            | BuiltinFunction::ListSet
            | BuiltinFunction::ListInsert
            | BuiltinFunction::ListRemove
            | BuiltinFunction::ListClear
            | BuiltinFunction::ListReserve
            | BuiltinFunction::ListExtend
            | BuiltinFunction::ListReverse
            | BuiltinFunction::ListSort
            | BuiltinFunction::ListTruncate
            | BuiltinFunction::ListShrinkToFit
            | BuiltinFunction::MapSet
            | BuiltinFunction::MapRemove
            | BuiltinFunction::MapClear
            | BuiltinFunction::MapReserve
            | BuiltinFunction::BytesSet
            | BuiltinFunction::BytesPush
            | BuiltinFunction::BytesExtend
            | BuiltinFunction::BytesTruncate
            | BuiltinFunction::BytesResize
            | BuiltinFunction::BytesClear
            | BuiltinFunction::BytesWriteInt { .. }
            | BuiltinFunction::BytesReserve
    )
}
