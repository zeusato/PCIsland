# 00 · Tầm nhìn & phạm vi

> Cập nhật: 2026-10-31 · Trạng thái: bản đầu. Các mục ghi "Đề xuất" đang chờ chốt (xem [Handoff §4](05-handoff.md#4-câu-hỏi-mở)).

## 1. Tầm nhìn

PCisland đưa trải nghiệm Dynamic Island lên Windows. Một vùng nhỏ ở giữa mép trên màn hình cho biết đang có gì diễn ra (nhạc, hẹn giờ, nhắc việc, trạng thái máy) và cho xử lý nhanh mà không phải chuyển cửa sổ. Khi không có gì, nó gần như vô hình.

## 2. Mục tiêu

| ID | Mục tiêu | Đo bằng |
|---|---|---|
| G1 | **Tương tác nhanh:** thao tác phổ biến (play/pause, đặt giờ, hoãn nhắc) xong trong ≤ 2 click từ island | Kiểm thử luồng trong module doc |
| G2 | **Không cản đường:** không cướp focus, không đè game/phim toàn màn hình, không bung ra khi chỉ lướt chuột qua | Checklist trong [island-window](modules/island-window.md) |
| G3 | **Nhẹ:** exe < 20 MB; RAM lõi < 150 MB; CPU khi rảnh < 0,5% | Đo mỗi bản phát hành ([system design §13](02-system-design.md#13-hiệu-năng)) |
| G4 | **Mượt:** chuyển trạng thái đạt 60 fps, không giật trên GPU tích hợp | Đo trong PoC và mỗi bản phát hành |
| G5 | **Portable:** chạy ngay không cần cài, không cần quyền admin, xóa thư mục là gỡ xong | Kiểm thử trên máy sạch |

## 3. Phi mục tiêu (ít nhất tới v1.0)

- Không thay thế app nghe nhạc đầy đủ như Spotify hay foobar2000. Chỉ phát, hàng đợi và playlist cơ bản.
- Không tách luồng âm thanh từ YouTube/Spotify, không chặn quảng cáo, không tải nhạc từ trang web ([ADR-004](06-decision-log.md#adr-004--nhạc-online-theo-mô-hình-lai)).
- Không hỗ trợ macOS/Linux.
- Không có tài khoản, đồng bộ đám mây hay telemetry.
- Không thay thế Notification Center hay OSD âm lượng/độ sáng gốc của Windows.

## 4. Người dùng mục tiêu

- **Người làm việc trên PC cả ngày** (văn phòng, dev, thiết kế): vừa nghe nhạc vừa làm, cần hẹn giờ và nhắc việc nhanh.
- **Người dùng laptop:** quan tâm pin, âm lượng, tài nguyên.
- **Người thích tùy biến desktop:** muốn giao diện đẹp, khác biệt.

## 5. Nguyên tắc sản phẩm

1. **Im lặng khi rảnh.** Không có hoạt động thì island nhỏ nhất có thể. Chuyển động duy nhất là đôi mắt thỉnh thoảng chớp, đủ để thấy "có sự sống" mà không gây chú ý.
2. **Nhìn là hiểu.** Trạng thái thu gọn phải đọc được trong 1 giây.
3. **Không cản đường.** Không cướp focus, không đè game/phim, không bung ra khi chỉ lướt chuột qua.
4. **Neon có ý nghĩa.** Màu cam phát sáng chỉ dành cho thứ đang hoạt động: đang phát, đang đếm, sắp đến giờ.
5. **Mọi thứ tắt được.** Mỗi module và mỗi kết nối mạng đều có công tắc.
6. **Offline trước.** Mọi tính năng lõi chạy được khi không có mạng.

## 6. Danh mục tính năng

Ưu tiên: **M** = bắt buộc cho v1.0 · **S** = nên có ở v1.0 · **C** = có thì tốt, có thể sau v1.0.

### Lõi

| ID | Tính năng | Ưu tiên | Phase | Module |
|---|---|---|---|---|
| F01 | Cửa sổ island: compact / peek / minimal / hidden, giữa mép trên, luôn nổi, click xuyên vùng trong suốt | M | P0–P1 | [island](modules/island-window.md) |
| F02 | Cửa sổ đầy đủ: dashboard, trang cho từng module, cài đặt | M | P1–P2 | [ui-main](modules/full-window.md) |
| F03 | Vòng đời app: tray, khởi động cùng Windows, chạy một bản, phím tắt toàn cục | M | P1 | [app](modules/app-lifecycle.md) |
| F04 | Nhận biết app toàn màn hình và cửa sổ phóng to (tự ẩn hoặc thu thành vạch) | M | P0–P1 | [island](modules/island-window.md) |
| F05 | Đa màn hình & DPI: chọn màn hình, theo màn hình có chuột | S | P3 | [island](modules/island-window.md) |
| F06 | Activity hub: chọn hoạt động chính/phụ, cảnh báo, hành động nhanh | M | P1 | [activity](modules/activity-hub.md) |
| F07 | Theme: biến thể màu nhấn, độ sáng glow, cỡ island | S | P3 | [ui-main](modules/full-window.md) |
| F08 | Bộ âm báo (chọn âm, âm lượng riêng). Âm báo cơ bản có từ P1 | S | P3 | [app](modules/app-lifecycle.md) |
| F09 | Đa ngôn ngữ vi/en, hạ tầng i18n có từ ngày đầu | M | P1 | [ui-main](modules/full-window.md) |
| F45 | Kiểm tra bản cập nhật qua GitHub Releases (không tự tải) | S | P3 | [app](modules/app-lifecycle.md) |

### Media

| ID | Tính năng | Ưu tiên | Phase | Module |
|---|---|---|---|---|
| F10 | Hiển thị và điều khiển media của app khác (Spotify, trình duyệt…) qua Windows Media Session | M | P1 | [media](modules/media-hub.md) |
| F11 | Trình phát file trên máy: thư viện, playlist, hàng đợi, tag, ảnh bìa, phím media | M | P2 | [local_player](modules/local-player.md) |
| F12 | Trình phát web nhúng: mở trang nhạc, đăng nhập, thu gọn chạy ngầm | M | P2 | [web_player](modules/web-player.md) |
| F13 | Radio online / link stream trực tiếp | C | P3 | [local_player](modules/local-player.md) |
| F14 | Lời bài hát chạy theo nhạc | S | P3 | [lyrics](modules/lyrics-visualizer.md) |
| F15 | Sóng nhạc theo âm thanh thật (visualizer) | S | P3 | [visualizer](modules/lyrics-visualizer.md) |

### Năng suất

| ID | Tính năng | Ưu tiên | Phase | Module |
|---|---|---|---|---|
| F20 | Hẹn giờ đếm ngược (nhiều bộ, nhãn, preset) | M | P1 | [timer](modules/timer.md) |
| F21 | Bấm giờ (lap) | M | P1 | [timer](modules/timer.md) |
| F22 | Pomodoro (chu kỳ làm/nghỉ, thống kê ngày) | M | P2 | [timer](modules/timer.md) |
| F23 | Focus mode: Pomodoro + chặn cảnh báo không quan trọng | S | P3 | [timer](modules/timer.md) |
| F24 | Nhắc việc: tạo/sửa, lặp lại, hoãn, xử lý nhắc bị lỡ | M | P2 | [reminders](modules/reminders.md) |
| F25 | Tạo nhắc việc bằng câu tiếng Việt tự nhiên ("họp 3h chiều mai") | S | P3 | [reminders](modules/reminders.md) |
| F26 | Ghi chú nhanh từ island, tìm kiếm, ghim lên island | S | P3 | [notes](modules/quick-notes.md) |
| F27 | Kệ file: kéo thả file vào island để giữ tạm, kéo ra chỗ khác | S | P3 | [shelf](modules/file-shelf.md) |

### Hệ thống

| ID | Tính năng | Ưu tiên | Phase | Module |
|---|---|---|---|---|
| F30 | Chỉ báo thay đổi âm lượng | S | P2 | [indicators](modules/system-indicators.md) |
| F31 | Chỉ báo pin & sạc (laptop) | S | P2 | [indicators](modules/system-indicators.md) |
| F32 | Chỉ báo mic/camera đang được dùng | C | P4 | [indicators](modules/system-indicators.md) |
| F33 | Màn hình chờ: đôi mắt chớp (mặc định) hoặc đồng hồ; biểu cảm và nhìn theo chuột ở P3 | M | P1, P3 | [indicators](modules/system-indicators.md) |
| F34 | Theo dõi CPU/RAM/mạng | C | P4 | [indicators](modules/system-indicators.md) |

### Tích hợp (backlog sau v1.0)

| ID | Tính năng | Ưu tiên | Phase | Module |
|---|---|---|---|---|
| F40 | Thời tiết (Open-Meteo) | C | P4 | [future](modules/future-modules.md) |
| F41 | Lịch Google/Outlook | C | P4 | [future](modules/future-modules.md) |
| F42 | Lịch sử clipboard | C | P4 | [future](modules/future-modules.md) |
| F43 | Đọc thông báo của app khác (cần nghiên cứu, xung đột portable) | C | P4 | [future](modules/future-modules.md) |
| F44 | Plugin API | C | P4 | [future](modules/future-modules.md) |

## 7. Ràng buộc

- **Hệ điều hành:** chỉ hỗ trợ chính thức **Windows 11**. Windows 10 không được hỗ trợ: không test, không sửa lỗi riêng. Nếu app tình cờ chạy được trên Win10 thì không chặn. Lý do: Microsoft đã ngừng hỗ trợ Win10 từ 10/2025 ([ADR-008](06-decision-log.md#adr-008--mặc-định-sản-phẩm)). Kiến trúc x64; ARM64 để sau v1.0.
- **Runtime:** WebView2 Evergreen, có sẵn trên Windows 11. Không đóng gói kèm runtime.
- **Quyền:** không cần admin, không cài service hay driver, không sửa cài đặt hệ thống.
- **Mạng:** không bắt buộc với tính năng lõi. Mọi kết nối mạng đều tắt được.

## 8. Sổ rủi ro

| ID | Rủi ro | Ảnh hưởng | Khả năng | Giảm thiểu | Kiểm chứng ở |
|---|---|---|---|---|---|
| R1 | Island đè tab trình duyệt / thanh tiêu đề ở giữa mép trên | Cao | Cao | Pill nhỏ; trễ hover; thu thành vạch khi cửa sổ phóng to; chỉnh được vị trí | Gate G0 |
| R2 | Google chặn đăng nhập trong WebView2 (ảnh hưởng YouTube Music) | TB | Cao | Hướng người dùng sang nguồn ngoài qua Media Session (F10) | G0 test matrix |
| R3 | Trang cần DRM (Spotify Web) không phát được trong WebView2 | TB | Cao | Như R2 | G0 test matrix |
| R4 | RAM WebView2 bị chê nặng | TB | TB | Cửa sổ tạo khi cần; hủy web player khi rảnh lâu; đo theo ngân sách | Mỗi bản phát hành |
| R5 | SmartScreen / antivirus cảnh báo exe chưa ký | TB | Cao | Không dùng global hook; phát hành qua GitHub kèm SHA256 và hướng dẫn SmartScreen; ký miễn phí qua SignPath nếu đủ điều kiện ([ADR-009](06-decision-log.md#adr-009--phân-phối--ký-số)) | Trước v1.0 |
| R6 | Adapter trang web hỏng khi trang đổi giao diện | Thấp | Cao | Dựa vào Media Session chuẩn; adapter tách riêng; test định kỳ | Mỗi bản phát hành |
| R7 | Bản cập nhật Windows đổi hành vi (z-order, SMTC…) | TB | Thấp | Gói mọi API nền tảng trong `platform/`; checklist kiểm thử thủ công | Mỗi bản phát hành |
| R8 | Phình phạm vi | Cao | TB | Feature ID + phase cố định; ý mới vào backlog | Cuối mỗi phase |
| R9 | Tính năng multiwebview của Tauri còn gắn cờ `unstable` | TB | TB | Phương án dự phòng trong [web-player](modules/web-player.md) | G0 |
| R10 | Island không hiện trên mọi virtual desktop (đây là yêu cầu bắt buộc) | Cao | TB | Kiểm tra trong PoC, tìm giải pháp trước G0 | G0 |

## 9. Backlog ý tưởng

Ý tưởng chưa có ID. Muốn đưa vào kế hoạch thì cấp ID mới (F46, F47…), gán phase và module, rồi cập nhật lộ trình.

- **F46 · Trình phát video nổi:** dán link YouTube/Facebook/trang có video → app tự lấy stream và phát trong 1 cửa sổ nhỏ không quảng cáo; ghim trên cùng, co kéo, về mặc định rộng 320, island điều khiển được, phím tắt Ctrl+Alt+V. [video-player](modules/video-player.md), ADR-018. Chưa xếp phase.
- Pin tai nghe Bluetooth khi kết nối
- Chỉ báo Caps Lock / Num Lock
- Chỉ báo đang quay màn hình
- Tiến độ tải file của trình duyệt
- Xuất thống kê Pomodoro
- Widget chứng khoán / tỷ giá
- Nhập nhắc việc bằng giọng nói
- Tiện ích dọn dẹp được chủ dự án yêu cầu triển khai 2026-11-01: [RAM toàn PC và dọn rác Windows/app/browser một chạm](modules/maintenance.md), ADR-019. Đã có source/test, chưa đóng gói; không phải toàn bộ F34 theo dõi CPU/mạng.
