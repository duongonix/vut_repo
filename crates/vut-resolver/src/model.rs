use std::{collections::HashMap, path::PathBuf};
use vut_ast::File;
use vut_source::{SourceId, Span};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleId(pub usize);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SymbolId(pub usize);
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MethodKey {
    pub receiver: SymbolId,
    pub name: String,
}
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModulePath(pub Vec<String>);
impl ModulePath {
    #[must_use]
    pub fn display(&self) -> String {
        self.0.join(".")
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Module,
    Function,
    Method,
    AnonymousFunction,
    Data,
    Interface,
    Enum,
    TypeAlias,
    TypeParameter,
    Binding,
    Parameter,
    Local,
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub id: SymbolId,
    pub module: ModuleId,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub public: bool,
    pub target_module: Option<ModuleId>,
    pub receiver: Option<SymbolId>,
    /// `static fn Type.name`: an associated function with no implicit `self`.
    pub is_static: bool,
    /// True when the declaration has generic type parameters, so `name[...]` is
    /// a generic application rather than a collection access.
    pub generic: bool,
}
#[derive(Clone, Debug)]
pub struct ModuleInput {
    pub logical_path: ModulePath,
    pub filesystem_path: Option<PathBuf>,
    pub file: File,
}
#[derive(Debug)]
pub struct Module {
    pub id: ModuleId,
    pub logical_path: ModulePath,
    pub filesystem_path: Option<PathBuf>,
    pub source: SourceId,
    pub file: File,
    pub symbols: HashMap<String, SymbolId>,
    pub imports: HashMap<String, SymbolId>,
    pub methods: HashMap<MethodKey, SymbolId>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedReference {
    pub span: Span,
    pub symbol: SymbolId,
}
/// A bare name inside a receiver body that resolves to the current receiver's
/// method, e.g. `Button("Save")` → `self.Button("Save")`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImplicitReceiverMethod {
    pub receiver: SymbolId,
    pub method: SymbolId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LambdaCapture {
    /// Name the closure body uses for the captured binding.
    pub name: String,
    /// Symbol of the enclosing local/parameter that is captured.
    pub symbol: SymbolId,
}
#[derive(Clone, Debug)]
pub struct LambdaInfo {
    pub symbol: SymbolId,
    pub module: ModuleId,
    pub parameters: Vec<vut_ast::LambdaParameter>,
    pub body: vut_ast::LambdaBody,
    pub is_async: bool,
    /// Receiver type inferred from the callee's expected receiver function type
    /// for a trailing `Call():` body.
    pub receiver: Option<SymbolId>,
    /// Enclosing locals/parameters captured by the closure, in first-use order.
    pub captures: Vec<LambdaCapture>,
    pub span: Span,
}
#[derive(Debug)]
pub struct Resolution {
    pub modules: Vec<Module>,
    pub symbols: Vec<Symbol>,
    pub references: Vec<ResolvedReference>,
    pub graph: Vec<Vec<ModuleId>>,
    pub compile_order: Vec<ModuleId>,
    pub diagnostics: vut_diagnostics::DiagnosticSink,
    /// Builtin nominal `Utf8Error` data type shared by every module.
    pub builtin_utf8_error: SymbolId,
    /// Builtin nominal `HexError` data type shared by every module.
    pub builtin_hex_error: SymbolId,
    /// Anonymous functions created by lambda expressions, in resolution order.
    pub lambdas: Vec<LambdaInfo>,
    /// Maps each lambda expression span to its lifted anonymous symbol.
    pub lambda_symbols: std::collections::HashMap<Span, SymbolId>,
    /// Implicit receiver method calls keyed by the callee name span.
    pub implicit_receiver_methods: HashMap<Span, ImplicitReceiverMethod>,
}
