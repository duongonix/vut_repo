//! MIR data types (functions, blocks, instructions, terminators).
use std::collections::HashMap;
pub use vut_ast::BinaryOp;
use vut_hir::TypeId;
pub use vut_resolver::SymbolId;
use vut_source::Span;
pub use vut_types::{BuiltinFunction, NumericKind};

use super::LayoutTable;
use super::future::{AwaitLive, FrameLayout};

macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(pub usize);
    };
}
id!(ValueId);
id!(LocalId);
id!(BlockId);

#[derive(Clone, Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub external_functions: Vec<ExternalFunction>,
    pub layouts: LayoutTable,
    /// Vtables required by interface/`dyn` conversions, keyed by the boxed
    /// concrete type and the target interface (or `None` for `dyn`).
    pub interface_vtables: Vec<InterfaceVtable>,
    /// Ownership diagnostics discovered while lowering (linear moves).
    pub diagnostics: vut_diagnostics::DiagnosticSink,
    /// Future frame layout per async function (B2.2).
    pub frames: HashMap<SymbolId, FrameLayout>,
    /// Locals live across each await per async function (B2.3).
    pub awaits: HashMap<SymbolId, Vec<AwaitLive>>,
    /// Heap environment layout for each capturing closure body, keyed by the
    /// lambda body symbol.
    pub closure_layouts: HashMap<SymbolId, ClosureLayout>,
}
#[derive(Clone, Debug)]
pub struct InterfaceVtable {
    /// The boxed concrete declaration when it is a `data`/`enum` (interface
    /// targets); `None` for `dyn` boxes of other concrete categories.
    pub concrete: Option<SymbolId>,
    pub interface: Option<SymbolId>,
    pub concrete_ty: TypeId,
    /// Concrete method symbols in the interface's canonical (sorted) order.
    pub methods: Vec<SymbolId>,
}
#[derive(Clone, Debug)]
pub struct ExternalFunction {
    pub symbol: SymbolId,
    pub link_name: String,
    pub parameters: Vec<TypeId>,
    pub return_type: Option<TypeId>,
}
#[derive(Clone, Debug)]
pub struct Function {
    pub symbol: SymbolId,
    pub receiver: Option<SymbolId>,
    pub parameter_count: usize,
    pub locals: Vec<Local>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub return_type: Option<TypeId>,
    /// `true` for `async fn`/`async fn Type.method` declarations. The async
    /// lowering pass turns these into future state machines.
    pub is_async: bool,
    /// For an async body, the `LocalId` holding the frame pointer passed to the
    /// resume/poll entry. Declared parameters are read from frame slots.
    pub frame_param: Option<usize>,
    /// `true` when this async body is its own poll entry `(frame, out) -> i32`
    /// (a real state machine). Otherwise a separate resume thunk calls it.
    pub is_poll: bool,
    /// For a poll body, the `LocalId` holding the output buffer pointer.
    pub out_param: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub ty: Option<TypeId>,
    pub span: Span,
    /// Where the local's value lives.
    pub storage: LocalStorage,
}
/// One captured value stored in a closure environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureCapture {
    pub ty: TypeId,
    /// Byte offset of the value inside the environment block.
    pub offset: usize,
}
/// Heap environment layout for a capturing closure body.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClosureLayout {
    pub size: usize,
    pub alignment: usize,
    pub captures: Vec<ClosureCapture>,
}
/// Storage class of a MIR local.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalStorage {
    /// An ordinary (register/stack) local within one poll.
    Stack,
    /// A frame-resident local at `frame + offset`, surviving suspension.
    Frame(usize),
}
#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}
#[derive(Clone, Debug)]
pub enum Instruction {
    ConstNull {
        value: ValueId,
        /// The optional type this `null` inhabits, when known. Managed optionals
        /// are the zero handle; tagged optionals need a zeroed aggregate block.
        ty: Option<TypeId>,
    },
    /// Wraps a present inner value into an optional value of type `ty`.
    OptionalWrap {
        value: ValueId,
        operand: ValueId,
        ty: TypeId,
        inner: TypeId,
    },
    /// Unwraps a present optional value to its inner type.
    OptionalUnwrap {
        value: ValueId,
        operand: ValueId,
        inner: TypeId,
    },
    /// Tests an optional value: nonzero when present.
    OptionalIsPresent {
        value: ValueId,
        operand: ValueId,
        inner: TypeId,
    },
    /// Builds an optional `ty` from a raw inner value plus a presence flag,
    /// without allocating a managed wrapper: a managed inner yields a nullable
    /// handle, a scalar/aggregate inner yields the tagged representation.
    /// `payload` is the address of the value storage, `present` is nonzero when
    /// the value is present.
    OptionalFromValue {
        value: ValueId,
        present: ValueId,
        payload: ValueId,
        ty: TypeId,
        inner: TypeId,
    },
    ConstInt {
        value: ValueId,
        literal: i64,
    },
    ConstFloat {
        value: ValueId,
        literal: f64,
        /// The destination numeric type, so a `f32` literal is emitted as a
        /// 32-bit constant rather than being narrowed at the store site.
        ty: Option<TypeId>,
    },
    ConstBool {
        value: ValueId,
        literal: bool,
    },
    ConstString {
        value: ValueId,
        literal: String,
    },
    FormatValue {
        value: ValueId,
        operand: ValueId,
        ty: TypeId,
        unsigned: bool,
    },
    ConcatString {
        value: ValueId,
        left: ValueId,
        right: ValueId,
    },
    Copy {
        value: ValueId,
        local: LocalId,
    },
    Borrow {
        value: ValueId,
        local: LocalId,
    },
    Move {
        value: ValueId,
        local: LocalId,
    },
    Store {
        local: LocalId,
        value: ValueId,
    },
    Drop(LocalId),
    /// Reads the resume state word from `frame`.
    FrameState {
        value: ValueId,
        frame: LocalId,
    },
    /// Writes the resume state word into `frame`.
    SetFrameState {
        frame: LocalId,
        state: u32,
    },
    /// Stores an in-flight child future handle into `frame` so it survives a
    /// suspension.
    SetFrameChild {
        frame: LocalId,
        value: ValueId,
    },
    /// Assigns `value` to the field `name` of the aggregate held in `base`.
    /// When `release` is set the previous field value is released first; a
    /// copy-on-write replacement clears it because the old reference was already
    /// consumed by `MakeUnique`.
    FieldStore {
        base: LocalId,
        name: String,
        value: ValueId,
        release: bool,
    },
    /// Produces a borrow (address) of the field `name` of the aggregate held in
    /// `base`, without retaining or releasing the field. Used for method
    /// receivers so mutating methods act on the caller's storage.
    BorrowField {
        value: ValueId,
        base: LocalId,
        name: String,
    },
    /// Produces the native pointer wrapped by a live `resource` handle without
    /// consuming it (the ABI representation of a borrowed resource).
    ResourceDeref {
        value: ValueId,
        handle: ValueId,
    },
    /// Saves a value that lives across a suspension into the frame at `slot`.
    SpillValue {
        slot: usize,
        value: ValueId,
    },
    /// Restores a value saved by [`Instruction::SpillValue`] from `slot`.
    ReloadValue {
        value: ValueId,
        slot: usize,
    },
    /// Polls the in-flight child handle stored in `frame` without blocking.
    /// `ready` is nonzero when the child produced a result; `value` (if any) is
    /// the child's result.
    PollFuture {
        value: Option<ValueId>,
        ready: ValueId,
        frame: LocalId,
        result_type: TypeId,
    },
    /// Marks a channel `recv` suspension site. The async pass rewrites it into
    /// [`Instruction::PollChannelRecv`]; `value` and `present` are produced on
    /// resume.
    AwaitChannelRecv {
        value: ValueId,
        present: ValueId,
        handle: ValueId,
        result_type: TypeId,
    },
    /// Polls the in-flight channel-recv child stored in `frame`. On `ready`,
    /// `value` is the address of the received inner value and `present` is
    /// nonzero when a value (rather than the closed/empty absence) was received.
    PollChannelRecv {
        value: ValueId,
        present: ValueId,
        ready: ValueId,
        frame: LocalId,
        result_type: TypeId,
    },
    Allocate {
        value: ValueId,
        ty: TypeId,
    },
    /// Loads a scalar/pointer of type `ty` from the raw address `pointer + offset`.
    LoadRaw {
        value: ValueId,
        pointer: ValueId,
        offset: i64,
        ty: TypeId,
    },
    /// Stores a scalar/pointer of type `ty` at the raw address `pointer + offset`.
    StoreRaw {
        pointer: ValueId,
        offset: i64,
        value: ValueId,
        ty: TypeId,
    },
    Retain {
        value: ValueId,
        ty: TypeId,
    },
    /// Returns a uniquely-referenced copy of a managed collection handle,
    /// deep-copying it (retaining managed elements) when the storage is shared.
    /// Preserves value semantics before an in-place mutation.
    MakeUnique {
        value: ValueId,
        operand: ValueId,
        ty: TypeId,
    },
    Release {
        value: ValueId,
        ty: TypeId,
    },
    Binary {
        /// Numeric operand type; required for target width and signedness even
        /// when both operands are literals without a typed local.
        operand_type: Option<TypeId>,
        value: ValueId,
        op: BinaryOp,
        left: ValueId,
        right: ValueId,
    },
    Unary {
        value: ValueId,
        op: vut_ast::UnaryOp,
        operand: ValueId,
    },
    IteratorInit {
        iterator: ValueId,
        iterable: ValueId,
        length: Option<usize>,
        /// Element count taken from a runtime value (variadic views).
        length_value: Option<ValueId>,
        stride: Option<usize>,
        /// When set, the 32-byte iterator state lives in the future frame at
        /// this byte offset (so it survives a suspension); otherwise it lives
        /// in a stack slot.
        slot: Option<usize>,
    },
    IteratorNext {
        has_value: ValueId,
        value: ValueId,
        index: ValueId,
        iterator: ValueId,
        element_type: TypeId,
    },
    /// Constructs a payload-carrying enum value (tag + payload storage).
    ConstructEnum {
        value: ValueId,
        ty: TypeId,
        variant_index: usize,
        payload: Vec<ValueId>,
    },
    /// Reads the active variant tag of an enum value as an integer.
    EnumTag {
        result: ValueId,
        value: ValueId,
        ty: TypeId,
    },
    /// Copies an inline aggregate into fresh storage that owns its managed
    /// fields. Used when an aggregate value (for example an aggregate field of
    /// a live aggregate) must escape independently of its source.
    CopyAggregate {
        value: ValueId,
        source: ValueId,
        ty: TypeId,
    },
    /// Projects a payload field out of an enum value.
    EnumPayload {
        value: ValueId,
        source: ValueId,
        ty: TypeId,
        variant_index: usize,
        field_index: usize,
    },
    /// Produces the address of the compiler-generated retain callback for `ty`,
    /// or a null pointer for trivially copyable types.
    TypeRetain {
        value: ValueId,
        ty: TypeId,
    },
    /// Produces the address of the compiler-generated release callback for `ty`,
    /// or a null pointer for types that need no destruction.
    TypeRelease {
        value: ValueId,
        ty: TypeId,
    },
    ConstructResult {
        value: ValueId,
        ty: TypeId,
        ok: bool,
        payload: ValueId,
    },
    ResultState {
        result: ValueId,
        value: ValueId,
        ok: bool,
    },
    ResultPayload {
        value: ValueId,
        source: ValueId,
        ok: bool,
    },
    /// Spawns a Vutcon from the callable `callable` with result type `T`. `start`
    /// is the callable's function pointer; the backend selects a poll entry and
    /// drop thunk for it.
    Spawn {
        value: ValueId,
        start: ValueId,
        callable: SymbolId,
        result_type: TypeId,
    },
    /// Awaits a poll task (`future[T]` or `vutcon[T]`) and yields `T`.
    AwaitFuture {
        value: ValueId,
        handle: ValueId,
        result_type: TypeId,
    },
    Call {
        value: Option<ValueId>,
        result_type: Option<TypeId>,
        target: SymbolId,
        arguments: Vec<ValueId>,
    },
    /// A call to a Vut `async fn`. At this stage it is lowered like an ordinary
    /// call; the async pass rewrites it into future creation plus suspension.
    StartFuture {
        value: Option<ValueId>,
        result_type: Option<TypeId>,
        target: SymbolId,
        arguments: Vec<ValueId>,
    },
    MakeFunction {
        value: ValueId,
        symbol: SymbolId,
    },
    /// Creates a capturing closure: allocates a reference-counted environment
    /// holding `captures`, bundles the body code pointer, and produces a tagged
    /// closure value.
    MakeClosure {
        value: ValueId,
        symbol: SymbolId,
        captures: Vec<ValueId>,
    },
    CallIndirect {
        value: Option<ValueId>,
        result_type: Option<TypeId>,
        callable_ty: Option<TypeId>,
        callee: ValueId,
        arguments: Vec<ValueId>,
    },
    RuntimeCall {
        value: Option<ValueId>,
        result_type: Option<TypeId>,
        function: BuiltinFunction,
        arguments: Vec<ValueId>,
    },
    Field {
        value: ValueId,
        base: ValueId,
        name: String,
    },
    Construct {
        value: ValueId,
        ty: TypeId,
        fields: Vec<(String, ValueId)>,
    },
    ConstructArray {
        value: ValueId,
        ty: TypeId,
        elements: Vec<ValueId>,
    },
    /// Builds a contiguous, caller-owned buffer of variadic arguments and
    /// yields its address. Elements are stored inline with the element stride.
    ConstructVariadicBuffer {
        value: ValueId,
        element: TypeId,
        elements: Vec<ValueId>,
    },
    /// Bounds-checked element access into a variadic view (`data`/`len`).
    VariadicAt {
        value: ValueId,
        data: ValueId,
        len: ValueId,
        index: ValueId,
        element: TypeId,
        /// Set by the optimizer when the index is provably within `len`; codegen
        /// then skips the bounds check.
        in_bounds: bool,
    },
    /// Boxes a concrete `data`/`enum` value into an interface or `dyn` value.
    ConstructInterface {
        value: ValueId,
        ty: TypeId,
        concrete_ty: TypeId,
        interface: Option<SymbolId>,
        source: ValueId,
    },
    /// Dispatches an interface method through the callee's vtable.
    InterfaceCall {
        value: Option<ValueId>,
        result_type: Option<TypeId>,
        callee: ValueId,
        method_index: usize,
        arguments: Vec<ValueId>,
    },
}
#[derive(Clone, Debug)]
pub enum Terminator {
    Jump(BlockId),
    Branch {
        condition: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return(Option<ValueId>),
    /// Returns a poll status code from a poll body (`0` pending, `1` ready).
    PollReturn(i32),
    Unreachable,
}
