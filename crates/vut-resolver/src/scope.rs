use crate::{
    ImplicitReceiverMethod, LambdaInfo, Module, ModuleId, ResolvedReference, Symbol, SymbolId,
    SymbolKind,
};
use std::collections::HashMap;
use vut_ast::{
    Block, Expr, ForKind, Item, LambdaBody, LambdaParameter, LambdaReceiver, MatchPattern, Name,
    Stmt, TemplateSegment, TypeExpr,
};
use vut_diagnostics::{Diagnostic, DiagnosticSink, codes};
use vut_source::{SourceId, Span};

/// Everything `resolve_bodies` produces alongside the mutated symbols.
type ResolvedBodies = (
    Vec<ResolvedReference>,
    SymbolId,
    SymbolId,
    Vec<LambdaInfo>,
    HashMap<Span, SymbolId>,
    HashMap<Span, ImplicitReceiverMethod>,
);

/// Result of inferring the receiver type of a callee's final parameter.
#[derive(Clone, Copy)]
enum ReceiverInference {
    /// The callee is statically known; carries its receiver type, if any.
    Known(Option<SymbolId>),
    /// The receiver type cannot be determined statically.
    Unknown,
}

pub(crate) fn resolve_bodies(
    modules: &[Module],
    symbols: &mut Vec<Symbol>,
    diagnostics: &mut DiagnosticSink,
    trailing_receivers: &HashMap<SymbolId, Option<SymbolId>>,
) -> ResolvedBodies {
    let mut references = Vec::new();
    let mut lambdas = Vec::new();
    let mut lambda_symbols = HashMap::new();
    let mut implicit_receiver_methods = HashMap::new();
    let builtin_source = modules
        .first()
        .map_or(SourceId::from_index(0), |module| module.source);
    let utf8_error = allocate(
        symbols,
        ModuleId(0),
        "Utf8Error",
        SymbolKind::Data,
        Span::new(builtin_source, 0, 0),
        true,
    );
    let hex_error = allocate(
        symbols,
        ModuleId(0),
        "HexError",
        SymbolKind::Data,
        Span::new(builtin_source, 0, 0),
        true,
    );
    for module in modules {
        let mut root = module.symbols.clone();
        root.extend(module.imports.iter().map(|(name, id)| (name.clone(), *id)));
        for name in ["print", "out", "input"] {
            let id = allocate(
                symbols,
                module.id,
                name,
                SymbolKind::Function,
                Span::new(module.source, 0, 0),
                true,
            );
            root.insert(name.to_owned(), id);
        }
        for name in [
            "void", "bool", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "isize",
            "f32", "f64", "int", "float", "str", "bytes", "dyn", "list", "map", "result", "ptr",
            "vutcon", "resource", "future",
        ] {
            let id = allocate(
                symbols,
                module.id,
                name,
                SymbolKind::TypeAlias,
                Span::new(module.source, 0, 0),
                true,
            );
            root.entry(name.to_owned()).or_insert(id);
        }
        root.entry("Utf8Error".to_owned()).or_insert(utf8_error);
        root.entry("HexError".to_owned()).or_insert(hex_error);
        let mut context = Context {
            module: module.id,
            symbols,
            modules,
            diagnostics,
            references: &mut references,
            lambdas: &mut lambdas,
            lambda_symbols: &mut lambda_symbols,
            lambda_floors: Vec::new(),
            scopes: vec![root],
            receivers: Vec::new(),
            pending_lambda_receiver: None,
            trailing_receivers,
            binding_receivers: HashMap::new(),
            implicit_receiver_methods: &mut implicit_receiver_methods,
        };
        for item in &module.file.items {
            context.item(item);
        }
    }
    (
        references,
        utf8_error,
        hex_error,
        lambdas,
        lambda_symbols,
        implicit_receiver_methods,
    )
}

struct Context<'a> {
    module: ModuleId,
    modules: &'a [Module],
    symbols: &'a mut Vec<Symbol>,
    diagnostics: &'a mut DiagnosticSink,
    references: &'a mut Vec<ResolvedReference>,
    lambdas: &'a mut Vec<LambdaInfo>,
    lambda_symbols: &'a mut HashMap<Span, SymbolId>,
    lambda_floors: Vec<usize>,
    scopes: Vec<HashMap<String, SymbolId>>,
    /// Lexical receiver type stack; the last entry is the current receiver.
    receivers: Vec<SymbolId>,
    /// Receiver inference seeded for the trailing lambda currently being visited.
    pending_lambda_receiver: Option<ReceiverInference>,
    /// Last-parameter receiver type per function/method symbol.
    trailing_receivers: &'a HashMap<SymbolId, Option<SymbolId>>,
    /// Receiver type of an annotated function-typed parameter or binding.
    binding_receivers: HashMap<SymbolId, Option<SymbolId>>,
    /// Output: bare-name calls resolved to the current receiver's method.
    implicit_receiver_methods: &'a mut HashMap<Span, ImplicitReceiverMethod>,
}
impl Context<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "item dispatch is an intentionally exhaustive declaration visitor"
    )]
    fn item(&mut self, item: &Item) {
        match item {
            Item::Function(value) => {
                self.push_scope();
                self.define_type_parameters(&value.type_parameters);
                for parameter in &value.parameters {
                    let symbol = self.define(
                        &parameter.name.text,
                        SymbolKind::Parameter,
                        parameter.name.span,
                    );
                    self.record_binding_receiver(symbol, &parameter.ty);
                    self.ty(&parameter.ty);
                }
                if let Some(ty) = &value.return_type {
                    self.ty(ty);
                }
                self.block(&value.body, false);
                self.pop_scope();
            }
            Item::Method(value) => {
                self.push_scope();
                self.define_type_parameters(&value.type_parameters);
                self.ty(&value.receiver);
                self.define("self", SymbolKind::Parameter, value.name.span);
                for parameter in &value.parameters {
                    if parameter.name.text == "self" {
                        continue;
                    }
                    let symbol = self.define(
                        &parameter.name.text,
                        SymbolKind::Parameter,
                        parameter.name.span,
                    );
                    self.record_binding_receiver(symbol, &parameter.ty);
                    self.ty(&parameter.ty);
                }
                if let Some(ty) = &value.return_type {
                    self.ty(ty);
                }
                self.block(&value.body, false);
                self.pop_scope();
            }
            Item::ExternFunction(value) => {
                self.push_scope();
                self.define_type_parameters(&value.type_parameters);
                for parameter in &value.parameters {
                    let symbol = self.define(
                        &parameter.name.text,
                        SymbolKind::Parameter,
                        parameter.name.span,
                    );
                    self.record_binding_receiver(symbol, &parameter.ty);
                    self.ty(&parameter.ty);
                }
                if let Some(ty) = &value.return_type {
                    self.ty(ty);
                }
                self.pop_scope();
            }
            Item::Data(value) => {
                self.push_scope();
                self.define_type_parameters(&value.type_parameters);
                for field in &value.fields {
                    self.ty(&field.ty);
                    if let Some(default) = &field.default {
                        self.expr(default);
                    }
                }
                self.pop_scope();
            }
            Item::Interface(value) => {
                self.push_scope();
                self.define_type_parameters(&value.type_parameters);
                for parent in &value.parents {
                    self.reference_interface_parent(&parent.text, parent.span);
                }
                for method in &value.methods {
                    self.push_scope();
                    for parameter in &method.parameters {
                        self.define(
                            &parameter.name.text,
                            SymbolKind::Parameter,
                            parameter.name.span,
                        );
                        self.ty(&parameter.ty);
                    }
                    if let Some(ty) = &method.return_type {
                        self.ty(ty);
                    }
                    self.pop_scope();
                }
                self.pop_scope();
            }
            Item::Enum(value) => {
                self.push_scope();
                self.define_type_parameters(&value.type_parameters);
                for variant in &value.variants {
                    for field in &variant.fields {
                        self.ty(&field.ty);
                    }
                }
                self.pop_scope();
            }
            Item::TypeAlias(value) => self.ty(&value.ty),
            Item::Statement(statement) => self.statement(statement, true),
            Item::Import(_) => {}
        }
    }

    /// Defines every type parameter of a declaration, then resolves its bounds.
    fn define_type_parameters(&mut self, parameters: &[vut_ast::TypeParameter]) {
        for parameter in parameters {
            self.define(
                &parameter.name.text,
                SymbolKind::TypeParameter,
                parameter.name.span,
            );
        }
        for parameter in parameters {
            for bound in &parameter.bounds {
                self.ty(bound);
            }
        }
    }
    fn block(&mut self, block: &Block, nested: bool) {
        if nested {
            self.push_scope();
        }
        for statement in &block.statements {
            self.statement(statement, false);
        }
        if nested {
            self.pop_scope();
        }
    }
    fn statement(&mut self, statement: &Stmt, module_level: bool) {
        match statement {
            Stmt::Binding {
                target,
                annotation,
                value,
                ..
            } => {
                self.expr(value);
                if let Some(ty) = annotation {
                    self.ty(ty);
                }
                match target {
                    Expr::Name(name) if !module_level => {
                        if let Some(symbol) = self.lookup(&name.text) {
                            self.references.push(ResolvedReference {
                                span: name.span,
                                symbol,
                            });
                        } else {
                            let symbol = self.define(&name.text, SymbolKind::Local, name.span);
                            if let Some(annotation) = annotation {
                                self.record_binding_receiver(symbol, annotation);
                            }
                        }
                    }
                    _ => self.expr(target),
                }
            }
            Stmt::Expression(value) => self.expr(value),
            Stmt::If(value) => {
                self.expr(&value.condition);
                self.block(&value.body, true);
                for (condition, block) in &value.elifs {
                    self.expr(condition);
                    self.block(block, true);
                }
                if let Some(block) = &value.otherwise {
                    self.block(block, true);
                }
            }
            Stmt::For(value) => {
                match &value.kind {
                    ForKind::Infinite => {}
                    ForKind::Conditional(condition) => self.expr(condition),
                    ForKind::Iterable {
                        value: binding,
                        index,
                        iterable,
                    } => {
                        self.expr(iterable);
                        self.push_scope();
                        self.define(&binding.text, SymbolKind::Local, binding.span);
                        if let Some(index) = index {
                            self.define(&index.text, SymbolKind::Local, index.span);
                        }
                        self.block(&value.body, false);
                        self.pop_scope();
                        return;
                    }
                }
                self.block(&value.body, true);
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            Stmt::Unsafe(body) => self.block(body, true),
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
        }
    }
    #[expect(
        clippy::too_many_lines,
        reason = "recursive AST visitor mirrors syntax structure"
    )]
    fn expr(&mut self, expression: &Expr) {
        match expression {
            Expr::Name(name) => self.reference(&name.text, name.span),
            Expr::List { values, .. } | Expr::Array { values, .. } => {
                for value in values {
                    self.expr(value);
                }
            }
            Expr::Map { entries, .. } => {
                for entry in entries {
                    self.expr(&entry.key);
                    self.expr(&entry.value);
                }
            }
            Expr::String { segments, .. } => {
                for segment in segments {
                    if let TemplateSegment::Expression(value) = segment {
                        self.expr(value);
                    }
                }
            }
            Expr::Unary { value, .. }
            | Expr::Group { value, .. }
            | Expr::ResultOk { value, .. }
            | Expr::ResultErr { value, .. }
            | Expr::ResultPropagate { value, .. }
            | Expr::Await { value, .. }
            | Expr::Spawn {
                callable: value, ..
            } => self.expr(value),
            Expr::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expr::Member { object, member, .. } => {
                self.expr(object);
                if let Expr::Name(module_name) = object.as_ref()
                    && let Some(binding) = self.lookup(&module_name.text)
                    && let Some(target) = self.symbols[binding.0].target_module
                {
                    if let Some(symbol) = self.modules[target.0].symbols.get(&member.text).copied()
                    {
                        if self.symbols[symbol.0].public {
                            self.references.push(ResolvedReference {
                                span: member.span,
                                symbol,
                            });
                        } else {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    codes::E2006,
                                    format!("private symbol `{}`", member.text),
                                    member.span,
                                    "private to imported module",
                                )
                                .with_related(self.symbols[symbol.0].span, "declared private here"),
                            );
                        }
                    } else {
                        self.diagnostics.push(Diagnostic::error(
                            codes::E3002,
                            format!("symbol `{}` not found", member.text),
                            member.span,
                            "imported module has no such symbol",
                        ));
                    }
                }
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                let inference = self.call_receiver_inference(callee);
                let mut handled = false;
                if let Expr::Name(name) = callee.as_ref() {
                    if self.lookup_local(&name.text).is_some() {
                        self.expr(callee);
                        handled = true;
                    } else if self.try_implicit_receiver_callee(callee) {
                        handled = true;
                    } else if self.lookup_root(&name.text).is_some() {
                        self.expr(callee);
                        handled = true;
                    } else if !self.receivers.is_empty() {
                        self.report_unresolved_receiver_call(name);
                        handled = true;
                    }
                }
                if !handled {
                    self.expr(callee);
                }
                for argument in arguments {
                    if matches!(
                        &argument.value,
                        Expr::Lambda {
                            receiver: LambdaReceiver::Inferred,
                            ..
                        }
                    ) {
                        match inference {
                            ReceiverInference::Known(receiver) => {
                                self.pending_lambda_receiver =
                                    Some(ReceiverInference::Known(receiver));
                            }
                            ReceiverInference::Unknown => self.diagnostics.push(Diagnostic::error(
                                codes::E1026,
                                "cannot infer receiver function type",
                                argument.value.span(),
                                "the callee's receiver function type is not statically known",
                            )),
                        }
                    }
                    self.expr(&argument.value);
                }
            }
            Expr::Lambda {
                receiver: _,
                parameters,
                body,
                is_async,
                span,
            } => self.lambda(parameters, body, *is_async, *span),
            Expr::If(value) => {
                self.expr(&value.condition);
                self.block(&value.body, true);
                for (condition, block) in &value.elifs {
                    self.expr(condition);
                    self.block(block, true);
                }
                if let Some(block) = &value.otherwise {
                    self.block(block, true);
                }
            }
            Expr::Block(block) => self.block(block, true),
            Expr::Match { value, arms, .. } => {
                self.expr(value);
                for arm in arms {
                    self.push_scope();
                    let mut bindings = Vec::new();
                    pattern_bindings(&arm.pattern, &mut bindings);
                    for (name, span) in bindings {
                        self.define(&name, SymbolKind::Local, span);
                    }
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.expr(&arm.value);
                    self.pop_scope();
                }
            }
            Expr::Integer { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Null(_)
            | Expr::Error(_) => {}
        }
    }
    fn lambda(
        &mut self,
        parameters: &[LambdaParameter],
        body: &LambdaBody,
        is_async: bool,
        span: Span,
    ) {
        let receiver = match self.pending_lambda_receiver.take() {
            Some(ReceiverInference::Known(receiver)) => receiver,
            _ => None,
        };
        let symbol = allocate(
            self.symbols,
            self.module,
            "<lambda>",
            SymbolKind::AnonymousFunction,
            span,
            false,
        );
        self.lambda_symbols.insert(span, symbol);
        self.lambdas.push(LambdaInfo {
            symbol,
            module: self.module,
            parameters: parameters.to_vec(),
            body: body.clone(),
            is_async,
            receiver,
            span,
        });
        let floor = self.scopes.len();
        self.push_scope();
        if let Some(receiver) = receiver {
            self.define("self", SymbolKind::Parameter, span);
            self.receivers.push(receiver);
        }
        for parameter in parameters {
            self.define(
                &parameter.name.text,
                SymbolKind::Parameter,
                parameter.name.span,
            );
            if let Some(ty) = &parameter.ty {
                self.ty(ty);
            }
        }
        self.lambda_floors.push(floor);
        match body {
            LambdaBody::Expression(value) => self.expr(value),
            LambdaBody::Block(block) => self.block(block, true),
        }
        self.lambda_floors.pop();
        if receiver.is_some() {
            self.receivers.pop();
        }
        self.pop_scope();
    }
    fn ty(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Named { path, .. } => {
                if let Some(first) = path.first() {
                    // `Self` is a contextual type name in interface requirements,
                    // resolved by the type checker, not a scoped symbol.
                    if first.text != "Self" {
                        self.reference(&first.text, first.span);
                        if path.len() > 1 {
                            self.qualified_type(path);
                        }
                    }
                }
            }
            TypeExpr::Applied {
                name, arguments, ..
            } => {
                self.reference(&name.text, name.span);
                for argument in arguments {
                    self.ty(argument);
                }
            }
            TypeExpr::Optional { inner, .. } => self.ty(inner),
            TypeExpr::Array { element, .. } => self.ty(element),
            TypeExpr::ExternFunction {
                parameters,
                return_type,
                ..
            } => {
                for param in parameters {
                    self.ty(param);
                }
                if let Some(return_type) = return_type {
                    self.ty(return_type);
                }
            }
            TypeExpr::Function {
                receiver,
                parameters,
                return_type,
                ..
            } => {
                if let Some(receiver) = receiver {
                    self.ty(receiver);
                }
                for param in parameters {
                    self.ty(param);
                }
                if let Some(return_type) = return_type {
                    self.ty(return_type);
                }
            }
            TypeExpr::Error(_) => {}
        }
    }
    /// Resolves the segments after the first of a qualified type path,
    /// following module targets and registering a reference for the final
    /// symbol (with the same visibility rules as qualified value access).
    fn qualified_type(&mut self, path: &[Name]) {
        let mut symbol = self.lookup(&path[0].text);
        for (index, segment) in path.iter().enumerate().skip(1) {
            let Some(target) = symbol.and_then(|id| self.symbols[id.0].target_module) else {
                self.diagnostics.push(Diagnostic::error(
                    codes::E3002,
                    format!("symbol `{}` not found", segment.text),
                    segment.span,
                    "module path does not resolve to a module",
                ));
                return;
            };
            let Some(next) = self.modules[target.0].symbols.get(&segment.text).copied() else {
                self.diagnostics.push(Diagnostic::error(
                    codes::E3002,
                    format!("symbol `{}` not found", segment.text),
                    segment.span,
                    "imported module has no such symbol",
                ));
                return;
            };
            if index + 1 == path.len() {
                if self.symbols[next.0].public {
                    self.references.push(ResolvedReference {
                        span: segment.span,
                        symbol: next,
                    });
                } else {
                    self.diagnostics.push(
                        Diagnostic::error(
                            codes::E2006,
                            format!("private symbol `{}`", segment.text),
                            segment.span,
                            "private to imported module",
                        )
                        .with_related(self.symbols[next.0].span, "declared private here"),
                    );
                }
            }
            symbol = Some(next);
        }
    }
    fn reference(&mut self, name: &str, span: Span) {
        if let Some(symbol) = self.lookup(name) {
            self.check_capture(name, symbol, span);
            self.references.push(ResolvedReference { span, symbol });
        } else {
            if name == "self" {
                self.diagnostics.push(Diagnostic::error(
                    codes::E2007,
                    "invalid `self`",
                    span,
                    "`self` is available only inside an instance method",
                ));
                return;
            }
            let mut diagnostic = Diagnostic::error(
                codes::E2001,
                format!("unknown symbol `{name}`"),
                span,
                "name is not visible in this scope",
            );
            if let Some(suggestion) = self.suggestion(name) {
                diagnostic = diagnostic.with_help(format!("did you mean `{suggestion}`?"));
            }
            self.diagnostics.push(diagnostic);
        }
    }
    /// Resolves a composed interface parent. Unlike a generic reference, an
    /// unresolved parent is reported as `E4001` (unknown interface).
    fn reference_interface_parent(&mut self, name: &str, span: Span) {
        if let Some(symbol) = self.lookup(name) {
            self.references.push(ResolvedReference { span, symbol });
            return;
        }
        let mut diagnostic = Diagnostic::error(
            codes::E4001,
            format!("unknown interface `{name}`"),
            span,
            "referenced interface is not visible in this scope",
        );
        if let Some(suggestion) = self.suggestion(name) {
            diagnostic = diagnostic.with_help(format!("did you mean `{suggestion}`?"));
        }
        self.diagnostics.push(diagnostic);
    }
    fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
    /// Looks up a name in local/parameter scopes only (never the module root).
    fn lookup_local(&self, name: &str) -> Option<SymbolId> {
        self.scopes
            .iter()
            .skip(1)
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
    /// Looks up a name in the module root scope only.
    fn lookup_root(&self, name: &str) -> Option<SymbolId> {
        self.scopes
            .first()
            .and_then(|scope| scope.get(name).copied())
    }
    /// Nearest enclosing receiver that declares `name` as an instance method.
    fn lookup_receiver_method(&self, name: &str) -> Option<(SymbolId, SymbolId)> {
        for receiver in self.receivers.iter().rev() {
            if let Some(method) = self.modules[self.module.0]
                .methods
                .get(&crate::MethodKey {
                    receiver: *receiver,
                    name: name.to_owned(),
                })
                .copied()
                && !self.symbols[method.0].is_static
            {
                return Some((*receiver, method));
            }
        }
        None
    }
    /// Receiver type inferred for a callee's final parameter, if it declares a
    /// trailing receiver function.
    fn call_receiver_inference(&mut self, callee: &Expr) -> ReceiverInference {
        match callee {
            Expr::Name(name) => {
                if let Some(symbol) = self.lookup_local(&name.text) {
                    return self.symbol_trailing_receiver(symbol);
                }
                if let Some((_, method)) = self.lookup_receiver_method(&name.text) {
                    return self.trailing_receiver_of(method);
                }
                match self.lookup_root(&name.text) {
                    Some(symbol) => self.symbol_trailing_receiver(symbol),
                    None => ReceiverInference::Unknown,
                }
            }
            Expr::Member { object, member, .. } => {
                let Expr::Name(base) = object.as_ref() else {
                    return ReceiverInference::Unknown;
                };
                let receiver = if base.text == "self" {
                    match self.receivers.last() {
                        Some(receiver) => *receiver,
                        None => return ReceiverInference::Unknown,
                    }
                } else {
                    let Some(symbol) = self.lookup_root(&base.text) else {
                        return ReceiverInference::Unknown;
                    };
                    if !matches!(
                        self.symbols[symbol.0].kind,
                        SymbolKind::Data | SymbolKind::Enum | SymbolKind::TypeAlias
                    ) {
                        return ReceiverInference::Unknown;
                    }
                    symbol
                };
                let Some(method) = self.modules[self.module.0]
                    .methods
                    .get(&crate::MethodKey {
                        receiver,
                        name: member.text.clone(),
                    })
                    .copied()
                else {
                    return ReceiverInference::Unknown;
                };
                self.trailing_receiver_of(method)
            }
            _ => ReceiverInference::Unknown,
        }
    }
    fn trailing_receiver_of(&self, symbol: SymbolId) -> ReceiverInference {
        self.trailing_receivers
            .get(&symbol)
            .copied()
            .map_or(ReceiverInference::Unknown, ReceiverInference::Known)
    }
    fn symbol_trailing_receiver(&self, symbol: SymbolId) -> ReceiverInference {
        if let Some(receiver) = self.trailing_receivers.get(&symbol) {
            return ReceiverInference::Known(*receiver);
        }
        if let Some(receiver) = self.binding_receivers.get(&symbol) {
            return ReceiverInference::Known(*receiver);
        }
        ReceiverInference::Unknown
    }
    /// Resolves a bare callee name to the current receiver's method, recording
    /// the binding so the checker/MIR can pass `self`.
    fn try_implicit_receiver_callee(&mut self, callee: &Expr) -> bool {
        let Expr::Name(name) = callee else {
            return false;
        };
        if self.lookup_local(&name.text).is_some() {
            return false;
        }
        let Some((receiver, method)) = self.lookup_receiver_method(&name.text) else {
            return false;
        };
        self.implicit_receiver_methods
            .insert(name.span, ImplicitReceiverMethod { receiver, method });
        self.references.push(ResolvedReference {
            span: name.span,
            symbol: method,
        });
        true
    }
    /// Reports a bare call name that no local, receiver, or module symbol
    /// provides: unknown to every enclosing receiver (E1023), or ambiguous
    /// across outer receivers (E1024).
    fn report_unresolved_receiver_call(&mut self, name: &Name) {
        let matches = self
            .receivers
            .iter()
            .rev()
            .filter(|receiver| {
                self.modules[self.module.0]
                    .methods
                    .get(&crate::MethodKey {
                        receiver: **receiver,
                        name: name.text.clone(),
                    })
                    .is_some_and(|method| !self.symbols[method.0].is_static)
            })
            .count();
        if matches > 1 {
            self.diagnostics.push(Diagnostic::error(
                codes::E1024,
                format!("ambiguous receiver method `{}`", name.text),
                name.span,
                "multiple enclosing receivers declare a method with this name",
            ));
        } else {
            self.diagnostics.push(Diagnostic::error(
                codes::E1023,
                format!("no enclosing receiver declares `{}`", name.text),
                name.span,
                "the name is not a local, a receiver method, or a module symbol",
            ));
        }
    }
    fn record_binding_receiver(&mut self, symbol: SymbolId, ty: &TypeExpr) {
        if let ReceiverInference::Known(receiver) = self.type_function_receiver(ty) {
            self.binding_receivers.insert(symbol, receiver);
        }
    }
    fn type_function_receiver(&self, ty: &TypeExpr) -> ReceiverInference {
        let TypeExpr::Function { receiver, .. } = ty else {
            return ReceiverInference::Unknown;
        };
        match receiver {
            Some(receiver) => match self.resolve_type_symbol(receiver) {
                Some(symbol) => ReceiverInference::Known(Some(symbol)),
                None => ReceiverInference::Unknown,
            },
            None => ReceiverInference::Known(None),
        }
    }
    fn resolve_type_symbol(&self, ty: &TypeExpr) -> Option<SymbolId> {
        let TypeExpr::Named { path, .. } = ty else {
            return None;
        };
        let first = path.first()?;
        let mut symbol = self.modules[self.module.0]
            .symbols
            .get(&first.text)
            .or_else(|| self.modules[self.module.0].imports.get(&first.text))
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
    fn check_capture(&mut self, name: &str, symbol: SymbolId, span: Span) {
        let Some(floor) = self.lambda_floors.last().copied() else {
            return;
        };
        if !matches!(
            self.symbols[symbol.0].kind,
            SymbolKind::Local | SymbolKind::Parameter | SymbolKind::Binding
        ) {
            return;
        }
        let defining_scope = self
            .scopes
            .iter()
            .position(|scope| scope.values().any(|id| *id == symbol));
        if defining_scope.is_some_and(|index| index < floor) {
            self.diagnostics.push(Diagnostic::error(
                codes::E1013,
                format!("lambda cannot capture `{name}`"),
                span,
                "anonymous functions may only use their own parameters and locals",
            ));
        }
    }
    fn suggestion(&self, name: &str) -> Option<&str> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.keys())
            .filter(|candidate| distance(candidate, name) <= 2)
            .min_by_key(|candidate| distance(candidate, name))
            .map(String::as_str)
    }
    fn define(&mut self, name: &str, kind: SymbolKind, span: Span) -> SymbolId {
        if let Some(previous) = self
            .scopes
            .last()
            .and_then(|scope| scope.get(name))
            .copied()
        {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::E2002,
                    format!("duplicate symbol `{name}`"),
                    span,
                    "already defined in this scope",
                )
                .with_related(self.symbols[previous.0].span, "previous definition"),
            );
            return previous;
        }
        let id = allocate(self.symbols, self.module, name, kind, span, false);
        self.scopes.last_mut().unwrap().insert(name.to_owned(), id);
        id
    }
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}
fn allocate(
    symbols: &mut Vec<Symbol>,
    module: ModuleId,
    name: &str,
    kind: SymbolKind,
    span: Span,
    public: bool,
) -> SymbolId {
    let id = SymbolId(symbols.len());
    symbols.push(Symbol {
        id,
        module,
        name: name.to_owned(),
        kind,
        span,
        public,
        target_module: None,
        receiver: None,
        is_static: false,
    });
    id
}
fn distance(left: &str, right: &str) -> usize {
    let mut previous: Vec<_> = (0..=right.chars().count()).collect();
    for (row, a) in left.chars().enumerate() {
        let mut current = vec![row + 1];
        for (column, b) in right.chars().enumerate() {
            current.push(
                (previous[column + 1] + 1)
                    .min(current[column] + 1)
                    .min(previous[column] + usize::from(a != b)),
            );
        }
        previous = current;
    }
    previous.last().copied().unwrap_or(0)
}

/// Collects the local bindings introduced by a match pattern, in source order.
///
/// Or-pattern alternatives are required to bind the same names, so only the
/// first alternative contributes definitions.
fn pattern_bindings(pattern: &MatchPattern, bindings: &mut Vec<(String, Span)>) {
    match pattern {
        MatchPattern::Wildcard(_)
        | MatchPattern::Literal { .. }
        | MatchPattern::Range { .. }
        | MatchPattern::Error(_) => {}
        MatchPattern::Binding(name) => bindings.push((name.text.clone(), name.span)),
        MatchPattern::Group { pattern, .. } => pattern_bindings(pattern, bindings),
        MatchPattern::ResultOk { pattern, .. } | MatchPattern::ResultErr { pattern, .. } => {
            pattern_bindings(pattern, bindings);
        }
        MatchPattern::Variant { fields, .. } => {
            for field in fields {
                pattern_bindings(field.pattern(), bindings);
            }
        }
        MatchPattern::Or { alternatives, .. } => {
            if let Some(first) = alternatives.first() {
                pattern_bindings(first, bindings);
            }
        }
        MatchPattern::List { items, rest, .. } => {
            for item in items {
                pattern_bindings(item, bindings);
            }
            if let Some(rest) = rest
                && rest.text != "_"
            {
                bindings.push((rest.text.clone(), rest.span));
            }
        }
    }
}
