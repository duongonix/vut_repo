//! Static type, function, data, and control-flow analysis for Vut.
use crate::attributes::{self, AttributeSemantics, Repr};
use std::collections::{HashMap, HashSet};
use vut_ast::{
    BinaryOp, Block, Expr, ForKind, Item, LambdaBody, LambdaParameter, LambdaReceiver,
    LiteralPattern, MatchPattern, Stmt, TypeExpr, UnaryOp,
};
use vut_diagnostics::{Diagnostic, DiagnosticSink, codes};
use vut_hir::TypeId;
pub use vut_interface::{InterfaceMethod, InterfaceShape, Satisfaction};
use vut_resolver::{MethodKey, Module, ModuleId, Resolution, SymbolId};
use vut_source::Span;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Type {
    Error,
    Void,
    /// The single zero-sized source-language value, distinct from no return.
    Unit,
    Bool,
    Int,
    Float,
    Str,
    Bytes,
    Dyn,
    Null,
    /// A declared generic type parameter, identified by its resolver symbol.
    Param(SymbolId),
    /// `Self` inside an interface requirement; substituted with the
    /// implementing type when the interface is satisfied.
    SelfType,
    Numeric(String),
    Optional(TypeId),
    Result(TypeId, TypeId),
    /// A typed handle for a lightweight concurrent execution unit.
    Vutcon(TypeId),
    /// A typed, scheduler-aware message channel (`channel[T]`).
    Channel(TypeId),
    /// A native asynchronous computation produced by an `extern "C" async fn`.
    Future(TypeId),
    /// An owned, move-only opaque native resource handle (`resource[T]`).
    Resource(TypeId),
    List(TypeId),
    /// A read-only view of the arguments bound by a variadic parameter
    /// (`...values: T`). Iterable, with `len()`/`at(i)`; never owned.
    Variadic(TypeId),
    Pointer(TypeId),
    FunctionPointer {
        abi: String,
        parameters: Vec<TypeId>,
        result: TypeId,
    },
    Array(TypeId, usize),
    Map(TypeId, TypeId),
    Data(SymbolId),
    Enum(SymbolId),
    /// A generic data/enum application that still mentions type parameters,
    /// e.g. `Box(T)` inside a generic declaration.
    Applied(SymbolId, Vec<TypeId>),
    Interface(SymbolId),
    Function(SymbolId),
    Callable {
        /// `fn(Receiver)(Args...) -> T`: the receiver is a distinct part of the
        /// function type and is not an ordinary parameter.
        receiver: Option<TypeId>,
        parameters: Vec<TypeId>,
        result: TypeId,
    },
    Range(TypeId),
}
#[derive(Debug)]
pub struct SemanticResult {
    pub types: Vec<Type>,
    /// Canonical alias targets, shared with generated syntax and lowering.
    pub type_aliases: HashMap<SymbolId, TypeId>,
    pub expression_types: HashMap<Span, TypeId>,
    pub diagnostics: DiagnosticSink,
    pub interface_shapes: HashMap<SymbolId, InterfaceShape>,
    pub interface_satisfaction: HashMap<(TypeId, SymbolId), Satisfaction>,
    /// Fully resolved data fields, consumed by MIR layout without reaching back into AST.
    pub data_fields: HashMap<SymbolId, Vec<DataFieldInfo>>,
    /// Default field expressions per data type, aligned with `data_fields` order.
    /// The semantic layer has already resolved names and type-checked each
    /// expression, so MIR lowers them as ordinary expressions.
    pub data_field_defaults: HashMap<SymbolId, Vec<Option<vut_ast::Expr>>>,
    /// Default parameter expressions per function/method, aligned with the
    /// resolved parameter order. Callers may omit a trailing parameter that has
    /// a default; the semantic layer has already resolved and type-checked it.
    pub parameter_defaults: HashMap<SymbolId, Vec<Option<vut_ast::Expr>>>,
    /// The reading chosen for every `object[...]` postfix: a generic application
    /// or a collection access. MIR lowers each subscript accordingly.
    pub subscripts: HashMap<Span, SubscriptKind>,
    pub opaque_data: HashSet<SymbolId>,
    /// Initializer expression recorded for each module-constant reference span.
    /// MIR inlines the initializer at the reference (module constants are
    /// immutable scalar values with no addressable storage).
    pub constant_references: HashMap<Span, vut_ast::Expr>,
    pub function_signatures: HashMap<SymbolId, FunctionSignatureInfo>,
    pub builtin_functions: HashMap<SymbolId, BuiltinFunction>,
    pub builtin_calls: HashMap<Span, BuiltinFunction>,
    pub call_targets: HashMap<Span, SymbolId>,
    /// Call spans that invoke a receiver function via `.call(receiver, ...args)`.
    pub receiver_calls: HashSet<Span>,
    /// Callee name spans that resolve to the enclosing receiver's method, e.g.
    /// `Button("Save")` inside a `fn(ColumnScope)() -> void` body.
    pub implicit_receiver_calls: HashSet<Span>,
    /// Deferred bound method calls on generic type parameters, keyed by call
    /// span. Monomorphization resolves each into a concrete `call_targets`
    /// entry for the specialization; no runtime dispatch is introduced.
    pub bound_calls: HashMap<Span, BoundCall>,
    pub async_symbols: HashSet<SymbolId>,
    /// Symbols declared `extern "C" async fn`; calling one yields `future[T]`.
    pub async_externs: HashSet<SymbolId>,
    /// Symbols declared as `extern`; their parameters are borrowed by native
    /// code, so callers must keep ownership of managed arguments.
    pub extern_symbols: HashSet<SymbolId>,
    /// Resolved enum variants (name + payload field types) keyed by enum symbol.
    pub enum_variants: HashMap<SymbolId, Vec<VariantInfo>>,
    /// Variant-construction call sites, consumed by MIR lowering.
    pub variant_constructions: HashMap<Span, VariantConstruction>,
    /// Concrete method resolution keyed by `(receiver type symbol, method name)`,
    /// consumed by interface vtable construction.
    pub method_symbols: HashMap<(SymbolId, String), SymbolId>,
    pub attributes: AttributeSemantics,
    /// Symbols declared `static fn Type.name` (associated, no `self`).
    pub static_methods: HashSet<SymbolId>,
    /// Captured bindings of each lambda, keyed by the lambda expression span, in
    /// first-use order. The type is the enclosing binding's type at creation.
    pub closure_captures: HashMap<Span, Vec<(String, TypeId)>>,
    /// Ordered type-parameter symbols for each generic declaration.
    pub generic_params: HashMap<SymbolId, Vec<SymbolId>>,
    /// Interface bounds declared for each type parameter.
    pub generic_param_bounds: HashMap<SymbolId, Vec<SymbolId>>,
    /// Generic function calls with a fully concrete type-argument list.
    pub generic_function_calls: Vec<GenericCall>,
    /// Generic data/enum applications with a fully concrete type-argument list.
    pub generic_type_applications: Vec<GenericApplication>,
    /// For each concrete `(template, arguments)` specialization, the mapping of
    /// every template `TypeId` to its substituted concrete form.
    pub generic_substitutions: HashMap<(SymbolId, Vec<TypeId>), HashMap<TypeId, TypeId>>,
    /// First free synthetic symbol id (above every data/enum instance symbol).
    pub next_instance_symbol: usize,
}
/// A method call resolved through a generic type parameter's interface bound.
///
/// Stored per call span until monomorphization substitutes `receiver` with a
/// concrete type and binds the call to that type's method of the same name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundCall {
    /// The `Type::Param` receiver type at the generic declaration.
    pub receiver: TypeId,
    /// The bound method name.
    pub method: String,
    /// `true` for a static/associated bound call (`T.from_repr(...)`).
    pub is_static: bool,
}
/// A concrete specialization request for a generic function.
#[derive(Clone, Debug)]
pub struct GenericCall {
    pub call_span: Span,
    pub template: SymbolId,
    pub arguments: Vec<TypeId>,
}
/// A concrete specialization request for a generic data/enum application.
#[derive(Clone, Debug)]
pub struct GenericApplication {
    pub span: Span,
    pub template: SymbolId,
    pub arguments: Vec<TypeId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<VariantFieldInfo>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantFieldInfo {
    pub name: String,
    pub ty: TypeId,
}
/// A resolved `Enum.variant(field: value, ...)` construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantConstruction {
    pub enum_symbol: SymbolId,
    pub variant_index: usize,
    /// Field index for each supplied argument, in argument order.
    pub fields: Vec<usize>,
}
pub use numeric::NumericKind;

impl Type {
    /// Canonical numeric metadata shared by semantic analysis and MIR.
    #[must_use]
    pub fn numeric_kind(&self) -> Option<NumericKind> {
        numeric::numeric_kind(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinFunction {
    Print,
    Out,
    Input,
    VariadicLen,
    VariadicAt,
    StringByteLen,
    StringCharLen,
    StringIsEmpty,
    StringContains,
    StringStartsWith,
    StringEndsWith,
    StringTrim,
    StringTrimStart,
    StringTrimEnd,
    StringToLower,
    StringToUpper,
    StringReplace,
    StringFind,
    StringSplit,
    StringSubstring,
    StringLines,
    StringSplitWhitespace,
    StringChars,
    StringCharAt,
    StringRepeat,
    StringPadLeft,
    StringPadRight,
    StringStripPrefix,
    StringStripSuffix,
    StringRfind,
    StringCompare,
    StringEquals,
    StringToI64,
    StringToF64,
    StringToBytes,
    BytesNew,
    BytesClone,
    BytesLen,
    BytesIsEmpty,
    BytesCapacity,
    BytesReserve,
    BytesAt,
    BytesSet,
    BytesFirst,
    BytesLast,
    BytesSlice,
    BytesClear,
    BytesByteAt,
    BytesPush,
    BytesExtend,
    BytesTruncate,
    BytesResize,
    BytesFind,
    BytesStartsWith,
    BytesEndsWith,
    BytesCompare,
    BytesToHex,
    BytesFromHex,
    BytesReadInt {
        width: u8,
        big_endian: bool,
        signed: bool,
    },
    BytesWriteInt {
        width: u8,
        big_endian: bool,
    },
    IntToChar,
    IntToFloat,
    IntAbs,
    IntPow,
    IntMin,
    IntMax,
    IntClamp,
    FloatAbs,
    FloatFloor,
    FloatCeil,
    FloatRound,
    FloatTrunc,
    FloatSqrt,
    FloatPow,
    FloatToInt,
    FloatIsNan,
    FloatIsFinite,
    FloatMin,
    FloatMax,
    FloatClamp,
    /// Fused multiply-add (`x.fma(multiplier, addend)`), lowered to a native
    /// `fma` instruction when the target supports it.
    FloatFma,
    /// Sign copy (`x.copysign(sign)`), lowered to a native instruction.
    FloatCopysign,
    /// Explicit numeric conversion (`value.to_u64()`, `value.to_int()`, ...).
    /// Codegen range-checks and traps on out-of-range values.
    NumericCast {
        from: NumericKind,
        to: NumericKind,
    },
    BoolToStr,
    ResultIsOk,
    ResultIsErr,
    ResultUnwrapOr,
    BytesToList,
    BytesFromList,
    BytesToStr,
    ListNew,
    ListLen,
    /// Internal: the contiguous element buffer of a list (spread). Not exposed
    /// as a user method.
    ListData,
    ListIsEmpty,
    ListCapacity,
    ListReserve,
    ListPush,
    ListAt,
    /// Optimizer-emitted unchecked element access; the index is proven in
    /// bounds, so the runtime bounds check is skipped.
    ListAtUnchecked,
    ListSet,
    ListInsert,
    ListRemove,
    ListClear,
    ListSlice,
    ListContains,
    ListPop,
    ListFirst,
    ListLast,
    ListFindIndex,
    ListExtend,
    ListReverse,
    ListSort,
    ListTruncate,
    ListSwap,
    ListShrinkToFit,
    /// `list[str].join(separator) -> str` (native runtime string building).
    ListJoin,
    /// Compiler-lowered higher-order `list` builtins (see
    /// `checker::collections` and the MIR inline lowering).
    ListMap,
    ListFilter,
    ListAny,
    ListAll,
    ListFold,
    ListFindIndexBy,
    ListSortBy,
    MapNew,
    MapLen,
    MapIsEmpty,
    MapCapacity,
    MapReserve,
    MapGet,
    MapSet,
    MapContainsKey,
    MapRemove,
    MapClear,
    MapKeys,
    MapValues,
    MapGetOr,
    /// `channel[T](capacity:)`: creates a typed channel.
    ChannelNew,
    /// `ch.send(value)`: suspends until the value is handed off.
    ChannelSend,
    /// `ch.recv()`: suspends until a value arrives; yields `T?`.
    ChannelRecv,
    /// `ch.close()`: closes the channel and wakes all waiters.
    ChannelClose,
    NumericToStr,
    ArrayLen,
    ArrayAt,
    /// Optimizer-emitted unchecked array access; the index is proven in bounds.
    ArrayAtUnchecked,
    ArraySet,
    ArrayFirst,
    ArrayLast,
    ArrayFill,
    ArrayToList,
    ArrayContains,
    ArrayReverse,
    ArraySort,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptKind {
    /// `object[types]`: an explicit generic application (`parse[int](x)`).
    Generic,
    /// `object[index]`: a collection access (`values[0]`).
    Index,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignatureInfo {
    /// Receiver type for a receiver function, which is not an ordinary parameter.
    pub receiver: Option<TypeId>,
    pub parameters: Vec<TypeId>,
    /// Element type of the trailing variadic parameter, if any.
    pub variadic: Option<TypeId>,
    pub result: TypeId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataFieldInfo {
    pub name: String,
    pub ty: TypeId,
    pub required: bool,
    pub public: bool,
}
#[derive(Clone)]
struct Signature {
    parameters: Vec<(String, TypeId)>,
    result: TypeId,
}
#[derive(Clone)]
struct Field {
    ty: TypeId,
    required: bool,
    public: bool,
}

pub struct Analyzer<'a> {
    resolution: &'a Resolution,
    pointer_bytes: usize,
    types: Vec<Type>,
    type_ids: HashMap<Type, TypeId>,
    expression_types: HashMap<Span, TypeId>,
    diagnostics: DiagnosticSink,
    signatures: HashMap<SymbolId, Signature>,
    fields: HashMap<SymbolId, HashMap<String, Field>>,
    aliases: HashMap<SymbolId, TypeId>,
    interface_shapes: HashMap<SymbolId, InterfaceShape>,
    satisfaction: HashMap<(TypeId, SymbolId), Satisfaction>,
    call_targets: HashMap<Span, SymbolId>,
    builtin_calls: HashMap<Span, BuiltinFunction>,
    bound_calls: HashMap<Span, BoundCall>,
    /// Element type of each variadic function/method's trailing parameter.
    variadic_functions: HashMap<SymbolId, TypeId>,
    /// Receiver type of each receiver function (anonymous functions with an
    /// inferred receiver). The receiver is not an ordinary parameter.
    receiver_functions: HashMap<SymbolId, TypeId>,
    /// Call spans that invoke a receiver function via `.call(receiver, ...)`.
    receiver_calls: HashSet<Span>,
    /// Callee name spans that resolve to the current receiver's method.
    implicit_receiver_calls: HashSet<Span>,
    extern_symbols: HashSet<SymbolId>,
    async_symbols: HashSet<SymbolId>,
    async_externs: HashSet<SymbolId>,
    async_calls: HashSet<Span>,
    enum_variants: HashMap<SymbolId, Vec<VariantInfo>>,
    variant_constructions: HashMap<Span, VariantConstruction>,
    method_symbols: HashMap<(SymbolId, String), SymbolId>,
    opaque_data: HashSet<SymbolId>,
    attributes: AttributeSemantics,
    builtin_utf8_error: SymbolId,
    /// Builtin nominal `HexError` data type (one `index: int` field).
    builtin_hex_error: SymbolId,
    /// Span → resolved symbol, used to recognize type-parameter references.
    reference_symbols: HashMap<Span, SymbolId>,
    /// Whether `Self` is currently allowed (inside an interface requirement).
    self_type_allowed: bool,
    generic_params: HashMap<SymbolId, Vec<SymbolId>>,
    generic_param_bounds: HashMap<SymbolId, Vec<SymbolId>>,
    generic_function_calls: Vec<GenericCall>,
    generic_type_applications: Vec<GenericApplication>,
    generic_substitutions: HashMap<(SymbolId, Vec<TypeId>), HashMap<TypeId, TypeId>>,
    /// Synthetic symbols for generic data/enum specializations.
    instance_symbols: HashMap<SymbolId, vut_resolver::Symbol>,
    instances: HashMap<(SymbolId, Vec<TypeId>), SymbolId>,
    /// Declaration-order field names for each data type (including instances).
    data_field_order: HashMap<SymbolId, Vec<String>>,
    /// Default parameter expressions per function/method, aligned with the
    /// resolved parameter order (including generic specializations).
    parameter_defaults: HashMap<SymbolId, Vec<Option<vut_ast::Expr>>>,
    /// The chosen reading for each `object[...]` postfix, shared with MIR.
    subscripts: HashMap<Span, SubscriptKind>,
    /// Captured bindings of each lambda, keyed by lambda expression span.
    closure_captures: HashMap<Span, Vec<(String, TypeId)>>,
    /// Module-level constant initializers, keyed by name (current module only).
    module_constants: HashMap<String, vut_ast::Expr>,
    /// Module-level constant initializers, keyed by symbol (all modules).
    constant_initializers: HashMap<SymbolId, vut_ast::Expr>,
    /// Initializer expression recorded for each module-constant reference span.
    constant_references: HashMap<Span, vut_ast::Expr>,
    next_instance: usize,
}

mod analyze;
mod builtins;
mod collections;
mod constants;
mod context;
mod expressions;
mod functions;
mod generics;
mod numeric;
mod pointees;
#[cfg(test)]
mod tests;
mod typing;
