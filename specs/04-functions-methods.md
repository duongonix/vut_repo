# Functions and Methods

## 1. Purpose

This document defines functions and methods in Vut.

Vut uses a unified function declaration model.

Both:

```text
free functions
instance methods
```

must use the keyword:

```text
fn
```

The difference between a free function and a method is determined by the declaration name.

---

# 2. Free Functions

A free function uses:

```vut
fn name(parameters) -> ReturnType:
  body
```

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

The final expression is returned automatically.

Equivalent conceptually to:

```text
return a + b
```

---

# 3. Functions Without Return Values

A function may perform work without returning a meaningful value.

Example:

```vut
fn greet(name: str):
  out("Hello $name")
```

The function conceptually returns:

```text
void
```

The compiler may represent this through its internal unit/void type.

An omitted return type always means `void`; it does not request return-type
inference. `-> void` is also accepted when an explicit annotation is useful.

Exact ABI representation is implementation-defined.

---

# 4. Explicit Return Type

Return types may be declared explicitly:

```vut
fn multiply(a: int, b: int) -> int:
  a * b
```

The function body must satisfy the declared return type.

Example:

```vut
fn get_name() -> str:
  "Nam"
```

Invalid:

```vut
fn get_name() -> str:
  10
```

Compiler must emit a type mismatch diagnostic.

---

# 5. Implicit Final-Expression Return

The final expression of a function may be returned automatically.

Example:

```vut
fn square(value: int) -> int:
  value * value
```

No explicit `return` is required.

This keeps common functions concise.

---

# 6. Explicit Return

The `return` keyword is supported for early returns.

Example:

```vut
fn divide(a: int, b: int) -> int:
  if b == 0:
    return 0

  a / b
```

Explicit `return` terminates the function immediately.

---

# 7. Return Type Checking

All return paths must satisfy the function return type.

Example:

```vut
fn value(ok: bool) -> int:
  if ok:
    return 10

  20
```

Valid.

Invalid:

```vut
fn value(ok: bool) -> int:
  if ok:
    return 10

  "hello"
```

The compiler must report:

```text
expected int
found str
```

with precise source spans.

---

# 8. Parameters

Function parameters use:

```text
name: Type
```

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Parameter types must be known statically.

Do not silently use `dyn` for untyped parameters.

---

# 9. Parameter Type Inference

Vut does not infer public function parameter types from call sites in the MVP.

The following is not valid:

```vut
fn add(a, b):
  a + b
```

Parameters must declare their types unless a later finalized generic/inference specification explicitly allows otherwise.

Use:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

---

# 10. Function Calls

Functions are called using parentheses:

```vut
result = add(10, 20)
```

Arguments are evaluated according to normal Vut expression semantics.

---

# 11. Named Arguments

Vut supports named arguments.

Example:

```vut
fn create_user(name: str, age: int) -> User:
  User(
    name = name,
    age = age
  )

user = create_user(
  name = "Nam",
  age = 20
)
```

Named arguments improve clarity for functions with multiple parameters.

---

# 12. Named Argument Rules

Named arguments must refer to valid parameter names.

Invalid:

```vut
create_user(
  username = "Nam",
  age = 20
)
```

if the function declares:

```vut
fn create_user(name: str, age: int) -> User:
  ...
```

Compiler must report that:

```text
username
```

is not a valid parameter.

---

# 13. Duplicate Named Arguments

Duplicate arguments are compile errors.

Invalid:

```vut
create_user(
  name = "Nam",
  name = "Ha",
  age = 20
)
```

---

# 14. Missing Arguments

Required arguments must be provided.

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Invalid:

```vut
add(10)
```

Compiler must report the missing parameter.

---

# 15. Too Many Arguments

Passing more arguments than declared is a compile error.

Example:

```vut
add(10, 20, 30)
```

for:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

must fail.

---

# 16. Default Parameters

Default function parameter values are not part of the finalized MVP unless explicitly defined by another specification.

Do not invent syntax such as:

```vut
fn connect(timeout: int = 10):
```

without a finalized specification.

Named `data` fields may have defaults independently of function parameters.

---

# 17. Methods

Methods are functions associated with a named type.

Methods must also use the keyword:

```text
fn
```

Method syntax:

```vut
fn Type.method(parameters) -> ReturnType:
  body
```

Example:

```vut
data Counter:
  value: int = 0

fn Counter.increment():
  self.value = self.value + 1
```

---

# 18. Unified Function Syntax

Both free functions and methods begin with:

```text
fn
```

Free function:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Method:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

This syntax is intentional.

The parser identifies a method by the qualified declaration name:

```text
Type.method
```

rather than by the absence of `fn`.

---

# 19. Removed Old Method Syntax

The old form:

```vut
Counter.increment():
  self.value = self.value + 1
```

is invalid.

The required syntax is:

```vut
fn Counter.increment():
  self.value = self.value + 1
```

Compiler/parser documentation and examples must use this syntax consistently.

---

# 20. Method Declaration Structure

A method declaration contains:

```text
fn
receiver type
.
method name
parameter list
optional return type
:
indented body
```

Example:

```vut
fn User.greet(prefix: str) -> str:
  "$prefix $self.name"
```

---

# 21. Implicit `self`

Instance methods receive the current instance through an implicit compiler-provided value:

```text
self
```

The programmer does not declare `self` in the parameter list.

Valid:

```vut
fn Counter.increment():
  self.value = self.value + 1
```

Invalid:

```vut
fn Counter.increment(self):
  self.value = self.value + 1
```

---

## 21.1 `self` Is Borrowed

`self` is a borrow of the caller's value. A method may read and mutate the
receiver in place:

```vut
fn Counter.increment():
  self.value = self.value + 1
```

Mutations through `self` (including assigning managed fields) are visible to the
caller after the call. The method does not take ownership of `self`; the caller
remains responsible for releasing it. Method calls on a projected field mutate
that field in the caller's aggregate. See `specs/08-memory-model.md` §30.1.

---

# 22. `self` Type

Inside:

```vut
fn Counter.increment():
```

the compiler knows:

```text
self: Counter
```

Inside:

```vut
fn User.greet():
```

the compiler knows:

```text
self: User
```

No runtime lookup is needed to determine the receiver type for statically resolved concrete methods.

---

# 23. `self` Scope

`self` is valid only inside instance methods.

Invalid:

```vut
fn test():
  out("$self")
```

Compiler must report that `self` is unavailable in that scope.

---

# 24. Method Calls

Methods are called using:

```text
value.method(...)
```

Example:

```vut
counter = Counter()

counter.increment()
counter.add(10)
```

The compiler resolves the receiver type statically whenever possible.

---

# 25. Method Example

```vut
data Counter:
  value: int = 0

fn Counter.increment():
  self.value = self.value + 1

fn Counter.add(amount: int):
  self.value = self.value + amount

fn Counter.get() -> int:
  self.value

counter = Counter()

counter.increment()
counter.add(10)

out("$(counter.get())")
```

---

# 26. Method Resolution

For:

```vut
counter.increment()
```

the compiler should conceptually perform:

```text
1. resolve counter
2. determine static type Counter
3. find public/private-accessible Counter.increment
4. validate argument count
5. validate argument types
6. create resolved method call
```

Method lookup must not require runtime string-based lookup for concrete types.

---

# 27. Direct Method Calls

Concrete method calls should compile to direct native calls whenever the concrete type is known.

Example:

```vut
user.login()
```

should not require:

```text
runtime method-name lookup
hash map lookup
reflection
dynamic method dispatch
```

for ordinary concrete values.

---

# 28. Internal Method Lowering

Although source syntax is:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

HIR/MIR may normalize this conceptually to:

```text
Counter.add(self: Counter, amount: int)
```

or another ownership-aware receiver representation.

This is internal compiler behavior.

The user-facing syntax remains free of an explicit `self` parameter.

---

# 29. Method Receiver Ownership

Receiver ownership and physical passing are defined by the memory model.

The compiler may internally use:

```text
register value
pointer/reference
borrow-like temporary internal representation
move
copy
```

depending on:

```text
type size
mutation
ownership
escape behavior
ABI
optimization
```

This must not require source-level borrow syntax.

Follow:

```text
specs/08-memory-model.md
```

---

# 30. Mutating Methods

Methods may modify fields of `self`.

Example:

```vut
data Counter:
  value: int = 0

fn Counter.increment():
  self.value = self.value + 1
```

No separate:

```text
mut
mutable method
&mut self
```

syntax is required in the MVP.

The compiler memory/ownership system is responsible for preserving Vut value semantics and memory safety.

---

# 31. Value Semantics and Method Mutation

Given:

```vut
a = user
b = a

b.update()
```

mutating `b` must not unexpectedly mutate `a` through hidden mutable aliasing.

Compiler/runtime may use:

```text
move
copy
copy-on-write
unique ownership
```

internally while preserving observable value semantics.

---

# 32. Private Methods

Method names beginning with `_` are private.

Example:

```vut
fn User._validate_password() -> bool:
  ...
```

This method is accessible only according to Vut module privacy rules.

Follow:

```text
specs/07-modules-imports.md
```

---

# 33. Public Methods

Methods not beginning with `_` are public.

Example:

```vut
fn User.login():
  ...
```

No `pub` keyword exists.

---

# 34. Interface Satisfaction

Public methods may satisfy structural interfaces.

Example:

```vut
interface Animal:
  speak() -> str

data Dog:
  name: str

fn Dog.speak() -> str:
  "Woof"
```

`Dog` automatically satisfies `Animal`.

No:

```text
impl Animal
implements Animal
```

is required.

---

# 35. Private Methods and Interfaces

Private methods do not satisfy public interface requirements.

Example:

```vut
interface Animal:
  speak() -> str

data Dog:
  name: str

fn Dog._speak() -> str:
  "Woof"
```

`Dog` does not satisfy `Animal`.

---

# 36. Interface Method Declarations

Interface method signatures do not use `fn`.

Example:

```vut
interface Animal:
  speak() -> str
  name() -> str
```

This remains intentionally different from function/method implementation declarations.

Inside an interface there is no function body.

Therefore:

```vut
interface Animal:
  fn speak() -> str
```

is not the finalized Vut syntax.

The `fn` requirement applies to implemented free functions and implemented methods.

---

# 37. Constructors

Named `data` types have constructor syntax:

```vut
User(
  name = "Nam",
  age = 20
)
```

The constructor is compiler-generated according to the data model.

It is not declared using:

```vut
fn User():
```

in the MVP.

---

# 38. Constructor Example

```vut
data User:
  name: str
  age: int = 0

user = User(
  name = "Nam"
)
```

The compiler validates:

```text
required fields
default fields
duplicate fields
unknown fields
field types
```

---

# 39. Custom Constructors

User-defined custom constructor syntax is not finalized in the MVP.

Do not invent:

```text
init
new
constructor
```

or special `fn Type()` semantics unless defined by a later specification.

Users can use ordinary functions where needed:

```vut
fn create_user(name: str) -> User:
  User(
    name = name,
    age = 0
  )
```

---

# 40. Static Methods

Static methods are not finalized for the MVP.

Do not infer that:

```vut
fn User.create():
```

is automatically static.

The `Type.method` syntax in this specification represents instance methods with implicit `self`.

Static method semantics require a separate finalized design.

---

# 41. Function Overloading

General function overloading is not part of the current MVP.

Do not allow multiple functions with identical names distinguished only by parameter types unless a later specification explicitly introduces overloading.

Example:

```vut
fn parse(value: int):
  ...

fn parse(value: str):
  ...
```

should not be accepted as general overload syntax in MVP.

---

# 42. Method Overloading

General method overloading is also not finalized.

Do not rely on:

```text
method name + parameter types
```

to distinguish multiple declarations with the same method name.

---

# 43. Recursion

Functions may recursively call themselves.

Example:

```vut
fn factorial(n: int) -> int:
  if n <= 1:
    return 1

  n * factorial(n - 1)
```

Methods may also call themselves recursively where semantically valid.

---

# 44. Mutual Recursion

Mutual recursion may be supported when name resolution can resolve all involved declarations.

Example conceptually:

```vut
fn is_even(n: int) -> bool:
  if n == 0:
    return true

  is_odd(n - 1)

fn is_odd(n: int) -> bool:
  if n == 0:
    return false

  is_even(n - 1)
```

Resolver architecture should not unnecessarily depend on declaration order when module-level declarations can be collected first.

---

# 45. Function Values

First-class function values are supported for non-capturing functions.

Vut supports:

```text
function references
callbacks
non-capturing anonymous functions (lambdas)
```

Function types are written as:

```vut
fn(int) -> int
fn(str)
fn()
```

A named function may be used wherever a function type is expected:

```vut
fn double(value: int) -> int:
  value * 2

fn apply(value: int, callback: fn(int) -> int) -> int:
  callback(value)

result = apply(21, double)
```

A function type may declare a distinct **receiver**:

```vut
fn(ColumnScope)() -> void
fn(UserCardScope)(Event) -> void
```

The receiver is not an ordinary parameter. Receiver functions are covered in
`specs/receiver/`; invocation uses `callback.call(receiver, ...args)`.

---

# 46. Anonymous Functions

Anonymous functions (lambdas) use one of two surface forms. Both produce the
same anonymous function type.

Single expression:

```vut
x => x * 2
(a, b) => a + b
() => out("Hello")
```

Multiple lines:

```vut
fn(x):
  value = x * 2
  value + 1
```

Used as callbacks:

```vut
transform(10, x => x * 2)

transform(10, fn(x):
  value = x * 2
  value + 10
)
```

Rules:

```text
arrow lambda parameters have no type annotations
fn lambda parameters may be annotated
types may be inferred from an expected fn(...) -> ... type
lambdas are non-capturing
```

Lambdas cannot reference locals or parameters of an enclosing function. Such
a reference is a compile error (E1013). Capturing closures remain deferred.

---

# 47. Generic Functions

Generic declarations use a parenthesized type-parameter list placed before the
value-parameter list:

```vut
fn identity(T)(value: T) -> T:
  value
```

Generic constraint semantics, including constraint-conditioned method access, are
defined in `06-interfaces.md` §26.

Do not invent angle-bracket syntax such as:

```text
fn identity<T>(...)
```

because Vut already prefers parentheses for type application and generic
declaration.

---

# 47a. Variadic Parameters

A trailing parameter may be declared variadic:

```vut
fn sum(...values: int) -> int:
  total = 0
  for value in values:
    total = total + value
  total

sum(1, 2, 3, 4)
sum()
```

Rules:

1. `...name: T` accepts **zero or more** arguments, each of type `T`.
2. The variadic parameter must be the **last** parameter.
3. Inside the body, `name` is a **read-only variadic view** of `T`: iterable with
   `for value in name`, with `name.len()` (the element count) and `name.at(i)`
   (bounds-checked element access). It is not a general value and cannot be
   returned or stored.
4. The representation is a contiguous element view (buffer pointer plus count);
   it does not require a `list(T)` or `array(T, N)` and allocates no heap for the
   arguments.
5. Static checking is strict: `...values: int` rejects `str` arguments and
   `str` spreads.
6. A call may spread a matching sequence as its final argument:

   ```vut
   values = @(1, 2, 3)
   sum(...values)
   ```

   The spread must be `list(T)`, `array(T, N)`, or another variadic view with the
   same element type, and must be the last argument.
7. `...args: array(T, N)` declares a variadic parameter whose each argument is an
   `array(T, N)`; it is not a variadic sequence of `T`.
8. Extern (`extern "C"`) functions cannot be variadic; C varargs remain deferred
   (see the FFI specification).

---

# 47b. Function Values and Chained Calls

A call's callee may be any expression whose type is callable, not only a name or
method. In particular, the result of a call may itself be called, and postfix
`(...)` applications chain:

```vut
fn make_adder(a: int) -> fn(int) -> int:
  fn(b):
    a + b

result = make_adder(10)(5)
```

This is equivalent to binding the intermediate value first:

```vut
temp = make_adder(10)
result = temp(5)
```

Rules:

1. Every postfix call level is type-checked independently: the callee of each
   level must have a function/callable type, and its arguments are checked
   against that level's parameters.
2. Chaining may be arbitrarily deep (`foo()(1)("x")`).
3. Calling a value whose type is not callable is a compile error (`E1014`).
4. Named functions used as values, anonymous functions (lambdas), and
   function-typed locals all participate in chaining.
5. This follows from ordinary call typing; there is no currying-specific rule.

Anonymous functions currently may only reference their own parameters and
locals; capturing enclosing bindings is not yet supported (`E1013`).

---

# 48. Function Type Checking

For each call, the compiler must validate:

```text
function exists
argument count
argument names
argument types
return type
visibility
```

Method calls additionally validate:

```text
receiver type
method existence
method visibility
interface dispatch rules when applicable
```

---

# 49. Argument Evaluation

Argument expressions are evaluated before entering the called function.

Precise evaluation order must remain deterministic.

The compiler must not reorder side-effecting argument evaluation in a way that changes observable behavior.

Optimization may reorder only when proven safe.

---

# 50. Memory and Function Calls

Function calls must integrate with:

```text
automatic move
Copy classification
automatic Drop
value semantics
```

Example:

```vut
fn consume(user: User):
  out("Hello $user.name")
```

If the caller no longer uses the passed value and ownership can safely transfer, compiler may avoid copying it.

If the caller still needs the value, compiler must preserve observable value semantics.

---

# 51. Return Value Optimization

Example:

```vut
fn create() -> User:
  User(
    name = "Nam",
    age = 20
  )
```

Compiler should avoid:

```text
construct temporary
copy temporary
drop temporary
```

where possible.

It should support destination-passing or ABI-appropriate return-value optimization.

---

# 52. Large Parameters

Large `data` values should not automatically incur unnecessary full copies.

The compiler may pass large values through efficient ABI/internal representations while preserving language-level value semantics.

Do not expose implementation pointer semantics to the user.

---

# 53. Small Parameters

Small Copy-compatible parameters should preferably use native registers according to the target ABI.

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

should compile to efficient native parameter passing.

---

# 54. Template Strings Inside Functions

Functions and methods may use template strings normally.

Example:

```vut
fn greet(name: str):
  out("Hello $name")
```

Method:

```vut
fn User.greet():
  out("Hello $self.name")
```

Expression interpolation is also valid:

```vut
fn User.birth_year(year: int):
  out("Birth year: $(year - self.age)")
```

Template expressions must be statically parsed and type checked.

No runtime `eval` is allowed.

---

# 55. Function Symbol Names

A free function declaration creates a module-level function symbol.

Example:

```vut
fn calculate():
  ...
```

creates:

```text
calculate
```

A method declaration:

```vut
fn User.calculate():
  ...
```

is semantically associated with:

```text
User
```

and method name:

```text
calculate
```

Compiler symbol representation should not rely solely on concatenated strings such as `"User.calculate"` internally.

Use structured IDs where appropriate.

---

# 56. Duplicate Functions

Duplicate declarations in the same valid namespace are compile errors.

Example:

```vut
fn test():
  ...

fn test():
  ...
```

must fail.

---

# 57. Duplicate Methods

Duplicate methods for the same receiver type and method name are compile errors in MVP.

Example:

```vut
fn User.login():
  ...

fn User.login():
  ...
```

must fail.

---

# 58. Method Receiver Validation

The receiver type in:

```vut
fn User.login():
```

must resolve to an appropriate named type.

Compiler must reject unknown receiver types.

Example:

```vut
fn Unknown.login():
  ...
```

when `Unknown` does not exist.

---

# 59. Method Placement

Methods may be declared separately from the `data` declaration.

Example:

```vut
data User:
  name: str

fn User.greet():
  out("Hello $self.name")
```

Behavior does not need to be nested inside `data`.

This keeps state declarations simple and separates data layout from behavior declarations.

---

# 60. Source Order

A method does not necessarily need to appear immediately after its `data` declaration.

Resolver should associate methods with their receiver type semantically.

Example:

```vut
data User:
  name: str

fn something():
  ...

fn User.greet():
  ...
```

is valid if all names resolve correctly.

---

# 61. Method Namespace

Methods belong to their receiver type's method namespace.

Therefore:

```vut
fn User.open():
  ...

fn File.open():
  ...
```

may coexist.

They are not duplicate free-function names.

---

# 62. Free Function vs Method Name

This is valid:

```vut
fn open():
  ...

fn File.open():
  ...
```

because:

```text
open
```

and:

```text
File.open
```

are different semantic declarations.

---

# 63. Entry Point

Project executable entry-point behavior is defined by the compiler/project specification.

The conventional executable entry point is:

```vut
fn main():
  ...
```

Example:

```vut
fn main():
  out("Hello Vut")
```

The entry point may also be asynchronous:

```vut
async fn main():
  data = await load()
  out("$data")
```

The compiler/runtime drives `async main` to completion using the single-thread
executor. The user does not call `block_on` or an executor API. Entry-point
parameter and return-type rules are otherwise unchanged.

Do not require a class or object wrapper around `main`.

Async functions are specified in:

```text
specs/async/
```

---

# 64. Example Program

```vut
data User:
  name: str
  age: int

fn User.greet():
  out("Hello $self.name")

fn User.birth_year(year: int) -> int:
  year - self.age

fn create_user(name: str, age: int) -> User:
  User(
    name = name,
    age = age
  )

fn main():
  name = input("Name: ")

  user = create_user(
    name = name,
    age = 20
  )

  user.greet()

  year = user.birth_year(2026)

  out("$user.name was born around $year")
```

---

# 65. Invalid Examples

## Method without `fn`

Invalid:

```vut
User.greet():
  out("Hello")
```

Correct:

```vut
fn User.greet():
  out("Hello")
```

---

## Explicit self parameter

Invalid:

```vut
fn User.greet(self):
  ...
```

Correct:

```vut
fn User.greet():
  ...
```

---

## Unknown receiver

Invalid:

```vut
fn Missing.test():
  ...
```

if `Missing` is not defined.

---

## Untyped parameter

Invalid in MVP:

```vut
fn add(a, b):
  a + b
```

Correct:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

---

# 66. Parser Requirements

The parser must distinguish:

```vut
fn add(a: int):
```

from:

```vut
fn User.add(a: int):
```

after reading the declaration name.

Conceptually:

```text
fn_decl :=
  "fn" function_name parameter_list return_type? ":" block

function_name :=
    identifier
  | type_path "." identifier
```

The exact grammar belongs in:

```text
specs/21-grammar.md
```

---

# 67. AST Requirements

The AST should distinguish free functions from methods structurally.

Possible conceptual representation:

```text
FunctionDecl
├── name
├── parameters
├── return_type
└── body
```

and:

```text
MethodDecl
├── receiver_type
├── name
├── parameters
├── return_type
└── body
```

Do not require later compiler stages to repeatedly parse `"Type.method"` strings.

---

# 68. HIR Requirements

HIR should normalize methods with explicit semantic receiver information.

Conceptually:

```text
HirMethod
├── receiver_type: TypeId
├── method: FunctionId
├── self_symbol: SymbolId
├── parameters
├── return_type
└── body
```

`self` should become a normal semantic symbol internally.

---

# 69. Codegen Requirements

Concrete method codegen should behave similarly to ordinary function codegen after receiver normalization.

Conceptually:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

may lower internally like:

```text
Counter.add(receiver, amount)
```

with receiver representation selected according to ABI and ownership analysis.

Do not build an independent runtime mechanism solely for normal methods.

---

# 70. Diagnostics

Function/method diagnostics should distinguish errors such as:

```text
unknown function
unknown method
unknown receiver type
duplicate function
duplicate method
missing argument
unknown named argument
duplicate argument
argument type mismatch
return type mismatch
invalid self usage
private method access
```

All diagnostics must follow:

```text
specs/09-errors-diagnostics.md
specs/22-error-codes.md
```

---

# 71. Performance Requirements

Ordinary function and concrete method calls should have performance comparable to native direct calls where possible.

Avoid unnecessary:

```text
heap allocation
boxing
runtime type lookup
reflection
dynamic dispatch
argument containers
temporary collections
```

Static information should be resolved at compile time.

---

# 72. Finalized Syntax Summary

Free function:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Method:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

Call:

```vut
add(1, 2)
counter.add(10)
```

Implicit receiver:

```text
self
```

Constructor:

```vut
Counter()
```

Interface signature:

```vut
interface CounterLike:
  add(amount: int)
```

---

# 73. Final Rules

The following rules are finalized for Vut MVP:

```text
free functions require fn
methods require fn
methods use Type.method syntax
self is implicit
self is not declared as a parameter
methods may mutate self
methods are separate from data declarations
public/private follows identifier naming
concrete method calls are statically resolved
implicit final-expression return is supported
explicit return is supported
named arguments are supported
general overloading is not MVP
static methods are deferred
capturing closures are deferred
non-capturing lambdas and function values are supported
custom constructors are deferred
```

The old method syntax without `fn` is removed.

Canonical method declaration:

```vut
fn Type.method(...):
  ...
```

---

# 74. Async Functions

A free function or instance method may be declared asynchronous:

```vut
async fn fetch() -> Data:
  data = await read_data()
  data
```

```vut
async fn Counter.refresh() -> bool:
  value = await fetch()
  value
```

`async` is a declaration modifier and applies only to `fn`.

The declared return type is the logical result type. Calling an async function
produces an internal awaitable value; the body is driven by `await`, which is a
prefix expression valid only inside an `async fn`.

Async functions:

```text
keep the unified fn and Type.method syntax
have no explicit self parameter
are checked against their logical return type
are not ordinary function values in this phase
```

Async functions are not a substitute for threads or parallelism. See:

```text
specs/async/
```
