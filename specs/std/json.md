# Vut Standard Library — JSON

## 1. Mục tiêu

Module `json` cung cấp JSON parsing, serialization và typed conversion chính thức cho Vut.

Public import:

```vut id="6nd2de"
import json
```

Không dùng:

```vut id="8swtwz"
import std.json
```

`std/` chỉ là vị trí source của standard library, không phải public module namespace.

Mục tiêu:

```text id="8rmm2w"
parse JSON
serialize JSON
dynamic JSON tree
typed decode
typed encode
pretty printing
UTF-8 correctness
typed errors
safe number handling
bytes/str input
cross-platform
```

`json` chỉ xử lý JSON.

Không biến module này thành:

```text id="ytps6g"
HTTP client
schema validator
database mapper
configuration framework
JSON5 parser
YAML parser
XML parser
```

---

# 2. Scope MVP

MVP phải hỗ trợ:

```text id="yz9r8q"
JSON null
JSON boolean
JSON number
JSON string
JSON array
JSON object

parse from str
parse from bytes

stringify
pretty stringify

dynamic JsonValue

typed decode
typed encode

data
enum khi representation được định nghĩa
list(T)
array(T, N)
map(str, T)
optional T?
result-based errors

UTF-8 validation
escape/unescape
number validation
nested JSON
```

Chưa cần:

```text id="d7f23j"
JSON Schema
JSON5
comments
custom serializer framework
streaming SAX parser
incremental parser
JSONPath
JSON Patch
JSON Pointer
automatic HTTP integration
reflection runtime
arbitrary user-defined serialization hooks
```

---

# 3. Kiến trúc

Public API:

```text id="65f4u3"
Vut application
      ↓
import json
      ↓
vut-stdlib/std/json/
```

Preferred architecture:

```text id="2m3zwd"
public/high-level API
      ↓
Vut type metadata / compiler-supported serialization metadata
      ↓
JSON parser/serializer implementation
```

Nếu native backend thực sự cần thiết:

```text id="pyf14a"
vut-stdlib/std/json/
      ↓
_internal/native.vut
      ↓
stable C ABI
      ↓
vut-stdlib/native/vut-runtime/src/json/
      ↓
serde_json
```

Không được hardcode toàn bộ JSON library vào compiler.

Compiler chỉ cung cấp những capability tổng quát thực sự cần thiết cho typed serialization.

---

# 4. Rust crate strategy

Nếu JSON native backend được sử dụng, ưu tiên:

```text id="61j4zp"
serde_json
```

Có thể sử dụng:

```text id="rt7mdk"
serde
```

nếu implementation architecture thực sự cần.

Không expose:

```text id="n8ln7f"
serde_json::Value
serde_json::Number
serde_json::Error
serde::Serialize
serde::Deserialize
Rust String
Rust Vec
Rust HashMap
```

qua Vut ABI.

Không tự viết native JSON parser bằng Rust nếu `serde_json` đã đáp ứng yêu cầu.

Tuy nhiên không bắt buộc dùng native Rust nếu implementation thuần Vut đủ đúng và hiệu quả.

Ưu tiên:

```text id="h2hswq"
correctness
→ clean architecture
→ reuse mature implementation
→ performance
```

---

# 5. Folder structure

Vut:

```text id="3bl7gn"
vut-stdlib/
└── std/
    └── json/
        ├── mod.vut
        ├── value.vut
        ├── parse.vut
        ├── encode.vut
        ├── decode.vut
        ├── options.vut
        ├── error.vut
        └── _internal/
            ├── mod.vut
            ├── parser.vut
            ├── serializer.vut
            └── convert.vut
```

Nếu cần native backend:

```text id="ic6o7c"
vut-stdlib/
└── native/
    └── vut-runtime/
        └── src/
            └── json/
                ├── mod.rs
                ├── parse.rs
                ├── serialize.rs
                └── error.rs
```

Không tạo native backend nếu không thực sự cần.

Không gom toàn bộ JSON implementation vào một file lớn.

---

# 6. Dynamic JSON type

Module phải cung cấp:

```vut id="ykcc69"
json.Value
```

Conceptual variants:

```text id="0o8yuv"
null
bool
number
string
array
object
```

Exact internal representation phải phù hợp với enum/data/generic system hiện tại.

Không dùng:

```vut id="syp47f"
dyn
```

làm representation mặc định cho JSON.

`json.Value` phải là typed representation rõ ràng.

---

# 7. Parse

Canonical API:

```vut id="zw15zd"
value = json.parse(source)?
```

Ví dụ:

```vut id="88tzmc"
import json

fn main() -> result(null, json.Error):
  value = json.parse("""
    {"name":"Vut","version":1}
  """)?

  out(value.to_str())

  ok(null)
```

Nếu overload không được Vut hỗ trợ sạch, cung cấp explicit API:

```vut id="zd4zzf"
json.parse_str(source)
json.parse_bytes(data)
```

Không thêm compiler overload hack chỉ để có `json.parse`.

---

# 8. Parse từ bytes

JSON bytes phải được coi là encoded JSON input.

Conceptual:

```vut id="8xb8ka"
value = json.parse_bytes(data)?
```

Phải validate encoding theo JSON requirements.

MVP tối thiểu phải hỗ trợ UTF-8 đúng chuẩn.

Không silently replace invalid UTF-8.

Invalid input:

```text id="92mjro"
err(json.Error)
```

---

# 9. JSON literals

Không tạo JSON-specific source literal trong MVP.

Không thêm:

```text id="n3g6y4"
json{...}
#{...}
@[json ...]
```

JSON phải được tạo thông qua API/type bình thường của Vut.

Điều này tránh thêm syntax đặc biệt không cần thiết vào compiler.

---

# 10. Access JsonValue

Dynamic value phải hỗ trợ typed access.

Conceptual:

```vut id="fjjf48"
value.is_null()
value.is_bool()
value.is_number()
value.is_str()
value.is_array()
value.is_object()
```

Typed access:

```vut id="ov9et3"
value.as_bool()
value.as_int()
value.as_float()
value.as_str()
value.as_array()
value.as_object()
```

Các operation có thể thất bại phải trả optional hoặc result phù hợp.

Không:

```text id="57dwfz"
silent coercion
string → number tự động
number → string tự động
bool → number tự động
```

---

# 11. Object access

Không sử dụng `[]` vì Vut không có indexing syntax.

Conceptual:

```vut id="ylifzy"
name = value.get("name")
```

Ví dụ:

```vut id="svawxc"
name = value.get("name")?.as_str()
```

Exact optional/error composition phải theo semantics hiện tại của Vut.

Object API tối thiểu:

```text id="cdgka9"
get
set
remove
contains
keys
len
is_empty
```

Không duplicate `map` API một cách không cần thiết nếu implementation có thể reuse abstractions.

---

# 12. Array access

Không dùng:

```text id="uh2b2v"
value[0]
```

Dùng API phù hợp với Vut:

```vut id="3oy0p1"
item = value.at(0)
```

Array operations tối thiểu:

```text id="7dssxy"
at
set
push
remove
len
is_empty
```

Nếu `json.Value` immutable trong implementation ban đầu, mutation APIs có thể được thiết kế theo value semantics tương ứng.

---

# 13. JSON number

JSON number cần được xử lý cẩn thận.

Không mặc định ép mọi JSON number thành `f64` nếu điều đó làm mất integer precision.

Dynamic representation phải có khả năng bảo toàn hợp lý:

```text id="4tr0c6"
integer
unsigned integer khi phù hợp
floating point
hoặc lossless JSON-number representation nội bộ
```

Typed decode phải kiểm tra:

```text id="g6o1iw"
range
sign
fraction
overflow
underflow
target type
```

Ví dụ JSON:

```text id="6ap33z"
9223372036854775807
```

không được silently round qua `f64`.

---

# 14. String escaping

Parser/serializer phải xử lý đúng:

```text id="y30wo2"
\"
\\
\/
\b
\f
\n
\r
\t
\uXXXX
```

Unicode escape pair/surrogate handling phải đúng theo JSON specification.

Không tự implement Unicode handling bằng shortcut không an toàn.

---

# 15. Stringify

Canonical:

```vut id="gv6qpx"
text = json.stringify(value)?
```

Nếu stringify dynamic `json.Value` không có failure path sau khi value hợp lệ, API có thể trả `str` trực tiếp.

Typed serialization có thể trả:

```vut id="l6vm1h"
result(str, json.Error)
```

nếu conversion có thể thất bại.

Exact distinction phải được documented và nhất quán.

---

# 16. Pretty printing

Required:

```vut id="5pry10"
text = json.stringify_pretty(value)?
```

Default pretty format nên:

```text id="0xxph7"
2-space indentation
stable valid JSON
no trailing comma
```

Có thể hỗ trợ options:

```vut id="xx3ir9"
options = json.EncodeOptions(
  pretty = true,
  indent = 2
)
```

Không cần formatter framework phức tạp trong MVP.

---

# 17. Typed decode

Đây là capability quan trọng.

Ví dụ:

```vut id="9nh41j"
data User:
  name: str
  age: int
  active: bool

user = json.decode(User, source)?
```

Hoặc nếu compiler/generic architecture yêu cầu:

```vut id="c5bngn"
user: User = json.decode(source)?
```

Ưu tiên type inference nếu generic system hiện tại hỗ trợ sạch.

Không thêm syntax riêng chỉ dành cho JSON.

---

# 18. Typed encode

Ví dụ:

```vut id="qgw2f4"
data User:
  name: str
  age: int

user = User(
  name = "Nam",
  age = 20
)

text = json.encode(user)?
```

Output:

```json id="3j1y4s"
{"name":"Nam","age":20}
```

Typed encode phải dựa trên Vut type metadata/static type information, không dựa vào runtime `dyn`.

---

# 19. Generic collections

Typed JSON phải hỗ trợ:

```vut id="jaf5hu"
list(User)
map(str, User)
array(int, 4)
```

Ví dụ:

```vut id="5f57v4"
users: list(User) = json.decode(source)?
```

JSON array:

```text id="51yvko"
→ list(T)
```

khi target là `list(T)`.

JSON object:

```text id="md95s7"
→ map(str, T)
```

khi target là `map(str, T)`.

---

# 20. Fixed arrays

Decode vào:

```vut id="cd2k97"
array(T, N)
```

phải yêu cầu JSON array có đúng `N` phần tử.

Ví dụ:

```vut id="m6fb8a"
position: array(float, 3) = json.decode("[1,2,3]")?
```

Nếu JSON có:

```text id="a9ys7m"
[1,2]
```

phải trả decode error.

Không truncate hoặc pad tự động.

---

# 21. Optional

Optional mapping:

```vut id="sy22cc"
str?
```

phải hỗ trợ JSON:

```text id="gmblbf"
null
```

Ví dụ:

```vut id="qg29ha"
data User:
  name: str
  nickname: str?
```

JSON:

```json id="ncm3a6"
{
  "name": "Nam",
  "nickname": null
}
```

phải decode hợp lệ.

Missing field và explicit `null` không nhất thiết có cùng semantics đối với mọi type.

Rules phải được document rõ.

---

# 22. Missing fields

Required field:

```vut id="k3ahab"
data User:
  name: str
```

JSON:

```json id="llw0u3"
{}
```

phải lỗi:

```text id="xq5q56"
missing_field
```

Nếu field có default value và Vut metadata cho phép nhận biết default:

```vut id="x5bj4f"
data User:
  active: bool = true
```

decoder nên sử dụng default khi field missing.

Không silently tạo zero value cho required field.

---

# 23. Unknown fields

MVP mặc định nên **ignore unknown fields**.

Ví dụ target:

```vut id="jdd8pk"
data User:
  name: str
```

JSON:

```json id="ll19gt"
{
  "name": "Nam",
  "future_field": 123
}
```

vẫn decode được.

Điều này giúp forward compatibility.

Sau này có thể thêm strict mode.

---

# 24. Decode options

Có thể cung cấp:

```vut id="h5flcc"
json.DecodeOptions(
  deny_unknown_fields = false
)
```

MVP chỉ cần options thực sự hữu ích.

Không xây một configuration framework quá lớn.

---

# 25. Field names

Mặc định Vut field name map trực tiếp sang JSON key.

```vut id="0itnp5"
data User:
  first_name: str
```

mặc định:

```json id="2fr7y5"
{
  "first_name": "Nam"
}
```

Không tự động camelCase nếu user không yêu cầu.

Predictability quan trọng hơn magic.

---

# 26. Serialization attributes

Không tự phát minh JSON-specific attribute syntax nếu Attribute system chưa có spec chính thức cho custom serialization.

Hướng tương lai có thể là:

```vut id="hml3na"
@json(name = "firstName")
first_name: str
```

hoặc tương đương.

Nhưng **không implement trong MVP** nếu attribute-on-field semantics chưa được language specs hỗ trợ chính thức.

Compiler không được special-case ad-hoc JSON annotations.

---

# 27. Enum

Enum serialization phải có một canonical representation.

Đối với simple enum:

```vut id="qhmvzx"
enum Status:
  pending
  running
  done
```

Recommended JSON representation:

```json id="av4zqh"
"pending"
```

Không encode ordinal:

```json id="s4yvll"
0
```

vì ordinal không ổn định khi enum thay đổi.

Decode unknown enum value phải trả typed error.

Payload enum/advanced enum chỉ hỗ trợ khi Vut enum model chính thức hỗ trợ và representation được spec rõ.

Không tự phát minh representation trong JSON layer.

---

# 28. `result(T, E)`

`result(T, E)` chủ yếu là error-handling abstraction, không nên tự động serialize như application data trong MVP.

Không mặc định biến:

```vut id="qj82fx"
ok(value)
err(error)
```

thành JSON.

Nếu user muốn encode Result, họ phải chuyển thành data representation rõ ràng.

---

# 29. `dyn`

Không sử dụng `dyn` làm fallback khi typed decode thất bại.

Ví dụ:

```vut id="g4f66j"
user: User = json.decode(source)?
```

nếu JSON không match `User`:

```text id="5j5mrw"
err(json.Error)
```

không:

```text id="s0bfm7"
fallback dyn
```

---

# 30. Error model

Canonical public error:

```vut id="qubgnn"
json.Error
```

Error kinds tối thiểu:

```text id="gk0wdr"
syntax
unexpected_end
invalid_utf8
invalid_escape
invalid_unicode
invalid_number
number_overflow
type_mismatch
missing_field
unknown_field
invalid_enum
invalid_length
unsupported_type
other
```

Conceptual:

```vut id="0bpzse"
data Error:
  kind: ErrorKind
  message: str
  line: int?
  column: int?
```

Parser errors nên có:

```text id="pj1j50"
line
column
```

khi có thể.

Không chỉ trả `"invalid json"` nếu vị trí lỗi đã biết.

---

# 31. Diagnostics example

Input:

```json id="o2y8ru"
{
  "name": "Nam",
  "age":
}
```

Error nên tương tự:

```text id="bkwqf8"
JSON syntax error at line 3, column 8:
expected a JSON value
```

Typed decode:

```text id="gkm9ze"
JSON decode error:
field `age`
expected int
found string
```

Nested error nên giữ path nếu implementation hỗ trợ:

```text id="cc69zp"
users[3].address.zip
```

Không bắt buộc source syntax `[]`; đây chỉ là diagnostic path notation.

---

# 32. UTF-8

`str` input đã phải là valid Vut string.

`bytes` input phải được validate.

Invalid UTF-8:

```text id="ywy38l"
err(json.Error)
```

Không dùng lossy replacement mặc định.

---

# 33. Memory model

JSON phải tuân thủ Vut memory model:

```text id="gw5x1c"
value semantics
automatic moves
deterministic drop
managed str
managed list
managed map
no tracing GC
no manual free trong safe Vut
```

Đặc biệt dynamic `json.Value` có thể chứa nested managed values.

Compiler/runtime phải xử lý chính xác:

```text id="v1idqr"
copy
move
drop
nested drop
error cleanup
partial parse cleanup
partial decode cleanup
```

Không workaround ownership bugs bằng leaking allocations.

---

# 34. Performance

Parser/serializer phải tránh:

```text id="53egai"
character-by-character heap allocation
temporary strings không cần thiết
repeated whole-document copies
convert bytes → str → bytes không cần thiết
boxing từng primitive nếu representation khác tốt hơn
```

Ưu tiên:

```text id="t6n49n"
single-pass parsing
buffer reuse
preallocation khi biết kích thước
efficient string escaping
move values thay vì copy khi có thể
```

Correctness vẫn quan trọng hơn micro-optimization.

---

# 35. Native backend ABI

Nếu dùng `serde_json`, boundary vẫn phải là stable C ABI.

Không expose Rust-specific data structures.

Ưu tiên tránh việc:

```text id="cq0k38"
Rust serde_json::Value
        ↓
xây một native tree
        ↓
copy toàn bộ thành Vut json.Value
```

nếu kiến trúc đó gây double representation và allocation quá lớn.

Agent phải đánh giá liệu:

```text id="rlwpxf"
pure Vut parser
```

hay:

```text id="fvbpk9"
native serde_json parser
```

phù hợp hơn với ownership/performance hiện tại.

Nếu native backend chỉ làm parsing validation nhưng sau đó phải copy toàn bộ tree nhiều lần, pure Vut implementation có thể tốt hơn.

---

# 36. Typed serialization architecture

Không hardcode từng user `data` type vào JSON runtime.

Typed encode/decode cần một cơ chế tổng quát dựa trên compiler-known type information.

Compiler có thể cung cấp compile-time metadata/generation cần thiết cho:

```text id="kxvttk"
field names
field types
field order
optional fields
default information khi supported
enum variants
collection element types
```

Nhưng:

```text id="rf3kq6"
json-specific business logic
JSON parser
JSON serializer
JSON error formatting
```

không thuộc compiler core.

Ưu tiên compile-time specialization thay vì runtime reflection nặng.

---

# 37. Generic typed API

Khi Generic của Vut hoàn thiện, preferred conceptual API:

```vut id="7pqjgc"
fn decode(T)(source: str) -> result(T, Error):
  ...

fn encode(T)(value: T) -> result(str, Error):
  ...
```

Usage:

```vut id="vdr8tp"
user: User = json.decode(source)?

text = json.encode(user)?
```

Nếu type inference không đủ:

```vut id="dlthxa"
user = json.decode(User, source)?
```

có thể được dùng tạm thời nếu phù hợp với language architecture.

Không invent explicit generic call syntax trái với generic specs.

---

# 38. HTTP integration

`json` không phụ thuộc `http`.

`http` không bắt buộc phụ thuộc `json`.

User composition:

```vut id="d79ckb"
import http
import json

data User:
  name: str
  age: int

async fn main() -> result(null, AppError):
  response = await http.get("https://api.example.com/user")?

  user: User = json.decode(response.text()?)?

  out(user.name)

  ok(null)
```

Hai module phải composable nhưng độc lập.

---

# 39. File integration

Không thêm:

```text id="61dlgj"
json.read_file
json.write_file
```

vào MVP.

Reuse `fs`:

```vut id="u2y3i6"
import fs
import json

source = fs.read_str("config.json")?
config: Config = json.decode(source)?
```

Serialize:

```vut id="4lsckw"
text = json.encode(config)?
fs.write_str("config.json", text)?
```

Không duplicate filesystem functionality.

---

# 40. Streaming

MVP không cần streaming parser.

`json.parse`/`decode` xử lý một complete JSON document.

Streaming JSON, NDJSON hoặc incremental parsing có thể thiết kế sau.

Không làm parser architecture hiện tại không thể mở rộng, nhưng cũng không over-engineer MVP cho streaming.

---

# 41. Security / limits

Parser không được recursion/allocate vô hạn mà không có guard.

Implementation nên có reasonable protection cho:

```text id="u0gvcf"
extreme nesting
huge malformed numbers
pathological escape sequences
integer overflow
allocation overflow
malformed UTF-8
```

Nếu cần options:

```text id="om69fm"
max_depth
```

có thể được hỗ trợ.

Default phải an toàn cho normal application use.

---

# 42. Deterministic output

`json.stringify` phải luôn tạo valid JSON.

Không yêu cầu object key ordering mang semantic meaning.

Nếu `map` không giữ insertion order, serializer không được hứa giữ order.

Pretty output chỉ đảm bảo formatting, không đảm bảo canonical JSON ordering.

Canonical JSON có thể là feature riêng sau.

---

# 43. Tests

Comprehensive tests phải bao gồm:

```text id="kjb44x"
null
bool
integer
float
large integer
negative number
string
Unicode
escape sequences
array
object
deep nesting

invalid syntax
unexpected EOF
invalid UTF-8
invalid escapes
invalid Unicode
invalid numbers
overflow

parse str
parse bytes

stringify
pretty stringify
parse → stringify → parse

JsonValue access
object access
array access

typed data encode
typed data decode
nested data
list(T)
map(str, T)
array(T, N)
optional
default fields
missing fields
unknown fields
enum

type mismatch
nested error path

managed ownership
moves
copies
drops
partial parse failure
partial decode failure

large documents
cross-platform behavior
```

Nếu native Rust backend tồn tại, test cả native layer và Vut end-to-end layer.

---

# 44. Public API summary

MVP public surface nên bao gồm:

```text id="41jtns"
json.Value

json.parse
json.parse_str
json.parse_bytes

json.stringify
json.stringify_pretty

json.decode
json.encode

json.Error
json.ErrorKind

json.DecodeOptions
json.EncodeOptions
```

Không bắt buộc giữ cả `parse` và `parse_str` nếu overload/type system khiến chúng redundant.

Chỉ có một canonical API cho mỗi operation.

---

# 45. Example — dynamic JSON

```vut id="y2vszx"
import json

fn main() -> result(null, json.Error):
  value = json.parse("""
    {
      "name": "Vut",
      "fast": true,
      "version": 1
    }
  """)?

  name = value.get("name")?.as_str()

  out("Language: $name")

  ok(null)
```

---

# 46. Example — typed JSON

```vut id="56n2pz"
import json

data User:
  name: str
  age: int
  active: bool

fn main() -> result(null, json.Error):
  source = """
    {
      "name": "Nam",
      "age": 20,
      "active": true
    }
  """

  user: User = json.decode(source)?

  out("$(user.name) - $(user.age)")

  encoded = json.encode(user)?

  out(encoded)

  ok(null)
```

---

# 47. Example — HTTP + JSON

```vut id="x92ygm"
import http
import json

data User:
  name: str
  age: int

async fn main() -> result(null, AppError):
  response = await http.get("https://api.example.com/user")?

  user: User = json.decode(response.text()?)?

  out(user.name)

  ok(null)
```

---

# 48. Definition of Done

JSON MVP hoàn thành khi:

```text id="ag1xpl"
import json
```

hoạt động như top-level stdlib import và:

```text id="w4scmv"
dynamic JSON parsing
dynamic JsonValue
string/bytes input
serialization
pretty serialization
typed data decode
typed data encode
list/map/array support
optional support
enum basic support
typed errors
UTF-8 correctness
number correctness
ownership correctness
nested structures
HTTP/fs composability
```

đều chạy end-to-end.

Không giảm typed API xuống `dyn`.

Không dùng integer-coded public errors để né compiler correctness bug.

Nếu generic/type metadata/ownership foundation bị lỗi, sửa foundation trước.

---

# 49. Nguyên tắc cuối cùng

```text id="cmj2ik"
Simple Vut API
+
Static typing
+
Dynamic JsonValue khi thực sự cần
+
Typed encode/decode
+
No dyn fallback
+
Correct UTF-8
+
Correct numbers
+
Typed errors
+
Value semantics
+
Compile-time specialization where possible
+
Mature Rust crates when native backend is justified
```

JSON phải cảm giác như một phần tự nhiên của Vut type system, không phải một Rust/Serde API được bọc lại.

Public semantics thuộc về Vut.

Rust/Serde nếu được sử dụng chỉ là implementation detail.
