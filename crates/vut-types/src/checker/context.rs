//! Lexical scope and control-flow context for expression checking.
use std::collections::HashMap;

use vut_hir::TypeId;
use vut_resolver::SymbolId;
use vut_source::Span;

pub(super) struct Context {
    pub(super) scopes: Vec<HashMap<String, TypeId>>,
    pub(super) loop_depth: usize,
    pub(super) function: bool,
    pub(super) return_type: Option<TypeId>,
    pub(super) unsafe_depth: usize,
    /// True while checking the body of an `async fn`/`async` method.
    pub(super) in_async: bool,
    /// True while checking the direct callable argument of `vut(...)`.
    pub(super) in_spawn_callable: bool,
    /// Span of the direct operand of the `await` currently being checked.
    pub(super) await_span: Option<Span>,
    /// Depth of `return` value checking, used to choose async-return diagnostics.
    pub(super) return_depth: usize,
    /// The function/lambda currently being checked, so a channel operation can
    /// mark it as implicitly suspendable.
    pub(super) current_function: Option<SymbolId>,
}
impl Context {
    pub(super) fn new(root: HashMap<String, TypeId>) -> Self {
        Self {
            scopes: vec![root],
            loop_depth: 0,
            function: false,
            return_type: None,
            unsafe_depth: 0,
            in_async: false,
            in_spawn_callable: false,
            await_span: None,
            return_depth: 0,
            current_function: None,
        }
    }
    pub(super) fn lookup(&self, name: &str) -> Option<TypeId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
}
