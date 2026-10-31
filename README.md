# PCisland

Dynamic Island cho Windows. Một "hòn đảo" nhỏ nằm giữa mép trên màn hình, luôn nổi trên cùng, khởi động cùng Windows và chạy ngầm. Nó gom các tương tác nhanh vào một chỗ: nhạc (file trên máy, trang nhạc online, app khác), hẹn giờ, Pomodoro, nhắc việc, ghi chú, kệ file và chỉ báo hệ thống.

> **Trạng thái: Phase 0, chưa có code.** Bộ tài liệu thiết kế đã xong. Bắt đầu từ [Handoff](docs/05-handoff.md).

## Điểm chính

- **4 trạng thái hiển thị:** thu gọn (compact), mở rộng nhỏ khi hover (peek), cửa sổ đầy đủ (full window). Thêm vạch mảnh (minimal) khi cửa sổ khác đang phóng to, và ẩn hẳn khi có app toàn màn hình.
- **Portable:** một file exe, không cần cài, không cần quyền admin.
- **Giao diện:** nền đen, neon cam, bo góc, chuyển động spring.
- **Stack:** Tauri 2 · Rust · Svelte 5 · TypeScript · WebView2 (xem [ADR-001](docs/06-decision-log.md#adr-001--stack-tauri-2--rust--svelte-5)).

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [00 · Tầm nhìn & phạm vi](docs/00-vision-scope.md) | Mục tiêu, phi mục tiêu, danh mục tính năng F-xx, rủi ro |
| [01 · Lộ trình](docs/01-roadmap.md) | Các phase, cổng quyết định, tiêu chí hoàn thành |
| [02 · System design](docs/02-system-design.md) | Kiến trúc, cửa sổ, module, Activity, IPC, lưu trữ, bảo mật, hiệu năng |
| [03 · Nguyên tắc code](docs/03-coding-principles.md) | Cấu trúc thư mục, quy ước Rust/Svelte, lỗi, log, test, git, review |
| [04 · Design system](docs/04-design-system.md) | Token màu, chữ, kích thước island, chuyển động, âm thanh |
| [05 · Handoff](docs/05-handoff.md) | Trạng thái hiện tại, việc tiếp theo, câu hỏi mở, nhật ký phiên |
| [06 · Nhật ký quyết định](docs/06-decision-log.md) | Các ADR: quyết định kiến trúc và lý do |
| [Modules](docs/modules/README.md) | Đặc tả từng module, template, bản đồ phụ thuộc |

**Thứ tự đọc cho người mới:** 05 → 00 → 02 → 03 → tài liệu module liên quan.

## Quy tắc vàng

1. Tài liệu đi trước code. Muốn làm khác đặc tả thì sửa đặc tả trước.
2. Mỗi tính năng có ID (F-xx), thuộc đúng một phase và một module.
3. Ý tưởng mới vào backlog, không chen vào phase đang làm.
4. Kết thúc mỗi phiên làm việc phải cập nhật Handoff.
