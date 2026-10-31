# Module tương lai (P4)

> Trạng thái: Phác thảo · Phase: P4 · Tính năng: F40–F44 · Cập nhật: 2026-10-31
>
> Đây là đặc tả sơ bộ để không mất ý tưởng và lường trước tác động kiến trúc. Khi một mục được đưa vào kế hoạch, tách thành module doc riêng theo [template](README.md#3-template).

## F40 · Thời tiết (`weather`)

- **Giá trị:** nhiệt độ + icon trên compact khi rảnh; peek có dự báo theo giờ.
- **Nguồn:** Open-Meteo (`api.open-meteo.com`), không cần API key. Điều khoản dành cho sử dụng phi thương mại; **kiểm tra lại** nếu PCisland có hình thức thu phí.
- **Vị trí:** người dùng tự tìm thành phố (Open-Meteo Geocoding). **Không** định vị theo IP, không dùng Location API của Windows (tránh xin quyền).
- **Tần suất:** 30 phút; chỉ khi có mạng và không khóa máy.
- **Activity:** `weather.now` Ambient (khi rảnh, nếu bật); `weather.alert` Alert khi sắp mưa trong 1 giờ (tùy chọn).
- **Tác động kiến trúc:** thêm domain vào danh sách mạng cho phép; module mới thuần event, không cần platform mới.

## F41 · Lịch Google / Outlook (`calendar`)

- **Giá trị:** sự kiện kế tiếp hiện Ambient; trước giờ họp 5 phút là Critical, có nút "Tham gia" nếu sự kiện có link Meet/Teams.
- **Kỹ thuật:**
  - OAuth 2.0 PKCE qua **trình duyệt hệ thống** với loopback redirect (`http://127.0.0.1:<port>`). Không dùng webview cho đăng nhập.
  - Google Calendar API / Microsoft Graph, quyền chỉ đọc.
  - Token mã hóa bằng DPAPI (`CryptProtectData`), lưu trong `kv`.
- **Tác động kiến trúc:**
  - Lần đầu app giữ bí mật, nên **bắt buộc review bảo mật** ([02 §12](../02-system-design.md#12-bảo-mật)).
  - Cần đăng ký OAuth client. Với Google, app dùng scope nhạy cảm phải qua xác minh nếu phát hành công khai.
- **Rủi ro:** quy trình xác minh của Google tốn thời gian. Có thể bắt đầu bằng Outlook (Microsoft Graph) hoặc đọc file `.ics` theo URL bí mật (không cần OAuth).

## F42 · Lịch sử clipboard (`clipboard`)

- **Câu hỏi giá trị:** Windows đã có `Win+V`. Chỉ làm nếu có điểm khác biệt rõ: ghim snippet, tìm kiếm không dấu, dán nhanh từ peek.
- **Kỹ thuật:** `AddClipboardFormatListener`. Bỏ qua nội dung có định dạng `ExcludeClipboardContentFromMonitorProcessing` hoặc `CanIncludeInClipboardHistory = 0` (trình quản lý mật khẩu). Giới hạn 50 mục, chỉ văn bản và ảnh nhỏ.
- **Quyền riêng tư:** tắt mặc định; dữ liệu chỉ ở máy; có nút xóa toàn bộ.

## F43 · Đọc thông báo của app khác (`notifications`)

- **Kỹ thuật:** `UserNotificationListener` (WinRT) yêu cầu **package identity** (MSIX hoặc sparse package có ký số), nên xung đột với portable ([ADR-005](../06-decision-log.md#adr-005--lưu-trữ--portable)).
- **Hướng nghiên cứu:**
  1. Bản phát hành "MSIX edition" song song với bản portable.
  2. Sparse package + ký số.
  3. Bỏ hẳn tính năng.
- **Trước khi làm:** viết ADR so sánh 3 hướng; hướng 1 và 2 đều cần ký số, mà [ADR-009](../06-decision-log.md#adr-009--phân-phối--ký-số) đang chọn không mua chứng chỉ, nên phải xét lại ADR-009 cùng lúc.

## F44 · Plugin API

- **Mục tiêu:** cho bên thứ ba tạo activity (widget chứng khoán, trạng thái build CI, đếm ngược sự kiện…) mà không phải sửa lõi.
- **Hướng đề xuất:**
  - Plugin **khai báo** (manifest + script sandbox) tạo activity bằng các view có sẵn (text, progress, list, icon).
  - Plugin **không** được tự render HTML trên island.
  - Hai lựa chọn chạy script: WASM (Wasmtime, chạy trong backend, cấp quyền theo manifest: mạng tới domain khai báo, timer) hoặc JS trong một webview ẩn cô lập. Đề xuất WASM vì nhẹ và an toàn hơn.
- **Điều kiện tiên quyết:** mô hình Activity đã ổn định sau v1.0 (không đổi payload chung trong ≥ 2 bản phát hành).
- **Tác động kiến trúc:** `ActivityKind::Plugin { plugin_id, template }`; view registry có các template chung; capability mới cho plugin.

## Ý tưởng khác (chưa có ID)

Xem [00 §9](../00-vision-scope.md#9-backlog-ý-tưởng). Ghi chú kỹ thuật nhanh:

| Ý tưởng | Ghi chú kỹ thuật |
|---|---|
| Pin tai nghe Bluetooth | `Windows.Devices.Enumeration` + thuộc tính pin (`System.Devices.BatteryLife`); không phải thiết bị nào cũng báo |
| Caps Lock / Num Lock | `GetKeyState` khi có `WM_INPUT`/hook bàn phím: **không** dùng hook toàn cục. Cân nhắc poll 4 Hz chỉ khi bật |
| Chỉ báo quay màn hình | Chưa có API ổn định; có thể suy ra từ Game Bar / Graphics Capture: cần nghiên cứu |
| Tiến độ tải file của trình duyệt | Không có API chung; bỏ |
| Process loopback cho visualizer | `ActivateAudioInterfaceAsync` + `AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS` (Win10 2004+), thu đúng app đang là nguồn media |

## Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo phác thảo |
