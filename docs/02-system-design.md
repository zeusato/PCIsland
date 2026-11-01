# 02 · System design

> Cập nhật: 2026-10-31 · Trạng thái: bản thiết kế đầu, chốt sau Gate G0. Mọi thay đổi lớn phải đi kèm ADR trong [06-decision-log](06-decision-log.md).

**Bổ sung 0.1.2 (2026-11-01):** [ADR-018](06-decision-log.md) thêm video; ADR-019 thêm maintenance; ADR-020 thêm note desktop/meeting và nâng kệ file. `note-desktop` là WebView local tạo theo yêu cầu, capability riêng; meeting dùng worker WASAPI/Opus và journal, metadata trong kv, file trong data/meetings. Protocol `meetingaudio` phục vụ file theo ID đã lưu. `shared/archive` phục vụ ZIP cho shelf/meeting, Win32 clipboard/publish ở platform. Groq gọi từ Rust với key build-time theo lựa chọn chủ dự án; nguồn này chỉ gửi clip khi người dùng yêu cầu. Xem module docs để có trạng thái triển khai hiện tại; các sơ đồ ban đầu bên dưới không liệt kê đầy đủ phần mở rộng.

## Mục lục

1. [Bối cảnh](#1-bối-cảnh)
2. [Tiến trình & cửa sổ](#2-tiến-trình--cửa-sổ)
3. [Kiến trúc backend](#3-kiến-trúc-backend)
4. [Hệ thống module](#4-hệ-thống-module)
5. [Hệ thống Activity](#5-hệ-thống-activity)
6. [Island: trạng thái & cơ chế cửa sổ](#6-island-trạng-thái--cơ-chế-cửa-sổ)
7. [Kiến trúc frontend](#7-kiến-trúc-frontend)
8. [Giao tiếp IPC](#8-giao-tiếp-ipc)
9. [Luồng & đồng thời](#9-luồng--đồng-thời)
10. [Dữ liệu & lưu trữ](#10-dữ-liệu--lưu-trữ)
11. [Kiến trúc media](#11-kiến-trúc-media)
12. [Bảo mật](#12-bảo-mật)
13. [Hiệu năng](#13-hiệu-năng)
14. [Lỗi, log & chẩn đoán](#14-lỗi-log--chẩn-đoán)
15. [Build, đóng gói & phát hành](#15-build-đóng-gói--phát-hành)
16. [Chiến lược kiểm thử](#16-chiến-lược-kiểm-thử)
17. [Bản đồ module](#17-bản-đồ-module)

---

## 1. Bối cảnh

```
              Người dùng: hover · click · phím tắt · kéo thả file
                                   │
┌──────────────────────────────────▼──────────────────────────────────┐
│ Windows                                                              │
│  Win32 / Shell : z-order, foreground, fullscreen, power, display     │
│  Core Audio    : âm lượng endpoint, WASAPI loopback                  │
│  WinRT         : GSMTC (đọc media app khác), SMTC (phím media)       │
│  App khác      : Spotify, Chrome, Edge… ── xuất phiên media ──▶ GSMTC│
│                                                                      │
│   ┌────────────────────── PCisland.exe ───────────────────────┐      │
│   │  Rust core  ◀── IPC ──▶  WebView2: island · main ·        │      │
│   │                          web-player (toolbar + site)      │      │
│   └───────────────────────────────────────────────────────────┘      │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │ HTTPS (từng kết nối đều tắt được)
     LRCLIB (lời bài hát) · GitHub Releases (bản cập nhật)
     · trang nhạc trong web player · Open-Meteo (P4)
```

## 2. Tiến trình & cửa sổ

Một tiến trình Rust (`PCisland.exe`) chứa toàn bộ logic. WebView2 sinh thêm các tiến trình con (browser, GPU, renderer…) cho phần giao diện.

| Label | Mục đích | Vòng đời | Thuộc tính chính | Capability |
|---|---|---|---|---|
| `island` | Island | Suốt vòng đời app | Trong suốt, không viền, không bóng, topmost, không hiện trên taskbar, `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW`, kích thước cố định (xem [04 §5](04-design-system.md#5-hình-học-island)) | `island` |
| `main` | Cửa sổ đầy đủ | Tạo khi mở, hủy khi đóng | Không viền (title bar tự vẽ), có trên taskbar, mặc định 1120×720, tối thiểu 960×640 | `main` |
| `web-player` | Trình phát web | Tạo khi dùng, ẩn khi thu gọn, hủy khi tắt hẳn hoặc rảnh lâu | 2 webview: `web-player-toolbar` (local) + `web-player-site` (remote) | `web-player-toolbar`, `web-player-remote` |

**Môi trường WebView2:**
- `island`, `main` và `web-player-toolbar` dùng chung thư mục dữ liệu `WebView2/app`, nên chung một browser process.
- `web-player-site` dùng thư mục riêng `WebView2/web-player` để tách cookie và cho xóa dữ liệu dễ dàng.
- Cái giá của việc tách: thêm một nhóm tiến trình WebView2 khi web player đang mở.

## 3. Kiến trúc backend

```
┌─────────────────────────────── app/ ───────────────────────────────┐
│ khởi động · tắt · tray · autostart · phím tắt · single instance ·  │
│ kiểm tra cập nhật                                                   │
├─────────────────────────────── ipc/ ───────────────────────────────┤
│ đăng ký command · ánh xạ lỗi → IpcError · hằng số tên event          │
├────────────────────────────── modules/ ────────────────────────────┤
│ island · media · local_player · web_player · lyrics · visualizer    │
│ timer · reminders · indicators · notes · shelf                       │
├─────────────────────────────── core/ ──────────────────────────────┤
│ module registry & supervisor · ActivityHub · EventBus · settings ·  │
│ storage · audio engine · artwork cache · clock · paths · error       │
├────────────────────────────── shared/ ─────────────────────────────┤
│ thư viện thuần, không phụ thuộc app: nlp_vi · lrc · fuzzy            │
├────────────────────────────── platform/ ───────────────────────────┤
│ window · foreground · fullscreen · monitor · gsmtc · smtc ·          │
│ audio_endpoint · loopback · power · privacy · shell                  │
│ (nơi duy nhất được dùng unsafe / Win32 / WinRT trực tiếp)            │
└────────────────────────────────────────────────────────────────────┘
```

**Luật phụ thuộc:**
1. Lớp trên chỉ phụ thuộc lớp dưới. `core` không biết `modules`; `platform` không biết `core`.
2. Module chỉ dùng `core`, `shared` và **trait** của `platform`, không dùng struct Windows cụ thể. Nhờ vậy module test được bằng bản giả.
3. Module không import module khác, trừ một ngoại lệ đã khai báo: `local_player` và `web_player` cài đặt trait `MediaSource` của module `media` ([§11](#11-kiến-trúc-media)).
4. Giao tiếp chéo module chỉ qua EventBus (sự kiện) hoặc ActivityHub (hiển thị).

**EventBus:** dùng `tokio::sync::broadcast` với enum `AppEvent` trong `core/events.rs`. Enum này chỉ chứa sự kiện **dùng chéo module**. Sự kiện nội bộ của một module không đưa vào đây.

```rust
pub enum AppEvent {
    ForegroundChanged(ForegroundInfo),
    VisibilityChanged(VisibilityMode),          // hidden | minimal | normal
    DisplayChanged,
    PowerChanged(PowerState),                   // AC/DC, % pin
    Suspending, Resumed,                        // sleep / wake
    SessionLocked, SessionUnlocked,
    SystemTimeChanged,                          // đổi giờ / múi giờ
    SettingsChanged { section: SettingsSection },
    NowPlayingChanged(Option<NowPlaying>),
    FocusModeChanged { active: bool },
    MediaIntent(MediaIntent),                   // Pause | PlayLocalPlaylist(id): request from another module (e.g. timer)
}
```

## 4. Hệ thống module

Mỗi tính năng là một **module** cài đặt trait `Module`. Registry khởi động các module được bật trong cài đặt và giám sát chúng.

```rust
pub trait Module: Send + Sync + 'static {
    fn id(&self) -> ModuleId;
    fn start(&self, ctx: ModuleContext) -> BoxFuture<'_, Result<(), ModuleError>>;
    fn stop(&self) -> BoxFuture<'_, Result<(), ModuleError>>;
    /// Called by ActivityHub when the user triggers an action on one of this module's activities.
    fn handle_action(&self, activity: &ActivityId, action: &ActionId) -> BoxFuture<'_, Result<(), ModuleError>>;
}

pub struct ModuleContext {
    pub app: tauri::AppHandle,
    pub hub: ActivityHubHandle,
    pub bus: EventBus,
    pub settings: SettingsHandle,   // scoped to this module's section
    pub storage: StorageHandle,
    pub audio: AudioEngineHandle,
    pub artwork: ArtworkCacheHandle,
    pub clock: Arc<dyn Clock>,
    pub shutdown: CancellationToken,
}
```

**Mẫu actor:** bên trong mỗi module là một task service sở hữu toàn bộ state. Bên ngoài chỉ giữ `Handle`: một struct clone được, bọc `mpsc::Sender<Command>`. Tauri command lấy handle qua `State<'_, TimerHandle>` rồi gửi lệnh và chờ phản hồi qua `oneshot`. Cách này giúp không phải dùng lock chung và không có race.

**Vòng đời & cô lập lỗi:**

```
Registered → Starting → Running → Stopping → Stopped
                 │          │
                 └──────────┴─▶ Failed → Restarting (tối đa 3 lần: 1s, 5s, 30s) → Disabled
```

- Mọi task của module được spawn qua `ModuleSupervisor`. Panic trong task được bắt qua `JoinError::is_panic()`. Vì vậy profile build phải giữ `panic = "unwind"`.
- Module bị `Disabled` sẽ bị ActivityHub gỡ hết activity. Trang Cài đặt hiện lý do kèm nút "Thử lại".
- Bật/tắt module lúc đang chạy: `settings.modules.<id>.enabled` đổi thì registry gọi `start`/`stop` tương ứng.
- Thời gian `start` tối đa 3 giây. Module chậm hơn bị đánh dấu Failed để không chặn khởi động.

## 5. Hệ thống Activity

Đặc tả đầy đủ ở [activity-hub](modules/activity-hub.md). Đây là trục trung tâm: **module không tự vẽ lên island, chỉ khai báo activity.**

```rust
pub struct Activity {
    pub id: ActivityId,              // "timer:01J…", "media:now-playing"
    pub source: ModuleId,
    pub kind: ActivityKind,          // quyết định frontend dùng view nào
    pub priority: Priority,
    pub lifetime: Lifetime,          // Persistent | Transient { ttl_ms }
    pub payload: ActivityPayload,    // enum có kiểu, sinh TS bằng ts-rs
    pub actions: Vec<ActivityAction>,// hành động nhanh hiện trên peek/alert
    pub placement: Placement,        // Auto | SecondaryOnly (chỉ báo quyền riêng tư)
    pub started_at: DateTime<Utc>,
    pub rev: u64,
}
```

| Mức ưu tiên | Giá trị | Ví dụ | Hành vi |
|---|---|---|---|
| Critical | 100 | Nhắc việc đến giờ, hẹn giờ kết thúc | Tự bung thành alert lớn, glow nhịp, âm báo; ở lại tới khi người dùng xác nhận |
| Alert | 80 | Đổi âm lượng, cắm/rút sạc, đổi bài | Tự bung alert nhỏ trong `ttl_ms`, rồi trả lại trạng thái cũ |
| Live | 60 | Timer, stopwatch, Pomodoro đang chạy | Ứng viên hoạt động chính |
| Media | 50 | Đang phát nhạc | Ứng viên hoạt động chính |
| Ambient | 30 | Nhạc tạm dừng (≤ 30 phút), kệ file có đồ, ghi chú ghim | Chỉ hiện khi không có gì cao hơn |
| Idle | 0 | Đôi mắt / đồng hồ / pill trống | Mặc định |

**Arbitration (tóm tắt):**
- **Chính** = activity Persistent có ưu tiên cao nhất. Hòa thì xét theo thứ tự: activity người dùng ghim → `started_at` mới hơn.
- **Phụ** = activity cao kế tiếp *từ module khác*, hiện thành bong bóng bên cạnh.
- **Alert:** hàng đợi riêng, hiện mỗi lúc một cái, gộp theo `id`.
- **Focus mode:** chặn Alert có ưu tiên dưới Critical, trừ của `timer`.

**Hành động nhanh:** frontend gọi `activity_invoke(activity_id, action_id)`. Hub chuyển tới `Module::handle_action` của module sở hữu activity đó. Nhờ vậy island không cần biết command riêng của từng module cho các thao tác cơ bản.

## 6. Island: trạng thái & cơ chế cửa sổ

Đặc tả đầy đủ ở [island-window](modules/island-window.md).

**Hai tầng trạng thái:**

| Tầng | Chủ sở hữu | Giá trị | Quyết định bởi |
|---|---|---|---|
| Visibility mode | Backend (`VisibilityPolicy`, hàm thuần) | `hidden` · `minimal` · `normal` | Fullscreen, cửa sổ phóng to, khóa máy, người dùng ẩn, có activity hay không |
| Presentation state | Frontend (máy trạng thái thuần `islandMachine`) | `compact` · `peek` · `alert` | Hover, click, alert từ hub |

**Cơ chế click xuyên:**
1. Cửa sổ `island` có kích thước **cố định**, đủ chứa trạng thái lớn nhất cộng lề glow. Không đổi kích thước cửa sổ theo từng frame, vì cách đó gây giật.
2. Frontend đo các vùng tương tác (pill, bong bóng phụ) và gửi lên qua `island_set_hit_regions`. Việc gửi diễn ra khi bắt đầu animation (gửi hợp của vùng trước và vùng sau) và khi animation dừng.
3. Backend chạy **cursor poller** (`GetCursorPos`): 60 Hz khi chuột ở gần cửa sổ, 10 Hz khi ở xa. Chuột vào hit region thì `set_ignore_cursor_events(false)`; ra ngoài thì đặt lại `true`. Poller chỉ gọi API khi trạng thái thay đổi.
4. Không dùng global mouse hook (`WH_MOUSE_LL`) vì dễ bị antivirus nghi ngờ ([ADR-003](06-decision-log.md#adr-003--cơ-chế-cửa-sổ-island)).

**Focus:** island không bao giờ nhận focus (`WS_EX_NOACTIVATE`), trừ khi cần nhập chữ (ghi chú nhanh, thêm nhắc việc). Khi đó frontend gọi `island_request_focus`: backend tạm bỏ `NOACTIVATE`, kích hoạt cửa sổ, và trả lại như cũ khi input mất focus hoặc người dùng nhấn `Esc`.

## 7. Kiến trúc frontend

- **Vite multi-page:** 3 điểm vào `island.html`, `main.html`, `web-player-toolbar.html`. Mỗi cửa sổ nạp đúng phần của mình.
- **Theo tính năng:** mỗi module backend có thư mục tương ứng `src/features/<module>/`:
  - `views/Compact.svelte`, `views/Peek.svelte`, `views/Alert.svelte`, `views/Bubble.svelte`: các view trên island.
  - `views/QuickPanel.svelte` (nếu có): panel tạo nhanh, mở từ launcher của island.
  - `views/Page.svelte`, `views/Settings.svelte`: trang trong cửa sổ đầy đủ.
  - `store.svelte.ts`: bản phản chiếu state backend, viết bằng rune Svelte 5.
  - `index.ts`: đăng ký view vào **view registry** theo `ActivityKind`, và đăng ký nút tạo nhanh vào **quick action registry** (launcher của island, xem [island-window §4](modules/island-window.md#4-activity)).
- **Island root** nhận `island://state` (chính, phụ, alert) và tra view registry để render. Thêm module mới không cần sửa island root.
- **Đồng bộ state (snapshot + subscribe):** store đăng ký event *trước*, rồi gọi `<module>_get_state` để lấy snapshot. Event nào có `rev` ≤ `rev` của snapshot thì bỏ qua. Cách này tránh mất cập nhật khi mở cửa sổ.
- **Chuyển động:** `src/lib/motion/` chứa preset spring (Svelte `Spring`) và các transition dùng chung. Component không tự đặt thông số spring.
- **i18n:** `src/lib/i18n/{vi,en}.json` với key phân cấp `module.area.key` và hàm `t()`. Mã lỗi IPC được ánh xạ sang `errors.<code>`.

## 8. Giao tiếp IPC

| Loại | Quy ước | Ví dụ |
|---|---|---|
| Command | `<module>_<verb>[_<noun>]`, snake_case, trả `Result<T, IpcError>` | `timer_start`, `reminders_snooze`, `media_seek` |
| Event | `<module>://<kebab-case>` | `island://state`, `media://now-playing`, `settings://changed` |
| Luồng tần số cao | `tauri::ipc::Channel<T>` | khung visualizer, tiến độ quét thư viện |

- **DTO:** `#[derive(Serialize, Deserialize, TS)]` với `#[serde(rename_all = "camelCase")]`. Enum dùng `#[serde(tag = "kind")]` để ra discriminated union bên TS. Type được ts-rs sinh vào `src/lib/ipc/bindings/`; CI kiểm tra bindings không lệch so với Rust.
- **Lỗi:** `IpcError { code: String, message: String }`.
  - `code` ổn định, dạng `module.reason` (ví dụ `timer.not_found`).
  - `message` viết tiếng Anh, dùng cho log.
  - Frontend hiển thị bằng `t("errors." + code)`.
- **Nguyên tắc thời gian:** backend gửi **trạng thái + mốc thời gian** (`endsAt`, `positionMs` + `positionAt` + `rate`), frontend tự nội suy. Backend **không** phát event mỗi giây.
- **Phạm vi phát:** dùng `emit_to` đúng cửa sổ cần (thường là `island` + `main`), không đánh thức web player.
- **Phiên bản state:** mọi DTO trạng thái có `rev: u64` tăng đơn điệu theo từng module ([§7](#7-kiến-trúc-frontend)).

## 9. Luồng & đồng thời

| Luồng / task | Thuộc | Ghi chú |
|---|---|---|
| Tokio runtime (Tauri async runtime) | Mọi service actor | Không gọi API chặn trên runtime; việc chặn đưa vào `spawn_blocking` hoặc luồng riêng |
| Luồng `win-events` | platform | Message loop: `SetWinEventHook` (foreground) và cửa sổ ẩn nhận `WM_POWERBROADCAST`, `WM_DISPLAYCHANGE`, `WM_WTSSESSION_CHANGE`, `WM_TIMECHANGE`, power setting notifications. Đổi thành `AppEvent` |
| Luồng `gsmtc` | platform → media | WinRT MTA, handler sự kiện GSMTC đẩy vào channel |
| Luồng `audio` | core::audio | Giữ `rodio::OutputStream` (không `Send`). Nhận lệnh qua channel. Dùng chung cho âm báo và local player |
| Luồng `loopback` | platform → visualizer | WASAPI event-driven; chỉ chạy khi visualizer đang hiện |
| Luồng `db` | core::storage | Một kết nối `rusqlite` (WAL), nhận job qua channel |
| Task cursor poller | island | `tokio::time::interval` thích ứng 16/100 ms |

**Luật:**
- COM được khởi tạo theo từng luồng trong `platform`; module không gọi `CoInitializeEx`.
- Không giữ guard của `std::sync::Mutex` qua `.await`. Ưu tiên channel thay vì lock.
- Mọi luồng và task lắng nghe `CancellationToken` để tắt sạch.

## 10. Dữ liệu & lưu trữ

Chi tiết ở [settings-storage](modules/settings-storage.md).

| Dữ liệu | Bản portable (có thư mục `data\` cạnh exe) | Bản cài đặt, hoặc `data\` không ghi được |
|---|---|---|
| `settings.json`, `pcisland.db`, `logs/`, `shelf/` | `<thư mục exe>\data\` | `%APPDATA%\PCisland\` |
| WebView2 của app | `%LOCALAPPDATA%\PCisland\WebView2\app\` | như bên trái |
| WebView2 của web player (cookie đăng nhập) | `%LOCALAPPDATA%\PCisland\WebView2\web-player\` | như bên trái |

- **Lý do tách WebView2:** gắn với máy để không vô tình gửi phiên đăng nhập khi nén thư mục chia sẻ. Ngoài ra cache WebView2 khá lớn ([ADR-005](06-decision-log.md#adr-005--lưu-trữ--portable)). Có lệnh `--cleanup` và nút trong Cài đặt để xóa hết phần ở `%LOCALAPPDATA%`.
- **Settings:** một file JSON, chia section theo module, có `schemaVersion`. Struct có kiểu với `#[serde(default)]`. Ghi nguyên tử (ghi file tạm rồi rename), debounce 500 ms, giữ `settings.json.bak` của lần đọc tốt gần nhất.
- **SQLite:** một file DB, WAL. Mỗi module sở hữu các bảng có tiền tố tên module (`reminders_items`, `timer_items`…). Migration là file SQL nhúng trong binary, đánh số `NNNN_<module>_<tên>.sql`.
- **Thời gian:** lưu UTC (epoch ms), đổi sang giờ địa phương ở UI. Thời lượng dùng đồng hồ monotonic.

## 11. Kiến trúc media

```
GSMTC (app khác) ──▶ ExternalSource ──┐
LocalPlayer ──────── (MediaSource) ───┼──▶ MediaHub ──▶ NowPlaying ──▶ Activity "media"
WebPlayer ────────── (MediaSource) ───┘       │              │       + media://now-playing
                                              │              └──▶ AppEvent::NowPlayingChanged
                                              │                    → lyrics, visualizer
                                              └──▶ SMTC của PCisland (khi nguồn là local/web):
                                                   phím media bàn phím + flyout của Windows
```

- `MediaSource` là trait do module `media` định nghĩa. Nó cung cấp state, các lệnh (play, pause, next, prev, seek) và capabilities.
- **Chọn nguồn hiện hành:**
  1. Nguồn vừa chuyển sang Playing gần nhất.
  2. Nếu không có nguồn nào đang phát: nguồn tạm dừng gần nhất trong 30 phút (mức Ambient).
  3. Người dùng có thể ghim một nguồn.
- **Lọc trùng:** phiên SMTC do chính PCisland đăng ký bị loại khỏi danh sách GSMTC.
- **Ảnh bìa:** mọi nguồn đổ về `ArtworkCache` (LRU trong bộ nhớ, 32 mục, mỗi ảnh ≤ 2 MB). Ảnh được phục vụ qua custom protocol `artwork` (`http://artwork.localhost/<hash>` trên Windows), dùng chung cho thumbnail GSMTC, ảnh nhúng trong file nhạc và ảnh từ web player.

## 12. Bảo mật

- **Capability theo cửa sổ:**
  - `island`, `main`: chỉ các command của app.
  - `web-player-toolbar`: chỉ command `web_player_*`.
  - `web-player-remote`: **duy nhất** command `web_player_report`, giới hạn theo domain trong danh sách trang hỗ trợ.
- **Không phơi filesystem cho frontend.** Không dùng plugin fs/shell ở frontend. Hộp thoại chọn file và mở thư mục đều đi qua command Rust.
- **Nội dung remote chỉ ở `web-player-site`.** Không bao giờ nạp URL ngoài vào `island` hoặc `main`.
- **CSP cho cửa sổ local:**
  ```
  default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
  img-src 'self' data: http://artwork.localhost; connect-src ipc: http://ipc.localhost
  ```
- **Mạng:** chỉ backend gọi mạng, qua `reqwest` (rustls). Danh sách cho phép: `lrclib.net`, `api.github.com` (P4 thêm `api.open-meteo.com`, `geocoding-api.open-meteo.com`). Mỗi kết nối có công tắc trong Cài đặt.
- **Web player:** chặn tải file, từ chối mặc định quyền thông báo/mic/camera/vị trí, không chặn quảng cáo, không giả user agent.
- **Không telemetry.** Log không chứa nội dung cá nhân như nội dung ghi chú, tiêu đề nhắc việc hay URL có token. Các thông tin này chỉ được ghi ở mức `trace` và chỉ khi bật `--debug`.
- **Bí mật:** v1.0 không lưu bí mật nào. Khi có OAuth (F41), token được mã hóa bằng DPAPI (`CryptProtectData`) và phải qua review bảo mật trước khi merge.

## 13. Hiệu năng

| Chỉ số | Ngân sách | Cách đo |
|---|---|---|
| Kích thước exe | < 20 MB | Kích thước file |
| RAM lõi khi rảnh (exe + WebView2 của app, chưa mở `main`) | < 150 MB | Process Explorer: tổng private bytes |
| RAM thêm khi mở `main` | < 80 MB | như trên |
| RAM thêm khi mở web player | ghi nhận theo từng trang (dự kiến 200–500 MB) | như trên |
| CPU khi rảnh (island đứng yên) | < 0,5% (máy 4 nhân) | Trung bình 60 giây |
| CPU khi đang phát nhạc, island compact | < 2% | như trên |
| Morph compact ⇄ peek | 60 fps, không frame > 32 ms | DevTools Performance |
| Từ lúc chạy tới lúc island hiện | < 1,5 s (khởi động ấm) | Mốc thời gian trong log |

**Kỹ thuật bắt buộc:**
- **Không vẽ khi đứng yên.** Không có animation vô hạn ở trạng thái rảnh. Ngoại lệ duy nhất là chớp mắt thưa của màn hình chờ, hẹn bằng `setTimeout`, không dùng vòng lặp frame ([04 §11](04-design-system.md#11-đôi-mắt-màn-hình-chờ)). Glow nhịp chỉ dùng cho Critical.
- **Nội suy thay vì tick** ([§8](#8-giao-tiếp-ipc)). Đồng hồ dùng 1 timer frontend, căn theo đầu mỗi giây.
- **Polling thích ứng** (cursor poller, fullscreen). Dừng hẳn khi khóa máy hoặc màn hình tắt.
- **Tạo khi cần:** cửa sổ `main` hủy khi đóng; web player bị hủy sau `web_player.unload_after_paused_min`; visualizer chỉ capture khi đang hiện.
- **Module tắt thì không tốn gì:** không task, không luồng, không handler.

## 14. Lỗi, log & chẩn đoán

- **Phân tầng lỗi:**
  1. Platform trả lỗi có kiểu.
  2. Module gói lại bằng enum `thiserror` của mình.
  3. Ranh giới IPC đổi thành `IpcError`.
  4. Lỗi người dùng cần biết thì đi lên thành Activity mức Alert, hoặc toast trong cửa sổ `main`.
- **Log:** `tracing` + `tracing-appender`, xoay file theo ngày `data/logs/pcisland.YYYY-MM-DD.log`, giữ 7 ngày. Mức mặc định `info`, `--debug` nâng lên `debug`. Log frontend được chuyển về backend qua `log_frontend` (giới hạn tần suất).
- **Crash:** panic hook ghi `data/logs/crash-<timestamp>.log`. Lần chạy sau, island hiện Alert "PCisland vừa gặp lỗi" kèm hành động "Mở thư mục log".
- **Gói chẩn đoán:** Cài đặt → Chẩn đoán → "Xuất gói chẩn đoán". Gói là một file zip gồm log 7 ngày, settings đã lọc trường nhạy cảm và thông tin hệ thống (phiên bản Windows, WebView2, DPI, số màn hình), lưu ra Desktop. Không tự gửi đi đâu.

## 15. Build, đóng gói & phát hành

- **Cargo profile release:** `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "unwind"` (bắt buộc, xem [§4](#4-hệ-thống-module)).
- **Sản phẩm:** `PCisland.exe` một file, asset frontend nhúng sẵn. Mỗi bản phát hành có 2 gói, mỗi gói kèm file `.sha256` ([ADR-009](06-decision-log.md#adr-009--phân-phối--ký-số)):
  - `PCisland-<ver>-setup.exe`: bộ cài NSIS (Tauri bundler), cài theo user vào `%LOCALAPPDATA%\Programs\PCisland`, không cần admin, có shortcut Start menu và trình gỡ cài đặt.
  - `PCisland-<ver>-portable.zip`: exe + thư mục `data\` rỗng + `README-portable.txt`.
- **Kênh phát hành:** GitHub Releases. Kiểm tra cập nhật (F45) gọi API `releases/latest` tối đa mỗi ngày một lần và chỉ thông báo, không tự tải.
- **Ký số:** theo [ADR-009](06-decision-log.md#adr-009--phân-phối--ký-số). Không mua chứng chỉ trả phí; v0.x không ký; trước v1.0 xin ký miễn phí qua SignPath Foundation nếu repo mã nguồn mở và đủ điều kiện.
- **CI:** mỗi PR chạy fmt, clippy, test, svelte-check, eslint, vitest, kiểm tra bindings, build. Tag `v*` thì build release, tạo zip và SHA256, đính kèm vào bản nháp release.

## 16. Chiến lược kiểm thử

| Tầng | Phạm vi | Bắt buộc |
|---|---|---|
| Unit (Rust) | Logic thuần: arbitration, visibility policy, scheduler, nlp_vi, parser LRC, migration settings, hàng đợi/shuffle | Có, với mọi logic mới |
| Integration (Rust) | Module chạy với platform giả (`FakeForeground`, `FakeMediaSessions`…) và `TestClock` | Có, với module có scheduler hoặc state phức tạp |
| Unit (frontend) | `islandMachine`, store, hàm format | Có |
| Thủ công | Checklist trong từng module doc + release checklist ([03 §9](03-coding-principles.md#9-quy-trình-phát-hành)) | Trước mỗi bản phát hành |
| Hiệu năng | Đo theo bảng §13, ghi vào `docs/reports/perf-<ver>.md` | Mỗi bản phát hành |

**Testability là ràng buộc thiết kế:**
- Mọi thứ phụ thuộc thời gian dùng trait `Clock`.
- Mọi thứ phụ thuộc Windows đi qua trait trong `platform`.
- Máy trạng thái frontend là hàm thuần `(state, event) → state`.

## 17. Bản đồ module

| Module | ID | Loại | Phase | Tài liệu |
|---|---|---|---|---|
| Island window | `island` | Module | P0–P3 | [island-window](modules/island-window.md) |
| Vòng đời app | `app` | Lõi | P1 | [app-lifecycle](modules/app-lifecycle.md) |
| Activity hub | `activity` | Lõi | P1 | [activity-hub](modules/activity-hub.md) |
| Settings & storage | `settings`, `storage` | Lõi | P1 | [settings-storage](modules/settings-storage.md) |
| Cửa sổ đầy đủ | `ui-main` | Frontend | P1–P3 | [full-window](modules/full-window.md) |
| Media hub | `media` | Module | P1 | [media-hub](modules/media-hub.md) |
| Local player | `local_player` | Module | P2–P3 | [local-player](modules/local-player.md) |
| Web player | `web_player` | Module | P2 | [web-player](modules/web-player.md) |
| Lời bài hát & visualizer | `lyrics`, `visualizer` | Module | P3 | [lyrics-visualizer](modules/lyrics-visualizer.md) |
| Timer | `timer` | Module | P1–P3 | [timer](modules/timer.md) |
| Nhắc việc | `reminders` | Module | P2–P3 | [reminders](modules/reminders.md) |
| Chỉ báo hệ thống | `indicators` | Module | P1–P4 | [system-indicators](modules/system-indicators.md) |
| Ghi chú nhanh | `notes` | Module | P3 | [quick-notes](modules/quick-notes.md) |
| Kệ file | `shelf` | Module | P3 | [file-shelf](modules/file-shelf.md) |
| Tương lai | — | Backlog | P4 | [future-modules](modules/future-modules.md) |
