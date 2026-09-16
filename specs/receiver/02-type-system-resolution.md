# Receiver Functions — Type System & Resolution

## 1. Receiver là một phần của Function Type

Type system phải phân biệt:

```vut
fn(int) -> int
```

và:

```vut
fn(Scope)(int) -> int
```

Receiver type phải tồn tại trong semantic function signature.

Không được xóa receiver type quá sớm trong AST/HIR rồi cố suy luận lại ở MIR.

---

## 2. Contextual Typing

Trailing receiver function được type-check dựa trên expected type của parameter cuối.

Ví dụ:

```vut
body: fn(UserCardScope)(Event, int) -> void
```

với:

```vut
UserCard(user) (event, index):
  ...
```

compiler suy ra:

```text
self  : UserCardScope
event : Event
index : int
```

Type-check body chỉ bắt đầu sau khi expected receiver/function signature đã được xác định.

---

## 3. Receiver Method Resolution

Trong:

```vut
Column():
  Button("Save")
```

nếu current receiver là:

```vut
ColumnScope
```

resolver có thể tìm:

```vut
fn ColumnScope.Button(text: str):
```

và bind call trực tiếp tới method symbol đó.

Không giữ `"Button"` để lookup lại ở runtime.

---

## 4. Nested Receiver Stack

Resolver duy trì lexical receiver stack:

```text
[
  AppScope,
  ColumnScope,
  UserCardScope
]
```

Receiver cuối là current receiver.

Lookup implicit receiver ưu tiên receiver gần nhất.

---

## 5. Explicit `self`

`self` luôn trỏ tới current receiver.

Nested receiver không thay đổi lexical captures của closure.

MVP chưa cần syntax đặc biệt để truy cập outer receiver.

Không thêm:

```text
self@Outer
outer.self
```

cho tới khi có use case thực tế.

---

## 6. Receiver và Ordinary Arguments

Với:

```vut
fn(UserCardScope)(Event, int) -> bool
```

semantic signature phải giữ riêng:

```text
receiver:
  UserCardScope

params:
  Event
  int

return:
  bool
```

Không flatten quá sớm thành:

```text
(UserCardScope, Event, int) -> bool
```

ở semantic layer.

Backend ABI có thể flatten receiver thành hidden first parameter.

---

## 7. `.call()`

Với:

```vut
callback: fn(UserCardScope)(Event) -> void
```

call:

```vut
callback.call(scope, event)
```

phải kiểm tra:

```text
scope : UserCardScope
event : Event
```

Sai receiver:

```vut
callback.call(column_scope, event)
```

phải compile error nếu `column_scope` không tương thích với `UserCardScope`.

---

## 8. Closure Captures

Ví dụ:

```vut
user = ...

UserCard(user) (event):
  select(user, event)
```

semantic closure:

```text
receiver:
  UserCardScope

params:
  event: Event

captures:
  user: User
```

Capture analysis không được nhầm receiver thành ordinary captured variable.

Receiver và capture có ownership/lifetime analysis riêng nhưng phải phối hợp trong closure lowering.

---

## 9. Escape Semantics

Compiler phải phân biệt:

```text
non-escaping receiver closure
escaping receiver closure
```

Non-escaping closure phải có khả năng:

- stack allocation;
- scalar replacement;
- complete inlining;
- elimination.

Escaping closure phải giữ receiver/captures đúng lifetime theo ownership model của Vut.

Không được giả định mọi closure đều heap allocated.

---

## 10. Static Dispatch

Implicit receiver calls phải được bind statically khi type đã biết.

Ví dụ:

```vut
Button("Save")
```

sau resolution nên giữ concrete target tương đương:

```text
ColumnScope.Button
```

MIR/codegen không được thực hiện string-based method lookup.

---

## 11. Ambiguity

Nếu receiver resolution có nhiều candidate hợp lệ và compiler không thể quyết định theo lexical/type rules:

```vut
Foo()
```

compiler phải báo diagnostic rõ ràng.

Không chọn candidate dựa trên declaration order.

---

## 12. Generic Compatibility

Receiver Functions phải được thiết kế tương thích với generics.

Conceptually:

```vut
fn(Scope(T))(T) -> void
```

Không tạo generic subsystem riêng cho receiver.

Specialization/monomorphization phải sử dụng infrastructure generic chung của Vut.

---

## 13. Ownership

Receiver invocation phải tuân theo memory model Vut:

```text
Value semantics
+
automatic move
+
deterministic drop
```

Không thêm receiver-specific reference counting.

Không retain receiver chỉ vì nó là receiver.

Ownership mode phải được xác định từ ordinary type/ownership rules.

---

## 14. Diagnostics

Compiler cần diagnostics riêng, rõ ràng cho:

- trailing body nhưng parameter cuối không phải function;
- receiver type mismatch;
- trailing callback argument count mismatch;
- implicit receiver method không tồn tại;
- ambiguous receiver resolution;
- invalid `.call()` receiver;
- capture ownership/lifetime violation.

Không để các lỗi trên biến thành generic "unknown symbol" nếu compiler có đủ context để báo lỗi tốt hơn.