# Vut Docs — Documentation System

# 1. Content

Documentation được viết chủ yếu bằng Markdown.

Không hard-code nội dung docs trực tiếp vào Svelte component.

Content phải dễ:

- viết
- sửa
- version control
- review
- search
- generate bằng agent

---

# 2. Documentation Structure

Thiết kế content hierarchy theo hướng:

```text
docs/

getting-started/
    introduction
    installation
    hello-world
    project-structure

language/
    lexical-structure
    variables
    constants
    types
    operators
    functions
    control-flow
    loops
    collections
    modules
    optional
    result
    interfaces

advanced/
    concurrency
    vutcon
    vutcom
    ffi
    native-libraries

tooling/
    compiler
    vut-cli
    vpm
    formatter
    linter
    lsp

stdlib/
    overview
    fs
    path
    os
    time

packages/
    overview
    vpm-toml
    dependencies
    publishing

reference/
    grammar
    keywords
    operators
    builtin-types
    builtin-functions
```

Đây là information architecture.

Agent không được tự phát minh semantics của Vut chỉ để điền nội dung.

---

# 3. Markdown Frontmatter

Mỗi page hỗ trợ metadata:

```yaml
---
title: Variables
description: Learn how variables and type inference work in Vut.
---
```

Có thể mở rộng:

```yaml
---
title: Variables
description: Variables in Vut.
section: Language
order: 3
---
```

---

# 4. Code Blocks

Code Vut:

````markdown
```vut
name: str = "Vut"
```
````

Phải có:

- syntax highlighting
- copy button
- language label
- horizontal scrolling
- optional filename

Ví dụ:

```text
main.vut                         Copy
────────────────────────────────────

fn main():
    out("Hello")
```

---

# 5. Vut Syntax Highlighting

Phải chuẩn bị custom syntax highlighting cho:

```text
.vut
```

Không map Vut sang Rust/Go/Python chỉ để có màu.

Nếu grammar chưa tồn tại:

- tạo integration point riêng
- cho phép fallback plain text
- sau này thay bằng Vut TextMate grammar

---

# 6. Callouts

Docs hỗ trợ:

```text
Note
Tip
Warning
Important
```

Visual phải đồng bộ dark theme.

Ví dụ:

```text
┌─ NOTE ───────────────────────────────┐
│ Vut infers the type from the first  │
│ assignment.                         │
└─────────────────────────────────────┘
```

Không sử dụng emoji lớn.

---

# 7. Tables

Tables phải:

- responsive
- border nhẹ
- readable trong dark mode

Đặc biệt cần tốt cho:

- operators
- types
- CLI commands
- stdlib APIs

---

# 8. API Reference

Reference page cần hỗ trợ component dạng:

```text
fn read(path: str) -> result(bytes, Error)
```

Sau đó:

```text
Parameters
Returns
Errors
Example
```

Chuẩn bị reusable component cho API documentation.

---

# 9. CLI Documentation

CLI command có layout rõ ràng:

```text
vut build [options]
```

Sections:

```text
Usage
Arguments
Options
Examples
```

Tương tự cho VPM:

```text
vpm add
vpm remove
vpm install
vpm update
vpm build
...
```

---

# 10. Heading Anchors

Heading:

```text
## Type inference
```

phải có anchor.

Hover heading hiện link icon.

URL:

```text
/docs/language/variables#type-inference
```

---

# 11. Copy Link

Người dùng có thể copy URL trực tiếp tới section.

---

# 12. Edit Page

Cuối docs có thể có:

```text
Edit this page on GitHub
```

Không hard-code URL sai.

Config qua project configuration.

---

# 13. Last Updated

Có thể hiển thị:

```text
Last updated ...
```

nếu metadata đáng tin cậy lấy được từ build/Git.

Không fake date.

---

# 14. Reading Experience

Documentation content phải ưu tiên:

- readability
- code
- examples
- hierarchy

Article không được quá rộng.

Khoảng 70–85 characters/line cho prose là hợp lý.

Code có thể rộng hơn prose.

---

# 15. Markdown + Svelte

Chỉ sử dụng Svelte component trong Markdown khi thực sự cần:

- interactive example
- API component
- tabs
- playground
- special visualization

Documentation bình thường phải giữ Markdown đơn giản.