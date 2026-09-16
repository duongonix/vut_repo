# Vut Docs — Homepage

Homepage phải bám sát mockup Vut đã được cung cấp.

Đây là landing page chính thức của programming language, không phải trang docs thông thường.

---

# 1. Hero

Desktop layout:

```text
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  Version badge                  Vut visual / Logo         │
│                                                          │
│  Build a brighter              ┌─────────────────────┐   │
│  tomorrow with Vut.            │ main.vut            │   │
│                                │                     │   │
│  description                   │ fn main():          │   │
│                                │   ...               │   │
│  [Get Started] [Read Docs]     │                     │   │
│                                └─────────────────────┘   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

Background:

- gần đen
- radial glow xanh/tím
- abstract visual rất nhẹ
- không làm ảnh hưởng readability

---

# 2. Version Badge

Ví dụ:

```text
v0.1    A new era for simple and powerful programming →
```

Badge nhỏ.

---

# 3. Hero Heading

Nội dung:

```text
Build a brighter
tomorrow with Vut.
```

Từ:

```text
tomorrow
```

dùng gradient xanh → tím.

Heading desktop rất lớn.

Không dùng gradient toàn heading.

---

# 4. Hero Description

Ví dụ:

```text
Vut is a modern, fast, and expressive programming language
designed for real-world development. Simple to learn,
powerful to build, and made for everyone.
```

Width giới hạn để dễ đọc.

---

# 5. Hero Actions

Primary:

```text
Get Started →
```

Secondary:

```text
Read the Docs
```

Bên dưới:

```text
Open source • Modern tooling • Built for the future
```

---

# 6. Code Preview

Một code window lớn.

Header:

```text
● ● ●        main.vut                      Run ▶
```

Ví dụ code phải dùng cú pháp Vut thực tế.

```vut
fn main():
    name: str = "World"
    out("Hello, $name!")
```

Code dùng syntax highlighting.

Không dùng cú pháp giả nếu Vut spec đã định nghĩa cú pháp tương ứng.

---

# 7. Feature Cards

Ngay dưới hero:

```text
┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐
│ Fast       │ │ Simple     │ │ Tooling    │ │ Open       │
│ by design  │ │ & clean    │ │            │ │ source     │
└────────────┘ └────────────┘ └────────────┘ └────────────┘
```

Các feature:

### Fast by design

Vut được thiết kế hướng tới native performance.

### Simple & clean

Syntax ngắn gọn, dễ đọc.

### Modern tooling

Compiler, package manager và developer tooling được thiết kế cùng hệ sinh thái.

### Open and community-driven

Open-source ecosystem.

---

# 8. Why Vut

Layout 2 cột.

Trái:

```text
WHY VUT

A language for
real possibilities.

Vut combines simplicity, performance,
and modern developer experience.
```

Checklist:

```text
✓ Modern and expressive syntax
✓ Strict static typing
✓ Native-oriented performance
✓ Cross-platform
✓ Modern package tooling
```

CTA:

```text
Learn more →
```

---

# 9. Interactive Code Showcase

Bên phải Why Vut.

Tabs:

```text
Simple | Typed | Flexible | Powerful
```

Mỗi tab hiển thị code Vut tương ứng.

Ví dụ Typed:

```vut
name: str = "Vut"
age: int = 1
```

Simple:

```vut
fn greet(name: str):
    out("Hello, $name!")
```

Không fake syntax.

---

# 10. Ecosystem Strip

Một horizontal card:

```text
Fast             Open source        Documentation      Cross-platform

Native-focused   Community          Always updated     Windows/macOS/Linux
```

Không sử dụng số liệu giả như:

```text
10K developers
1M downloads
```

trừ khi project thực sự có dữ liệu đó.

---

# 11. Ecosystem Section

Phần cuối landing page.

Visual bên trái có thể là:

- abstract globe
- network
- Vut ecosystem visualization

Không cần animation nặng.

Bên phải:

```text
A BRIGHTER TOMORROW

More than a language.
A growing ecosystem.
```

Giới thiệu:

```text
Vut Compiler
VPM
Standard Library
Vutcon
Vutcom
Developer Tooling
```

---

# 12. Final CTA

Cuối trang:

```text
Ready to build with Vut?

[Get Started] [Explore Documentation]
```

---

# 13. Footer

Footer tối giản:

```text
Vut

Documentation
Guide
Reference
Standard Library

Ecosystem
VPM
GitHub
Community

Resources
Playground
Changelog
Releases
```

Bottom:

```text
© Vut
Open source
```

Không nhồi quá nhiều thông tin.

---

# 14. Homepage Responsive

Mobile:

Hero chuyển thành:

```text
Heading
Description
Buttons
Code Preview
```

Không cố giữ layout 2 cột.

Feature cards:

```text
1 column mobile
2 columns tablet
4 columns desktop
```

Code preview phải scroll ngang nếu cần.