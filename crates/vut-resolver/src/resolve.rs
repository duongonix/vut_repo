use crate::{
    MethodKey, Module, ModuleId, ModuleInput, ModulePath, Resolution, Symbol, SymbolId, SymbolKind,
};
use std::collections::{HashMap, HashSet};
use vut_ast::{Expr, ImportMode, Item};
use vut_diagnostics::{Diagnostic, DiagnosticSink, codes};
use vut_source::Span;

pub struct Resolver {
    modules: Vec<Module>,
    module_ids: HashMap<ModulePath, ModuleId>,
    symbols: Vec<Symbol>,
    diagnostics: DiagnosticSink,
    graph_edges: Vec<Vec<(ModuleId, Span)>>,
    /// For every function and method symbol, the receiver type of its last
    /// parameter when that parameter is a receiver function (`fn(R)(...)`).
    trailing_receivers: HashMap<SymbolId, Option<SymbolId>>,
}
impl Resolver {
    #[must_use]
    pub fn new(mut inputs: Vec<ModuleInput>) -> Self {
        inputs.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
        let mut module_ids = HashMap::new();
        let modules: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(index, input)| {
                let id = ModuleId(index);
                module_ids.insert(input.logical_path.clone(), id);
                Module {
                    id,
                    logical_path: input.logical_path,
                    filesystem_path: input.filesystem_path,
                    source: input.file.source,
                    file: input.file,
                    symbols: HashMap::new(),
                    imports: HashMap::new(),
                    methods: HashMap::new(),
                }
            })
            .collect();
        let graph_edges = vec![Vec::new(); modules.len()];
        Self {
            modules,
            module_ids,
            symbols: Vec::new(),
            diagnostics: DiagnosticSink::new(),
            graph_edges,
            trailing_receivers: HashMap::new(),
        }
    }

    #[must_use]
    pub fn resolve(mut self) -> Resolution {
        self.collect_declarations();
        self.resolve_imports();
        self.resolve_method_declarations();
        self.resolve_function_receivers();
        let compile_order = self.detect_cycles();
        let (
            references,
            builtin_utf8_error,
            builtin_hex_error,
            lambdas,
            lambda_symbols,
            implicit_receiver_methods,
        ) = crate::scope::resolve_bodies(
            &self.modules,
            &mut self.symbols,
            &mut self.diagnostics,
            &self.trailing_receivers,
        );
        self.diagnostics.sort_deterministically();
        let graph = self
            .graph_edges
            .iter()
            .map(|edges| edges.iter().map(|(module, _)| *module).collect())
            .collect();
        Resolution {
            modules: self.modules,
            symbols: self.symbols,
            references,
            graph,
            compile_order,
            diagnostics: self.diagnostics,
            builtin_utf8_error,
            builtin_hex_error,
            lambdas,
            lambda_symbols,
            implicit_receiver_methods,
        }
    }

    fn collect_declarations(&mut self) {
        for module_index in 0..self.modules.len() {
            let module_id = ModuleId(module_index);
            let declarations: Vec<_> = self.modules[module_index]
                .file
                .items
                .iter()
                .filter_map(declaration)
                .collect();
            for (name, kind, span) in declarations {
                if let Some(previous) = self.modules[module_index].symbols.get(&name).copied() {
                    if kind == SymbolKind::Binding
                        && self.symbols[previous.0].kind == SymbolKind::Binding
                    {
                        continue;
                    }
                    let previous_span = self.symbols[previous.0].span;
                    self.diagnostics.push(
                        Diagnostic::error(
                            codes::E2002,
                            format!("duplicate symbol `{name}`"),
                            span,
                            "declared more than once",
                        )
                        .with_related(previous_span, "previous declaration"),
                    );
                    continue;
                }
                let id = self.allocate_symbol(
                    module_id,
                    name.clone(),
                    kind,
                    span,
                    !name.starts_with('_'),
                );
                self.modules[module_index].symbols.insert(name, id);
            }
        }
    }

    fn resolve_imports(&mut self) {
        for module_index in 0..self.modules.len() {
            let imports: Vec<_> = self.modules[module_index]
                .file
                .items
                .iter()
                .filter_map(|item| {
                    if let Item::Import(import) = item {
                        Some(import.clone())
                    } else {
                        None
                    }
                })
                .collect();
            for import in imports {
                let Some(path) = self.normalize_import(ModuleId(module_index), &import) else {
                    continue;
                };
                let Some(target) = self.module_ids.get(&path).copied() else {
                    self.diagnostics.push(Diagnostic::error(
                        codes::E3001,
                        "module not found",
                        import.span,
                        format!("module `{}` was not found", path.display()),
                    ));
                    continue;
                };
                self.graph_edges[module_index].push((target, import.span));
                match import.mode {
                    ImportMode::Whole => {
                        if let Some(name) = import.path.last() {
                            self.bind_module(module_index, name.text.clone(), target, name.span);
                        }
                    }
                    ImportMode::Alias(alias) => {
                        self.bind_module(module_index, alias.text, target, alias.span);
                    }
                    ImportMode::Selected(names) => {
                        for name in names {
                            self.bind_selected(module_index, target, name.text, name.span);
                        }
                    }
                }
            }
        }
    }

    fn resolve_method_declarations(&mut self) {
        for module_index in 0..self.modules.len() {
            let methods: Vec<_> = self.modules[module_index]
                .file
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Method(method) => Some(method.clone()),
                    _ => None,
                })
                .collect();
            for method in methods {
                let Some(receiver) = self.resolve_receiver(module_index, &method.receiver) else {
                    continue;
                };
                let key = MethodKey {
                    receiver,
                    name: method.name.text.clone(),
                };
                if let Some(previous) = self.modules[module_index].methods.get(&key).copied() {
                    self.diagnostics.push(
                        Diagnostic::error(
                            codes::E2008,
                            format!("duplicate method `{}`", method.name.text),
                            method.name.span,
                            "this receiver already defines a method with this name",
                        )
                        .with_related(self.symbols[previous.0].span, "previous method declaration"),
                    );
                    continue;
                }
                let symbol = self.allocate_symbol(
                    ModuleId(module_index),
                    method.name.text.clone(),
                    SymbolKind::Method,
                    method.name.span,
                    !method.name.text.starts_with('_'),
                );
                self.symbols[symbol.0].receiver = Some(receiver);
                self.symbols[symbol.0].is_static = method.is_static;
                self.modules[module_index].methods.insert(key, symbol);
                let trailing = self.last_parameter_receiver(module_index, &method.parameters);
                self.trailing_receivers.insert(symbol, trailing);
            }
        }
    }

    /// Records, for every free function, the receiver type of its last parameter
    /// when that parameter is a receiver function.
    fn resolve_function_receivers(&mut self) {
        for module_index in 0..self.modules.len() {
            let functions: Vec<_> = self.modules[module_index]
                .file
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Function(function) => Some(function.clone()),
                    _ => None,
                })
                .collect();
            for function in functions {
                let Some(symbol) = self.modules[module_index]
                    .symbols
                    .get(&function.name.text)
                    .copied()
                else {
                    continue;
                };
                let trailing = self.last_parameter_receiver(module_index, &function.parameters);
                self.trailing_receivers.insert(symbol, trailing);
            }
        }
    }

    /// Receiver type of the declaration's final parameter, when it is written
    /// `fn(Receiver)(Args...) -> T`.
    fn last_parameter_receiver(
        &mut self,
        module: usize,
        parameters: &[vut_ast::Parameter],
    ) -> Option<SymbolId> {
        let last = parameters.last()?;
        let vut_ast::TypeExpr::Function {
            receiver: Some(receiver),
            ..
        } = &last.ty
        else {
            return None;
        };
        self.resolve_type_symbol(module, receiver)
    }

    /// Resolves a named type expression to a visible symbol (data, enum, alias,
    /// or imported type), without the same-module restriction of methods.
    fn resolve_type_symbol(&mut self, module: usize, ty: &vut_ast::TypeExpr) -> Option<SymbolId> {
        let vut_ast::TypeExpr::Named { path, .. } = ty else {
            return None;
        };
        let first = path.first()?;
        let mut symbol = self.modules[module]
            .symbols
            .get(&first.text)
            .or_else(|| self.modules[module].imports.get(&first.text))
            .copied();
        for segment in path.iter().skip(1) {
            let target = symbol.and_then(|id| self.symbols[id.0].target_module);
            symbol = target.and_then(|id| self.modules[id.0].symbols.get(&segment.text).copied());
        }
        symbol.filter(|symbol| {
            matches!(
                self.symbols[symbol.0].kind,
                SymbolKind::Data | SymbolKind::Enum | SymbolKind::TypeAlias
            )
        })
    }

    fn resolve_receiver(
        &mut self,
        module: usize,
        receiver: &vut_ast::TypeExpr,
    ) -> Option<SymbolId> {
        let vut_ast::TypeExpr::Named { path, span } = receiver else {
            self.diagnostics.push(Diagnostic::error(
                codes::E2001,
                "invalid method receiver",
                receiver.span(),
                "a method receiver must be a named type",
            ));
            return None;
        };
        let first = path.first()?;
        let mut symbol = self.modules[module]
            .symbols
            .get(&first.text)
            .or_else(|| self.modules[module].imports.get(&first.text))
            .copied();
        for segment in path.iter().skip(1) {
            let target = symbol.and_then(|id| self.symbols[id.0].target_module);
            symbol = target.and_then(|id| self.modules[id.0].symbols.get(&segment.text).copied());
        }
        let Some(symbol) = symbol else {
            self.diagnostics.push(Diagnostic::error(
                codes::E2001,
                "unknown method receiver type",
                *span,
                "receiver type is not visible in this module",
            ));
            return None;
        };
        if !matches!(
            self.symbols[symbol.0].kind,
            SymbolKind::Data | SymbolKind::Enum | SymbolKind::TypeAlias
        ) {
            self.diagnostics.push(Diagnostic::error(
                codes::E2001,
                "invalid method receiver type",
                *span,
                "receiver must resolve to a named data, enum, or type alias",
            ));
            return None;
        }
        if self.symbols[symbol.0].module != ModuleId(module) {
            self.diagnostics.push(Diagnostic::error(
                codes::E2001,
                "method receiver is not local",
                *span,
                "extension methods for types declared in another module are not supported",
            ));
            return None;
        }
        Some(symbol)
    }

    fn normalize_import(
        &mut self,
        importer: ModuleId,
        import: &vut_ast::Import,
    ) -> Option<ModulePath> {
        let suffix: Vec<_> = import.path.iter().map(|name| name.text.clone()).collect();
        if !import.relative {
            return Some(ModulePath(suffix));
        }
        let mut base = self.modules[importer.0].logical_path.0.clone();
        base.pop();
        if import.parent_depth > base.len() {
            self.diagnostics.push(Diagnostic::error(
                codes::E3006,
                "relative import escapes source root",
                import.span,
                "cannot move above the source root",
            ));
            return None;
        }
        for _ in 0..import.parent_depth {
            base.pop();
        }
        base.extend(suffix);
        Some(ModulePath(base))
    }

    fn bind_module(&mut self, module: usize, name: String, target: ModuleId, span: Span) {
        if self.binding_exists(module, &name, span) {
            return;
        }
        let id = self.allocate_symbol(
            ModuleId(module),
            name.clone(),
            SymbolKind::Module,
            span,
            true,
        );
        self.modules[module].imports.insert(name, id);
        self.symbols[id.0].target_module = Some(target);
    }
    fn bind_selected(&mut self, module: usize, target: ModuleId, name: String, span: Span) {
        let Some(export) = self.modules[target.0].symbols.get(&name).copied() else {
            self.diagnostics.push(Diagnostic::error(
                codes::E3002,
                format!("symbol `{name}` not found"),
                span,
                format!(
                    "module `{}` has no symbol `{name}`",
                    self.modules[target.0].logical_path.display()
                ),
            ));
            return;
        };
        if !self.symbols[export.0].public {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::E3004,
                    format!("private symbol `{name}`"),
                    span,
                    "private symbols cannot be imported",
                )
                .with_related(self.symbols[export.0].span, "declared private here"),
            );
            return;
        }
        if self.binding_exists(module, &name, span) {
            return;
        }
        self.modules[module].imports.insert(name, export);
    }
    fn binding_exists(&mut self, module: usize, name: &str, span: Span) -> bool {
        let previous = self.modules[module]
            .symbols
            .get(name)
            .or_else(|| self.modules[module].imports.get(name))
            .copied();
        if let Some(previous) = previous {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::E3003,
                    format!("import binding `{name}` collides"),
                    span,
                    "binding already exists",
                )
                .with_related(self.symbols[previous.0].span, "previous binding"),
            );
            true
        } else {
            false
        }
    }
    fn allocate_symbol(
        &mut self,
        module: ModuleId,
        name: String,
        kind: SymbolKind,
        span: Span,
        public: bool,
    ) -> SymbolId {
        let id = SymbolId(self.symbols.len());
        self.symbols.push(Symbol {
            id,
            module,
            name,
            kind,
            span,
            public,
            target_module: None,
            receiver: None,
            is_static: false,
        });
        id
    }

    fn detect_cycles(&mut self) -> Vec<ModuleId> {
        let mut state = vec![0u8; self.modules.len()];
        let mut stack = Vec::new();
        let mut order = Vec::new();
        let mut reported = HashSet::new();
        for module in 0..self.modules.len() {
            self.visit(
                ModuleId(module),
                &mut state,
                &mut stack,
                &mut order,
                &mut reported,
            );
        }
        order
    }
    fn visit(
        &mut self,
        module: ModuleId,
        state: &mut [u8],
        stack: &mut Vec<ModuleId>,
        order: &mut Vec<ModuleId>,
        reported: &mut HashSet<Vec<ModuleId>>,
    ) {
        if state[module.0] == 2 {
            return;
        }
        if state[module.0] == 1 {
            return;
        }
        state[module.0] = 1;
        stack.push(module);
        let edges = self.graph_edges[module.0].clone();
        for (next, span) in edges {
            if state[next.0] == 1 {
                let start = stack.iter().position(|item| *item == next).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(next);
                if reported.insert(cycle.clone()) {
                    let names = cycle
                        .iter()
                        .map(|id| self.modules[id.0].logical_path.display())
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    self.diagnostics.push(Diagnostic::error(
                        codes::E3010,
                        "circular module dependency",
                        span,
                        names,
                    ));
                }
            } else {
                self.visit(next, state, stack, order, reported);
            }
        }
        stack.pop();
        state[module.0] = 2;
        order.push(module);
    }
}

fn declaration(item: &Item) -> Option<(String, SymbolKind, Span)> {
    Some(match item {
        Item::Function(value) => (
            value.name.text.clone(),
            SymbolKind::Function,
            value.name.span,
        ),
        Item::ExternFunction(value) => (
            value.name.text.clone(),
            SymbolKind::Function,
            value.name.span,
        ),
        Item::Data(value) => (value.name.text.clone(), SymbolKind::Data, value.name.span),
        Item::Interface(value) => (
            value.name.text.clone(),
            SymbolKind::Interface,
            value.name.span,
        ),
        Item::Enum(value) => (value.name.text.clone(), SymbolKind::Enum, value.name.span),
        Item::TypeAlias(value) => (
            value.name.text.clone(),
            SymbolKind::TypeAlias,
            value.name.span,
        ),
        Item::Statement(vut_ast::Stmt::Binding {
            target: Expr::Name(name),
            ..
        }) => (name.text.clone(), SymbolKind::Binding, name.span),
        _ => return None,
    })
}
