# Module: Chỉ báo hệ thống (`indicators`)

> Trạng thái: Thiết kế · Phase: P1 (F33), P2 (F30, F31), P4 (F32, F34) · Tính năng: F30–F34 · Cập nhật: 2026-10-31

## 1. Mục đích

Cho biết nhanh trạng thái máy mà không chiếm chỗ: đổi âm lượng, cắm/rút sạc, pin yếu, màn hình chờ khi rảnh (đôi mắt hoặc đồng hồ); sau v1.0 có thêm mic/camera đang dùng và CPU/RAM/mạng.

## 2. Phạm vi

**Trong phạm vi:**
- Âm lượng endpoint mặc định (F30).
- Nguồn điện và pin (F31).
- Màn hình chờ khi rảnh: đôi mắt hoặc đồng hồ (F33).
- Mic/camera đang dùng (F32, P4).
- Theo dõi hệ thống (F34, P4).

**Ngoài phạm vi:**
- Thay hoặc ẩn OSD âm lượng/độ sáng gốc của Windows ([00 §3](../00-vision-scope.md#3-phi-mục-tiêu-ít-nhất-tới-v10)).
- Chỉnh âm lượng từng app.
- Độ sáng màn hình.

## 3. Hành vi & luồng người dùng

1. **Chỉnh âm lượng** (phím, flyout, app khác): alert nhỏ 260×40 gồm icon loa + thanh mức + `%` trong 1,5 s. Chỉnh liên tục thì cập nhật trong cùng alert. Tắt tiếng thì icon gạch.
2. **Cắm sạc:** alert "Đang sạc · 64%" với hiệu ứng tia sáng chạy một lần dọc pill. **Rút sạc:** alert "Dùng pin · 64% · còn ~3 giờ" (nếu Windows ước tính được).
3. **Pin 20% / 10%:** alert màu `--c-warning` / `--c-danger`, chỉ một lần cho mỗi ngưỡng trong mỗi lần xả. Pin 10% dùng alert lớn có nút "Bật tiết kiệm pin" (mở Cài đặt pin của Windows).
4. **Rảnh:**
   - `island.idle_display = "eyes"` (mặc định): pill hiện đôi mắt, thỉnh thoảng chớp ([04 §11](../04-design-system.md#11-đôi-mắt-màn-hình-chờ)).
   - `"clock"`: compact hiện `15:30`.
   - Hover vào pill thì peek là launcher, có ngày đầy đủ và pin.
5. **(P4) Một app bật mic:** bong bóng phụ có chấm cam + icon mic; peek cho biết app nào (nếu biết).
6. **(P4) Theo dõi hệ thống:** widget CPU/RAM/mạng trong `main › Trang chủ` và tùy chọn trên peek khi rảnh.

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `indicators.volume` | Alert (ttl 1,5 s) | — | — | Nhỏ: icon + thanh + % | — |
| `indicators.power` | Alert (ttl 3 s) | — | — | Nhỏ: icon sạc/pin + % (+ thời gian còn lại) | — |
| `indicators.battery_low` | Alert (ttl 6 s; ở 10% là alert lớn ttl 10 s) | — | — | "Pin còn 10%" | `open_power_settings` |
| `indicators.idle` | Idle | Đôi mắt chớp, hoặc `HH:mm` (payload `{ mode: "eyes" \| "clock" }`) | Ngày đầy đủ, pin, launcher | — | — |
| `indicators.mic_cam` (P4) | Live | — | Danh sách app đang dùng | — | `open_privacy_settings` |

Đôi mắt và đồng hồ chạy hoàn toàn ở frontend (`features/indicators/views/IdleEyes.svelte`, `IdleClock.svelte`), không có event từ backend. Activity `indicators.idle` chỉ là cờ cho hub biết khi nào hiện. Với `idle_display = "pill"` hoặc `"hidden"`, module không tạo activity này.

## 5. Thiết kế kỹ thuật

**Âm lượng (F30):**
- Platform `AudioEndpointVolume`: `IMMDeviceEnumerator::GetDefaultAudioEndpoint(eRender, eMultimedia)` → `IAudioEndpointVolume::RegisterControlChangeNotify`. Callback chạy trên luồng COM của hệ thống; đẩy `{level, muted}` qua channel.
- Theo `AudioDevices` (IMMNotificationClient): đổi thiết bị mặc định thì đăng ký lại callback trên thiết bị mới.
- Bỏ qua thay đổi < 1% (nhiễu). Gộp bằng `hub.alert(id = "indicators:volume")` để cập nhật cùng một alert.
- Không báo khi thay đổi đến từ chính lúc khởi động (giá trị đầu tiên).

**Nguồn & pin (F31):**
- Platform `PowerEvents`: `RegisterPowerSettingNotification` cho `GUID_ACDC_POWER_SOURCE` và `GUID_BATTERY_PERCENTAGE_REMAINING` trên cửa sổ ẩn của luồng `win-events`.
- Lúc khởi động đọc `GetSystemPowerStatus`. Thời gian còn lại lấy `BatteryLifeTime` nếu khác `-1`.
- Máy không có pin (`BatteryFlag & 128`) thì module tắt phần pin, giữ phần khác.
- Ngưỡng pin yếu: giữ cờ "đã báo" cho mỗi ngưỡng; cắm sạc thì đặt lại cờ.

**Màn hình chờ (F33):**
- **Đôi mắt:** đặc tả hình và chuyển động ở [04 §11](../04-design-system.md#11-đôi-mắt-màn-hình-chờ). Cách lập lịch chớp:
  - Một `setTimeout` ngẫu nhiên 3–7 s bật class CSS keyframes; giữa các lần chớp không vẽ frame nào.
  - Hủy timer khi view bị gỡ hoặc khi `document.hidden`.
  - P1 chỉ có chớp mắt. Các biểu cảm và nhìn theo chuột để P3.
- **Đồng hồ:** một `setTimeout` căn theo đầu phút để cập nhật `HH:mm`. Định dạng theo `ui.time_format`.

**Mic/camera (F32, P4):**
- Đọc registry `HKCU\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\{microphone,webcam}\…`. Mục có `LastUsedTimeStop == 0` là đang dùng.
- Theo dõi bằng `RegNotifyChangeKeyValue` trên luồng riêng (không poll). Tên app lấy từ tên khóa (đường dẫn exe hoặc package).
- Hiện bong bóng phụ (mức Live, nhưng hub luôn ưu tiên cho làm phụ). **Không bao giờ ẩn chỉ báo này khi đang có**, kể cả trong Focus mode.

**Theo dõi hệ thống (F34, P4):**
- CPU qua `GetSystemTimes`, RAM qua `GlobalMemoryStatusEx`, mạng qua `GetIfTable2` (chênh lệch byte), lấy mẫu 1 Hz.
- **Chỉ lấy mẫu khi widget đang hiện** (`main` mở ở Trang chủ hoặc peek có widget); không thì dừng.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `indicators_get_state` | `() → IndicatorsSnapshot { volume, muted, power: PowerDto?, micCam?, rev }` |
| `indicators_open_power_settings` | `() → ()` (`ms-settings:batterysaver`) |
| `indicators_open_privacy_settings` | `() → ()` (P4, `ms-settings:privacy-microphone`) |
| `indicators_subscribe_system` (P4) | `(channel: Channel<SystemSample>) → SubscriptionId` |
| `indicators_unsubscribe_system` (P4) | `(id) → ()` |

| Event | Payload |
|---|---|
| `indicators://state` | `IndicatorsSnapshot` |

## 7. Dữ liệu & cài đặt

Không có bảng DB.

| Khóa | Mặc định |
|---|---|
| `indicators.eyes_follow_cursor` | `false` (P3) |
| `indicators.eyes_moods` | `true` (P3: vui, buồn ngủ, ngạc nhiên) |
| `indicators.volume_alert` | `true` |
| `indicators.power_alert` | `true` |
| `indicators.battery_low_thresholds` | `[20, 10]` |
| `indicators.mic_cam` | `true` (P4) |
| `indicators.system_monitor_on_peek` | `false` (P4) |

## 8. Phụ thuộc

- **Core:** ActivityHub, EventBus, settings.
- **Platform trait:** `AudioEndpointVolume`, `AudioDevices`, `PowerEvents`, `PrivacyUsage` (P4), `SystemCounters` (P4).
- **Crate:** `windows` (`Win32_Media_Audio`, `Win32_System_Power`, `Win32_System_Registry`, `Win32_NetworkManagement_IpHelper`).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Không có thiết bị âm thanh | Tắt phần âm lượng, ghi `info`; bật lại khi có thiết bị |
| Bluetooth headset đổi profile liên tục | Bỏ qua alert âm lượng trong 1 s sau khi đổi thiết bị |
| Máy bàn (không pin) | Không có phần pin |
| Alert âm lượng khi đang Focus | Bị hub chặn (đúng thiết kế) |
| Registry ConsentStore không có (Windows lạ) | Tắt F32, ghi `warn` |

## 10. Hiệu năng

Toàn bộ dựa trên event, không có polling (trừ F34 khi đang hiện). Khi rảnh, CPU ≈ 0.

## 11. Kiểm thử

**Unit:** logic ngưỡng pin (báo một lần, đặt lại khi sạc); lọc nhiễu âm lượng; phân tích khóa ConsentStore (P4).

**Checklist thủ công:**
- [ ] Phím âm lượng, flyout, chỉnh từ app khác → alert đúng, mượt khi giữ phím
- [ ] Đổi tai nghe ↔ loa → vẫn bắt được thay đổi âm lượng
- [ ] Cắm/rút sạc (laptop); giả lập ngưỡng pin bằng cấu hình `[99, 98]`
- [ ] Đồng hồ đổi đúng phút, định dạng 12h/24h
- [ ] Đôi mắt: chớp ngẫu nhiên 3–7 s, có chớp đôi; dừng khi có activity hoặc khi peek; đo CPU khi rảnh 60 s vẫn < 0,5%
- [ ] Bật giảm chuyển động: mắt đứng yên

## 12. Câu hỏi mở

Không có.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
