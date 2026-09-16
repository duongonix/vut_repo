# Receiver Functions — Compiler & Runtime

## 1. Nguyên tắc

Receiver Function phải hướng tới zero-cost abstraction.

Compiler implementation không được tạo một DSL runtime.

Target architecture:

```text
Parser
↓
AST/HIR
↓
Resolver
↓
Type Checker
↓
Closure/Capture Analysis
↓
MIR
↓
Optimization
↓
Codegen
```

---

## 2. AST/HIR

Function type phải có khả năng biểu diễn receiver riêng:

```text
FunctionType
  receiver: Type?
  params: [...]
  return_type: Type
```

Trailing function syntax phải được biểu diễn đủ rõ để semantic analysis lấy expected function type.

Không hardcode UI/DSL concepts.

---

## 3. Resolver

Resolver phải duy trì lexical receiver context.

Ví dụ:

```vut
App():
  Column():
    Button()
```

conceptual stack:

```text
AppScope
AppScope → ColumnScope
```

Khi resolve `Button`, target phải được xác định tại compile time.

---

## 4. MIR

MIR phải biểu diễn receiver call mà không cần dynamic DSL machinery.

Có thể dùng operation semantic tương đương:

```text
CallReceiver {
  callee,
  receiver,
  args
}
```

hoặc lower sang generic call representation nếu vẫn giữ đủ ownership/type information.

Không tạo:

```text
DslCall
UiCall
ComposeCall
ReceiverLookupByName
```

---

## 5. ABI

Native/codegen ABI có thể lower:

```vut
callback.call(scope, event)
```

thành concept:

```text
callback(scope, event)
         ↑
    hidden receiver
```

Receiver là hidden first parameter ở backend ABI.

Không tạo argument array.

Không boxing receiver chỉ để gọi.

Không reflection.

Không runtime method lookup.

---

## 6. Direct vs Indirect Calls

Nếu callee được biết tại compile time:

```text
direct call
```

phải được ưu tiên.

Nếu receiver function là function value chưa biết target:

```text
indirect function call
```

được phép.

Optimizer phải devirtualize/resolve indirect call khi target thực tế có thể chứng minh được.

---

## 7. Non-Capturing Receiver Functions

Non-capturing receiver function không được yêu cầu closure environment allocation.

Representation có thể tối giản thành:

```text
function pointer
```

receiver được truyền lúc `.call()`.

---

## 8. Capturing Receiver Functions

Capturing receiver function có thể cần:

```text
code pointer
+
environment
```

Receiver vẫn được truyền riêng tại invocation.

Conceptually:

```text
code(environment, receiver, args...)
```

Không bắt buộc ABI chính xác phải như trên, nhưng implementation phải tránh boxing/allocation không cần thiết.

---

## 9. Non-Escaping Closures

Đây là fast path quan trọng nhất cho DSL.

Ví dụ:

```vut
Column():
  Text("A")
  Text("B")
```

nếu callback chỉ được gọi trong `Column` và không escape:

```text
closure allocation should be avoidable
```

Compiler phải thiết kế representation để sau này hỗ trợ:

- stack environment;
- inlining;
- scalar replacement;
- dead closure elimination.

Không khóa ABI vào heap closure object bắt buộc.

---

## 10. Escaping Closures

Nếu callback được lưu:

```vut
Button("Save") (event):
  save(user)
```

và library giữ callback để chạy sau, environment có thể cần allocation.

Allocation chỉ được thực hiện khi lifetime thực sự yêu cầu.

Ownership phải deterministic.

Exactly-once destruction phải được đảm bảo cho managed captures/resources.

---

## 11. `.call()` không phải dynamic method

Source:

```vut
callback.call(scope, event)
```

`.call` là canonical language operation cho Receiver Function.

Không implement bằng ordinary runtime method lookup.

Compiler phải nhận biết receiver-function type và lower trực tiếp.

---

## 12. Integration với Closures

Receiver Function phải dùng generic closure infrastructure.

Không tạo:

```text
ReceiverClosureBox
DslClosureBox
UiClosureBox
```

nếu generic closure representation có thể biểu diễn nó.

Receiver metadata vẫn phải được giữ trong semantic type system.

---

## 13. Integration với Vutcon/Async

Không tạo ABI riêng chỉ cho receiver.

Nếu receiver closure được dùng với Vutcon/async trong tương lai, nó phải đi qua generic callable/closure infrastructure.

Không hardcode receiver vào:

- Vutcon scheduler;
- async poll ABI;
- HTTP;
- executor.

Receiver feature không được tự ý thay đổi concurrency semantics.

---

## 14. Codegen Target

Mục tiêu cuối:

```vut
Column():
  Button("A")
  Button("B")
```

sau optimization có khả năng trở thành gần tương đương code thủ công:

```text
create ColumnScope
ColumnScope.Button("A")
ColumnScope.Button("B")
finish ColumnScope
```

Callback object/call có thể bị eliminate nếu compiler chứng minh được.

---

## 15. Không được làm

Không:

- reflection;
- string-based dispatch;
- runtime receiver stack cho language semantics;
- global current receiver;
- hidden thread-local receiver;
- mandatory heap allocation;
- mandatory RC;
- argument boxing;
- `Any`/`dyn` fallback;
- DSL-specific MIR;
- UI-specific compiler logic;
- receiver-specific GC;
- receiver-specific async runtime.

Receiver là compile-time typed language feature.