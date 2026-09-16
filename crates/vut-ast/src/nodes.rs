//! Syntax-oriented, span-preserving Vut abstract syntax tree.
use vut_source::{SourceId, Span};

#[derive(Clone, Debug)]
pub struct File {
    pub source: SourceId,
    pub items: Vec<Item>,
}
#[derive(Clone, Debug)]
pub enum Item {
    Import(Import),
    Function(Function),
    Method(Method),
    ExternFunction(ExternFunction),
    Data(Data),
    Interface(Interface),
    Enum(Enum),
    TypeAlias(TypeAlias),
    Statement(Stmt),
}
#[derive(Clone, Debug)]
pub struct Name {
    pub text: String,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: Name,
    pub ty: TypeExpr,
    /// `...name: T`: accepts zero or more arguments of type `T`.
    pub variadic: bool,
    pub span: Span,
}
/// A declared generic type parameter: `T` or `T: Comparable`.
#[derive(Clone, Debug)]
pub struct TypeParameter {
    pub name: Name,
    pub bounds: Vec<TypeExpr>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct LambdaParameter {
    pub name: Name,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}
#[derive(Clone, Debug, Default)]
pub enum LambdaReceiver {
    /// An ordinary lambda with no receiver.
    #[default]
    None,
    /// A trailing `Call():` / `Call(args) (params):` body. Its receiver type is
    /// inferred from the callee's expected receiver-function type.
    Inferred,
}
#[derive(Clone, Debug)]
pub enum LambdaBody {
    Expression(Box<Expr>),
    Block(Block),
}
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: Name,
    pub args: Vec<AttributeArg>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub enum AttributeArg {
    Ident(Name),
    String { value: String, span: Span },
    Integer { text: String, span: Span },
    Bool { value: bool, span: Span },
}
impl AttributeArg {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(name) => name.span,
            Self::String { span, .. } | Self::Integer { span, .. } | Self::Bool { span, .. } => {
                *span
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct Function {
    pub attributes: Vec<Attribute>,
    pub is_async: bool,
    pub name: Name,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Method {
    pub attributes: Vec<Attribute>,
    pub is_async: bool,
    /// `static fn Type.name(...)`: an associated function with no `self`.
    pub is_static: bool,
    pub receiver: TypeExpr,
    pub name: Name,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct ExternFunction {
    pub attributes: Vec<Attribute>,
    pub abi: String,
    pub abi_span: Span,
    /// `extern "C" async fn`: calling it starts a native async operation and
    /// yields a `future(T)`.
    pub is_async: bool,
    pub name: Name,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Data {
    pub attributes: Vec<Attribute>,
    pub opaque: bool,
    pub name: Name,
    pub type_parameters: Vec<TypeParameter>,
    pub fields: Vec<DataField>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct DataField {
    pub name: Name,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Interface {
    pub attributes: Vec<Attribute>,
    pub name: Name,
    pub type_parameters: Vec<TypeParameter>,
    pub parents: Vec<Name>,
    pub methods: Vec<Signature>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Signature {
    pub name: Name,
    /// `static name(...)`: an associated requirement with no implicit `self`.
    pub is_static: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Enum {
    pub attributes: Vec<Attribute>,
    pub name: Name,
    pub type_parameters: Vec<TypeParameter>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}
/// A single `enum` variant, optionally carrying named payload fields.
#[derive(Clone, Debug)]
pub struct EnumVariant {
    pub name: Name,
    pub fields: Vec<VariantField>,
    pub span: Span,
}
/// A named payload field of an enum variant: `circle(radius: float)`.
#[derive(Clone, Debug)]
pub struct VariantField {
    pub name: Name,
    pub ty: TypeExpr,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub attributes: Vec<Attribute>,
    pub name: Name,
    pub ty: TypeExpr,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Import {
    pub relative: bool,
    pub parent_depth: usize,
    pub path: Vec<Name>,
    pub mode: ImportMode,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub enum ImportMode {
    Whole,
    Alias(Name),
    Selected(Vec<Name>),
}
#[derive(Clone, Debug)]
pub enum TypeExpr {
    Named {
        path: Vec<Name>,
        span: Span,
    },
    Applied {
        name: Name,
        arguments: Vec<TypeExpr>,
        span: Span,
    },
    Array {
        element: Box<TypeExpr>,
        length: usize,
        length_span: Span,
        span: Span,
    },
    Optional {
        inner: Box<TypeExpr>,
        span: Span,
    },
    Function {
        /// `fn(Receiver)(Args...) -> T`: a receiver is a distinct part of the
        /// function type and is not an ordinary parameter.
        receiver: Option<Box<TypeExpr>>,
        parameters: Vec<TypeExpr>,
        return_type: Option<Box<TypeExpr>>,
        span: Span,
    },
    ExternFunction {
        abi: String,
        abi_span: Span,
        parameters: Vec<TypeExpr>,
        return_type: Option<Box<TypeExpr>>,
        span: Span,
    },
    Error(Span),
}
impl TypeExpr {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Named { span, .. }
            | Self::Applied { span, .. }
            | Self::Array { span, .. }
            | Self::Optional { span, .. }
            | Self::Function { span, .. }
            | Self::ExternFunction { span, .. }
            | Self::Error(span) => *span,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Stmt {
    Binding {
        target: Expr,
        annotation: Option<TypeExpr>,
        value: Expr,
        span: Span,
    },
    Expression(Expr),
    If(If),
    For(For),
    Unsafe(Block),
    Break(Span),
    Continue(Span),
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Error(Span),
}
#[derive(Clone, Debug)]
pub struct If {
    pub condition: Expr,
    pub body: Block,
    pub elifs: Vec<(Expr, Block)>,
    pub otherwise: Option<Block>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct For {
    pub kind: ForKind,
    pub body: Block,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub enum ForKind {
    Infinite,
    Conditional(Expr),
    Iterable {
        value: Name,
        index: Option<Name>,
        iterable: Expr,
    },
}
#[derive(Clone, Debug)]
pub enum Expr {
    Name(Name),
    Integer {
        text: String,
        span: Span,
    },
    Float {
        text: String,
        span: Span,
    },
    String {
        segments: Vec<TemplateSegment>,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Null(Span),
    List {
        values: Vec<Expr>,
        span: Span,
    },
    Array {
        values: Vec<Expr>,
        span: Span,
    },
    Map {
        entries: Vec<MapEntry>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        member: Name,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Argument>,
        span: Span,
    },
    Lambda {
        receiver: LambdaReceiver,
        parameters: Vec<LambdaParameter>,
        body: LambdaBody,
        /// `true` for an anonymous `async fn(): ...` callable.
        is_async: bool,
        span: Span,
    },
    ResultOk {
        value: Box<Expr>,
        span: Span,
    },
    ResultErr {
        value: Box<Expr>,
        span: Span,
    },
    ResultPropagate {
        value: Box<Expr>,
        span: Span,
    },
    Await {
        value: Box<Expr>,
        span: Span,
    },
    Spawn {
        callable: Box<Expr>,
        span: Span,
    },
    If(Box<If>),
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Group {
        value: Box<Expr>,
        span: Span,
    },
    /// An indented block used as an expression. Its value is the block's final
    /// expression, or `void` when the block has no final expression.
    Block(Block),
    Error(Span),
}
#[derive(Clone, Debug)]
pub enum TemplateSegment {
    Text { text: String, span: Span },
    Expression(Expr),
}
#[derive(Clone, Debug)]
pub struct Argument {
    pub name: Option<Name>,
    pub value: Expr,
    /// `...value`: spreads a sequence into a variadic parameter.
    pub spread: bool,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct MapEntry {
    pub key: Expr,
    pub value: Expr,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<Expr>,
    pub value: Expr,
    pub span: Span,
}
/// A recursive pattern used by `match`.
#[derive(Clone, Debug)]
pub enum MatchPattern {
    /// `_`
    Wildcard(Span),
    /// A lowercase binding pattern: `value`
    Binding(Name),
    /// A literal constant pattern.
    Literal {
        value: LiteralPattern,
        span: Span,
    },
    /// An enum variant pattern: `point` or `circle(radius = r)`.
    Variant {
        name: Name,
        fields: Vec<FieldPattern>,
        span: Span,
    },
    /// `ok(pattern)`
    ResultOk {
        pattern: Box<MatchPattern>,
        span: Span,
    },
    /// `err(pattern)`
    ResultErr {
        pattern: Box<MatchPattern>,
        span: Span,
    },
    /// `left or right`
    Or {
        alternatives: Vec<MatchPattern>,
        span: Span,
    },
    /// `0..10` / `0..=10`
    Range {
        start: LiteralPattern,
        end: LiteralPattern,
        inclusive: bool,
        span: Span,
    },
    /// `@(a, b, rest..)` / `@()`
    List {
        items: Vec<MatchPattern>,
        rest: Option<Name>,
        span: Span,
    },
    /// `(pattern)`
    Group {
        pattern: Box<MatchPattern>,
        span: Span,
    },
    Error(Span),
}
impl MatchPattern {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Wildcard(span)
            | Self::Literal { span, .. }
            | Self::Variant { span, .. }
            | Self::ResultOk { span, .. }
            | Self::ResultErr { span, .. }
            | Self::Or { span, .. }
            | Self::Range { span, .. }
            | Self::List { span, .. }
            | Self::Group { span, .. }
            | Self::Error(span) => *span,
            Self::Binding(name) => name.span,
        }
    }
}
/// One argument inside a variant pattern.
#[derive(Clone, Debug)]
pub enum FieldPattern {
    /// `field = pattern`
    Named { field: Name, pattern: MatchPattern },
    /// Positional shorthand `circle(r)` for single-field variants.
    Positional(MatchPattern),
}
impl FieldPattern {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Named { field, .. } => field.span,
            Self::Positional(pattern) => pattern.span(),
        }
    }
    #[must_use]
    pub fn pattern(&self) -> &MatchPattern {
        match self {
            Self::Named { pattern, .. } | Self::Positional(pattern) => pattern,
        }
    }
}
/// A compile-time literal usable in a pattern.
#[derive(Clone, Debug)]
pub enum LiteralPattern {
    Integer(String),
    Float(String),
    Bool(bool),
    Str(String),
    Null,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
    Positive,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    RangeExclusive,
    RangeInclusive,
}
impl Expr {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Name(x) => x.span,
            Self::Integer { span, .. }
            | Self::Float { span, .. }
            | Self::String { span, .. }
            | Self::Bool { span, .. }
            | Self::List { span, .. }
            | Self::Array { span, .. }
            | Self::Map { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Member { span, .. }
            | Self::Call { span, .. }
            | Self::Lambda { span, .. }
            | Self::ResultOk { span, .. }
            | Self::ResultErr { span, .. }
            | Self::ResultPropagate { span, .. }
            | Self::Await { span, .. }
            | Self::Spawn { span, .. }
            | Self::Match { span, .. }
            | Self::Group { span, .. }
            | Self::Error(span)
            | Self::Null(span) => *span,
            Self::If(value) => value.span,
            Self::Block(block) => block.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_receiver_and_name_are_structurally_separate() {
        let source = SourceId::from_index(0);
        let receiver_name = Name {
            text: "Counter".into(),
            span: Span::new(source, 3, 10),
        };
        let method = Method {
            attributes: Vec::new(),
            is_async: false,
            is_static: false,
            receiver: TypeExpr::Named {
                path: vec![receiver_name],
                span: Span::new(source, 3, 10),
            },
            name: Name {
                text: "increment".into(),
                span: Span::new(source, 11, 20),
            },
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: None,
            body: Block {
                statements: Vec::new(),
                span: Span::new(source, 23, 23),
            },
            span: Span::new(source, 0, 23),
        };
        let TypeExpr::Named { path, .. } = &method.receiver else {
            panic!("method receiver must be a named type")
        };
        assert_eq!(path[0].text, "Counter");
        assert_eq!(method.name.text, "increment");
    }
}
