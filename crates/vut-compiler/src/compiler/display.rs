//! Compiler-generated `display` conversions for `out`/`print` and template
//! interpolation.
//!
//! Vut has no reflection and no runtime type metadata. To print a value
//! directly (`out(User(a = 1))`, `"$data"`), the compiler generates a small
//! Vut function per concrete type that builds its textual representation and
//! rewrites the call sites to use it. Everything then flows through the normal
//! resolver/type-checker/MIR pipeline, so ownership and drops stay correct and
//! there is no new runtime or ABI surface.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use vut_ast::{
    Argument, BinaryOp, Block, Expr, For, ForKind, If, Item, LambdaBody, MapEntry, MatchArm, Name,
    Stmt, TemplateSegment,
};
use vut_hir::TypeId;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_resolver::{ModuleInput, Resolution};
use vut_source::{SourceId, SourceManager, Span};
use vut_types::{SemanticResult, Type};

/// Name of the shared nested-string formatting helper.
const STR_HELPER: &str = "__vut_disp_str_q";

/// Rewrites `out`/`print` arguments and template expressions of non-`str` types
/// to call compiler-generated display functions, appending those functions to
/// each affected module. Returns `true` when anything changed.
pub(crate) fn apply(
    modules: &mut [ModuleInput],
    semantics: &SemanticResult,
    resolution: &Resolution,
    sources: &mut SourceManager,
) -> bool {
    let mut changed = false;
    for module in modules.iter_mut() {
        let mut generator = Generator::new(semantics, resolution);
        let rewritten;
        {
            let mut visitor = Visitor {
                semantics,
                generator: &mut generator,
                sources,
                span_source: None,
                next_span: 0,
                rewritten: false,
            };
            for item in &mut module.file.items {
                visit_item(item, &mut visitor);
            }
            rewritten = visitor.rewritten;
        }
        if !rewritten {
            continue;
        }
        let generated = generator.generate();
        let source = sources.add_text("<generated display>", generated.clone());
        let (tokens, lexical) = Lexer::new(source, &generated).lex();
        if lexical.has_errors() {
            continue;
        }
        let (file, diagnostics) = Parser::new(source, &generated, tokens).parse();
        if diagnostics.has_errors() {
            continue;
        }
        module.file.items.extend(file.items);
        changed = true;
    }
    changed
}

fn visit_item(item: &mut Item, visitor: &mut Visitor<'_, '_>) {
    match item {
        Item::Function(function) => visit_block(&mut function.body, visitor),
        Item::Method(method) => visit_block(&mut method.body, visitor),
        Item::Statement(statement) => visit_stmt(statement, visitor),
        Item::Data(data) => {
            for field in &mut data.fields {
                if let Some(default) = &mut field.default {
                    visit_expr(default, visitor);
                }
            }
        }
        Item::ExternFunction(_)
        | Item::Interface(_)
        | Item::Enum(_)
        | Item::TypeAlias(_)
        | Item::Import(_) => {}
    }
}

fn visit_block(block: &mut Block, visitor: &mut Visitor<'_, '_>) {
    for statement in &mut block.statements {
        visit_stmt(statement, visitor);
    }
}

fn visit_stmt(statement: &mut Stmt, visitor: &mut Visitor<'_, '_>) {
    match statement {
        Stmt::Binding { target, value, .. } => {
            visit_expr(target, visitor);
            visit_expr(value, visitor);
        }
        Stmt::Expression(expr) => visit_expr(expr, visitor),
        Stmt::If(value) => visit_if(value, visitor),
        Stmt::For(value) => visit_for(value, visitor),
        Stmt::Unsafe(block) => visit_block(block, visitor),
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                visit_expr(value, visitor);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Error(_) => {}
    }
}

fn visit_if(value: &mut If, visitor: &mut Visitor<'_, '_>) {
    visit_expr(&mut value.condition, visitor);
    visit_block(&mut value.body, visitor);
    for (condition, block) in &mut value.elifs {
        visit_expr(condition, visitor);
        visit_block(block, visitor);
    }
    if let Some(block) = &mut value.otherwise {
        visit_block(block, visitor);
    }
}

fn visit_for(value: &mut For, visitor: &mut Visitor<'_, '_>) {
    match &mut value.kind {
        ForKind::Infinite => {}
        ForKind::Conditional(condition) => visit_expr(condition, visitor),
        ForKind::Iterable { iterable, .. } => visit_expr(iterable, visitor),
    }
    visit_block(&mut value.body, visitor);
}

fn visit_arms(arms: &mut [MatchArm], visitor: &mut Visitor<'_, '_>) {
    for arm in arms {
        if let Some(guard) = &mut arm.guard {
            visit_expr(guard, visitor);
        }
        visit_expr(&mut arm.value, visitor);
    }
}

fn visit_expr(expr: &mut Expr, visitor: &mut Visitor<'_, '_>) {
    match expr {
        Expr::Name(_)
        | Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Null(_)
        | Expr::Error(_) => {}
        Expr::String { segments, .. } => {
            for segment in segments.iter_mut() {
                if let TemplateSegment::Expression(value) = segment {
                    visit_expr(value, visitor);
                    let Some(ty) = visitor.type_of(value) else {
                        continue;
                    };
                    if !needs_display(visitor.semantics, ty) {
                        continue;
                    }
                    let span = value.span();
                    let original = std::mem::replace(value, Expr::Error(span));
                    *value = visitor.wrap(ty, original);
                }
            }
        }
        Expr::List { values, .. } | Expr::Array { values, .. } => {
            for value in values {
                visit_expr(value, visitor);
            }
        }
        Expr::Map { entries, .. } => {
            for MapEntry { key, value, .. } in entries {
                visit_expr(key, visitor);
                visit_expr(value, visitor);
            }
        }
        Expr::Unary { value, .. }
        | Expr::Group { value, .. }
        | Expr::ResultOk { value, .. }
        | Expr::ResultErr { value, .. }
        | Expr::ResultPropagate { value, .. }
        | Expr::Await { value, .. } => visit_expr(value, visitor),
        Expr::Spawn { callable, .. } => visit_expr(callable, visitor),
        Expr::Binary { left, right, .. } => {
            visit_expr(left, visitor);
            visit_expr(right, visitor);
        }
        Expr::Member { object, .. } => visit_expr(object, visitor),
        Expr::Call {
            callee, arguments, ..
        } => {
            visit_expr(callee, visitor);
            for argument in arguments.iter_mut() {
                visit_expr(&mut argument.value, visitor);
            }
            visitor.rewrite_out_call(expr);
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expression(value) => visit_expr(value, visitor),
            LambdaBody::Block(block) => visit_block(block, visitor),
        },
        Expr::If(value) => visit_if(value, visitor),
        Expr::Match { value, arms, .. } => {
            visit_expr(value, visitor);
            visit_arms(arms, visitor);
        }
        Expr::Block(block) => visit_block(block, visitor),
    }
}

struct Visitor<'a, 's> {
    semantics: &'s SemanticResult,
    generator: &'a mut Generator<'s>,
    sources: &'a mut SourceManager,
    span_source: Option<SourceId>,
    next_span: usize,
    rewritten: bool,
}

impl Visitor<'_, '_> {
    fn span(&mut self) -> Span {
        if self.span_source.is_none() {
            self.span_source = Some(self.sources.add_text("<display sites>", String::new()));
        }
        let source = self.span_source.unwrap();
        let start = self.next_span;
        self.next_span += 1;
        Span::new(source, start, start)
    }

    fn type_of(&self, expr: &Expr) -> Option<TypeId> {
        self.semantics.expression_types.get(&expr.span()).copied()
    }

    /// Wraps `value` in a call to its display function, unless it is `str`.
    fn wrap(&mut self, ty: TypeId, value: Expr) -> Expr {
        let name = self.generator.function_name(ty);
        let plain = self.generator.shape(ty) == Shape::Plain;
        self.rewritten = true;
        let span = self.span();
        if plain {
            // A placeholder conversion for a type without a textual form takes
            // no argument.
            return Expr::Call {
                callee: Box::new(Expr::Name(Name { text: name, span })),
                arguments: Vec::new(),
                span,
            };
        }
        Expr::Call {
            callee: Box::new(Expr::Name(Name { text: name, span })),
            arguments: vec![Argument {
                name: None,
                value,
                spread: false,
                span,
            }],
            span,
        }
    }

    /// Rewrites `out(args...)` / `print(args...)` so its single `str` argument
    /// is the space-joined display of every original argument.
    fn rewrite_out_call(&mut self, expr: &mut Expr) {
        let Expr::Call {
            callee, arguments, ..
        } = expr
        else {
            return;
        };
        let Expr::Name(name) = callee.as_ref() else {
            return;
        };
        if !matches!(name.text.as_str(), "out" | "print") {
            return;
        }
        let mut joined: Option<Expr> = None;
        for argument in arguments.iter() {
            let Some(ty) = self.type_of(&argument.value) else {
                return;
            };
            let displayed = if is_str(self.semantics, ty) {
                argument.value.clone()
            } else {
                self.wrap(ty, argument.value.clone())
            };
            joined = Some(match joined {
                None => displayed,
                Some(previous) => {
                    let space_span = self.span();
                    let space = string_literal(" ", space_span);
                    let add_span = self.span();
                    Expr::Binary {
                        left: Box::new(previous),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Binary {
                            left: Box::new(space),
                            op: BinaryOp::Add,
                            right: Box::new(displayed),
                            span: add_span,
                        }),
                        span: add_span,
                    }
                }
            });
        }
        let joined = joined.unwrap_or_else(|| {
            let span = self.span();
            string_literal("", span)
        });
        let span = self.span();
        arguments.clear();
        arguments.push(Argument {
            name: None,
            value: joined,
            spread: false,
            span,
        });
        self.rewritten = true;
    }
}

fn string_literal(text: &str, span: Span) -> Expr {
    Expr::String {
        segments: if text.is_empty() {
            vec![TemplateSegment::Text {
                text: String::new(),
                span,
            }]
        } else {
            vec![TemplateSegment::Text {
                text: text.to_owned(),
                span,
            }]
        },
        span,
    }
}

fn is_str(semantics: &SemanticResult, ty: TypeId) -> bool {
    matches!(semantics.types[ty.0], Type::Str)
}

/// Template interpolation keeps the existing scalar formatting for numbers,
/// `bool`, and optionals; only aggregates need the generated display.
fn needs_display(semantics: &SemanticResult, ty: TypeId) -> bool {
    matches!(
        semantics.types[ty.0],
        Type::Data(_)
            | Type::Enum(_)
            | Type::List(_)
            | Type::Array(_, _)
            | Type::Result(_, _)
            | Type::Bytes
    )
}

/// A function parameter shape for a display helper.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Takes `value: T` and formats it.
    Value,
    /// Takes no parameter; prints a fixed placeholder.
    Plain,
}

struct Generator<'a> {
    semantics: &'a SemanticResult,
    resolution: &'a Resolution,
    worklist: Vec<(String, TypeId)>,
    seen: BTreeSet<String>,
}

impl<'a> Generator<'a> {
    fn new(semantics: &'a SemanticResult, resolution: &'a Resolution) -> Self {
        Self {
            semantics,
            resolution,
            worklist: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    /// Registers the display function for `ty` (and the shared string helper)
    /// and returns its name.
    fn function_name(&mut self, ty: TypeId) -> String {
        if matches!(self.semantics.types[ty.0], Type::Str) {
            self.seen.insert(STR_HELPER.to_owned());
            return STR_HELPER.to_owned();
        }
        let name = format!("__vut_disp_{}", self.mangle(ty));
        if self.seen.insert(name.clone()) {
            self.worklist.push((name.clone(), ty));
        }
        name
    }

    fn shape(&self, ty: TypeId) -> Shape {
        match self.semantics.types[ty.0] {
            Type::Int
            | Type::Float
            | Type::Numeric(_)
            | Type::Bool
            | Type::Bytes
            | Type::List(_)
            | Type::Array(_, _)
            | Type::Data(_)
            | Type::Enum(_)
            | Type::Result(_, _) => Shape::Value,
            _ => Shape::Plain,
        }
    }

    fn generate(&mut self) -> String {
        let mut source = String::new();
        while let Some((name, ty)) = self.worklist.pop() {
            self.emit_function(&name, ty, &mut source);
        }
        // The string helper is discovered while emitting aggregate bodies, so
        // emit it after the worklist has been drained.
        if self.seen.contains(STR_HELPER) {
            let _ = write!(
                source,
                "fn {STR_HELPER}(value: str) -> str:\n  34.to_char() + value + 34.to_char()\n\n"
            );
        }
        source
    }

    fn emit_function(&mut self, name: &str, ty: TypeId, source: &mut String) {
        let shape = self.shape(ty);
        if shape == Shape::Plain {
            let literal = placeholder(&self.semantics.types[ty.0]);
            let _ = write!(source, "fn {name}() -> str:\n  \"{literal}\"\n\n");
            return;
        }
        let parameter = self.type_syntax(ty);
        let body = self.body(ty);
        let _ = write!(source, "fn {name}(value: {parameter}) -> str:\n{body}\n\n");
    }

    fn body(&mut self, ty: TypeId) -> String {
        match self.semantics.types[ty.0].clone() {
            Type::Int | Type::Float | Type::Numeric(_) => "  value.to_str()".to_owned(),
            Type::Bool => "  text = \"false\"\n  if value:\n    text = \"true\"\n  text".to_owned(),
            Type::Bytes => match self.list_u8() {
                Some(list) => {
                    let helper = self.function_name(list);
                    format!("  {helper}(value.to_list())")
                }
                None => "  \"<bytes>\"".to_owned(),
            },
            Type::List(element) | Type::Array(element, _) => self.sequence_body(element),
            Type::Data(symbol) => self.data_body(symbol),
            Type::Enum(symbol) => self.enum_body(symbol),
            Type::Result(ok, err) => self.result_body(ok, err),
            _ => format!("  \"{}\"", placeholder(&self.semantics.types[ty.0])),
        }
    }

    fn sequence_body(&mut self, element: TypeId) -> String {
        let display = self.display_call(element, "item");
        format!(
            "  result = \"[\"\n  first = true\n  for item in value:\n    if first:\n      first = false\n    else:\n      result = result + \", \"\n    result = result + {display}\n  result + \"]\""
        )
    }

    fn data_body(&mut self, symbol: vut_resolver::SymbolId) -> String {
        let name = self.resolution.symbols[symbol.0].name.clone();
        let fields = self
            .semantics
            .data_fields
            .get(&symbol)
            .cloned()
            .unwrap_or_default();
        if fields.is_empty() {
            return format!("  \"{name}()\"");
        }
        let pieces: Vec<String> = fields
            .iter()
            .map(|field| {
                let display = self.display_call(field.ty, &format!("value.{}", field.name));
                format!("{} = $({display})", field.name)
            })
            .collect();
        format!("  \"{name}({})\"", pieces.join(", "))
    }

    fn enum_body(&mut self, symbol: vut_resolver::SymbolId) -> String {
        let variants = self
            .semantics
            .enum_variants
            .get(&symbol)
            .cloned()
            .unwrap_or_default();
        let mut arms = Vec::new();
        for variant in &variants {
            if variant.fields.is_empty() {
                arms.push(format!("    {}: \"{}\"", variant.name, variant.name));
                continue;
            }
            // Bind each payload field by its declared name.
            let bindings: Vec<String> = variant
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    if field.name.is_empty() || field.name == "_" {
                        format!("field{index}")
                    } else {
                        field.name.clone()
                    }
                })
                .collect();
            let mut pieces = vec![format!("\"{}(\"", variant.name)];
            for (index, field) in variant.fields.iter().enumerate() {
                if index > 0 {
                    pieces.push("\", \"".to_owned());
                }
                pieces.push(self.display_call(field.ty, &bindings[index]));
            }
            pieces.push("\")\"".to_owned());
            arms.push(format!(
                "    {}({}): {}",
                variant.name,
                bindings.join(", "),
                pieces.join(" + ")
            ));
        }
        format!("  match value:\n{}", arms.join("\n"))
    }

    fn result_body(&mut self, ok: TypeId, err: TypeId) -> String {
        let ok_display = self.display_call(ok, "inner");
        let err_display = self.display_call(err, "error");
        format!(
            "  match value:\n    ok(inner): \"ok(\" + {ok_display} + \")\"\n    err(error): \"err(\" + {err_display} + \")\""
        )
    }

    fn display_call(&mut self, ty: TypeId, expr: &str) -> String {
        if matches!(self.semantics.types[ty.0], Type::Str) {
            self.seen.insert(STR_HELPER.to_owned());
            return format!("{STR_HELPER}({expr})");
        }
        let name = self.function_name(ty);
        if self.shape(ty) == Shape::Plain {
            format!("{name}()")
        } else {
            format!("{name}({expr})")
        }
    }

    /// The interned `list(u8)` type used by `bytes.to_list()`.
    fn list_u8(&self) -> Option<TypeId> {
        self.semantics
            .types
            .iter()
            .position(|ty| {
                matches!(
                    ty,
                    Type::List(element)
                        if matches!(&self.semantics.types[element.0], Type::Numeric(name) if name == "u8")
                )
            })
            .map(TypeId)
    }

    fn type_syntax(&self, ty: TypeId) -> String {
        match self.semantics.types[ty.0].clone() {
            Type::Float => "float".to_owned(),
            Type::Bool => "bool".to_owned(),
            Type::Str => "str".to_owned(),
            Type::Bytes => "bytes".to_owned(),
            Type::Void => "void".to_owned(),
            Type::Dyn => "dyn".to_owned(),
            Type::Numeric(name) => name,
            Type::List(element) => format!("list({})", self.type_syntax(element)),
            Type::Array(element, length) => {
                format!("array({}, {length})", self.type_syntax(element))
            }
            Type::Data(symbol) | Type::Enum(symbol) => {
                self.resolution.symbols[symbol.0].name.clone()
            }
            Type::Result(ok, err) => {
                format!(
                    "result({}, {})",
                    self.type_syntax(ok),
                    self.type_syntax(err)
                )
            }
            _ => "int".to_owned(),
        }
    }

    fn mangle(&self, ty: TypeId) -> String {
        match self.semantics.types[ty.0].clone() {
            Type::Int => "int".to_owned(),
            Type::Float => "float".to_owned(),
            Type::Bool => "bool".to_owned(),
            Type::Str => "str".to_owned(),
            Type::Bytes => "bytes".to_owned(),
            Type::Dyn => "dyn".to_owned(),
            Type::Void => "void".to_owned(),
            Type::Numeric(name) => format!("num_{}", sanitize(&name)),
            Type::Optional(inner) => format!("opt_{}", self.mangle(inner)),
            Type::Result(ok, err) => format!("res_{}_{}", self.mangle(ok), self.mangle(err)),
            Type::List(element) => format!("list_{}", self.mangle(element)),
            Type::Array(element, length) => format!("arr_{}_{}", self.mangle(element), length),
            Type::Map(key, value) => format!("map_{}_{}", self.mangle(key), self.mangle(value)),
            Type::Pointer(inner) => format!("ptr_{}", self.mangle(inner)),
            Type::Vutcon(inner) => format!("vutcon_{}", self.mangle(inner)),
            Type::Future(inner) => format!("future_{}", self.mangle(inner)),
            Type::Resource(inner) => format!("resource_{}", self.mangle(inner)),
            Type::Data(symbol) => format!("d{}", symbol.0),
            Type::Enum(symbol) => format!("e{}", symbol.0),
            Type::Applied(symbol, args) => {
                let mut name = format!("a{}", symbol.0);
                for argument in args {
                    name.push('_');
                    name.push_str(&self.mangle(argument));
                }
                name
            }
            Type::Interface(symbol) => format!("i{}", symbol.0),
            Type::Param(symbol) => format!("p{}", symbol.0),
            Type::Function(_) | Type::Callable { .. } | Type::FunctionPointer { .. } => {
                "fn".to_owned()
            }
            Type::SelfType => "self".to_owned(),
            Type::Range(inner) => format!("range_{}", self.mangle(inner)),
            Type::Error => "error".to_owned(),
            Type::Null => "null".to_owned(),
            Type::Variadic(inner) => format!("variadic_{}", self.mangle(inner)),
        }
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn placeholder(ty: &Type) -> &'static str {
    match ty {
        Type::Optional(_) => "<optional>",
        Type::Map(_, _) => "<map>",
        Type::Function(_) | Type::Callable { .. } | Type::FunctionPointer { .. } => "<function>",
        Type::Vutcon(_) => "<vutcon>",
        Type::Future(_) => "<future>",
        Type::Resource(_) => "<resource>",
        Type::Pointer(_) => "<ptr>",
        Type::Interface(_) | Type::Dyn => "<dyn>",
        Type::Range(_) => "<range>",
        Type::Null => "null",
        _ => "<value>",
    }
}
