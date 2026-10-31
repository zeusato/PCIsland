# PCisland

Dynamic Island cho Windows. Một "hòn đảo" nhỏ nằm giữa mép trên màn hình, luôn nổi trên cùng, khởi động cùng Windows và chạy ngầm. Nó gom các tương tác nhanh vào một chỗ: nhạc (file trên máy, trang nhạc online, app khác), hẹn giờ, Pomodoro, nhắc việc, ghi chú, kệ file và chỉ báo hệ thống.

> **v0.1.0:** đã triển khai các tính năng P1–P3. Một số kịch bản tích hợp Windows và web player còn cần kiểm thử tay; xem [ghi chú phát hành](docs/releases/v0.1.0.md) và [Handoff](docs/05-handoff.md).

## Tải và chạy

Tải từ [GitHub Releases](https://github.com/zeusato/PCIsland/releases/tag/v0.1.0):

- **Bộ cài:** `PCisland-0.1.0-setup.exe`, cài theo tài khoản Windows hiện tại.
- **Portable:** giải nén `PCisland-0.1.0-portable.zip`, chạy `PCisland.exe`; thư mục `data` cạnh exe giữ dữ liệu portable.
- Mỗi gói có file `.sha256` để đối chiếu bằng `Get-FileHash <tên-file> -Algorithm SHA256`.

Yêu cầu Windows 11 x64 và Microsoft Edge WebView2 Runtime. Khi nâng cấp, thoát bản cũ từ tray trước; giữ nguyên thư mục `data` nếu dùng portable. Bản v0.x chưa ký số. Nếu SmartScreen hiện cảnh báo, chỉ tiếp tục sau khi kiểm tra nguồn tải và SHA256; chọn **More info → Run anyway** để chạy bản đã kiểm tra.

## Điểm chính

- **Các trạng thái hiển thị:** thu gọn (compact), mở rộng khi hover (peek), vạch mảnh (minimal), ẩn (hidden) và cửa sổ đầy đủ. Mặc định còn tác vụ thì giữ island ở minimal khi cần; ẩn hoàn toàn là tùy chọn. Khi đang soạn thảo, rời chuột không làm thu gọn island.
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
