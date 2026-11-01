# 04 · Design system

> Cập nhật: 2026-10-31 · Giá trị chuyển động và kích thước là điểm khởi đầu; chốt sau Spike A (P0.3).

## 1. Tinh thần

**Đen sâu, cam neon, mềm mại.** Island phải trông như một phần của màn hình chứ không phải một cửa sổ. Neon là tín hiệu, không phải trang trí: màu cam phát sáng chỉ xuất hiện khi có thứ đang hoạt động.

## 2. Màu

Định nghĩa trong `src/lib/styles/tokens.css`. Component chỉ dùng biến, không dùng hex trần.

```css
:root {
  /* Nền */
  --c-island:        #000000;   /* thân island: đen tuyệt đối để hòa vào viền màn hình */
  --c-surface-1:     #0B0B0D;   /* nền cửa sổ main, panel */
  --c-surface-2:     #141418;   /* thẻ, ô nhập */
  --c-surface-3:     #1C1C22;   /* hover, phần tử nổi */
  --c-border:        rgba(255, 255, 255, 0.08);
  --c-border-strong: rgba(255, 255, 255, 0.14);

  /* Chữ */
  --c-text-1: #F5F5F7;          /* chính */
  --c-text-2: #A1A1AA;          /* phụ */
  --c-text-3: #6B6B76;          /* ghi chú, placeholder */

  /* Nhấn: neon cam */
  --c-accent:        #FF6A1A;   /* tương phản 7,3:1 trên #000 */
  --c-accent-bright: #FF9F43;   /* hover, điểm sáng */
  --c-accent-deep:   #E04E0B;   /* nhấn giữ, viền đậm */
  --c-accent-soft:   rgba(255, 106, 26, 0.14);  /* nền chọn, chip */
  --c-glow:          rgba(255, 106, 26, 0.45);

  /* Trạng thái (dùng tiết kiệm) */
  --c-success: #34D399;
  --c-warning: #FBBF24;
  --c-danger:  #F43F5E;
}
```

**Luật màu:**
- Ở trạng thái rảnh, chỉ đôi mắt được dùng màu nhấn, ở mức `opacity` 0.9 và không có glow. Đồng hồ và pill trống không có màu nhấn.
- Mỗi lúc chỉ một phần tử glow. Nếu có hai activity, chỉ activity chính được glow.
- Không dùng cam cho chữ thân bài dài. Cam dành cho icon, thanh tiến độ, số đếm, viền trạng thái.
- Màu trạng thái (success/warning/danger) chỉ dùng cho ý nghĩa tương ứng: pin yếu, lỗi…; không dùng để trang trí.
- **Biến thể nhấn (F07):** Cam (mặc định), Hổ phách `#FFB020`, Hồng neon `#FF3D7F`, Xanh điện `#3DA9FF`. Mỗi biến thể phải đạt tương phản ≥ 4,5:1 trên `--c-island`.

## 3. Chữ

```css
:root {
  --font-text:    "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  --font-display: "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif;

  --fs-caption: 11px;  --fs-small: 12px;  --fs-body-island: 13px;  --fs-body: 14px;
  --fs-title-s: 16px;  --fs-title: 20px;  --fs-display: 28px;      --fs-timer-xl: 40px;

  --fw-regular: 400;   --fw-medium: 500;  --fw-semibold: 600;
}
```

- Segoe UI Variable có sẵn trên Windows 11 và hiển thị tiếng Việt tốt, nên không cần nhúng font. Font dự phòng là Segoe UI.
- Mọi con số đổi theo thời gian (đồng hồ, đếm ngược, tiến độ) dùng `font-variant-numeric: tabular-nums` để không nhảy chiều rộng.
- Tiêu đề bài hát quá dài: trên island thì cắt bằng `…`; trong peek thì cuộn chữ (marquee) chậm, dừng 1,5 s ở hai đầu, và chỉ chạy khi đang hover.

## 4. Khoảng cách, bo góc, đổ bóng

```css
:root {
  --sp-1: 2px; --sp-2: 4px; --sp-3: 6px; --sp-4: 8px; --sp-5: 12px;
  --sp-6: 16px; --sp-7: 20px; --sp-8: 24px; --sp-9: 32px; --sp-10: 40px;

  --r-pill: 999px;   --r-peek: 28px;   --r-alert-s: 20px;
  --r-card: 14px;    --r-control: 10px; --r-window: 12px;

  --glow-live:
    0 0 0 1px rgba(255, 106, 26, 0.35),
    0 0 12px rgba(255, 106, 26, 0.35),
    0 0 28px rgba(255, 106, 26, 0.18);
  --shadow-peek: 0 12px 32px rgba(0, 0, 0, 0.55);
}
```

## 5. Hình học island

Đơn vị px logic ở scale 100%; Windows tự nhân theo DPI.

**Kiểu dính mép (mặc định, đã chốt):** island dính chặt vào mép trên màn hình, giống notch của MacBook.
- Cạnh trên phẳng, 2 góc trên vuông. Chỉ 2 góc dưới bo tròn.
- Hai bên có "vai" lõm bán kính 6 px nối êm vào mép màn hình. Vai được vẽ bằng `mask`/SVG, không thuộc hit region.
- Glow chỉ tỏa ra hai bên và phía dưới.
- Kiểu nổi cách mép (`island.style = "floating"`, giống Dynamic Island) vẫn còn trong Cài đặt. Khi đó cả 4 góc bo và `offset_y` mới có tác dụng.

Cột "Bo góc" dưới đây là bán kính 2 góc dưới; ở kiểu nổi thì áp cho cả 4 góc.

| Trạng thái | Kích thước | Bo góc | Ghi chú |
|---|---|---|---|
| Cửa sổ `island` | 480 × 264 khi nghỉ; cao tối đa 616 khi tương tác | — | Nới native window trước morph theo hit region; thu lại sau morph, giữ 16 px lề dưới (ADR-017) |
| Khoảng cách tới mép trên | 0 | — | Kiểu nổi: mặc định 8, chỉnh được 0–24 (`island.offset_y`) |
| Minimal | 96 × 4 | 2 | Vạch mảnh dính mép; vùng hover vô hình 160 × 14 |
| Compact rảnh | 120 × 30 | 15 | Đôi mắt (mặc định), đồng hồ, hoặc trống ([§11](#11-đôi-mắt-màn-hình-chờ)) |
| Compact có activity | 160–280 × 32 | 16 | Trái: icon/ảnh bìa 20 px; phải: thông tin ngắn hoặc sóng nhạc |
| Phần phụ (secondary) | 32 × 32 | 16 | Một "tab" riêng cũng dính mép, cách pill 6 px, nằm bên phải |
| Peek | 400 × 120–200 | 28 | Chiều cao theo nội dung, tối đa 200 |
| Peek có điều hướng tính năng (2026-11-01) | 400 × (nội dung + 28), tối đa 600 cao | 28 | Launcher tối thiểu 220 (tổng 248), đo nội dung để mở rộng; hàng điều hướng 28. Queue mở rộng theo số bài, vùng danh sách tối đa 340 rồi mới cuộn; quay lại player thu về 200 |
| Alert nhỏ | 260 × 40 | 20 | Âm lượng, sạc, đổi bài |
| Alert lớn | 400 × 112 | 28 | Nhắc việc, hết giờ: có nút hành động |

Khi morph, island luôn "mọc" xuống từ mép trên: cạnh trên đứng yên, chiều rộng giãn đều ra hai bên, chiều cao giãn xuống dưới.

**Bố cục compact** chia 3 vùng: `[trái 20px] — [giữa co giãn] — [phải 20px]`.
- Media: ảnh bìa ở trái, sóng nhạc ở phải.
- Timer: vòng tiến độ ở trái, `mm:ss` ở phải.
- Nhắc việc sắp tới: icon chuông ở trái, giờ ở phải.

**Bố cục peek** có 3 hàng:
1. Hàng đầu: ảnh hoặc icon 56–64 px, tiêu đề và mô tả.
2. Hàng giữa: thanh tiến độ hoặc thông tin chi tiết.
3. Hàng cuối: tối đa 5 nút hành động 32 px; nút chính có nền `--c-accent-soft`.

## 6. Chuyển động

Preset trong `src/lib/motion/springs.ts`, dùng lớp `Spring` của Svelte (`stiffness`, `damping` trong khoảng 0–1).

| Preset | stiffness | damping | Dùng cho |
|---|---|---|---|
| `morph` | 0.12 | 0.62 | Đổi kích thước và bo góc của island; hơi nảy nhẹ |
| `snappy` | 0.20 | 0.80 | Nút, hover, chip; không nảy |
| `gentle` | 0.06 | 0.90 | Thanh tiến độ, chuyển giá trị số |

**Biên đạo một lần morph:**
1. Nội dung cũ mờ đi trong 120 ms (`opacity` 1 → 0, `blur` 0 → 4px).
2. Container morph bằng `morph`: width, height và radius chạy đồng thời.
3. Container đi được khoảng 40% quãng đường thì nội dung mới hiện trong 180 ms (`opacity` 0 → 1, `blur` 4px → 0, `translateY` 4px → 0).
4. Hit region được gửi lên lúc bắt đầu (hợp của vùng trước và sau) và khi spring dừng hẳn.

**Glow:**
- Live: `--glow-live` tĩnh, không nhịp.
- Critical: nhịp thở, opacity glow 0.35 ↔ 0.75 trong 2 s ease-in-out, kéo dài tới khi được xác nhận.
- Alert: sáng lên một lần trong 400 ms rồi giữ tĩnh.

**Giảm chuyển động** (khi hệ thống bật `prefers-reduced-motion` hoặc trong Cài đặt): bỏ nảy (dùng `snappy` cho mọi thứ), crossfade 120 ms, tắt marquee, glow Critical giữ tĩnh, mắt không chớp.

## 7. Icon

- `lucide-svelte`: nét 1.75, cỡ 16 (island compact), 18 (peek), 20 (main).
- Icon ứng dụng/trang nhạc lấy từ nguồn (GSMTC app icon, favicon trang). Nếu không có thì dùng icon chung theo loại.
- Không dùng emoji trong UI.

## 8. Âm thanh

- Âm thanh ngắn và mềm, có fade-in; không dùng âm chói.
- Bộ âm mặc định: `chime` (nhắc việc), `bell` (hết giờ), `soft-tick` (Pomodoro chuyển pha), `pop` (thả file vào kệ, tắt mặc định).
- Âm lượng âm báo tách khỏi âm lượng nhạc (`app.sound_volume`).
- Âm báo lặp tối đa 60 giây rồi dừng; activity chuyển sang trạng thái "đã lỡ".

## 9. Nội dung & ngôn ngữ

- Giọng văn ngắn, thân thiện, không trịnh trọng. Ưu tiên động từ: "Hoãn 5 phút", "Mở trình phát".
- Giờ dùng định dạng 24h mặc định (`15:30`), có tùy chọn 12h. Ngày: `T7, 31/10`.
- Thời lượng: `25:00`, `1:02:15`; tương đối: "sau 5 phút", "2 giờ trước".
- Không dùng dấu chấm than, trừ cảnh báo thật sự (pin < 10%).

## 10. Cửa sổ đầy đủ

- Mặc định 1120 × 720, tối thiểu 960 × 640. Nền `--c-surface-1`. Title bar tự vẽ cao 40 px có vùng kéo, các nút thu nhỏ/phóng to/đóng theo kiểu Windows 11.
- Sidebar 220 px gồm: Trang chủ · Nhạc · Online · Hẹn giờ · Nhắc việc · Ghi chú · Kệ file · Cài đặt. Mục đang chọn có thanh cam 3 px bên trái.
- Lưới nội dung: lề 32, khoảng cách 16; thẻ có `--r-card` và nền `--c-surface-2`.
- Có focus ring rõ cho bàn phím: viền 2 px `--c-accent-bright`, offset 2 px.

## 11. Đôi mắt (màn hình chờ)

Linh vật của PCisland: khi không có tác vụ, pill hiện một đôi mắt thỉnh thoảng chớp. Người dùng chọn giữa đôi mắt, đồng hồ, pill trống hoặc ẩn (`island.idle_display`).

**Hình dạng** (px logic, trong pill 120 × 30):
- Mỗi mắt là một viên bo tròn 7 × 13, bo góc 3.5. Hai mắt cách nhau 16 px, đặt chính giữa pill.
- Màu `--c-accent`, `opacity` 0.9, không glow.

**Chớp mắt (P1):**
- Một lần chớp kéo dài 170 ms: nhắm 70 ms (`scaleY` 1 → 0.1, ease-in), giữ 30 ms, mở 70 ms (ease-out). Hai mắt chớp cùng lúc.
- Khoảng cách giữa hai lần chớp ngẫu nhiên 3–7 s. 20% số lần là chớp đôi, cách nhau 120 ms.
- Chỉ dùng một `setTimeout` để hẹn lần chớp kế tiếp, rồi bật class chạy CSS keyframes. Giữa các lần chớp không có frame nào được vẽ.
- Dừng hẳn khi island không ở compact rảnh: `hidden`, `minimal`, peek, có activity, hoặc khóa máy.

**Biểu cảm (P3, tùy chọn, mặc định tắt):** mặc định giữ hình mắt gốc ở trên; biểu cảm chỉ bật khi người dùng chọn trong Cài đặt (`indicators.eyes_moods`).

| Biểu cảm | Khi nào | Hình |
|---|---|---|
| Nhìn theo chuột | Chuột trong bán kính 240 px quanh island (`indicators.eyes_follow_cursor`, mặc định tắt) | Mắt dịch tối đa ±3 px theo hướng chuột, dùng spring `gentle` |
| Vui | Hover vào pill, trước khi bung peek | Mắt cong lên kiểu `^ ^` trong 250 ms |
| Buồn ngủ | Không dùng máy ≥ 10 phút, hoặc 23:00–6:00 | Mắt khép một nửa, chớp chậm hơn (8–12 s) |
| Ngạc nhiên | Có alert mới | Mắt mở to (`scale` 1.2) 150 ms rồi chuyển sang alert |

Nhìn theo chuột cần backend gửi vị trí chuột, và chỉ gửi khi chuột đang ở gần (tối đa 30 Hz). Phần này dùng lại cursor poller của [island-window](modules/island-window.md).

**Giảm chuyển động:** mắt đứng yên, không chớp, không biểu cảm.

### Scrollbar dùng chung (2026-11-01)

Mọi vùng cuộn của UI app dùng scrollbar 8 px: nền surface-2, tay kéo cam neon accent bo tròn ngay ở trạng thái mặc định, hover accent-bright. Khai báo trong base.css với token; color-scheme dark cho điều khiển native. Áp dụng island, playlist, search, textarea và cửa sổ đầy đủ; không chèn CSS vào website nhạc bên ngoài.
