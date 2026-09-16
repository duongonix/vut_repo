# Vut Docs — Layout & Navigation

# 1. Global Layout

Website gồm hai loại layout chính:

```text
Marketing Layout
└── Homepage

Documentation Layout
├── Docs
├── Guide
├── Reference
└── Ecosystem
```

Header được dùng chung.

---

# 2. Header

Desktop header:

```text
┌─────────────────────────────────────────────────────────────┐
│ Vut   Docs Guide Reference Playground Community  Search ...│
└─────────────────────────────────────────────────────────────┘
```

Bên trái:

```text
Vut logo
vut
```

Navigation:

```text
Docs
Guide
Reference
Playground
Community
```

Bên phải:

```text
Search
GitHub
Theme
```

Header:

- sticky
- top: 0
- backdrop blur nhẹ
- border-bottom rất nhẹ khi scroll

Không làm header quá cao.

---

# 3. Search

Search box desktop:

```text
⌕ Search docs...                 ⌘ K
```

Click hoặc:

```text
Ctrl + K
Cmd + K
```

mở Search Dialog.

Search sử dụng Pagefind/SveltePress search.

Search dialog cần:

- keyboard navigation
- highlight result
- title
- section
- breadcrumb
- close bằng Escape

---

# 4. Documentation Layout

Desktop:

```text
┌──────────────┬──────────────────────────┬─────────────┐
│              │                          │             │
│   Sidebar    │       Documentation      │     TOC     │
│              │                          │             │
│              │                          │             │
└──────────────┴──────────────────────────┴─────────────┘
```

Tỷ lệ tham khảo:

```text
sidebar: 250–280px
content: 700–850px
toc: 200–240px
```

Toàn layout nằm trong max-width hợp lý.

---

# 5. Sidebar

Sidebar sticky.

Ví dụ:

```text
GETTING STARTED

Introduction
Installation
Hello Vut
Project Structure

LANGUAGE

Variables
Types
Functions
Control Flow
Collections
Modules
Error Handling

ADVANCED

Vutcon
Vutcom
FFI

TOOLING

Vut CLI
VPM
Compiler

STANDARD LIBRARY

fs
path
os
time
...
```

Section title nhỏ, uppercase, muted.

Active item:

- text sáng
- background tím rất nhẹ
- indicator tím

Group có thể collapse.

---

# 6. Table of Contents

Bên phải:

```text
On this page

Overview
Type inference
Type annotations
Constants
Dynamic values
Examples
```

Theo dõi heading hiện tại khi scroll.

Active heading dùng primary color.

---

# 7. Breadcrumb

Phía trên article:

```text
Docs / Language / Variables
```

Breadcrumb nhỏ và muted.

---

# 8. Previous / Next

Cuối mỗi trang:

```text
← Previous                         Next →
Types                              Functions
```

Hai card nhẹ.

---

# 9. Mobile

Mobile header:

```text
☰   Vut                       ⌕
```

Sidebar chuyển thành drawer.

TOC chuyển thành:

```text
On this page ▾
```

Content chiếm toàn width.

---

# 10. Version Selector

Docs phải chuẩn bị cho versioning.

Header/sidebar có:

```text
Vut v0.1
```

Click:

```text
v0.1
v0.2
Latest
```

Không hard-code kiến trúc chỉ hỗ trợ một version.

---

# 11. Navigation Principle

Người dùng phải có thể đi từ:

```text
Homepage
→ Get Started
→ Installation
→ Hello World
```

trong tối đa vài thao tác.

Documentation phải ưu tiên tốc độ truy cập hơn hiệu ứng.