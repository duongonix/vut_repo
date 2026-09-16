# Vut Attributes

## 1. Purpose

Attributes attach compiler metadata to the declaration immediately following
them.

Attributes are not macros. They do not execute user code, expand syntax, run
plugins, or invoke external programs.

## 2. Syntax

```text
attribute :=
    "@" identifier
  | "@" identifier "(" [ attribute_argument { "," attribute_argument } [ "," ] ] ")"

attribute_argument :=
    identifier
  | string_literal
  | integer_literal
  | boolean_literal

attributed_declaration :=
    { attribute } declaration
```

Examples:

```vut
@repr(C)
data Point:
  x: f32
  y: f32

@link_name("native_add")
extern "C" fn add(a: i32, b: i32) -> i32
```

Multiple attributes are written on separate lines:

```vut
@repr(C)
@other
data Example:
  value: i32
```

The canonical formatter preserves one attribute per line before the target
declaration.

## 3. List Literal Compatibility

The existing list literal syntax remains unchanged:

```vut
items = @(1, 2, 3)
```

The parser distinguishes the two forms by the token after `@`:

```text
"@("          list literal
"@" ident     attribute
```

No contextual reinterpretation is allowed.

## 4. AST and HIR

Attributes are represented as structured nodes, not raw strings:

```text
Attribute
  name: identifier
  args: AttributeArg*
  span: Span

AttributeArg
  Ident
  String
  Integer
  Bool
```

Supported declaration nodes carry `attributes`. Semantic attributes needed by
later phases are lowered into HIR and semantic metadata so backend phases never
read AST to recover them.

## 5. Targets

The framework supports attributes on:

```text
fn
data
enum
interface
type alias
extern fn
```

Each built-in attribute declares its allowed targets. Applying an attribute to
the wrong target is a compile error.

## 6. Built-In Registry

Built-in attributes are defined in one compiler registry with:

```text
name
allowed targets
argument schema
duplicates allowed
semantic meaning
```

Unknown attributes are rejected.

Duplicate attributes are rejected unless their definition explicitly allows
duplicates.

## 7. Built-In Attributes

### `@repr(C)`

Valid on `data`.

Arguments:

```text
identifier C
```

`@repr(C)` requests a C ABI-compatible field layout. Field order is source order.
All fields must have a defined FFI-safe representation.

### `@repr(transparent)`

Valid on `data`.

Arguments:

```text
identifier transparent
```

The declaration must have exactly one field. Its representation follows the
single field once the target ABI layer supports transparent aggregate passing.

### `@link_name("symbol")`

Valid on `extern fn`.

Arguments:

```text
string literal
```

The Vut source name remains unchanged, but native code generation imports the
specified linker symbol.

## 8. Diagnostics

The compiler reports:

```text
unknown attribute
invalid attribute target
invalid number of attribute arguments
invalid attribute argument type
duplicate attribute
dangling attribute
invalid repr value
link_name requires a string literal
```

## 9. Future Attributes

Future built-ins such as these may be added through the same registry:

```vut
@inline
@deprecated
@test
@cold
@no_mangle
@must_use
@align(16)
```

They have no behavior until specified and implemented.
