# Receiver Functions — Performance & Implementation Roadmap

## 1. Performance Goal

Receiver Function phải được thiết kế để:

```text
abstraction cost → zero when optimizable
```

Mục tiêu:

```text
receiver DSL code
≈
equivalent handwritten function/method code
```

sau optimization khi callback target và lifetime được biết.

---

## 2. Runtime Priorities

Theo thứ tự:

1. static receiver resolution;
2. direct calls khi target known;
3. no mandatory closure allocation;
4. stack/register allocation cho non-escaping closure;
5. closure inlining;
6. devirtualization;
7. escape analysis;
8. environment scalar replacement;
9. copy elision;
10. deterministic optimized drop.

---

## 3. Compile-Time Priorities

Receiver không được tạo specialization explosion.

Compiler phải:

- resolve receiver statically;
- cache method/type resolution;
- reuse generic type checker;
- reuse generic closure analysis;
- reuse generic monomorphization;
- không clone MIR chỉ vì syntax là trailing function;
- không tạo DSL-specific compilation pipeline.

---

# Phase 1 — Syntax Foundation

Implement:

```vut
fn(Receiver)(Args...) -> Return
```

và parse distinction với normal function types.

Tests:

```vut
fn(Scope)() -> void
fn(Scope)(Event) -> bool
fn(Scope)(Event, int) -> str
```

Không implement runtime hacks.

---

# Phase 2 — Receiver Semantic Type

Thêm receiver vào semantic function type.

Yêu cầu:

```text
receiver != ordinary parameter
```

Type checker phải giữ receiver riêng.

Add type pretty-printer support.

Add diagnostics.

---

# Phase 3 — `.call()`

Implement:

```vut
callback.call(scope, ...args)
```

Receiver luôn đứng đầu.

Lower trực tiếp thành receiver call.

Không tạo:

- temporary list;
- argument array;
- reflection;
- dynamic `.call` lookup.

Benchmark generated MIR/codegen so với equivalent ordinary indirect call.

---

# Phase 4 — Trailing Function Integration

Implement:

```vut
Column():
  body
```

và:

```vut
UserCard(user) (event):
  body
```

Expected type của parameter cuối quyết định receiver/argument types.

Trailing syntax phải lower về generic anonymous receiver function representation.

Không tạo DSL AST/MIR subsystem.

---

# Phase 5 — Implicit Receiver Resolution

Implement:

```vut
Column():
  Button()
```

→ statically bind:

```text
ColumnScope.Button
```

Receiver lookup chỉ ở compile time.

Implement nested lexical receiver stack.

Add ambiguity diagnostics.

---

# Phase 6 — Closure Capture Integration

Receiver và captures phải tách biệt:

```text
receiver
params
captures
```

Reuse generic closure infrastructure.

Không implement receiver-specific capture box.

Test:

```vut
user = ...

UserCard(user) (event):
  select(user, event)
```

---

# Phase 7 — Non-Escaping Fast Path

Đây là phase performance bắt buộc.

Compiler phải nhận diện receiver closure không escape.

Mục tiêu:

```text
no heap allocation
```

cho các DSL body thông thường khi có thể.

Add escape-analysis metadata sớm để optimizer sử dụng.

---

# Phase 8 — Inlining

Ưu tiên inline:

- small receiver functions;
- known trailing callbacks;
- small scope methods.

Example:

```vut
Column():
  Button("A")
  Button("B")
```

mục tiêu optimized MIR gần:

```text
create scope
add button A
add button B
finish scope
```

Không còn closure object nếu không cần.

---

# Phase 9 — Devirtualization

Nếu function value target có thể chứng minh:

```text
indirect receiver call
↓
direct call
```

Apply generic devirtualization infrastructure.

Không tạo receiver-specific optimizer nếu generic callable optimizer có thể làm được.

---

# Phase 10 — Environment Optimization

Implement/enable:

- scalar replacement;
- dead capture elimination;
- copy elision;
- move propagation;
- stack promotion.

Ví dụ:

```vut
title = "Hello"

Column():
  Text(title)
```

không được tạo heap environment nếu closure không escape.

---

# Phase 11 — Escaping Closure Correctness

Khi closure thực sự escape:

- allocate only when necessary;
- preserve ownership;
- exactly-once drop;
- no leaks;
- no double-free;
- no unconditional RC.

Test managed values/resources.

---

# Phase 12 — Nested DSL Stress Tests

Test:

```vut
App():
  Header():
    Text("Vut")

  Column():
    if loading:
      Spinner()
    else:
      for user in users:
        UserCard(user) (event):
          Text(user.name)

          if event.clicked:
            select(user)

  Footer():
    Text("2026")
```

Verify:

- receiver resolution;
- closure capture;
- nested scopes;
- `if`;
- `for`;
- contextual typing;
- ownership;
- drops.

---

# Phase 13 — Compile-Time Benchmarks

Add compiler benchmarks for:

- 100 nested receiver calls;
- 1,000 receiver method calls;
- many small trailing callbacks;
- nested receiver scopes;
- generic receiver types;
- capturing/non-capturing callbacks.

Compare against equivalent explicit method/function code.

Receiver syntax should add only small resolver/type-checker overhead.

---

# Phase 14 — Runtime Benchmarks

Compare:

```text
receiver DSL
vs
explicit scope.method(...)
```

Measure:

- allocations;
- instruction count where practical;
- runtime;
- binary size;
- closure object count.

Critical target for non-escaping known callbacks:

```text
heap allocations attributable to receiver abstraction = 0
```

after optimization.

---

# Phase 15 — Regression Gates

Before declaring Receiver stable:

- parser tests green;
- resolver tests green;
- type checker tests green;
- ownership tests green;
- closure tests green;
- MIR tests green;
- codegen tests green;
- nested DSL E2E green;
- no receiver-specific memory leaks;
- no mandatory heap closure;
- no reflection/runtime lookup;
- compile-time regression measured;
- runtime regression measured.

---

## Definition of Done

Receiver Functions are complete when this:

```vut
App():
  Column():
    for user in users:
      UserCard(user) (event):
        Text(user.name)

        if event.clicked:
          select(user)
```

is:

- statically typed;
- statically resolved;
- ownership-correct;
- closure-correct;
- compatible with normal Vut control flow;
- free of DSL-specific runtime machinery;
- optimizable using generic compiler infrastructure;
- capable of zero heap allocation attributable solely to receiver/trailing-function abstraction when callbacks do not escape;
- runtime performance close to equivalent explicit method/function code.