# Module: Trình phát web (`web_player`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P0 (spike D), P2 · Tính năng: F12 · Cập nhật: 2026-11-01

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

**Repeat trên island / trang Nhạc (2026-11-01):** nút lặp mở lựa chọn 1 bài / cả danh sách / không lặp. Command `web_player_set_repeat(mode: off|all|one)` chỉ cấp cho cửa sổ local; bridge xác nhận qua `WebPlayerSnapshot.repeat` và `repeatError`. Lặp 1 áp dụng lên media element; lặp danh sách bắt sự kiện ended trước site để chuyển bài và quay về đầu, không lặp dừng ở cuối danh sách. Chỉ áp dụng khi có media hữu hạn và, với danh sách, nhận diện được bài hiện tại trong queue. Thiếu queue, bị chặn hoặc lỗi chọn bài thì báo lỗi; không hiển thị thành công giả. Giữ lựa chọn qua điều hướng trong cùng phiên. Đã test bridge offline; cần kiểm chứng trên website thật, không tự truy cập lại SoundCloud đang bị chặn.

1. **Chọn trang:** island › Nhạc › Online hoặc main › Nhạc › Online. Trang chưa có lịch sử mở thẳng trang gốc; trang có lịch sử hiện tên bài cuối cùng và hai nút Chạy tiếp / Mở trang gốc.
2. **Thiết lập:** cửa sổ web player mở ra với toolbar mỏng `← → ⟳ ⌂ | [ổ khóa] music.youtube.com | Đổi trang ▾ | Thu gọn | ⋯`. Người dùng đăng nhập, chọn playlist, bấm phát.
3. **Thu gọn:** cửa sổ ẩn (không hủy), nhạc tiếp tục. Island hiện ảnh bìa + sóng nhạc; peek có ⏮ ⏯ ⏭, thanh tiến độ và nút "Mở trình phát".
4. **Mở lại:** bấm ảnh bìa hoặc "Mở trình phát" → cửa sổ hiện lại đúng chỗ cũ.
5. **Tắt hẳn:** menu ⋯ › "Tắt trình phát" → hủy webview, trả RAM. Lần sau mở lại trang cuối, cookie vẫn còn (không phải đăng nhập lại).
6. **Rảnh lâu:** đang ẩn **và** không phát suốt `unload_after_paused_min` (30 phút) thì tự hủy. Thời gian tính từ mốc muộn nhất giữa lần phát cuối và lần mở/hiện/ẩn cuối; đang khôi phục phiên, phát kết quả tìm kiếm hoặc đang tìm thì không hủy. Island hiện Ambient "Tiếp tục nghe Zing MP3" trong 10 phút để mở lại nhanh.
7. **Quy tắc chọn bài (chốt với chủ dự án 2026-11-01):** chọn lại cùng một trang và bấm "Tiếp tục" → phát đúng bài nghe lần cuối của trang đó, trừ khi người dùng tự chọn bài khác (hàng đợi, trang chủ, tìm kiếm). Phát từ kết quả tìm kiếm → phải phát đúng URL bài đã chọn, chỉ báo thành công khi bridge báo đúng URL đó đang Playing. Quảng cáo không bao giờ được lưu làm "bài nghe lần cuối". "Tiếp tục" giữ nguyên cửa sổ đang mở, không tự ẩn.
8. **Đăng xuất & xóa dữ liệu:** menu ⋯ → xác nhận → xóa cookie và cache của hồ sơ web player.
9. **Trang không đăng nhập được (Google) hoặc không phát được (DRM):** hiện thẻ hướng dẫn "Trang này cần mở bằng trình duyệt/app. PCisland vẫn điều khiển được qua Media Session" kèm nút "Mở trong trình duyệt".

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
- Hiện thực từ 2026-11-01: tải bất đồng bộ, chỉ HTTPS (kể cả redirect), tối đa 3 redirect; giới hạn dung lượng cả khi không có Content-Length. Metadata phát ngay với placeholder, sau đó cập nhật URL `http://artwork.localhost/...`. Gộp yêu cầu cùng URL; kết quả tải muộn không được ghi đè bài mới hoặc khôi phục nguồn đã đóng. URL ảnh không bị cắt còn 512 ký tự vì có thể chứa chữ ký dài. Không nới CSP của UI.

**Chính sách điều hướng:**
- **Popup** (`window.open`, `target=_blank`): domain thuộc `allowed_domains` hoặc nhà cung cấp đăng nhập đã biết của trang (ví dụ `accounts.spotify.com`, `id.zalo.me`) thì mở cửa sổ popup con dùng cùng hồ sơ. Còn lại mở trình duyệt hệ thống. Đã kiểm chứng (2026-11-01): Tauri 2.12 mặc định **chặn** mọi popup, nên phải đặt `on_new_window`. `about:blank` được mở trong app vì Zing (và nhiều luồng OAuth) mở `about:blank` trước rồi mới điều hướng; nếu bị chặn, Zing chuyển cả trang chính sang `about:blank` (màn hình trắng). Popup trong app dùng cửa sổ popup mặc định của WebView2: giữ `window.opener`, không có Tauri IPC. Danh sách nhà cung cấp đăng nhập: `sites::LOGIN_PROVIDERS`.
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

### Phiên nghe và danh sách phát (2026-11-01)

- Cookie, local storage, cache và đăng nhập dùng hồ sơ WebView2 bền vững ở %LOCALAPPDATA%/PCisland/WebView2/web-player; đóng/ẩn app không xóa. Lệnh Xóa dữ liệu mới xóa hồ sơ và lịch sử.
- `kv: web_player.sessions` lưu riêng mỗi site: page/playlist URL, URL bài, tên bài, vị trí, hàng đợi. Ghi khi đổi bài/trang/danh sách, dừng hoặc vị trí lệch ít nhất 5 giây.
- Chạy tiếp giữ phiên đang mở nếu cùng nguồn; khi nạp lại thì restore một lần sau page load, tìm bài trong playlist, đợi media sẵn sàng rồi seek/play. Không seek nhầm bài khác. Sau 30 giây chưa phát được thì mở cửa sổ để người dùng xử lý đăng nhập/DRM/autoplay.
- Zing v1.22.4 đã trace thực tế: metadata playlist ở `/api/v2/page/get/playlist`, metadata bài ở `/api/v2/song/get/info`, queue lưu tại `zmp3_np_queue.v1.1`. Bridge chỉ lấy tên/link/ID nhạc từ response mà trang đã nhận và đúng khóa queue này; không đọc cookie, token hay URL stream. Queue sidebar có thể chưa mount; trang album dùng span thay vì link. Link trên thanh phát trỏ album, vì vậy phải lưu riêng album và bài hiện tại.
- **Spotify (trace thật 2026-11-02, web player bản 2026-10-30):** response `connect-state/v1/devices/*` (PUT) và `connect-state/v1/cluster` (GET), cùng payload `cluster` trên WebSocket dealer, chứa `player_state`: URI bài hiện tại, `context_uri`, `prev_tracks`/`next_tracks` (chỉ URI, phần lớn không có tên) và `options.repeating_context/repeating_track`. Tên bài lấy từ response pathfinder `decorateContextTracks` mà trang tự gọi, cộng `metadata.title` trong cluster. Bridge móc `fetch` và `WebSocket` của trang để đọc các response này; không gọi API riêng, không giữ token, device ID hay URL audio/licence. Queue = tối đa 20 bài trước + bài hiện tại + bài tiếp, chỉ bài đã biết tên; URL `open.spotify.com/track/<id>`, pageUrl là album/playlist/artist của context. Chọn bài: bấm hàng có link `/track/<id>` trên trang đang mở hoặc hàng cùng tên trong panel Queue.
- **SoundCloud (trace DOM 2026-11-02, đã đăng nhập):** queue chỉ render khi panel “Next up” mở (`.playbackSoundBadge__showQueue`); mỗi hàng `.queueItemView` có link `.queueItemView__title a`, người đăng `.queueItemView__username`, nút `.queueItemView__playButton .sc-button-play`, hàng đang phát `m-active`. Link thanh phát có `?in=<context>` nên trackUrl bỏ query để khớp hàng queue; context vẫn làm pageUrl.
- **Repeat Spotify/SoundCloud** dùng nút repeat của chính trang thay cho `media.loop`/bắt `ended` (trang tự chuyển bài, can thiệp `ended` sẽ phá queue của trang). Bridge bấm nút tới khi trạng thái khớp, tối đa 3 lần cách 500 ms; Spotify đọc trạng thái từ `options` của cluster, dự phòng theo `aria-label` (“Enable repeat” / “Enable repeat one” / “Disable repeat”, phụ thuộc ngôn ngữ trang); SoundCloud đọc class `m-one`/`m-all` của `.repeatControl`. Không khớp thì báo `repeatError`.
- Tối đa 100 bài; virtualization có thể làm thiếu bài chưa tải. Không gọi API tải nhạc riêng. Zing đã kiểm thử native đầy đủ chọn queue và chạy tiếp sau restart; Spotify/SoundCloud mới có test logic với dữ liệu trace, chủ dự án tự nghiệm thu trên exe.
- Nguồn Online trên island mặc định hiện tên trang/bài và trạng thái, cùng Mở trang, Đổi trang, Tìm bài. Mở trang chỉ show cửa sổ hiện tại, không điều hướng về home. Danh sách nguồn chỉ hiện khi chưa mở nguồn hoặc chủ động Đổi trang.
- Audio tách khỏi DOM được đăng ký sự kiện riêng để lưu tiến trình mỗi 5 giây. Metadata phục hồi mà chưa có vị trí không được ghi đè vị trí đã lưu bằng 0. Lệnh đổi nguồn chủ động dừng bài cũ và tránh hộp thoại trước khi rời trang bị kẹt trong cửa sổ ẩn.

### Tìm kiếm trong app và giới hạn kiểm thử (2026-11-01)

- Zing MP3, Spotify và SoundCloud: nhập từ khóa, nhận tối đa 30 bài, chọn để phát trong nền. WebView tìm kiếm dùng chung profile nhưng không phát âm thanh; tách khỏi player nên tìm kiếm không điều hướng/dừng bài hiện tại. Mỗi lần submit chỉ mở một lượt tìm, timeout 22 giây, không tự gửi lại.
- Cửa sổ `web-search-*` chỉ có quyền `web_player_search_report`; backend xác minh request ID, label, URL tìm kiếm và origin, chỉ cho phát URL trong kết quả đã nhận. Không lấy credential từ Chrome hay chuyển cookie giữa trình duyệt.
- Đã kiểm chứng kết quả tìm từ ba nguồn và tìm→phát Zing. **SoundCloud xuất hiện “You have been blocked / unusual activity” trong QA. Đã dừng toàn bộ test online và vô hiệu tìm kiếm nền SoundCloud**, bỏ domain khỏi capability tìm kiếm. Không đổi IP/profile, không xóa cookie hoặc vượt challenge. Không xác định được thời hạn hay nguyên nhân chính xác từ thông báo.
- 2026-11-02: chủ dự án đã đăng nhập SoundCloud và phát được, yêu cầu bật lại tìm kiếm. Tìm SoundCloud mở `soundcloud.com/search/sounds?q=` một lần cho mỗi lần người dùng bấm tìm (như gõ trên trang), đọc `.searchList__item a.soundTitle__title` và tên tài khoản đăng (`uploader`, không phải ca sĩ). Domain được thêm lại vào capability `web-search-remote`. Không tự gửi lại, không chạy tìm nền; gặp trang chặn thì dừng như cũ. Selector trang tìm kiếm chưa được kiểm trên WebView thật.
- Restore chỉ kích hoạt phát một lần rồi chờ media sẵn sàng; không click Play mỗi vài giây. Khi thấy nội dung chặn/challenge thì dừng. Các thay đổi cuối sau sự cố chỉ được unit test, không thử lại online. Spotify tìm kiếm đã có kết quả thật; chọn phát và khôi phục trong WebView cần kiểm chứng riêng, không suy ra từ Chrome.
- Backend chỉ nhận URL HTTPS của đúng site, không chứa credentials; queue khử trùng và giới hạn độ dài. Trang từ xa vẫn chỉ có quyền report.

## 6. IPC

| Command | Ai gọi | Chữ ký |
|---|---|---|
| `web_player_get_state` | main, island | `() → WebPlayerSnapshot { lifecycle, siteId?, url?, history, queue, rev }` |
| `web_player_open` | main, island | `(siteId?: string, url?: string) → ()` |
| `web_player_resume` | main, island | `(siteId: string) → ()` |
| `web_player_select_queue` | island | `(url: string) → ()` (phải thuộc queue hiện tại) |
| `web_player_request_queue` | island | `() → ()` mở queue của site nếu cần |
| `web_player_search` | main, island | `(siteId, query) → WebQueueItem[]`; Zing/Spotify/SoundCloud |
| `web_player_play_result` | main, island | `(siteId, url) → ()`; URL phải trong kết quả tìm gần nhất |
| `web_player_search_report` | web-search-* | `(requestId, items) → ()`; quyền riêng, không được điều khiển app |
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
- Bridge: sự kiện media, tối đa 10 tin/giây; restore có vòng chờ hữu hạn và chỉ một lần kích hoạt. Search poll DOM hữu hạn, không poll HTTP.
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

Island: chọn site có lịch sử tự mở rộng panel để thấy đủ các nút. Chọn kết quả tìm kiếm hiển thị trạng thái đang mở, chỉ chuyển về player sau khi bridge báo đúng URL bài đã chọn ở trạng thái Playing. Hết 24 giây chưa xác nhận thì giữ kết quả, báo lỗi và cho mở trang nguồn để kiểm tra. Queue mở rộng island theo nội dung, danh sách dài mới cuộn.

Zing có thể chuyển URL bài hát sang album trong khi player và queue vẫn giữ bài cũ. Bridge giữ ánh xạ hàng trên trang (`pageNodes`) riêng với queue, dùng metadata playlist để nhận diện đúng URL rồi bấm nút phát trên hàng đó. Không thêm cả album vào queue cũ khi đọc trang. Restore kiểm tra URL thay vì chỉ tên bài, chỉ kích hoạt một lần và giới hạn một lần điều hướng dự phòng; timeout hủy restore. Regression test bao gồm hai bản cùng tên nhưng URL khác, album mới và queue cũ.

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Tự tắt khi tạm dừng quá N phút; đổi trang, xóa dữ liệu đăng nhập; toolbar đặt ở tab Nhạc › Online (ADR-013) |
| 2026-11-01 | Sửa: popup đăng nhập bị chặn (SoundCloud "enable popup windows", Zing màn hình trắng). Thêm `sites::popup_policy` theo §5; `web_player_open` chạy async |
| 2026-11-01 | Ảnh online tải bất đồng bộ qua ArtworkCache; giới hạn 2 MB/5 s, bảo toàn signed URL, gộp tải và bỏ kết quả cũ. |
| 2026-11-01 | Lưu phiên từng site, Chạy tiếp/Mở trang gốc, khôi phục vị trí và queue trên island; giữ cookie/cache qua lần chạy. |
| 2026-11-01 | Tìm ngay trong player; kết quả không co dòng, tối thiểu 40 px, tên dài xuống dòng, list cuộn tối đa 340 px. QA offline 30 kết quả. |
| 2026-11-01 | Trace DOM công khai Zing/Spotify; search có ca sĩ và uploader tùy nguồn, không suy diễn người đăng. Xem [bằng chứng](../reports/search-credits-2026-11-01.md). |
| 2026-11-01 | Sửa chọn bài tìm kiếm ngoài queue cũ; xác nhận đúng URL đang phát trước khi đóng kết quả, báo lỗi thay vì hiện player cũ như thành công. |
| 2026-11-01 | Sửa tự hủy sai: player ẩn mà chưa phát bị hủy sau < 1 phút, có thể cắt ngang khôi phục/phát kết quả tìm kiếm. Không lưu quảng cáo làm bài cuối. "Tiếp tục" không ẩn cửa sổ đang xem. Bridge parse queue Zing một lần mỗi báo. Queue web tô bài hiện tại. |
| 2026-11-02 | Spotify: queue/bài hiện tại/repeat từ connect-state + dealer WebSocket, tên bài từ `decorateContextTracks` (trace thật). SoundCloud: queue từ panel Next up, repeat qua nút trang, bật lại tìm kiếm theo yêu cầu chủ dự án. Test bridge cho cả hai; chưa nghiệm thu trên exe. |
