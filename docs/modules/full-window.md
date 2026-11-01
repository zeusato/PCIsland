# Module: Cửa sổ đầy đủ (`ui-main`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P1 (khung, i18n), P2 (các trang), P3 (theme) · Tính năng: F02, F07, F09 · Cập nhật: 2026-10-31

## 1. Mục đích

Chế độ hiển thị thứ ba: một cửa sổ riêng, hiển thị rõ mọi tính năng và đủ công cụ thiết lập. Island dành cho thao tác nhanh; cửa sổ này dành cho quản lý (thư viện nhạc, danh sách nhắc việc, playlist) và cài đặt.

## 2. Phạm vi

**Trong phạm vi:**
- Cửa sổ `main`: title bar tự vẽ, sidebar, router.
- Trang chủ (dashboard), khung trang của từng module, toàn bộ trang Cài đặt.
- Hạ tầng i18n, theme (F07).
- Toast và hộp thoại xác nhận dùng chung.

**Ngoài phạm vi:**
- Nội dung nghiệp vụ của từng trang: `features/<module>/views/Page.svelte` do module sở hữu, tuân theo khung ở đây.
- Cửa sổ web player: thuộc [web-player](web-player.md).

## 3. Hành vi & luồng người dùng

1. **Mở:** từ tray, phím tắt `Ctrl+Alt+I`, double-click island, hoặc nút ⤢ trong peek (mở đúng trang của activity).
2. **Đóng:** nút X hủy cửa sổ để trả RAM. Lần mở sau khôi phục kích thước, vị trí và trang cuối cùng.
3. **Điều hướng:** sidebar; `Ctrl+1…8` nhảy trang; `Ctrl+,` mở Cài đặt; `Esc` đóng hộp thoại.
4. **Đổi ngôn ngữ:** áp dụng ngay cho cả `main` và island, không cần khởi động lại.

## 4. Activity

Không áp dụng. Trang chủ hiển thị lại các activity đang có dưới dạng thẻ lớn và dùng chung `island://state`.

## 5. Thiết kế kỹ thuật

**Cửa sổ:** label `main`, `decorations(false)`, mặc định 1120×720, tối thiểu 960×640, có trên taskbar. Kích thước và vị trí lưu ở `ui.main_window_bounds`. Lúc mở, kiểm tra khung còn nằm trong màn hình nào không; không còn thì căn giữa màn hình chính.

**Title bar:** cao 40 px; `data-tauri-drag-region`; 3 nút (thu nhỏ, phóng to/khôi phục, đóng) dùng API cửa sổ Tauri. Hỗ trợ snap layout của Windows 11 khi hover nút phóng to: **kiểm tra trong P1**; nếu không được thì chấp nhận thiếu.

**Router:** router nhỏ tự viết theo hash (`#/media`, `#/settings/island`…). Không dùng thư viện router vì chỉ có khoảng 15 route.

| Route | Trang | Phase |
|---|---|---|
| `#/home` | Trang chủ: thẻ activity đang chạy, nhắc việc hôm nay, nhạc đang phát, shortcut tạo nhanh | P1 |
| `#/media` | Nhạc: Đang phát · Nhạc trên máy · Playlist · Radio · Online | P2 |
| `#/online` | Route tương thích cũ về trang Nhạc, không có mục sidebar riêng | P2 |
| `#/timer` | Hẹn giờ · Bấm giờ · Pomodoro (thống kê) | P1/P2 |
| `#/reminders` | Nhắc việc: Hôm nay · Sắp tới · Đã lỡ · Hoàn thành | P2 |
| `#/notes` | Ghi chú | P3 |
| `#/shelf` | Kệ file | P3 |
| `#/settings/<section>` | Cài đặt: Chung · Island · Giao diện · Phím tắt · Âm thanh · từng module · Dữ liệu & chẩn đoán · Giới thiệu | P1–P3 |

**Trang Cài đặt:**
- Mỗi module có `views/Settings.svelte` dựng từ bộ control dùng chung: `SettingRow`, `Toggle`, `Select`, `Slider`, `ShortcutInput`, `PathPicker`.
- Ghi thông qua `settings_update(section, patch)`.
- Trang "Module" liệt kê mọi module, có công tắc bật/tắt và trạng thái (Running / Failed + lý do + nút Thử lại).

**i18n (F09):**
- `src/lib/i18n/vi.json`, `en.json`; key dạng `timer.peek.addMinute`.
- `t(key, params)` hỗ trợ tham số `{name}` và số nhiều đơn giản.
- Ngôn ngữ lấy từ `app.language`; thiếu key thì lùi về `vi` và ghi `warn` ở bản dev.
- Script `pnpm i18n:check` báo key thiếu hoặc thừa giữa hai file; chạy trong CI.

**Theme (F07):**
- `ui.accent` đổi biến `--c-accent*` qua lớp `data-accent` trên `<html>` của mọi cửa sổ.
- `ui.glow_intensity` (0–1) nhân độ mờ của glow.
- `ui.island_scale` (0.9 / 1 / 1.15) nhân kích thước island; cửa sổ island đổi kích thước tương ứng.

**Thành phần dùng chung** (`src/lib/components/`): Button, IconButton, Toggle, Slider, Select, TextInput, ShortcutInput, Card, Tabs, Toast, ConfirmDialog, EmptyState, ProgressRing, ProgressBar, ArtworkImage (có placeholder và fade-in).

## 6. IPC

Cửa sổ này dùng command của các module khác. Command riêng:

| Command | Chữ ký |
|---|---|
| `ui_get_window_state` | `() → { bounds, lastRoute }` |
| `ui_save_window_state` | `(bounds, lastRoute) → ()` (gọi khi đóng, có debounce) |

## 7. Dữ liệu & cài đặt

| Khóa | Mặc định |
|---|---|
| `ui.main_window_bounds` | `null` (căn giữa) |
| `ui.last_route` | `"#/home"` |
| `ui.accent` | `"orange"` (`orange` · `amber` · `pink` · `blue`) |
| `ui.glow_intensity` | `1.0` |
| `ui.island_scale` | `1.0` |
| `ui.time_format` | `"24h"` |

## 8. Phụ thuộc

- **Frontend:** `lib/ipc`, `lib/i18n`, `lib/motion`, `lib/components`, view của từng `features/*`.
- **Backend:** command của mọi module; `settings`.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Mở cửa sổ khi màn hình cũ đã rút | Căn giữa màn hình chính |
| Module bị tắt mà người dùng vẫn vào trang của nó | Hiện `EmptyState` "Module đang tắt" kèm nút bật |
| Lỗi IPC trong trang | Toast có mã lỗi đã dịch; không làm trắng trang |

## 10. Hiệu năng

- RAM thêm khi mở < 80 MB. Đóng thì hủy cửa sổ.
- Danh sách dài (thư viện nhạc hàng chục nghìn bài) dùng virtual list.

## 11. Kiểm thử

**Unit:** router (phân tích route), `t()` với tham số, `i18n:check`.

**Checklist thủ công:**
- [ ] Mở/đóng 10 lần liên tục: RAM quay về mức nền
- [ ] Điều hướng hoàn toàn bằng bàn phím, focus ring luôn nhìn thấy
- [ ] Đổi ngôn ngữ và màu nhấn: áp dụng ngay cho cả island
- [ ] DPI 150%: không vỡ layout ở kích thước tối thiểu

## 12. Câu hỏi mở

- Có dùng nền Mica (trong suốt mờ của Windows 11) cho cửa sổ `main` không? Đề xuất: không, giữ nền đen đặc cho đúng phong cách. Có thể xét lại ở P3.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Nhớ vị trí, kích thước, trạng thái phóng to của cửa sổ; bỏ qua vị trí nằm trên màn hình đã rút |
| 2026-11-01 | Gộp Online vào Nhạc; nhãn Nhạc trên máy rõ nguồn file; chọn trang có lịch sử hiện hai hành động. |
