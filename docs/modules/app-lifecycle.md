# Module: Vòng đời app (`app`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P1 (lõi), P3 (âm báo tùy chỉnh, cập nhật) · Tính năng: F03, F08, F45 · Cập nhật: 2026-10-31

## 1. Mục đích

Đưa app từ lúc được chạy tới lúc thoát một cách có trật tự: chạy đúng một bản, khởi động cùng Windows, icon tray, phím tắt toàn cục, âm báo, phản ứng với sleep/khóa máy, kiểm tra bản cập nhật, tắt sạch.

## 2. Phạm vi

**Trong phạm vi:**
- Trình tự khởi động và tắt, single instance, autostart, tray, phím tắt toàn cục.
- `SoundService` (âm báo).
- Cờ dòng lệnh, panic hook, kiểm tra cập nhật.

**Ngoài phạm vi:**
- Vị trí và hiển thị của island: thuộc [island-window](island-window.md).
- Đường dẫn dữ liệu, settings: thuộc [settings-storage](settings-storage.md).

## 3. Hành vi & luồng người dùng

1. **Đăng nhập Windows:** app tự chạy qua khóa `Run`. Island hiện sau < 1,5 s. Không mở cửa sổ nào khác.
2. **Chạy exe lần nữa:** không mở bản thứ hai. Bản đang chạy mở cửa sổ `main` (nếu chạy từ Explorer) hoặc xử lý tham số dòng lệnh.
3. **Chuột phải tray:**
   - Mở PCisland
   - Ẩn/hiện island
   - Tạm dừng cảnh báo 1 giờ
   - Cài đặt
   - Thoát
4. **Click trái tray:** mở/đóng `main`.
5. **Di chuyển thư mục app rồi chạy lại:** nếu autostart đang bật, khóa `Run` được sửa sang đường dẫn mới.
6. **Có bản mới (F45):** alert nhỏ "Có bản 1.2.0" với hành động "Xem" (mở trang release) và "Bỏ qua bản này".

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `app.update` | Alert (ttl 8 s) | — | — | Icon tải + "Có bản x.y.z" | Xem · Bỏ qua bản này |
| `app.crash_recovered` | Alert (ttl 10 s) | — | — | "PCisland vừa gặp lỗi" | Mở thư mục log |
| `app.alerts_paused` | Ambient | Icon chuông gạch | Còn lại bao lâu | — | Bật lại |

## 5. Thiết kế kỹ thuật

**Trình tự khởi động** (`app/startup.rs`), mỗi bước được log thời gian:
1. Single instance (`tauri-plugin-single-instance`). Bản thứ hai gửi `argv` sang bản đầu rồi thoát.
2. Phân tích cờ dòng lệnh.
3. Xác định đường dẫn và portable mode ([settings-storage](settings-storage.md)).
4. Khởi tạo log và panic hook.
5. Nạp settings (có migration).
6. Mở DB (có migration) trên luồng `db`.
7. Tạo EventBus, ActivityHub, AudioEngine, ArtworkCache, Clock.
8. Tạo cửa sổ `island` (ẩn); khởi động luồng `win-events`.
9. Khởi động các module đang bật, song song, mỗi module có timeout 3 s.
10. Hiện island; tạo tray; đăng ký phím tắt.
11. Đồng bộ khóa autostart; lên lịch kiểm tra cập nhật (sau 60 s).

**Trình tự tắt:**
1. Hủy `CancellationToken` gốc.
2. Gọi `stop()` mọi module (song song, timeout 2 s) để lưu state: timer, vị trí phát nhạc, hàng đợi.
3. Flush settings, đóng DB, gỡ hook.
4. Thoát.

**Autostart:**
- Windows đăng ký trực tiếp qua `platform/autostart.rs`: HKCU Run/PCisland dùng đường dẫn exe có dấu nháy, `--autostart` và `--data-dir` nếu có; đặt StartupApproved enabled rồi đọc lại xác minh. Lỗi trả về UI; rollback dấu tick. Cài đặt có nút Đăng ký lại.
- Lúc khởi động: nếu `app.autostart = true` mà giá trị trong registry khác đường dẫn exe hiện tại thì ghi lại. Nếu `false` mà khóa vẫn còn thì xóa.

**Phím tắt toàn cục** (`tauri-plugin-global-shortcut`):
- Mặc định theo [ADR-008](../06-decision-log.md#adr-008--mặc-định-sản-phẩm).
- Đăng ký thất bại (trùng app khác) thì ghi `warn` và hiện cảnh báo trong trang Cài đặt › Phím tắt. Không chặn khởi động.

**SoundService** (`core/audio` + `app/sound.rs`):
- Phát file âm báo nhúng (`assets/sounds/*.ogg`) qua AudioEngine, âm lượng `app.sound_volume`.
- `play(sound, loop_until: Option<Instant>)` trả về handle để dừng.
- F08 (P3) thêm: chọn âm cho từng loại, nghe thử, tắt hẳn âm.

**Sự kiện hệ thống** (luồng `win-events` → `AppEvent`):
- `WM_POWERBROADCAST` (`PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC`) → `Suspending` / `Resumed`.
- `WM_WTSSESSION_CHANGE` (cần `WTSRegisterSessionNotification`) → `SessionLocked` / `SessionUnlocked`.
- `WM_TIMECHANGE` → `SystemTimeChanged`.
- `WM_DISPLAYCHANGE` → `DisplayChanged`.

**Cờ dòng lệnh:**

| Cờ | Tác dụng |
|---|---|
| `--debug` | Log mức `debug`, bật DevTools |
| `--data-dir <path>` | Ép thư mục dữ liệu |
| `--reset-settings` | Sao lưu `settings.json` thành `.reset-<ts>` rồi dùng mặc định |
| `--cleanup` | Xóa `%LOCALAPPDATA%\PCisland` và khóa autostart, rồi thoát (dùng trước khi xóa thư mục app) |
| `--open <route>` | Mở `main` tại route |

**Kiểm tra cập nhật (F45):**
- `GET https://api.github.com/repos/<owner>/<repo>/releases/latest`, tối đa 1 lần mỗi 24 h, chỉ khi `app.check_updates = true`.
- So SemVer với phiên bản hiện tại; bỏ qua bản nằm trong `app.skipped_version`. Không tải, không cài.

**Panic hook:** ghi `crash-<ts>.log` gồm message, backtrace, phiên bản. Lần chạy sau thấy file mới hơn lần chạy trước thì phát alert `app.crash_recovered`.

`--autostart` trong lần gọi thứ hai không mở cửa sổ chính; cờ chỉ dùng cho lần đăng nhập Windows. Đường dẫn thực tế trong `app_get_info.startupCommand` giúp phát hiện đăng ký trỏ tới exe đã di chuyển. Chưa coi test lệnh/registry là bằng chứng boot thành công; cần khởi động lại Windows.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `app_get_info` | `() → AppInfo { version, portable, dataDir, webview2Version }` |
| `app_quit` | `() → ()` |
| `settings_update` | `("app", {autostart: bool}) → Settings` (đăng ký và xác minh) |
| `app_show_toast` | `(title: string, body: string) → ()` (native, 3 giây) |
| `app_pause_alerts` | `(minutes: u32) → ()` |
| `app_resume_alerts` | `() → ()` |
| `app_check_update_now` | `() → UpdateInfo?` |
| `app_open_logs_folder` | `() → ()` |
| `app_play_sound` | `(sound: SoundId) → ()` (nghe thử trong Cài đặt) |
| `app_set_shortcut` | `(action: ShortcutAction, accelerator: string?) → ()` |

| Event | Payload |
|---|---|
| `app://shortcut-error` | `{ action, accelerator, reason }` |

## 7. Dữ liệu & cài đặt

| Khóa | Mặc định |
|---|---|
| `app.autostart` | `true` |
| `app.language` | `"vi"` |
| `app.sound_volume` | `0.6` |
| `app.sounds` | `{ reminder: "chime", timer: "bell", pomodoro: "soft-tick", shelf: "pop" }`. Công tắc bật/tắt âm nằm ở cài đặt của từng module, ví dụ `shelf.sound_on_drop` |
| `app.shortcuts` | `{ togglePeek: "Ctrl+Alt+Space", openMain: "Ctrl+Alt+I", toggleHidden: "Ctrl+Alt+H", quickNote: "Ctrl+Alt+N" }` |
| `app.check_updates` | `true` |
| `app.skipped_version` | `null` |

## 8. Phụ thuộc

- **Core:** tất cả (đây là nơi dựng lõi).
- **Platform:** `SessionEvents`, `PowerEvents`, `TimeEvents`, `DisplayEvents`.
- **Crate:** `tauri-plugin-single-instance`, `tauri-plugin-autostart`, `tauri-plugin-global-shortcut`, `reqwest`, `semver`.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Module khởi động quá 3 s | Đánh dấu Failed, app vẫn chạy tiếp |
| Chạy từ thư mục chỉ đọc (USB khóa ghi, Program Files) | Dữ liệu sang `%APPDATA%`; ghi `info` |
| Phím tắt bị chiếm | Cảnh báo trong Cài đặt, không chặn |
| Tắt Windows khi đang chạy | `WM_QUERYENDSESSION`/`WM_ENDSESSION` → chạy trình tự tắt nhanh (timeout 1 s) |
| Không có mạng khi kiểm tra cập nhật | Bỏ qua im lặng, thử lại lần sau |
| GitHub giới hạn tần suất API | Lùi lại 24 h |

## 10. Hiệu năng

- Khởi động: island hiện < 1,5 s. Module chậm không chặn island.
- Không có tác vụ định kỳ nào ngoài lịch kiểm tra cập nhật.

## 11. Kiểm thử

**Unit:** so sánh SemVer và bỏ qua phiên bản; phân tích cờ dòng lệnh; đồng bộ autostart (registry giả).

**Checklist thủ công:**
- [ ] Đăng xuất rồi đăng nhập: app tự chạy, island hiện
- [ ] Chạy exe lần 2: không có bản thứ hai
- [ ] Di chuyển thư mục app rồi chạy: khóa autostart cập nhật đường dẫn mới
- [ ] Sleep 2 phút rồi wake: không crash, module nhận `Resumed`
- [ ] Khóa máy rồi mở: island ẩn khi khóa, hiện lại khi mở
- [ ] Mọi mục trên tray hoạt động
- [ ] `--cleanup` xóa sạch `%LOCALAPPDATA%\PCisland` và khóa `Run`

## 12. Câu hỏi mở

Không có. Q4 đã chốt: giữ phím tắt mặc định, người dùng tự đổi trong Cài đặt › Phím tắt.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Kiểm tra cập nhật từ GitHub Releases (hằng ngày + nút trong Cài đặt); manifest Common-Controls v6 nhúng qua linker (ADR-012); trạng thái module trong Cài đặt |
| 2026-11-01 | Đăng ký Run có quoting/cờ startup, đọc lại xác minh, báo lỗi/repair trong Settings; IPC toast Windows. |
| 2026-11-02 | Lần chạy đầu (`app.welcomeDone = false`) tự mở cửa sổ chính với hộp chào: hỏi tải yt-dlp và gói nhạc tập trung, không tải gì khi chưa xác nhận; Để sau/Bắt đầu dùng đặt `welcomeDone = true`. Bản đã cài trước đó cũng thấy hộp này một lần. |
