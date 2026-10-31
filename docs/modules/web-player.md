# Module: Trình phát web (`web_player`)

> Trạng thái: Đang làm (code có, chưa test tay; xem Handoff §2) · Phase: P0 (spike D), P2 · Tính năng: F12 · Cập nhật: 2026-10-31

## 1. Mục đích

Cho người dùng mở trang nhạc online (Zing MP3, SoundCloud, YouTube…), đăng nhập và chọn playlist ở chế độ đầy đủ, rồi thu gọn để nhạc chạy ngầm và điều khiển từ island, giống app nhạc trên điện thoại. Đây là nhánh "nhúng" của [ADR-004](../06-decision-log.md#adr-004--nhạc-online-theo-mô-hình-lai). Trang nào không chạy được trong WebView2 thì dùng nhánh nguồn ngoài qua [media-hub](media-hub.md).

## 2. Phạm vi

**Trong phạm vi:**
- Cửa sổ `web-player` gồm hai webview: toolbar (local) và site (remote).
- Danh sách trang và adapter; bridge script; vòng đời ẩn/hủy.
- Hồ sơ cookie riêng, xóa dữ liệu; chính sách popup, tải file và quyền.

**Ngoài phạm vi:**
- Tách luồng âm thanh, tải nhạc, chặn quảng cáo, giả user agent: **cấm** theo ADR-004.
- Nhiều trang chạy song song: v1 chỉ một trang tại một thời điểm.

## 3. Hành vi & luồng người dùng

1. **Chọn trang:** island › nút "Online", hoặc `main › Online`. Lưới thẻ các trang hỗ trợ, cùng ô "Nhập URL khác".
2. **Thiết lập:** cửa sổ web player mở ra với toolbar mỏng `← → ⟳ ⌂ | [ổ khóa] music.youtube.com | Đổi trang ▾ | Thu gọn | ⋯`. Người dùng đăng nhập, chọn playlist, bấm phát.
3. **Thu gọn:** cửa sổ ẩn (không hủy), nhạc tiếp tục. Island hiện ảnh bìa + sóng nhạc; peek có ⏮ ⏯ ⏭, thanh tiến độ và nút "Mở trình phát".
4. **Mở lại:** bấm ảnh bìa hoặc "Mở trình phát" → cửa sổ hiện lại đúng chỗ cũ.
5. **Tắt hẳn:** menu ⋯ › "Tắt trình phát" → hủy webview, trả RAM. Lần sau mở lại trang cuối, cookie vẫn còn (không phải đăng nhập lại).
6. **Rảnh lâu:** tạm dừng quá `unload_after_paused_min` (30 phút) thì tự hủy. Island hiện Ambient "Tiếp tục nghe Zing MP3" trong 10 phút để mở lại nhanh.
7. **Đăng xuất & xóa dữ liệu:** menu ⋯ → xác nhận → xóa cookie và cache của hồ sơ web player.
8. **Trang không đăng nhập được (Google) hoặc không phát được (DRM):** hiện thẻ hướng dẫn "Trang này cần mở bằng trình duyệt/app. PCisland vẫn điều khiển được qua Media Session" kèm nút "Mở trong trình duyệt".

## 4. Activity

Module là `MediaSource` (`SourceId = "web"`); `media` tạo `media.now_playing` với `app_name` = tên trang, `app_icon` = favicon. Riêng:

| Kind | Ưu tiên | Compact | Peek | Hành động |
|---|---|---|---|---|
| `web_player.resume_hint` | Ambient (10 phút) | Favicon trang | "Tiếp tục nghe <trang>" | Mở lại · Bỏ qua |

## 5. Thiết kế kỹ thuật

**Cửa sổ & webview:**
- Window `web-player`: mặc định 1100×760, có trên taskbar khi đang hiện, không có khi ẩn.
- Webview `web-player-toolbar` cao 44 px nạp `web-player-toolbar.html` (local).
- Webview `web-player-site` chiếm phần còn lại, nạp URL trang, thư mục dữ liệu `WebView2\web-player`.
- Dùng API multiwebview của Tauri (`Window::add_child`, cờ `unstable`).
- **Phương án dự phòng** nếu multiwebview không ổn ở G0: cửa sổ chỉ chứa site webview với title bar mặc định của Windows; các nút điều hướng chuyển sang trang `main › Online` và island. Ghi kết quả vào ADR-004.

**Hồ sơ trang** (`sites.rs`, dữ liệu tĩnh + trang người dùng thêm):

```rust
pub struct SiteProfile {
    pub id: &'static str,                 // "youtube_music"
    pub name: &'static str,
    pub home_url: &'static str,
    pub allowed_domains: &'static [&'static str],   // used for remote IPC capability and popups
    pub adapter: AdapterKind,             // MediaSession | Custom(&'static str)
    pub known_limits: &'static [LimitKind], // GoogleLoginBlocked, NeedsDrm, …
}
```

Danh sách test ở G0 (đã chốt với chủ dự án ngày 2026-10-31): **Zing MP3, SoundCloud, YouTube, Spotify Web, NhacCuaTui**. Danh sách hỗ trợ chính thức là những trang qua test matrix.
- YouTube được test ở `www.youtube.com`. Nếu qua thì thêm `music.youtube.com` (cùng adapter, cùng rủi ro đăng nhập Google).
- NhacCuaTui được chọn làm trang thứ năm vì là trang nhạc Việt lớn thứ hai và dùng tài khoản riêng, không phụ thuộc Google. Nhờ vậy nó là đối chứng tốt cho rủi ro R2. URL do người dùng thêm dùng adapter `MediaSession` chung, và IPC bị giới hạn theo domain của URL đó.

**Bridge script** (`bridge/bridge.js`, nhúng bằng `include_str!`, chèn bằng `initialization_script` trước khi trang chạy):
1. **Móc Media Session:**
   - Bọc setter `MediaSession.prototype.metadata` để bắt `title`, `artist`, `album`, `artwork[]` và báo lên.
   - Bọc `setActionHandler` để lưu handler của `play`, `pause`, `nexttrack`, `previoustrack`, `seekto`, `seekforward`, `seekbackward`.
   - Bọc `playbackState` và `setPositionState`.
2. **Theo dõi media element:**
   - Lắng nghe ở pha capture trên `document` các sự kiện `play`, `pause`, `ended`, `durationchange`, `ratechange`, `seeked`, `timeupdate` (`timeupdate` chỉ báo khi lệch > 2 s so với nội suy).
   - Bọc `HTMLMediaElement.prototype.play` để bắt cả element không gắn vào DOM.
3. **Báo cáo:** `report({ kind: "state", ... })` gọi command duy nhất `web_player_report`. Tối đa 10 tin/giây, gộp tin trong 100 ms.
4. **Điều khiển:**
   - Rust gọi `webview.eval("window.__pcisland?.exec('nexttrack')")`.
   - `exec` ưu tiên handler đã bắt; không có thì dùng element (`play()`/`pause()`/`currentTime`); vẫn không được thì dùng adapter riêng (selector nút của trang).
5. **Quảng cáo:** adapter riêng có thể báo `isAd` (ví dụ YouTube: lớp `.ad-showing`). Bridge **không** bỏ qua hay ẩn quảng cáo.
6. Object `__pcisland` được đóng băng và không liệt kê được (non-enumerable). Trang vẫn có thể đọc nó; rủi ro chấp nhận được, vì trường hợp xấu nhất là trang tự giả metadata của chính nó.

**Bảo mật** (`capabilities/web-player-remote.json`):
- `webviews: ["web-player-site"]`.
- `remote.urls`: domain của các trang hỗ trợ (`https://music.youtube.com/*`, …).
- `permissions`: **chỉ** `allow-web-player-report`.
- `web_player_report` kiểm tra schema chặt (độ dài chuỗi ≤ 512, URL ảnh phải là `https`), từ chối nếu origin không thuộc trang đang mở.
- Ảnh bìa từ URL được backend tải về (giới hạn 2 MB, timeout 5 s) rồi đưa vào `ArtworkCache`.

**Chính sách điều hướng:**
- **Popup** (`window.open`, `target=_blank`): domain thuộc `allowed_domains` hoặc nhà cung cấp đăng nhập đã biết của trang (ví dụ `accounts.spotify.com`, `id.zalo.me`) thì mở cửa sổ popup con dùng cùng hồ sơ. Còn lại mở trình duyệt hệ thống. Cách xử lý new-window của Tauri 2 phải **kiểm chứng trong spike D**.
- **Tải file:** chặn.
- **Quyền** (notification, mic, camera, vị trí, clipboard-read): từ chối.
- **Autoplay:** thêm tham số trình duyệt `--autoplay-policy=no-user-gesture-required` cho môi trường web player, để lệnh play từ island chạy được khi cửa sổ đang ẩn.

**Vòng đời:**

```
Closed ──open(site)──▶ Visible ──thu gọn──▶ Hidden ──tạm dừng ≥ N phút──▶ Unloaded
   ▲                     ▲  │                 │                              │
   │                     │  └──── hiện lại ◀──┘                              │
   └──── tắt hẳn ────────┴───────────────────────────────── mở lại ──────────┘
```

- Khi `Hidden`: không gọi `TrySuspend`. Kiểm chứng ở spike D là âm thanh tiếp tục và throttling không làm hỏng việc chuyển bài.
- `Unloaded` giữ `{site_id, last_url}` trong `kv` để mở lại.

## 6. IPC

| Command | Ai gọi | Chữ ký |
|---|---|---|
| `web_player_get_state` | main, island | `() → WebPlayerSnapshot { lifecycle, site?, url?, title?, canGoBack, canGoForward, rev }` |
| `web_player_open` | main, island | `(siteId?: string, url?: string) → ()` |
| `web_player_show` / `web_player_hide` | main, island, toolbar | `() → ()` |
| `web_player_close` | main, toolbar | `() → ()` |
| `web_player_navigate` | toolbar | `(action: "back" \| "forward" \| "reload" \| "home") → ()` |
| `web_player_clear_data` | main, toolbar | `() → ()` |
| `web_player_open_external` | toolbar | `() → ()` (mở URL hiện tại trong trình duyệt hệ thống) |
| `web_player_list_sites` | main | `() → SiteDto[]` |
| `web_player_add_custom_site` / `web_player_remove_custom_site` | main | … |
| `web_player_report` | **web-player-site (remote)** | `(msg: BridgeMessage) → ()` |

| Event | Payload |
|---|---|
| `web_player://state` | `WebPlayerSnapshot` |

## 7. Dữ liệu & cài đặt

**Bảng:** `web_player_custom_sites (id, name, url, created_at)`. `kv`: `web_player.last_session`.

| Khóa | Mặc định |
|---|---|
| `web_player.unload_after_paused_min` | `30` (0 = không bao giờ) |
| `web_player.restore_on_start` | `"hint"` (`"off"` · `"hint"` · `"load"`) |
| `web_player.window_bounds` | `null` |

## 8. Phụ thuộc

- **Core:** ArtworkCache, storage, settings, ActivityHub.
- **Module:** `media` (trait `MediaSource`).
- **Crate:** `tauri` (feature `unstable` cho multiwebview), `reqwest`, `serde_json`.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Google chặn đăng nhập | Phát hiện điều hướng tới `accounts.google.com` kèm trang lỗi → thẻ hướng dẫn chuyển sang nguồn ngoài |
| Trang cần DRM không phát | Phát hiện lỗi `MediaKeys`/`EME` qua bridge → thẻ hướng dẫn tương tự |
| Trang đổi giao diện làm adapter hỏng | Media Session vẫn chạy thì không ảnh hưởng; adapter lỗi thì ghi `warn`, nút tương ứng bị mờ |
| WebView2 renderer crash | Phát hiện `ProcessFailed` → tải lại trang cuối, trạng thái tạm dừng, báo alert |
| Mất mạng | Trang tự xử lý; island hiện trạng thái tạm dừng |
| Người dùng đóng bằng Alt+F4 | Coi như "Thu gọn" (ẩn), không hủy; chỉ hủy qua "Tắt trình phát" |

## 10. Hiệu năng

- RAM: ghi nhận theo từng trang ở G0 (dự kiến 200–500 MB), ghi vào bảng test matrix.
- Bridge: không chạy timer; chỉ phản ứng theo sự kiện; tối đa 10 tin/giây.
- Tự hủy khi rảnh theo cài đặt.

## 11. Kiểm thử

**Unit:** kiểm tra schema `BridgeMessage`; khớp origin với danh sách domain; logic máy trạng thái vòng đời.

**Bridge (vitest + jsdom):** giả `MediaSession` và `HTMLMediaElement` để kiểm tra móc metadata, bắt handler, `exec` và chuỗi fallback.

**Test matrix (spike D ở P0.6, chạy lại mỗi bản phát hành):**

| Trang | Đăng nhập | Phát | Ẩn 30 phút vẫn phát | Tự chuyển bài | ⏯/⏭ từ island | Phím media | RAM thêm | Kết luận |
|---|---|---|---|---|---|---|---|---|
| Zing MP3 | | | | | | | | |
| SoundCloud | | | | | | | | |
| YouTube | | | | | | | | |
| Spotify Web | | | | | | | | |
| NhacCuaTui | | | | | | | | |

**Checklist thủ công:**
- [ ] Popup đăng nhập (Zalo, Facebook, Spotify) mở và quay về đúng
- [ ] Thu gọn, mở lại, tắt hẳn, mở lại vẫn còn đăng nhập
- [ ] Xóa dữ liệu thì phải đăng nhập lại
- [ ] Trang thử tải file hoặc xin quyền mic đều bị chặn

## 12. Câu hỏi mở

- Có cho mở nhiều trang song song (hai web player) không? Đề xuất: không ở v1.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
