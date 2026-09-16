# Vut Docs — Design System

## 1. Mục tiêu

Vut Docs là website documentation chính thức cho ngôn ngữ lập trình Vut.

Tech stack:

- SvelteKit
- SveltePress
- Svelte 5
- TypeScript
- Markdown
- Shiki
- Pagefind
- Lucide Icons nếu cần icon UI

Website phải có cảm giác:

- hiện đại
- kỹ thuật
- cao cấp
- tối giản
- nhanh
- dành cho developer

Không thiết kế giống blog.

Phong cách tham chiếu chính là mockup Vut đã được cung cấp.

Agent phải bám sát ngôn ngữ thiết kế của mockup đó.

---

# 2. Theme

Website ưu tiên Dark Mode.

Background chính:

```css
--background: #07090d;
--background-secondary: #0a0d13;
--surface: #0d1118;
--surface-hover: #111722;
```

Không dùng màu đen tuyệt đối `#000000` cho toàn bộ website.

Background nên có sự khác biệt rất nhẹ giữa các layer.

---

# 3. Brand Colors

Màu nhận diện chính:

```css
--primary: #7868ff;
--primary-light: #9b8cff;
--primary-dark: #5745ee;

--blue: #4f7cff;
--violet: #8b5cf6;
--purple: #a855f7;
```

Gradient thương hiệu:

```css
background:
  linear-gradient(
    135deg,
    #4f7cff,
    #7868ff,
    #a855f7
  );
```

Gradient chỉ dùng cho:

- logo
- CTA chính
- keyword quan trọng
- glow
- active indicator
- decorative element

Không biến toàn bộ UI thành gradient.

---

# 4. Text Colors

```css
--text-primary: #f5f7fb;
--text-secondary: #a5adbb;
--text-muted: #697386;
```

Heading gần trắng.

Body text xám sáng.

Metadata và secondary information dùng muted text.

---

# 5. Border

Border cực kỳ nhẹ.

```css
--border: rgba(255, 255, 255, 0.08);
--border-hover: rgba(255, 255, 255, 0.14);
```

Không sử dụng border trắng rõ nét.

---

# 6. Glow

Một đặc điểm quan trọng của Vut UI là ánh sáng tím/xanh rất nhẹ.

Ví dụ:

```css
box-shadow:
  0 0 60px rgba(99, 102, 241, 0.08);
```

Hero có thể sử dụng radial gradient:

```css
background:
  radial-gradient(
    circle,
    rgba(99, 102, 241, 0.15),
    transparent 65%
  );
```

Glow phải tinh tế.

Không tạo giao diện cyberpunk hoặc neon quá mức.

---

# 7. Typography

Ưu tiên:

```text
Inter
Geist
system-ui
sans-serif
```

Code:

```text
JetBrains Mono
Geist Mono
monospace
```

Heading:

- font weight: 600–700
- letter spacing nhẹ âm
- line-height ngắn

Body:

- line-height thoáng
- dễ đọc tài liệu dài

---

# 8. Border Radius

Các component sử dụng radius vừa phải.

```css
--radius-sm: 6px;
--radius-md: 10px;
--radius-lg: 14px;
--radius-xl: 18px;
```

Button có thể pill nhẹ nhưng không quá tròn.

---

# 9. Cards

Card style:

```css
background: rgba(13, 17, 24, 0.75);
border: 1px solid rgba(255,255,255,0.08);
border-radius: 14px;
```

Hover:

- border sáng hơn
- background sáng hơn rất nhẹ
- translateY tối đa 1–2px

Không dùng animation lớn.

---

# 10. Buttons

## Primary

Gradient tím → xanh.

Text trắng.

Có glow nhẹ.

Ví dụ:

```text
Get Started →
```

## Secondary

Background gần transparent.

Border nhẹ.

Ví dụ:

```text
Read the Docs
```

Hover phải nhanh và tinh tế.

---

# 11. Icons

Ưu tiên Lucide.

Icon:

- stroke mảnh
- 16–20px
- không dùng emoji thay icon

Feature icon có thể nằm trong vùng gradient/glow nhẹ.

---

# 12. Logo

Logo Vut phải được sử dụng nhất quán.

Header:

```text
[logo] vut
```

Logo không được tự vẽ lại bằng CSS.

Dùng asset chính thức trong project.

---

# 13. Motion

Animation duration:

```text
120–250ms
```

Cho phép:

- fade
- border transition
- opacity
- subtle translate
- glow transition

Không sử dụng:

- bounce
- animation quá mạnh
- parallax nặng
- hiệu ứng làm giảm khả năng đọc docs

---

# 14. Responsive

Các breakpoint chính:

```text
mobile
tablet
desktop
wide desktop
```

Docs phải đọc tốt trên mobile.

Desktop tối ưu cho màn hình developer 1440px trở lên.

---

# 15. Nguyên tắc

UI phải:

- dark
- sạch
- có nhiều khoảng thở
- hierarchy rõ
- code là thành phần quan trọng
- navigation cực nhanh

Không:

- glassmorphism quá mạnh
- border dày
- shadow lớn
- màu sắc lộn xộn
- component quá bo tròn
- animation gây mất tập trung