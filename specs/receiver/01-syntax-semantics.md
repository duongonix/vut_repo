# Receiver Functions — Syntax & Semantics

## 1. Receiver Function Type

Canonical syntax:

```vut
fn(Receiver)(Args...) -> Return
```

Examples:

```vut
fn(AppScope)() -> void
fn(ColumnScope)() -> Node
fn(UserCardScope)(Event) -> void
fn(FormScope)(Event, int) -> bool
```

Không dùng syntax Kotlin trực tiếp.

---

## 2. Receiver Function không phải curried function

Hai type sau khác nhau:

```vut
fn(Scope)(Event) -> void
```

và:

```vut
fn(Scope) -> fn(Event) -> void
```

Type đầu là Receiver Function.

Type sau là normal function trả về một function.

---

## 3. Invocation

Receiver Function được gọi bằng:

```vut
callback.call(receiver, ...args)
```

Examples:

```vut
body.call(scope)
body.call(scope, event)
body.call(scope, event, index)
```

Receiver là operand đầu tiên của `.call()` nhưng không trở thành ordinary source-level function parameter.

Compiler phải kiểm tra:

```text
receiver type
argument count
argument types
return type
```

---

## 4. Trailing Function không có arguments

Khi parameter cuối còn thiếu có expected type:

```vut
fn(ColumnScope)() -> void
```

thì:

```vut
Column():
  Button("A")
  Button("B")
```

tạo Receiver Function tương ứng.

`:` chỉ được dùng theo trailing-function semantics khi parameter cuối còn thiếu là function type phù hợp.

---

## 5. Trailing Function có arguments

Nếu:

```vut
fn UserCard(
  user: User,
  body: fn(UserCardScope)(Event) -> void
):
  ...
```

thì:

```vut
UserCard(user) (event):
  Text(user.name)
```

`event` được contextually typed thành:

```vut
Event
```

Không yêu cầu:

```vut
(event: Event)
```

trừ khi sau này language specification cho phép optional explicit annotations.

---

## 6. Chained call vs trailing function

Hai syntax khác nhau:

```vut
foo()(10)
```

là chained function call.

```vut
foo() (value):
  body
```

là trailing function argument.

Parser sử dụng `:` + indented body để phân biệt.

Không có `:`:

```vut
foo()(10)
```

phải giữ semantics chained call.

---

## 7. `self`

Trong Receiver Function:

```vut
self
```

trỏ tới receiver.

Ví dụ:

```vut
fn ColumnScope.Button(text: str):
  self.children.push(...)
```

Trong:

```vut
Column():
  self.Button("A")
```

`self` có type `ColumnScope`.

Implicit form:

```vut
Column():
  Button("A")
```

được resolver ánh xạ tới receiver method khi hợp lệ.

---

## 8. Nested Receivers

Ví dụ:

```vut
App():
  Column():
    UserCard(user):
      Text(user.name)
```

Receiver scopes:

```text
AppScope
  ↓
ColumnScope
  ↓
UserCardScope
```

Receiver gần nhất là receiver hiện tại.

Khi thoát nested callback, lexical receiver trước đó được phục hồi.

---

## 9. Receiver Resolution

MVP precedence:

```text
1. local bindings
2. current receiver
3. outer lexical receivers
4. module/import symbols
```

Explicit qualification luôn có nghĩa rõ ràng:

```vut
self.Button()
module.Button()
object.Button()
```

Nếu resolution không thể xác định duy nhất symbol hợp lệ, compiler phải báo ambiguity thay vì chọn tùy tiện.

---

## 10. Normal Vut code trong Receiver Function

Receiver Function body vẫn là Vut bình thường:

```vut
Column():
  title = load_title()

  if title.empty():
    Text("Empty")
  else:
    Text(title)

  for user in users:
    UserCard(user):
      Text(user.name)
```

Các construct sau không cần phiên bản DSL riêng:

- local variables;
- assignments;
- function calls;
- methods;
- `if`;
- `for`;
- `match`;
- block expressions;
- `result(T,E)`;
- `?`;
- ordinary anonymous functions.

---

## 11. Receiver không phải runtime context lookup

Compiler phải resolve:

```vut
Button("Save")
```

tới symbol cụ thể tại compile time.

Không được implement bằng:

```text
find current receiver at runtime
lookup "Button"
dynamic invoke
```

---

## 12. Canonical DSL Example

```vut
app = App():
  Header():
    Text("Vut")

  Column():
    if users.empty():
      Text("No users")
    else:
      for user in users:
        UserCard(user) (event):
          Text(user.name)

          if event.clicked:
            select(user)

  Footer():
    Text("2026")
```

Đây vẫn là Vut bình thường sử dụng Receiver Functions và trailing functions, không phải một sub-language.