# Vut Standard Library — HTTP

## 1. Mục tiêu

Module `http` cung cấp HTTP client chính thức của Vut.

Mục tiêu:

```text
simple API
async-first
HTTP + HTTPS
typed errors
streaming
connection reuse
timeout
redirect
Range Request
Vutcon compatible
cross-platform
```

`http` là HTTP client foundation, không phải web framework.

Public import:

```vut
import http
```

Không dùng:

```vut
import std.http
```

`std/` chỉ là vị trí source của standard library, không phải public module namespace.

---

# 2. Scope MVP

MVP phải hỗ trợ:

```text
GET
POST
PUT
PATCH
DELETE
HEAD

HTTP
HTTPS

Client
Request
Response
Headers
Method
Status
HttpError

query parameters
request headers
request body
string body
bytes body
response text
response bytes
streaming response
timeouts
redirects
connection pooling / reuse
Range Requests

async/await
Vutcon compatibility
```

Chưa triển khai:

```text
HTTP server
WebSocket
HTTP/3
multipart/form-data
cookie jar
authentication framework
automatic retry framework
HTTP cache
SSE
proxy configuration
advanced TLS configuration
JSON integration
```

Các feature trên có thể được bổ sung sau nhưng không được làm phình MVP.

---

# 3. Kiến trúc

HTTP phải tuân theo kiến trúc stdlib:

```text
Vut application
      ↓
import http
      ↓
vut-stdlib/std/http/
      ↓
_internal/native.vut
      ↓
stable C ABI
      ↓
vut-stdlib/native/vut-runtime/src/http/
      ↓
reqwest
      ↓
Tokio
      ↓
rustls
      ↓
OS/network
```

High-level API và semantics ưu tiên viết bằng Vut.

Rust chỉ chịu trách nhiệm cho native HTTP/network functionality và bridge tới Rust ecosystem.

Không hardcode `http.get`, `Request`, `Response`, `Client`, v.v. vào compiler.

Compiler chỉ được thay đổi nếu có language/runtime capability thực sự cần thiết.

---

# 4. Rust backend — MVP đã chốt

MVP sử dụng hướng:

```text
reqwest
+
Tokio
+
rustls
+
url
+
bytes
```

## 4.1 `reqwest`

`reqwest` là HTTP client backend chính.

Ưu tiên sử dụng functionality có sẵn của `reqwest` cho:

```text
HTTP/1.1
HTTP/2
request execution
response handling
headers
redirects
timeouts
connection pooling
keep-alive
streaming
request body
response body
HTTPS integration
```

Không tự viết HTTP parser hoặc HTTP connection engine nếu `reqwest` đã cung cấp functionality phù hợp.

---

## 4.2 `Tokio`

Tokio được phép sử dụng làm native async runtime cho HTTP backend trong MVP.

Kiến trúc:

```text
Vut async/Vutcon
      ↓
native HTTP ABI
      ↓
Tokio runtime
      ↓
reqwest
```

Tokio là **implementation detail**.

Không expose:

```text
tokio::Task
tokio::Runtime
tokio::JoinHandle
Rust Future
```

sang Vut.

Không thiết kế public Vut concurrency API dựa trên Tokio.

Vut vẫn sử dụng:

```text
async fn
await
vut(...)
vutcon[T]
```

làm concurrency model chính thức.

Kiến trúc phải cho phép sau này thay Tokio bằng integration trực tiếp với Vut scheduler mà không thay đổi public `http` API.

---

## 4.3 `rustls`

Ưu tiên Rustls cho TLS.

Không tự implement:

```text
TLS protocol
certificate validation
cipher implementation
certificate chain parsing
```

HTTP module phải hỗ trợ HTTPS mặc định:

```text
https://
```

User không cần:

```vut
import tls
```

chỉ để gọi HTTPS.

Certificate validation phải bật mặc định.

Không được silently disable TLS verification.

---

## 4.4 `url`

Sử dụng crate `url` cho:

```text
URL parsing
URL validation
query encoding
percent encoding
URL manipulation cần native correctness
```

Không tự viết URL parser phức tạp.

---

## 4.5 `bytes`

Sử dụng `bytes` khi phù hợp cho efficient native network buffers.

Không expose `bytes::Bytes` qua ABI.

Phía Vut vẫn sử dụng core type:

```vut
bytes
```

Bridge phải tuân thủ managed bytes ABI của Vut.

---

# 5. Dependency rule

Ưu tiên:

```text
Rust std
→ mature Rust crate
→ platform API
→ custom implementation
```

Không tự implement functionality phức tạp đã được các dependency chính giải quyết tốt.

Không thêm crate lớn nếu functionality đã có trong dependency hiện tại.

Không để type của dependency Rust leak qua stable C ABI.

---

# 6. Folder structure

Public Vut stdlib:

```text
vut-stdlib/
└── std/
    └── http/
        └── mod.vut
```

Vut has no `export`/re-export and no submodule aggregation
(`specs/07-modules-imports.md` §7, §34): the public namespace of a module is
exactly the public symbols declared in its own `mod.vut`. Moving public types
(for example `Headers`) into a sibling file would rename them to
`http.headers.Headers` for users. The `http` public API therefore stays in one
`mod.vut`; helpers that are not part of the API are `_`-prefixed in that file.

Native Rust backend:

```text
vut-stdlib/
└── native/
    └── vut-runtime/
        └── src/
            └── http/
                ├── mod.rs
                ├── client.rs
                ├── request.rs
                ├── response.rs
                ├── stream.rs
                ├── future.rs
                ├── runtime.rs
                └── error.rs
```

Có thể tách thêm file nếu responsibility thực tế yêu cầu.

Không gom toàn bộ HTTP implementation vào một `http.rs` khổng lồ.

Tuân thủ file/module rules trong `AGENTS.md`.

---

# 7. Async-first

Network operations phải async-first.

Canonical usage:

```vut
import http

async fn main() -> result[unit, http.Error]:
  response = await http.get("https://example.com")?

  text = response.text()?

  out(text)

  ok(unit)
```

Không block OS thread trong lúc chỉ đang chờ network I/O nếu backend có khả năng async.

---

# 8. Convenience request API

Các request phổ biến:

```vut
response = await http.get(url)?
response = await http.delete(url)?
response = await http.head(url)?
```

Body methods:

```vut
response = await http.post(url, body)?
response = await http.put(url, body)?
response = await http.patch(url, body)?
```

`body` tối thiểu phải hỗ trợ:

```text
str
bytes
```

Nếu type system không hỗ trợ overload sạch ở thời điểm implementation, thiết kế API tương đương phù hợp với Vut thay vì thêm compiler hack.

Không dùng `dyn` chỉ để giả lập overload.

---

# 9. Client

`Client` quản lý reusable HTTP state:

```vut
client = http.Client()

response = await client.get("https://example.com")?
```

Client chịu trách nhiệm native cho:

```text
connection pooling
keep-alive
TLS configuration mặc định
redirect defaults
timeout defaults
HTTP backend state
```

Không tạo connection engine hoàn toàn mới cho mỗi request nếu backend có thể reuse connection.

Ví dụ cấu hình:

```vut
import http
import time

client = http.Client(
  timeout: time.seconds(30),
  redirects: 5
)
```

Nếu constructor/default syntax hiện tại của Vut chưa hỗ trợ chính xác API trên, triển khai API tương đương theo language specs hiện tại.

Không thay language syntax chỉ để phục vụ HTTP.

---

# 10. Request

Request API dùng khi convenience API không đủ.

Ví dụ:

```vut
request = http.Request(
  method: http.Method.post,
  url: "https://api.example.com/users"
)

request.header("Authorization", "Bearer $token")
request.header("Content-Type", "application/octet-stream")
request.body(data)

response = await request.send()?
```

Request phải hỗ trợ:

```text
method
url
headers
query parameters
body
timeout
redirect behavior khi cần
Range Request
send
```

---

# 11. HTTP methods

Public method representation:

```vut
http.Method.get
http.Method.post
http.Method.put
http.Method.patch
http.Method.delete
http.Method.head
http.Method.options
```

MVP phải hỗ trợ ít nhất:

```text
GET
POST
PUT
PATCH
DELETE
HEAD
OPTIONS
```

Không hardcode logic HTTP method vào compiler.

---

# 12. Headers

Cần type:

```vut
http.Headers
```

Ví dụ:

```vut
headers = http.Headers()

headers.set("Authorization", "Bearer $token")
headers.set("Accept", "application/json")

content_type = headers.get("Content-Type")
```

Required API:

```text
get
set
append
remove
contains
```

HTTP headers không được implementation đơn giản như một `map[str, str]` nếu điều đó làm mất multi-value semantics.

Header names phải được xử lý case-insensitive theo HTTP semantics.

`append` phải cho phép nhiều value khi protocol cho phép.

---

# 13. Query parameters

Không bắt user tự nối query string.

Ví dụ:

```vut
request.query("page", "2")
request.query("limit", "20")
```

Backend phải encode query parameter đúng.

Ví dụ logical result:

```text
?page=2&limit=20
```

Special characters phải percent-encode đúng.

Ưu tiên dùng crate `url`.

---

# 14. Request body

MVP hỗ trợ:

```text
bytes body
UTF-8 string body
```

Ví dụ:

```vut
request.body(data)
```

hoặc API tương đương phù hợp với type system.

String body phải được encode UTF-8 rõ ràng.

Không tự động serialize arbitrary Vut data thành JSON.

JSON thuộc module/package riêng.

---

# 15. Response

Response phải expose ít nhất:

```text
status
headers
body access
```

Conceptual usage:

```vut
response = await http.get(url)?

out("status = $(response.status.code)")
```

Required helpers:

```text
is_success
text
bytes
header
stream
```

Ví dụ:

```vut
if response.is_success():
  data = response.bytes()
```

---

# 16. Status

Status phải giữ numeric HTTP status code.

Conceptual:

```vut
response.status.code
```

Helpers:

```text
is_informational
is_success
is_redirect
is_client_error
is_server_error
```

Semantics:

```text
100–199 informational
200–299 success
300–399 redirect
400–499 client error
500–599 server error
```

Không cần enum riêng cho mọi HTTP status code.

Thiết kế phải vẫn hỗ trợ unknown/future status codes.

---

# 17. HTTP status không phải transport error

Điều này là bắt buộc.

Ví dụ:

```text
404
401
500
503
```

không tự động trở thành:

```vut
err(...)
```

Ví dụ:

```vut
response = await http.get(url)?

if response.status.code == 404:
  out("Not found")
```

`err(HttpError)` dành cho lỗi transport/request execution như:

```text
invalid URL
DNS failure
connection failure
TLS failure
timeout
redirect failure
invalid response
body/stream transport failure
cancellation
```

---

# 18. Response text

```vut
text = response.text()?
```

Phải validate UTF-8.

Không silently replace invalid UTF-8.

Nếu body không phải UTF-8 hợp lệ:

```text
err(HttpError)
```

hoặc typed error phù hợp với error architecture cuối cùng.

Không dùng lossy conversion mặc định.

---

# 19. Response bytes

```vut
data = response.bytes()
```

Trả core Vut:

```vut
bytes
```

Ownership phải tuân thủ managed bytes ABI.

Không expose native Rust buffer trực tiếp.

Không leak hoặc double-free native buffer.

---

# 20. Streaming response

HTTP phải hỗ trợ response streaming để xử lý file/body lớn.

Không bắt toàn bộ body phải được load vào RAM.

Conceptual:

```vut
response = await http.get(url)?

stream = response.stream()

for:
  chunk = await stream.read()?

  if chunk == null:
    break

  file.write(chunk)?
```

Streaming phải:

```text
bounded-memory
incremental
async
ownership-safe
```

Nếu `io` đã có abstraction phù hợp, phải reuse hoặc integrate với `io`.

Không tạo một hệ stream hoàn toàn độc lập chỉ dành cho HTTP nếu abstraction chung có thể sử dụng được.

---

# 21. Range Request

HTTP phải hỗ trợ byte range requests.

Conceptual:

```vut
request = http.Request(
  method: http.Method.get,
  url: url
)

request.range(start, end)

response = await request.send()?
```

Range semantics:

```text
start inclusive
end inclusive
```

Backend gửi header tương đương:

```text
Range: bytes=start-end
```

Phải cho phép kiểm tra response status để biết server có thực sự hỗ trợ range hay không.

Không giả định mọi server đều hỗ trợ Range Request.

---

# 22. Vutcon integration

HTTP phải tương thích với Vutcon.

Canonical Vutcon syntax:

```vut
job = vut(() => operation())
```

Multiline:

```vut
job = vut(fn():
  value = operation()
  value
)
```

Ví dụ concurrent HTTP requests:

```vut
async fn main() -> result[unit, http.Error]:
  a = vut(() => http.get("https://example.com/a"))
  b = vut(() => http.get("https://example.com/b"))

  response_a = await a?
  response_b = await b?

  ok(unit)
```

HTTP không được tạo concurrency syntax riêng.

Không expose Tokio task APIs.

Vutcon vẫn là concurrency abstraction của Vut.

---

# 23. Parallel-style segmented download

Range Request + Vutcon phải cho phép xây downloader nhiều segment.

Conceptual:

```vut
a = vut(() => download_part(url, 0, 24_999_999))
b = vut(() => download_part(url, 25_000_000, 49_999_999))
c = vut(() => download_part(url, 50_000_000, 74_999_999))
d = vut(() => download_part(url, 75_000_000, 99_999_999))

part_a = await a?
part_b = await b?
part_c = await c?
part_d = await d?
```

Sau đó có thể merge bằng `fs`/`io`.

Không cần API download manager đặc biệt trong `http` MVP.

HTTP chỉ cung cấp primitives cần thiết.

---

# 24. Timeout

Reuse:

```vut
time.Duration
```

Không tạo `HttpDuration`.

Ví dụ:

```vut
import http
import time

request = http.Request(
  method: http.Method.get,
  url: url
)

request.timeout(time.seconds(30))

response = await request.send()?
```

Client cũng có thể có default timeout.

Timeout phải được propagate thành typed `HttpError`.

---

# 25. Redirects

MVP hỗ trợ redirect handling.

Client phải có configurable redirect limit.

Ví dụ conceptual:

```vut
client = http.Client(
  redirects: 5
)
```

Phải tránh redirect loop vô hạn.

Redirect failure phải trả typed error.

Không silently follow vô hạn.

---

# 26. Error model

Canonical public error:

```vut
http.Error
```

Nếu naming conventions hiện tại ưu tiên `HttpError`, có thể sử dụng `HttpError`, nhưng chỉ được có một canonical public type.

Error kind tối thiểu:

```text
invalid_url
dns
connect
tls
timeout
redirect
request
response
body
cancelled
unsupported
other
```

Conceptual:

```vut
data Error:
  kind: ErrorKind
  message: str
```

Exact representation phải tuân thủ generic/error/data semantics hiện tại của Vut.

Không hạ xuống integer error codes trong public API chỉ để né compiler bug.

Nếu compiler correctness bug block typed error, sửa compiler foundation thay vì giảm API.

---

# 27. Cancellation

MVP error model phải reserve:

```text
cancelled
```

Nếu Vutcon/task cancellation chưa tồn tại, không cần phát minh cancellation API riêng trong HTTP.

Thiết kế native backend phải tránh khóa kiến trúc khiến cancellation sau này không thể thêm.

---

# 28. Connection reuse

`Client` phải reuse native HTTP client state.

Không tạo một native HTTP stack mới cho mỗi request nếu không cần.

Expected:

```text
connection pooling
keep-alive
TLS session reuse khi backend hỗ trợ
```

Ưu tiên để `reqwest` quản lý.

---

# 29. Default client

Convenience:

```vut
http.get(...)
http.post(...)
```

có thể sử dụng internal default client.

Default client phải được quản lý an toàn.

Không expose global mutable client state cho user.

Không dùng unsafe global state hack.

Nếu runtime architecture không cho phép default client sạch ở MVP, thiết kế explicit Client trước và convenience API sau.

---

# 30. Native handles

Các object native như:

```text
Client
Request
Response
ResponseStream
```

có thể được biểu diễn bằng opaque native handles phía ABI nếu cần.

Ownership phải explicit:

```text
create
use
transfer nếu có
release/drop
```

Mỗi handle phải có owner rõ ràng.

Không:

```text
double free
double close
use-after-free
dangling pointer
leak
```

Vut user không gọi manual free.

Cleanup phải deterministic theo Vut memory model.

---

# 31. ABI boundary

Stable C ABI là boundary duy nhất giữa Vut và Rust HTTP backend.

Không expose:

```text
Rust Future
reqwest::Client
reqwest::Request
reqwest::Response
tokio::Runtime
tokio::Task
bytes::Bytes
url::Url
Rust enum layout
Rust String
Rust Vec
```

qua ABI.

Chỉ dùng ABI-safe primitives/handles/buffers đã được Vut runtime định nghĩa.

Native symbols:

```text
vut_rt_http_<operation>
```

Ví dụ:

```text
vut_rt_http_client_create
vut_rt_http_client_release
vut_rt_http_request_create
vut_rt_http_request_send
vut_rt_http_response_status
vut_rt_http_response_read
vut_rt_http_response_release
```

Exact primitive split được quyết định trong implementation dựa trên architecture hiện tại.

Không tạo quá nhiều tiny FFI calls nếu có thể batch safely.

---

# 32. Async bridge

Đây là phần kiến trúc quan trọng.

Vut async runtime và Tokio là hai runtime khác nhau trong MVP.

Không được giả định Rust Future có thể trực tiếp trở thành Vut awaitable.

Phải có explicit bridge:

```text
Vut async operation
      ↓
native request start
      ↓
Tokio/reqwest operation
      ↓
completion state
      ↓
Vut scheduler wake/resume
      ↓
Vut await continues
```

Không busy-wait.

Không block Vut scheduler thread chỉ để chờ Tokio Future hoàn thành.

Không dùng polling loop nóng.

Plan/implementation phải inspect async runtime hiện tại và thiết kế bridge phù hợp.

---

# 33. Future runtime replacement

Public API không được phụ thuộc vào Tokio/reqwest.

Mục tiêu dài hạn cho phép:

```text
MVP:

Vut
 ↓
C ABI
 ↓
Tokio + reqwest
```

sau này đổi thành:

```text
Vut
 ↓
Vut scheduler/reactor
 ↓
lower-level HTTP implementation
```

mà code:

```vut
response = await http.get(url)?
```

vẫn giữ nguyên.

Tương tự:

```vut
job = vut(() => http.get(url))
```

không thay đổi.

---

# 34. Cross-platform

MVP phải hướng tới:

```text
Windows
Linux
macOS
```

Không viết public API chỉ hoạt động trên một OS nếu có thể tránh.

Platform-specific native code phải được isolate.

HTTP semantics public phải nhất quán giữa các platform.

---

# 35. Security

HTTPS certificate verification bật mặc định.

Không:

```text
disable TLS verification by default
accept invalid certificates silently
downgrade HTTPS silently
log Authorization header
log cookies/tokens/body secrets mặc định
```

Redirect behavior phải tránh làm leak sensitive headers sang destination không phù hợp nếu backend có security policy hỗ trợ.

Ưu tiên secure defaults của `reqwest`/`rustls`.

---

# 36. Performance

Implementation phải tránh:

```text
copy body không cần thiết
allocate temporary string liên tục
load toàn bộ stream vào RAM
recreate client liên tục
unnecessary UTF-8 conversion
unnecessary ABI round trips
```

Ưu tiên:

```text
connection pooling
streaming
bounded buffers
bytes-native transfer
preallocation khi biết Content-Length
reuse client
```

Không sacrifice correctness/ownership safety chỉ để tối ưu sớm.

---

# 37. Interop với stdlib khác

`http` phải reuse:

```text
time
    Duration cho timeout

io
    stream abstraction nếu phù hợp

fs
    user có thể stream HTTP response vào File

bytes
    binary body representation

result
    error propagation
```

Không duplicate:

```text
HttpDuration
HttpBytes
HttpFile
HttpReader
```

nếu stdlib/core đã có abstraction phù hợp.

---

# 38. JSON

HTTP không tự động parse JSON.

Không thêm:

```vut
response.json()
request.json(...)
```

vào MVP nếu điều đó tạo dependency trực tiếp giữa HTTP và JSON.

User có thể:

```text
HTTP response bytes/string
        ↓
JSON module/package
```

HTTP phải usable độc lập với JSON.

---

# 39. HTTP server

`http` MVP là client-only.

Không implement:

```text
listen
serve
route
middleware
request handler server
HTTP server
```

Server-side HTTP nên được thiết kế riêng sau.

Không làm client architecture phức tạp chỉ để chuẩn bị server.

---

# 40. Tests

Phải có basic verification trong quá trình implementation.

Sau khi module hoàn thiện, comprehensive tests phải bao gồm:

```text
GET
POST
PUT
PATCH
DELETE
HEAD
OPTIONS

HTTP
HTTPS

headers
multi-value headers
query encoding
string body
bytes body

status classification
404/500 are valid responses

timeout
redirect
redirect limit
invalid URL

response bytes
UTF-8 response text
invalid UTF-8

streaming
large streaming body
Range Request

connection reuse
multiple sequential requests
multiple concurrent requests

Vutcon + HTTP
multiple Vutcons
await result
error propagation

resource cleanup
client drop
request drop
response drop
stream drop

native buffer ownership
no leaks
no double-free
no use-after-free
```

Native Rust tests phải kiểm tra backend độc lập khi phù hợp.

Vut integration tests phải kiểm tra end-to-end qua public API.

Không phụ thuộc hoàn toàn vào public internet trong deterministic test suite.

Ưu tiên local test HTTP server cho integration tests.

---

# 41. Diagnostics

Compiler diagnostics liên quan đến:

```text
await
vutcon
result
type mismatch
```

vẫn thuộc compiler.

HTTP runtime errors thuộc `http.Error`.

Không biến runtime network failure thành compiler diagnostic.

Error messages phải đủ context nhưng không leak secrets.

---

# 42. Documentation examples

Docs tối thiểu phải có:

```text
simple GET
POST string/bytes
custom headers
query parameters
Client reuse
timeout
status handling
streaming download
Range Request
Vutcon concurrent requests
error handling
```

Ví dụ phải dùng canonical Vut syntax hiện tại:

```text
@[...]          list literal
[...]      array literal
Type(...)       generic/type application
vut(...)        Vutcon spawn
await           await
result[T, E]    Result
```

Không dùng syntax cũ hoặc syntax từ ngôn ngữ khác.

---

# 43. Implementation boundary

High-level:

```text
vut-stdlib/std/http/
```

Native:

```text
vut-stdlib/native/vut-runtime/src/http/
```

Compiler/core chỉ thay đổi khi HTTP phát hiện một missing language/runtime primitive thực sự cần thiết.

Không implement HTTP high-level API trong:

```text
compiler
parser
HIR
MIR
codegen
```

Không tạo HTTP-specific compiler intrinsic nếu không có lý do kiến trúc bắt buộc.

---

# 44. Definition of Done

HTTP MVP chỉ được coi là hoàn thành khi:

```text
import http
```

hoạt động như top-level stdlib import và các feature sau chạy end-to-end:

```text
HTTP + HTTPS
GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS
Client
Request
Response
Headers
Method
Status
typed HttpError
query parameters
string/bytes body
timeout
redirect
connection reuse
response bytes/text
streaming
Range Request
async/await
Vutcon compatibility
deterministic resource cleanup
Windows/Linux/macOS architecture
```

Implementation phải giữ boundary:

```text
Vut public API
    ↓
stable C ABI
    ↓
Rust native backend
    ↓
reqwest + Tokio + rustls
```

Tokio/reqwest là implementation detail, không phải một phần của Vut language semantics.

---

# 45. Nguyên tắc cuối cùng

```text
Simple Vut API
+
Async-first
+
Vutcon compatible
+
Typed errors
+
Streaming
+
Secure HTTPS defaults
+
Mature Rust networking crates
+
Stable C ABI
+
Replaceable native backend
```

Không tự xây lại HTTP/TLS/network stack khi Rust ecosystem đã có implementation trưởng thành.

Không hy sinh kiến trúc Vut chỉ để tích hợp nhanh với Tokio.

Public API phải thuộc về Vut; Rust crates chỉ là backend implementation.
