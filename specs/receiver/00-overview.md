# Vut Receiver Functions — Overview

## 1. Mục tiêu

Receiver Function là một tính năng ngôn ngữ tổng quát cho phép một function có một receiver ngầm tương tự `self` của method.

Mục tiêu chính:

- hỗ trợ DSL type-safe theo phong cách Vut;
- không tạo DSL subsystem riêng;
- không cần Vutcom;
- không reflection;
- không runtime name lookup;
- không dynamic DSL dispatch;
- không bắt buộc heap allocation;
- giữ runtime performance gần code viết method/function trực tiếp;
- có thể tối ưu thành zero-cost abstraction khi compiler biết callback tại compile time.

Receiver Function phải là tính năng chung của function/type system, không được hardcode cho UI.

Nó có thể được dùng cho:

- UI DSL;
- HTML DSL;
- router;
- testing;
- build/config;
- transaction;
- serialization builder;
- các library DSL khác.

---

## 2. Function type bình thường

```vut
fn(int, str) -> bool
```

Function không có receiver.

---

## 3. Receiver Function

Syntax canonical:

```vut
fn(Receiver)(Args...) -> Return
```

Ví dụ:

```vut
fn(ColumnScope)() -> void
```

nghĩa là:

- receiver: `ColumnScope`
- arguments: không có
- return: `void`

Có arguments:

```vut
fn(UserCardScope)(Event) -> void
```

nghĩa là:

- receiver: `UserCardScope`
- argument thứ nhất: `Event`
- return: `void`

Receiver không phải ordinary argument trong type system.

---

## 4. `self`

Bên trong Receiver Function:

```vut
self
```

là receiver hiện tại.

Nếu expected receiver type là:

```vut
ColumnScope
```

thì:

```vut
self: ColumnScope
```

---

## 5. Implicit receiver lookup

Nếu receiver hiện tại có method:

```vut
fn ColumnScope.Button(text: str):
  ...
```

thì bên trong Receiver Function:

```vut
Button("Save")
```

có thể được resolve thành:

```vut
self.Button("Save")
```

Việc resolve này xảy ra tại compile time.

Không được tìm method theo tên ở runtime.

---

## 6. Trailing Function

Receiver Function kết hợp với trailing function syntax.

Ví dụ:

```vut
fn Column(body: fn(ColumnScope)() -> void):
  ...
```

user có thể viết:

```vut
Column():
  Button("Save")
  Button("Cancel")
```

Body là một anonymous Receiver Function có receiver `ColumnScope`.

Callback có argument:

```vut
fn UserCard(
  user: User,
  body: fn(UserCardScope)(Event) -> void
):
  ...
```

gọi:

```vut
UserCard(user) (event):
  Text(user.name)

  if event.clicked:
    select(user)
```

Compiler suy ra:

```text
receiver = UserCardScope
event    = Event
```

---

## 7. Gọi Receiver Function

Canonical invocation:

```vut
callback.call(receiver, ...args)
```

Receiver luôn đứng trước ordinary arguments.

Ví dụ:

```vut
body: fn(ColumnScope)() -> void

body.call(scope)
```

Có argument:

```vut
body: fn(UserCardScope)(Event) -> void

body.call(scope, event)
```

Nhiều arguments:

```vut
body: fn(FormScope)(Event, int, str) -> void

body.call(scope, event, index, name)
```

Không sử dụng:

```vut
body(scope)(event)
```

Receiver invocation phải phân biệt rõ với chained function calls.

---

## 8. Receiver và closure capture

Receiver và capture là hai khái niệm riêng.

```vut
title = "Users"

Column():
  Text(title)
```

Callback có:

```text
receiver:
  ColumnScope

capture:
  title
```

Receiver không được triển khai như một capture thông thường chỉ để đơn giản hóa compiler nếu điều đó làm mất thông tin semantic hoặc cản trở optimization.

---

## 9. Control flow

`if`, `for`, `match` bên trong DSL vẫn là control flow Vut bình thường.

```vut
Column():
  if loading:
    Spinner()
  else:
    for user in users:
      UserCard(user):
        Text(user.name)
```

Không tạo:

```text
DSLIf
DSLFor
DSLMatch
ComposeBranch
ComposeLoop
```

---

## 10. Nguyên tắc kiến trúc

Receiver phải được thiết kế theo:

```text
source syntax
↓
static receiver resolution
↓
typed receiver function
↓
MIR
↓
normal/native call lowering
```

Không thiết kế:

```text
source
↓
DSL runtime
↓
receiver lookup table
↓
reflection
↓
dynamic invocation
```

Receiver Function phải hướng tới zero-cost abstraction.