# Module: Island window (`island`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P0 (spike), P1 (lõi), P3 (đa màn hình) · Tính năng: F01, F04, F05 · Cập nhật: 2026-11-01

## 1. Mục đích

Sở hữu cửa sổ island: tạo, đặt vị trí, giữ luôn nổi trên cùng, click xuyên ở vùng trong suốt, ẩn hiện theo ngữ cảnh (fullscreen, cửa sổ phóng to, khóa máy) và nhận focus khi cần nhập chữ. Đây là module quyết định cảm giác "không cản đường" (G2).

## 2. Phạm vi

**Trong phạm vi:**
- Cửa sổ `island` (Rust) và máy trạng thái hiển thị `islandMachine` (frontend).
- Visibility policy, cursor poller, hit region, đặt lại topmost.
- Vị trí theo màn hình và DPI (F05).

**Ngoài phạm vi:**
- Chọn nội dung hiển thị: thuộc [activity-hub](activity-hub.md).
- View của từng activity: thuộc thư mục `features/` của từng module.
- Cửa sổ `main`: thuộc [full-window](full-window.md).

## 3. Hành vi & luồng người dùng

1. **Lướt chuột qua:** chuột đi ngang island để tới tab trình duyệt. Island giữ compact vì chuột chưa dừng đủ `hover_delay_ms` (250 ms). Click đi xuyên xuống tab bên dưới.
2. **Hover để xem:** chuột dừng trên pill ≥ 250 ms thì morph sang peek. Chuột rời vùng peek ≥ `collapse_delay_ms` (400 ms) thì thu về compact, trừ khi đang mở một form soạn thảo.
3. **Click:** click pill thì mở peek và ghim lại, peek không tự thu khi rời chuột. Click ra ngoài, nhấn `Esc` hoặc gọi lại phím tắt thì thu về.
4. **Mở cửa sổ đầy đủ:** double-click pill hoặc bấm nút ⤢ trong peek mở `main` ở đúng trang của activity đang hiện.
5. **Xem phim toàn màn hình:** còn tác vụ thì mặc định giữ minimal; chỉ ẩn hẳn khi cho phép `hide_while_active` hoặc không còn tác vụ. Có Critical (nhắc việc) thì xử lý theo cài đặt `allow_alert_over_fullscreen`.
6. **Cửa sổ phóng to và island rảnh:** island thu thành vạch mảnh. Hover vào vạch thì bung lại compact.
7. **Nhập chữ (ghi chú mới/sửa, thêm ô tick, nhắc việc):** island xin focus; chừng nào editor còn mở thì rời chuột không thu gọn, kể cả ô trống, đã blur hoặc tạm disabled khi lưu. Kiểm tra editor tại thời điểm hết delay để bảo vệ cả form vừa mở khi timer rời chuột đã chạy. Khi lưu/hủy đóng editor, quy tắc hover bình thường áp dụng lại. Input/textarea/contenteditable được nhận diện chung, không cần từng module đăng ký riêng; checkbox, range, nút và trường readonly/hidden không giữ mở.

## 4. Activity

Module này không khai báo activity. Nó render những gì hub gửi qua `island://state`.

**Máy trạng thái hiển thị (frontend, hàm thuần `islandMachine(state, event)`):**

| Từ | Sự kiện | Đến |
|---|---|---|
| compact | `pointerDwell(≥ hover_delay_ms)` | peek |
| compact | `click` | peek (pinned) |
| compact | `doubleClick` | gửi `island_open_main`; giữ compact |
| peek | `pointerLeave(≥ collapse_delay_ms)`, không pinned và không mở editor | compact |
| peek (pinned) | `clickOutside` / `escape` / `hotkeyToggle` | compact |
| compact, peek | `alertShown` | alert (nhớ trạng thái trước đó) |
| alert | `alertEnded` | trạng thái trước đó |
| bất kỳ | `visibility(hidden)` | hidden (giữ trạng thái để khôi phục) |
| compact, hidden | `visibility(minimal)` từ backend (kể cả có Live) | minimal |
| minimal | `pointerEnter` / `visibility(normal)` | compact |

Alert lớn (Critical) chặn việc thu về compact cho tới khi người dùng xác nhận.

**Peek khi rảnh: launcher.**
- Không có activity nào thì peek là **launcher**: hàng ngày giờ + pin, bên dưới là lưới nút tạo nhanh (Hẹn giờ, Bấm giờ, Pomodoro, Nhắc việc, Ghi chú, Online, Mở PCisland).
- Nút do từng module đăng ký vào **quick action registry** của frontend (`src/lib/registry/quickActions.ts`), kèm thứ tự và điều kiện hiện. Module bị tắt thì nút ẩn.
- Bấm một nút sẽ mở panel do module cung cấp (`features/<module>/views/QuickPanel.svelte`) ngay trong peek. Ví dụ: chip preset của hẹn giờ.
- Khi đang có activity, peek của activity có nút lưới nhỏ ở góc để chuyển sang launcher.

## 5. Thiết kế kỹ thuật

**Thành phần (Rust):**

| Thành phần | Vị trí | Vai trò |
|---|---|---|
| `IslandWindow` | `modules/island/window.rs` | Tạo cửa sổ, áp style mở rộng, đặt vị trí, đặt lại topmost |
| `CursorPoller` | `modules/island/poller.rs` | Đọc vị trí chuột theo nhịp thích ứng, bật/tắt `ignore_cursor_events` |
| `HitRegions` | `modules/island/hit.rs` | Lưu các hình chữ nhật tương tác (px vật lý) |
| `VisibilityPolicy` | `modules/island/policy.rs` | **Hàm thuần:** đầu vào → `hidden \| minimal \| normal` |
| `ForegroundWatcher` | `platform/foreground.rs` (trait) | `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, thông tin cửa sổ foreground |
| `FullscreenDetector` | `platform/fullscreen.rs` (trait) | `SHQueryUserNotificationState` + so khung cửa sổ với màn hình |
| `MonitorInfo` | `platform/monitor.rs` (trait) | Danh sách màn hình, work area, DPI |

**Tạo cửa sổ:**
- Tauri `WebviewWindowBuilder` với các thuộc tính: `transparent(true)`, `decorations(false)`, `shadow(false)`, `resizable(false)`, `skip_taskbar(true)`, `always_on_top(true)`, `visible(false)`, kích thước 480×264 logic.
- Sau khi tạo, platform đặt thêm `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` qua `SetWindowLongPtrW` rồi `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE…)`.

**Đặt vị trí:**

```
x = monitor.left + (monitor.width − window_width) / 2
y = monitor.work_area.top   // equals monitor.top unless the taskbar is docked at the top; offset_y is applied inside the window via CSS
```

- Màn hình đích chọn theo `island.monitor`: `"primary"` | `"cursor"` | `{ "id": <device name> }`.
- Với `"cursor"`, island chuyển màn hình khi chuột *dừng* ở màn hình khác ≥ 1 s. Không chuyển khi chuột chỉ lướt qua.
- Đặt lại vị trí khi nhận `AppEvent::DisplayChanged` hoặc khi DPI đổi.

**Hit region & cursor poller:**
- Frontend gửi `island_set_hit_regions(rects)` (px logic, gốc tọa độ là cửa sổ). Backend nhân scale factor để ra px vật lý.
- Nhịp poller:
  - 16 ms khi chuột trong khung cửa sổ mở rộng thêm 48 px.
  - 100 ms khi ở xa.
  - Dừng khi `hidden` do khóa máy.
- Chuột vào hoặc ra khỏi region thì gọi `set_ignore_cursor_events(false/true)` và phát `island://pointer { inside }`.
- Khi đang kéo thả file (nút trái đang giữ và chuột vào region), poller vẫn tắt ignore để cửa sổ nhận `DragEnter` ([file-shelf](file-shelf.md)).

**Visibility policy:**

| Đầu vào | Nguồn |
|---|---|
| `fullscreen_on_island_monitor` | FullscreenDetector (QUNS_BUSY / QUNS_RUNNING_D3D_FULL_SCREEN / QUNS_PRESENTATION_MODE, hoặc khung foreground == khung màn hình) |
| `maximized_on_island_monitor` | `IsZoomed(foreground)` + cùng màn hình |
| `top_activity_priority` | ActivityHub |
| `user_hidden` | Phím tắt / tray |
| `session_locked` | `AppEvent::SessionLocked` |
| Cài đặt | `hide_in_fullscreen`, `minimal_when_maximized`, `idle_display`, `allow_alert_over_fullscreen` |

**Luật, theo thứ tự ưu tiên:**
1. `session_locked` → `hidden`. `user_hidden` → `minimal` nếu còn activity không phải Idle và `hide_while_active = false`; nếu không thì `hidden`.
2. Fullscreen và `hide_in_fullscreen`:
   - Có Critical và `allow_alert_over_fullscreen ∈ {critical, all}` → `normal` (chỉ hiện alert).
   - Ngược lại, còn activity không phải Idle hoặc alert và `hide_while_active = false` → `minimal`; nếu không thì `hidden`.
3. `idle_display == "hidden"` và không có activity ≥ Ambient → `hidden` (vẫn giữ vùng hover vô hình ở mép trên để gọi ra).
4. Cửa sổ phóng to, `minimal_when_maximized` và activity cao nhất < Live → `minimal`.
5. Còn lại → `normal`.

`island.hide_while_active` mặc định `false` (ADR-014). Công tắc **Cho phép ẩn hẳn khi có tác vụ** trong Cài đặt cho phép hành vi ẩn cũ. Frontend tuân theo minimal từ backend kể cả khi có Live activity; sau khi hover/peek hoặc alert kết thúc phải trở lại minimal.

Ở minimal, vạch 96×4 hiển thị tiến trình của media/countdown/Pomodoro chính nếu có thời lượng. Media compact cũng có thanh tiến trình giữa ảnh bìa và sóng nhạc. Stream không biết thời lượng không hiển thị phần trăm giả.

Peek có hàng điều hướng cao 28 px với nút **Tính năng** khi có activity: mở launcher/panel ngay trên island, giữ tác vụ và nhạc chạy; **Quay lại tác vụ** trở về activity chính. Chiều cao peek tăng thêm 28 px, vẫn nằm trong cửa sổ 480×264. Launcher không còn chỉ dành cho trạng thái rảnh.

Các loại cửa sổ không bao giờ tính là fullscreen/phóng to: desktop (`Progman`, `WorkerW`), taskbar (`Shell_TrayWnd`), chính các cửa sổ của PCisland.

**Topmost:** đặt lại `HWND_TOPMOST` (không activate) mỗi khi foreground đổi và mỗi 5 s khi `normal` (rẻ, chống app khác chiếm topmost). Nhận thông điệp `TaskbarCreated` (Explorer khởi động lại) thì tạo lại style và vị trí.

**Focus khi nhập chữ:**
- `island_request_focus()`: bỏ `WS_EX_NOACTIVATE`, `SetForegroundWindow` (kèm kỹ thuật `AllowSetForegroundWindow` nếu cần), ghi nhớ HWND foreground trước đó.
- `island_release_focus()`: đặt lại `NOACTIVATE`, trả foreground về HWND cũ nếu nó vẫn còn.

## 6. IPC

| Command | Chữ ký | Ghi chú |
|---|---|---|
| `island_set_hit_regions` | `(rects: Rect[]) → ()` | Tối đa 4 rect |
| `island_request_focus` | `() → ()` | |
| `island_release_focus` | `() → ()` | |
| `island_toggle_hidden` | `() → VisibilityMode` | Cũng gọi từ tray và phím tắt |
| `island_open_main` | `(route?: string) → ()` | Mở/kích hoạt `main` tại route |
| `island_get_state` | `() → IslandSnapshot` | Visibility, monitor, scale |

| Event | Payload |
|---|---|
| `island://visibility` | `{ mode: "hidden" \| "minimal" \| "normal", reason, rev }` |
| `island://pointer` | `{ inside: boolean }` |
| `island://hotkey` | `{ action: "togglePeek" \| "toggleHidden" }` |

## 7. Dữ liệu & cài đặt

Không có bảng DB.

| Khóa | Mặc định | Ghi chú |
|---|---|---|
| `island.monitor` | `"primary"` | `"primary"` · `"cursor"` · `{ id }` |
| `island.style` | `"attached"` | `"attached"` (dính chặt mép trên, kiểu notch) · `"floating"` (nổi, kiểu Dynamic Island) |
| `island.offset_y` | `8` | 0–24; chỉ có tác dụng khi `style = "floating"` |
| `island.idle_display` | `"eyes"` | `"eyes"` · `"clock"` · `"pill"` · `"hidden"` |
| `island.hover_delay_ms` | `250` | 100–800 |
| `island.collapse_delay_ms` | `400` | 150–1500 |
| `island.hide_in_fullscreen` | `true` | |
| `island.minimal_when_maximized` | `true` | |
| `island.allow_alert_over_fullscreen` | `"critical"` | `"never"` · `"critical"` · `"all"` |
| `island.reduce_motion` | `"system"` | `"system"` · `"on"` · `"off"` |

## 8. Phụ thuộc

- **Core:** ActivityHub (đọc ưu tiên cao nhất), EventBus, settings.
- **Platform trait:** `ForegroundWatcher`, `FullscreenDetector`, `MonitorInfo`, `WindowStyler`, `CursorPosition`.
- **Crate:** `tauri`, `windows` (`Win32_UI_WindowsAndMessaging`, `Win32_UI_Shell`, `Win32_Graphics_Gdi`, `Win32_UI_Accessibility`).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Explorer khởi động lại | Bắt `TaskbarCreated` → áp lại style, vị trí, topmost; tray tự tạo lại |
| Đổi độ phân giải / cắm rút màn hình | `DisplayChanged` → chọn lại màn hình, đặt lại vị trí; màn hình đích mất thì về `primary` |
| Kéo cửa sổ qua màn hình khác DPI | Tauri phát scale changed → gửi lại hit region, đặt lại vị trí |
| App khác cũng topmost (Task Manager "Always on top") | Chấp nhận chồng lấn; đặt lại topmost khi foreground đổi |
| Taskbar đặt ở cạnh trên | Đặt island dưới work area (`y = work_area.top`) |
| RDP / màn hình tắt | Khóa phiên → `hidden`, dừng poller |
| Virtual desktop (Win+Tab) | **Yêu cầu bắt buộc** (Q2): island phải hiện trên mọi virtual desktop. Kiểm chứng ở P0.4; nếu tool window không tự theo mọi desktop thì phải tìm cách ghim trước khi qua G0 (rủi ro R10) |
| Win+D (Show desktop) | Kiểm chứng ở P0.4; island phải vẫn hiện |
| Click island khi app khác đang ở chế độ gõ IME tiếng Việt (Unikey) | Không cướp focus nên không ảnh hưởng; kiểm tra thủ công |
| WebView2 crash (process failed) | Tạo lại webview island, log `error`, giữ trạng thái backend |

## 10. Hiệu năng

- Poller: < 0,1% CPU. Không gọi Win32 nếu trạng thái không đổi.
- Không có animation vô hạn khi rảnh. Ngoại lệ duy nhất là chớp mắt thưa (170 ms mỗi 3–7 s), hẹn bằng `setTimeout`. DOM island < 200 node.
- Gửi hit region tối đa 2 lần mỗi lần morph.

## 11. Kiểm thử

**Unit:**
- `VisibilityPolicy`: bảng tất cả tổ hợp đầu vào chính (≥ 20 case).
- `islandMachine`: mọi chuyển trạng thái trong bảng §4, gồm cả trường hợp alert chen ngang peek pinned.
- Chuyển đổi tọa độ logic ↔ vật lý ở 100/125/150/175%.

**Spike A (P0.3), kết quả cần ghi lại:**
- [ ] Morph 60 fps (DevTools Performance), ảnh chụp trace
- [ ] RAM/CPU khi rảnh
- [ ] 5 phút thao tác app bên dưới không mất click nào
- [ ] Click island không làm mất focus của Notepad đang gõ

**Checklist thủ công (trước mỗi bản phát hành):**
- [ ] Lướt chuột qua island tới tab Chrome: không bung, click trúng tab
- [ ] YouTube fullscreen, game borderless, trình chiếu PowerPoint: island ẩn
- [ ] Nhắc việc đến giờ khi đang fullscreen: đúng theo `allow_alert_over_fullscreen`
- [ ] Cửa sổ phóng to: vạch mảnh; hover vào thì bung
- [ ] Khởi động lại Explorer: island vẫn đúng chỗ, vẫn topmost
- [ ] Đa màn hình + DPI khác nhau; cắm/rút màn hình
- [ ] Virtual desktop; Win+D
- [ ] Ghi chú nhanh: gõ được, `Esc` trả focus về app cũ

## 12. Câu hỏi mở

Không có. Đã chốt: Q1 (đôi mắt khi rảnh), Q2 (luôn hiện, ở màn hình chính, trên mọi virtual desktop), Q3 (dính mép trên).
- Có cần hiệu ứng "gooey" (pill và bong bóng phụ dính nhau khi tách) không? Đề xuất: thử trong P3 nếu còn ngân sách hiệu năng.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Tùy chọn màn hình "theo chuột" (dừng 1 s mới chuyển); phát sự kiện ánh nhìn cho đôi mắt; phát `IslandHidden`/`SessionLocked` lên EventBus; giảm tiến trình renderer |
| 2026-11-01 | Thêm hide_while_active mặc định false (ADR-014), progress minimal/compact và launcher từ mọi activity; trở lại minimal sau hover/alert. |
| 2026-11-01 | Launcher cao 220 + hàng điều hướng 28 để xem/tick ghi chú; nút Pomodoro phản ánh trạng thái, toast nằm ngoài nội dung activity để giữ khi activity đổi. |
| 2026-11-01 | Giữ peek khi rời chuột trong mọi form soạn thảo; test tích hợp ghi chú mới/sửa/thêm ô tick và nhắc việc, gồm ô trống, blur, timeout đang chờ và thu gọn lại sau hủy. |
