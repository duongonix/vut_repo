# Vut Docs — Features & Interactions

# 1. Mục tiêu

Vut Docs không chỉ là nơi render Markdown.

Nó phải là developer documentation platform hoàn chỉnh cho hệ sinh thái Vut.

---

# 2. Search

Sử dụng khả năng search phù hợp của SveltePress/Pagefind.

Yêu cầu:

```text
Ctrl/Cmd + K
```

mở search.

Search:

- title
- heading
- content
- keywords

Result:

```text
Variables
Language › Variables

Type inference
Language › Variables › Type inference
```

---

# 3. Code Copy

Mọi code block có nút:

```text
Copy
```

Sau click:

```text
Copied
```

Sau khoảng ngắn quay lại trạng thái ban đầu.

---

# 4. Theme

Dark là giao diện chính.

Kiến trúc không được ngăn cản việc hỗ trợ light theme sau này.

Theme state không được gây flash sai theme khi load.

---

# 5. Playground

Navigation chuẩn bị route:

```text
/playground
```

Playground có layout:

```text
┌───────────────────────────────────────┐
│ main.vut                     Run ▶   │
├───────────────────┬───────────────────┤
│                   │                   │
│      Editor       │      Output       │
│                   │                   │
└───────────────────┴───────────────────┘
```

Nếu compiler web/runtime chưa tồn tại:

- chỉ dựng architecture/UI cần thiết
- không fake execution result

---

# 6. Code Examples

Docs hỗ trợ example tabs:

```text
Example | Output
```

Output chỉ hiển thị nếu example có output được xác định rõ.

---

# 7. Language Version

Version selector phải hỗ trợ architecture:

```text
Latest
v0.x
v1.x
```

Không cần tạo version giả.

Chỉ hiển thị version thực tế được config.

---

# 8. Copy Install Command

Installation page có command:

```bash
...
```

và copy button.

Không tự phát minh install command.

Command phải lấy từ Vut documentation/spec chính thức.

---

# 9. OS Tabs

Installation có thể hỗ trợ:

```text
Windows | macOS | Linux
```

Mỗi OS có instruction riêng.

---

# 10. Code Tabs

Reusable component:

```text
Vut | Output
```

hoặc:

```text
Windows | macOS | Linux
```

Tabs:

- keyboard accessible
- responsive
- không reset page scroll

---

# 11. Anchor Navigation

Khi user click TOC:

- smooth scroll nhẹ
- URL hash cập nhật
- heading không bị sticky header che

---

# 12. Active TOC

IntersectionObserver hoặc cơ chế phù hợp để highlight section hiện tại.

Không chạy scroll listener nặng liên tục.

---

# 13. Sidebar State

Sidebar group có thể:

```text
expanded
collapsed
```

Active page phải luôn nhìn thấy.

---

# 14. Mobile Drawer

Mobile sidebar:

- mở từ menu button
- đóng bằng Escape
- đóng khi navigate
- khóa background scroll
- focus management đúng

---

# 15. Keyboard Navigation

Hỗ trợ:

```text
Cmd/Ctrl + K → Search
Escape       → Close dialog
↑ ↓          → Search results
Enter        → Open result
```

---

# 16. Accessibility

Bắt buộc:

- semantic HTML
- keyboard navigation
- visible focus state
- ARIA khi cần
- contrast đủ cao
- buttons có accessible name

Không hy sinh accessibility để giống mockup.

---

# 17. Performance

Homepage visual không được làm docs nặng.

Ưu tiên:

- CSS gradients
- SVG
- optimized PNG/WebP/AVIF
- lazy loading
- minimal JS

Không thêm Three.js/WebGL chỉ để tạo background.

---

# 18. SEO

Mỗi Markdown page phải tạo:

```text
title
description
canonical
Open Graph
```

Homepage có metadata riêng cho Vut.

---

# 19. Social Preview

Chuẩn bị Open Graph image cho:

```text
Vut Programming Language
```

Có thể hỗ trợ OG image riêng cho docs page sau này.

---

# 20. 404

Custom 404 theo Vut design.

Ví dụ:

```text
404

This path doesn't exist.

[Back to Docs]
```

Không cần animation phức tạp.

---

# 21. Changelog

Chuẩn bị:

```text
/changelog
```

Dùng cho:

- language changes
- compiler changes
- tooling changes
- breaking changes

---

# 22. GitHub Integration

Header có GitHub icon.

Có thể hỗ trợ:

```text
Edit this page
Report documentation issue
```

URL phải lấy từ config.

---

# 23. Documentation Feedback

Cuối article có thể có:

```text
Was this page helpful?

Yes   No
```

Chỉ triển khai backend tracking khi thực sự có hệ thống lưu dữ liệu.

Không fake statistics.

---

# 24. Component Architecture

Agent phải tạo reusable components.

Ví dụ:

```text
components/
├── layout/
├── navigation/
├── docs/
├── code/
├── search/
├── home/
└── ui/
```

Không nhét toàn bộ homepage vào một component khổng lồ.

Không nhét toàn bộ docs layout vào một file.

---

# 25. SveltePress

Phải tận dụng feature có sẵn của SveltePress trước khi tự implement lại.

Nguyên tắc:

> Nếu SveltePress đã giải quyết tốt một chức năng docs thì ưu tiên sử dụng hoặc customize nó.

Không tự viết lại:

- Markdown pipeline
- routing
- search infrastructure
- heading extraction
- syntax highlighting pipeline

nếu framework đã cung cấp giải pháp phù hợp.

---

# 26. Agent Rules

Agent được phép:

- tạo folder
- tạo component
- refactor
- thêm utility
- thêm dependency thực sự cần thiết

Agent phải:

- ưu tiên dependency hiện có
- kiểm tra package.json trước khi cài package
- giữ component nhỏ và có trách nhiệm rõ ràng
- dùng Svelte 5 conventions
- giữ TypeScript strict
- tránh duplicate logic

Agent không được:

- tự phát minh syntax Vut
- tự phát minh compiler behavior
- tự phát minh benchmark
- tự phát minh số lượng user/download
- thêm dependency chỉ để giải quyết một UI trivial
- viết toàn bộ website trong một component
- thay đổi Vut language specification để phù hợp UI

Khi cần nội dung Vut nhưng spec chưa có:

```text
TODO/content placeholder
```

hoặc sử dụng nội dung trung tính.

Không bịa semantics.

---

# 27. Definition of Done

Vut Docs được coi là hoàn thiện UI foundation khi:

- homepage sát mockup đã chốt
- dark theme hoàn chỉnh
- responsive
- docs sidebar hoạt động
- TOC hoạt động
- Markdown rendering hoạt động
- Vut code blocks hoạt động
- copy code hoạt động
- search hoạt động
- mobile navigation hoạt động
- accessibility cơ bản đạt
- docs architecture có thể mở rộng
- homepage Lighthouse performance tốt
- không có fake Vut semantics
- không có fake project statistics

Mục tiêu cuối cùng:

```text
Vut Docs phải tạo cảm giác đây là documentation
của một programming language hiện đại và nghiêm túc,
không phải một template documentation được đổi logo.
```