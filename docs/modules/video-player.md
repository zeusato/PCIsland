# Module: Trình phát video nổi (`video`)

> Trạng thái: Đã code đủ B + A, đã chạy thử trên bản release; chờ test tay (Facebook, dò trang) · Phase: chưa xếp (backlog, F46) · Tính năng: F46 · Cập nhật: 2026-11-01

## 1. Mục đích

Người dùng dán một link video (YouTube, Facebook, hoặc một trang/link có video), bấm Play. App mở **một** cửa sổ nhỏ chỉ có video và bộ nút của PCisland:
- Không có giao diện, quảng cáo hay gợi ý của trang gốc.
- Cửa sổ kéo co được, ghim trên cùng được, về kích thước mặc định (rộng 320 px) bằng một nút.
- Island điều khiển được video mà không cần kích hoạt cửa sổ video.

Hai phương thức triển khai:
- **B. Lấy stream rồi tự phát.** Đây là **phương thức chính**, chủ dự án chốt ngày 2026-11-01 (ADR-018). App tự giải link ra luồng thật rồi phát bằng trình phát của mình, nên không bị trang gốc chèn quảng cáo hay giao diện thừa.
- **A. Trình phát nhúng chính thức.** Chỉ là **đường lui**, dùng khi B không phát được: luồng có DRM, yt-dlp hỏng chờ cập nhật, hoặc người dùng tắt B trong Cài đặt. Thanh nút của PCisland vẫn giữ nguyên, nhưng vùng phát là iframe của YouTube/Facebook.

## 2. Phạm vi

**Trong phạm vi:**
- YouTube: `watch?v=`, `youtu.be/`, `shorts/`, `embed/`, `live/`, có hoặc không có `t=`.
- Facebook: `facebook.com/.../videos/...`, `fb.watch/...`, `facebook.com/reel/...`, `facebook.com/watch/?v=...`.
- Link file trực tiếp `.mp4`, `.webm`, `.m3u8` (HLS không mã hóa).
- Trang web bất kỳ có video (dò luồng như addon stream-pocket), cho người dùng chọn luồng.
- **Tối đa 1 cửa sổ video.** Dán link mới thì thay video đang phát.
- Phím tắt mở ô dán link.
- Island điều khiển video: phát/dừng, tua, bật/tắt tiếng, đóng.
- Chọn chất lượng (tự động hoặc chọn tay theo độ phân giải có sẵn).

**Ngoài phạm vi:**
- Tải video về máy, playlist, hàng đợi.
- Video riêng tư/cần đăng nhập ở bản đầu (xem §12).
- Luồng có DRM (Widevine/PlayReady) hoặc HLS mã hóa: chuyển sang A hoặc báo không hỗ trợ.

## 3. Hành vi & luồng người dùng

1. **Mở ô dán link:** phím tắt `Ctrl+Alt+V` (đổi được), hoặc island › Tính năng › **Video**, hoặc cửa sổ đầy đủ › Video. Island hiện ô "Dán link video" có sẵn focus, nút **Phát**. Clipboard đang có link hợp lệ thì tự điền.
2. **Kiểm tra link:** sai/không hỗ trợ thì báo ngay dưới ô, không mở cửa sổ.
3. **Lần đầu dùng:** chưa có công cụ giải link (yt-dlp) thì hiện "Cần tải công cụ giải link (~23 MB) từ GitHub chính thức. Tải?" với hai nút Tải / Dùng trình phát nhúng. Tải xong mới phát.
4. **Giải link:** island hiện "Đang lấy video…" (tối đa 20 s). Nguồn là trang web bất kỳ có nhiều luồng thì hiện danh sách luồng để chọn.
5. **Phát:**
   - Mở cửa sổ video ở góc dưới phải của màn hình đang có chuột, 320×180 (hoặc vị trí và kích thước đã nhớ). Tự phát.
   - Island chuyển sang hiển thị nguồn "Video" như một nguồn media.
6. **Điều khiển trong cửa sổ** (rê chuột vào thì hiện, 2 s không động thì ẩn; dưới 480 px thì ẩn thanh âm lượng, chỉ giữ nút tắt tiếng):
   - **Trên:** Ghim · Về mặc định · Mở bằng trình duyệt · Tắt. Kéo thanh trên để di chuyển cửa sổ.
   - **Dưới:** ⏯ · thanh tua (có thời gian) · âm lượng/tắt tiếng · chất lượng · toàn màn hình.
   - **Phím khi cửa sổ có focus:** Space (phát/dừng), ←/→ (tua 5 s), ↑/↓ (âm lượng), M (tắt tiếng), F (toàn màn hình), Esc (thoát toàn màn hình, nhấn lần nữa thì tắt).
7. **Điều khiển từ island** (không kích hoạt cửa sổ video): compact hiện ảnh thumbnail + tiến trình. Peek dùng player media chung: ⏮/⏭ là tua ±10 s, ⏯. Ba nút "Tắt tiếng / Hiện cửa sổ / Tắt video" nằm ở cuối **cùng hàng** với nút phát (slot `trailing` của `Peek`), không thêm hàng mới. Tiêu đề dài tự cắt bằng "…".
8. **Đường lui:** B lỗi (DRM, giải link thất bại sau 1 lần cập nhật yt-dlp, mạng) thì hiện lỗi kèm nút "Phát bằng trình phát nhúng" (A) và "Mở bằng trình duyệt".
9. **Tắt:** hủy cửa sổ, dừng proxy, gỡ nguồn khỏi media hub. Lần sau mở lại theo vị trí và kích thước đã nhớ.
10. **Phát tiếp từ đoạn dở** (chủ dự án chốt 2026-11-01):
    - App lưu vị trí xem của từng video: 5 s một lần khi đang phát, và mỗi khi dừng/đổi trạng thái. Khóa lưu là ID YouTube hoặc link đã bỏ `#…`, `kv` `video.progress`, tối đa 200 video, bỏ video cũ nhất trước.
    - Mở lại đúng video đó thì phát tiếp từ chỗ dở, hiện "Phát tiếp từ 3:17 · Xem từ đầu" trong 8 s. Link có `t=` thì theo `t=`.
    - Không phát tiếp khi vị trí < 5 s, hoặc cách cuối < 15 s; xem hết (`ended`) thì xóa mục đó. Trình phát nhúng cũng bắt đầu từ vị trí đã lưu.
11. **Link hết hạn giữa chừng** (403/410, hoặc lỗi mạng của trình phát): app tự lấy link mới từ **URL gốc người dùng đã dán**, rồi phát tiếp đúng vị trí:
    - YouTube/Facebook: chạy lại yt-dlp (cập nhật yt-dlp 1 lần nếu lỗi).
    - Trang dò được: dò lại trang, ưu tiên luồng cùng đường dẫn với luồng cũ.
    - Link trực tiếp: đăng ký lại cùng link.

    Tối đa 1 lần/60 s. Không lấy lại được thì báo "Link video đã hết hạn và không lấy lại được", kèm nút Thử lại / Trình phát nhúng.

## 4. Activity

Video là một **nguồn media** (`source_id = "video"`) của media hub, không tạo activity riêng. Media hub dựng activity `media.now_playing` như các nguồn khác. Island hiển thị bằng view media sẵn có, cộng thêm nút "Hiện cửa sổ" và "Tắt video" khi nguồn là `video`.

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `media.now_playing` (nguồn `video`) | MEDIA 50 khi phát, AMBIENT khi dừng | Thumbnail + tiến trình | ⏯, tua, tắt tiếng, Hiện cửa sổ, Tắt | — | `play_pause`, `seek`, `mute`, `show`, `close` |

Khi cửa sổ video đang mở, phiên GSMTC của WebView2 do video tạo ra bị ẩn, giống cách web player đang làm, để không thành nguồn trùng.

## 5. Thiết kế kỹ thuật

### 5.1 Thành phần

```
island/main ──video_open(url)──▶ modules/video
                                   ├─ link.rs      phân loại link (thuần)
                                   ├─ tools.rs     yt-dlp: kiểm tra, tải, cập nhật, gọi
                                   ├─ resolve.rs   yt-dlp JSON → ResolvedMedia (thuần, test bằng fixture)
                                   ├─ sniff.rs     trang bất kỳ: WebView ẩn + bắt response (B1-web)
                                   ├─ proxy.rs     giao thức vstream:// gắn header + Range
                                   ├─ window.rs    cửa sổ duy nhất, ghim, về mặc định, nhớ vị trí
                                   └─ mod.rs       trạng thái, nối media hub, lệnh từ island
cửa sổ `video` ── video.html (trang cục bộ, Svelte) ── <video> + <audio> qua vstream://
```

### 5.2 Phân loại link (`link.rs`, thuần)

- Trả về `Youtube { id, start_s }` · `Facebook { url }` · `Direct { url, kind: File | Hls }` · `Page { url }` · lỗi.
- Chỉ nhận `https` (riêng `Direct` nhận cả `http`), không nhận user/password, độ dài ≤ 2048.
- ID YouTube phải khớp `[A-Za-z0-9_-]{11}`.
- `t=` đọc được dạng `90`, `90s`, `1m30s`, `1h2m3s`.

### 5.3 B1. Giải link

**YouTube / Facebook: dùng yt-dlp chạy như tiến trình con.**

- **Vị trí:** `%LOCALAPPDATA%\PCisland\tools\yt-dlp.exe`, không đóng gói trong bộ cài.
- **Tải:** chỉ khi người dùng đồng ý, từ `https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe`. Kiểm SHA-256 với file `SHA2-256SUMS` của cùng release; sai thì xóa và báo lỗi. Ghi phiên bản vào `kv` `video.tools`.
- **Cập nhật:**
  - Nút "Cập nhật công cụ giải link" trong Cài đặt.
  - Tự kiểm tra bản mới tối đa 1 lần/ngày khi mở video (đọc API release của GitHub).
  - Giải link YouTube thất bại thì thử cập nhật 1 lần rồi giải lại.
- **Gọi:** `yt-dlp --dump-single-json --no-playlist --no-warnings --no-progress -- <url>`.
  - Có cờ `CREATE_NO_WINDOW`; timeout 20 s; stdout tối đa 8 MB.
  - Đặt trong `platform/` vì có cờ Win32. URL luôn đứng sau `--` để không bị hiểu thành tham số.
- **Đọc JSON** (`resolve.rs`, test bằng file JSON mẫu): lấy `title`, `uploader`, `duration`, `thumbnail`, `is_live`, `http_headers` và `formats[]`. Mỗi format có `format_id`, `url`, `manifest_url`, `protocol`, `vcodec`, `acodec`, `ext`, `height`, `tbr`, `filesize`, `has_drm`.
- **Chọn luồng** (hàm thuần, có test):
  1. Bỏ format `has_drm`, `protocol` lạ (chỉ nhận `https`, `m3u8_native`, `m3u8`), và codec WebView2 không giải được. Nhận video `avc1`/`vp9`/`av01`, audio `mp4a`/`opus`.
  2. **HLS** (`manifest_url` master): đây là **mặc định cho YouTube**, theo spike S2. MSE tự đồng bộ, có chất lượng tự động, phát nền khi cửa sổ ẩn. Giới hạn mức theo `video.max_height`.
  3. **Ghép đôi:** video-only cao nhất ≤ `video.max_height`, kèm audio-only tốt nhất cùng họ container (`mp4`+`m4a`, `webm`+`webm`). Dùng khi không có HLS, hoặc người dùng chọn tay một mức HLS không có.
  4. Luồng progressive có cả hình và tiếng: Facebook, link trực tiếp. YouTube đã bỏ loại này (itag 18), xem báo cáo S2.
  5. Không còn gì thì báo lỗi và mời dùng A.
  - Danh sách chất lượng cho menu là các mức `height` có cặp hợp lệ.

**Trang web bất kỳ (`sniff.rs`, đúng cách addon stream-pocket):**
- Mở trang trong WebView ẩn (profile riêng `video-sniff`, tắt âm, chặn popup), tối đa 25 s.
- Nghe `WebResourceResponseReceived` của WebView2 (viết trong `platform/`), tương đương `webRequest.onHeadersReceived` của addon.
- Phân loại theo đuôi file + Content-Type, port từ `classify()` trong `core.mjs` của addon:
  - `.m3u8` / `mpegurl` → HLS;
  - `.mpd` → DASH (bản đầu báo chưa hỗ trợ);
  - `video/*`, `audio/*`, `.mp4`… → file;
  - bỏ qua `.ts`, `.m4s`.
- Quét thêm DOM bằng script tương đương `scanner.js`: `video`, `audio`, `source`, meta `og:video`, link có đuôi media, `performance.getEntriesByType('resource')`. Đếm số `blob:`; nếu chỉ có blob thì báo "trang dùng luồng đóng gói, thử dán link gốc YouTube/Facebook".
- Ghi lại Referer = trang gốc để proxy gắn vào khi phát.

### 5.4 B2. Proxy luồng (`proxy.rs`)

- Giao thức tùy chỉnh `vstream://localhost/<token>`. Mỗi luồng đã giải link được cấp một token ngẫu nhiên, ánh xạ tới `{ url, headers }`.
- Proxy chỉ chuyển tiếp tới đúng URL của token, không phải proxy tự do. Token hết hiệu lực khi đóng video.
- Chuyển tiếp `Range` / `If-Range` và trả về `206`, `Content-Range`, `Accept-Ranges`, `Content-Type`. Stream thân phản hồi theo khối, không đọc hết vào RAM.
- Gắn `http_headers` từ yt-dlp (User-Agent, Referer, Accept-Language); với nguồn dò trang thì gắn Referer = trang gốc.
- HLS: viết lại URL trong playlist `.m3u8` (cả playlist con và đoạn) thành `vstream://` để mọi request đều đi qua proxy.
  - Chỉ coi là playlist khi Content-Type là `mpegurl` hoặc đường dẫn **kết thúc** bằng `.m3u8`. URL đoạn của YouTube chứa `index.m3u8` ở giữa đường dẫn (bài học S2).
  - Chỉ chuyển tiếp tới host mà luồng gốc đã dùng (với YouTube là `*.googlevideo.com`).
- Gặp `403`/`410` (URL hết hạn) thì báo frontend. Frontend gọi `video_refresh`: giải link lại, cấp token mới, phát tiếp từ vị trí cũ (tối đa 1 lần mỗi 60 s).

### 5.5 B3. Trình phát (`video.html`)

- Trang cục bộ (Svelte, chung design token), không có iframe ở chế độ B.
- **HLS (mặc định cho YouTube):** hls.js đóng gói sẵn (không dùng CDN), nạp playlist qua `vstream://`, `autoLevelCapping` theo `video.max_height`. Menu chất lượng = Tự động + các mức của manifest.
- **Cặp tách rời** (khi không có HLS): `<video muted>` phát luồng hình, `<audio>` phát luồng tiếng, đồng bộ theo đồng hồ tiếng.
  - Lệch > 80 ms thì chỉnh `playbackRate` của video ±3%; lệch > 400 ms thì đặt lại `currentTime`.
  - Sau khi tua, căn `video.currentTime = audio.currentTime` ngay khi cả hai `canplay`.
  - Chờ buffer: bên nào `waiting` thì dừng bên kia.
  - Khi `document.hidden` (cửa sổ bị che hoặc thu nhỏ), Chromium tự dừng `<video>` chỉ có hình: bỏ qua sự kiện của video, chỉ phát tiếng; hiện lại thì đưa hình về vị trí tiếng.
  - Spike S2 đo được lệch trung bình 16 ms, tối đa 17 ms trong 244 s ở 1080p60, nên không cần MSE tự viết.
- **Progressive:** một `<video>`.
- **Bộ nút của PCisland** như §3 bước 6. Đổi chất lượng giữ vị trí đang phát.
- Báo trạng thái cho backend mỗi khi phát/dừng/tua/đổi âm lượng/ended và 1 lần/5 s khi đang phát. Backend đẩy sang media hub nguồn `video`.

### 5.6 Cửa sổ (`window.rs`)

- Label `video`, chỉ 1 cửa sổ. Không viền, `resizable`, tối thiểu 240×135, mặc định 320×180 theo DPI, không có trong taskbar.
- **Thư mục WebView2 riêng** `WebView2/video`, với `additional_browser_args`:
  - giữ các cờ mặc định của app;
  - thêm `--disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-background-media-suspend --autoplay-policy=no-user-gesture-required`.
  - Hai lý do (S2): không có các cờ này thì cửa sổ bị che là video đứng; và WebView2 bắt buộc cùng args cho cùng một thư mục dữ liệu.
- Lệnh tạo cửa sổ là **async** (03 §3).
- **Ghim:** `set_always_on_top`, nhớ trong `kv`. Island vẫn nằm trên cửa sổ video.
- **Về mặc định:** 320×180 theo DPI, giữ góc trên trái, kéo vào trong màn hình nếu tràn.
- **Nhớ vị trí:** dùng lại mẫu `app/main_window.rs`, khóa `video_window.geometry`.
- **Giữ tỉ lệ:** bản đầu dùng viền đen. Khóa tỉ lệ khi kéo (`WM_SIZING` trong `platform/window.rs`) làm sau.

### 5.7 Điều khiển từ island

- Media hub đăng ký `register_control("video", …)`. `MediaCommand::PlayPause/Pause` cùng các lệnh mới `Seek(ms)`, `Mute` gửi vào cửa sổ video qua event `video://command` (trang cục bộ có IPC).
- Island không bao giờ gọi `set_focus` cửa sổ video, trừ khi bấm "Hiện cửa sổ".
- Phím media của bàn phím đi qua SMTC của app (đã có), nên cũng điều khiển được video khi video là nguồn hiện tại.

### 5.8 Đường lui A (trình phát nhúng)

- YouTube: iframe `https://www.youtube-nocookie.com/embed/<id>?autoplay=1&playsinline=1&rel=0&start=<s>`; Facebook: `https://www.facebook.com/plugins/video.php?href=…&autoplay=true`.
- Trang iframe không có IPC. Island chỉ điều khiển được phát/dừng ở mức GSMTC.
- Trước khi coi A là sẵn sàng phải chạy spike S1: lỗi 153 (referrer) của YouTube embed khi chạy từ origin của app.

## 6. IPC

| Command | Gọi từ | Chữ ký |
|---|---|---|
| `video_classify` | island, main | `(url) → LinkKind \| null` (kiểm tra khi gõ) |
| `video_clipboard` | island, main | `() → string \| null` (chỉ trả link mà trình phát nhận) |
| `video_open` | island, main, video | **async** `(url, embed?) → VideoOpenResult` = `playing` · `needsTools` · `choose { streams }` |
| `video_open_stream` | island, main | **async** `(id) → ()` (chọn luồng sau khi dò trang) |
| `video_get_state` | island, main, video | `() → VideoSnapshot` |
| `video_tools_status` | island, main | `() → ToolsStatus { installed, version }` |
| `video_tools_install` | island, main | **async** `() → ToolsStatus`. Cài hoặc cập nhật bản mới nhất; chỉ gọi sau khi người dùng đồng ý |
| `video_control` | island, main | `(action, value?)`: `play_pause`, `pause`, `play`, `mute`, `back`, `forward`, `seek` (ms) |
| `video_window` | island, main, video | **async** `(action)`: `pin`, `unpin`, `reset`, `show`, `close` |
| `video_open_in_browser` | video | `() → ()`. Chỉ mở link người dùng đã dán |
| `video_current` | video | `() → PlaybackSource \| null` (trang nạp xong thì hỏi nguồn hiện tại) |
| `video_report` | video | `(report: VideoReport)` (chỉ nhận từ label `video`) |
| `video_refresh` | video | **async** `(positionMs) → PlaybackSource`. Giải lại link khi bị 403/410; tối đa 1 lần/60 s |
| `video_set_quality` | video | **async** `(height \| null, positionMs) → PlaybackSource`. Chỉ dùng cho nguồn không phải HLS; HLS đổi mức ngay trong trang |
| `video_sniff_report` | video-sniff | `(requestId, items, blobs, done)`. Lệnh duy nhất trang dò được gọi |

- Event `video://source` → cửa sổ video: `PlaybackSource { kind: "hls" | "pair" | "single" | "embed", video, audio?, title, uploader?, durationMs?, isLive, qualities[], quality?, startMs, pageUrl, rev }`. Mọi URL là `http://vstream.localhost/<token>`, trừ `embed`.
- Event `video://command` → cửa sổ video: `VideoCommand { action, value? }`.
- Event `video://state` → island/main/video: `VideoSnapshot { open, pinned, muted, title?, status?, rev }`.
- Media hub: nguồn `video`. Next/previous của island và phím media = tua ±10 s.
- **Capability:**
  - `capabilities/video.json` (cửa sổ `video`): các lệnh `video_*` của trang video, `settings_get`, `core:window` (kéo, toàn màn hình, thu nhỏ).
  - `capabilities/video-sniff-remote.json` (cửa sổ `video-sniff`, mọi http/https): chỉ `video_sniff_report`.
- **CSP:** `media-src http://vstream.localhost blob:`, `connect-src http://vstream.localhost` (hls.js), `frame-src` cho `youtube-nocookie.com` và `facebook.com`. hls.js chạy không dùng worker nên không cần `worker-src blob:`.

## 7. Dữ liệu & cài đặt

- `kv`:
  - `video.progress`: `{ <khóa video>: { pos_ms, dur_ms, at } }`, tối đa 200;
  - `video_window.geometry`: `{x, y, width, height}`;
  - `video_window.pinned`;
  - `video.tools`: `{ version, sha256, checked_at }`.

| Khóa cài đặt | Mặc định |
|---|---|
| `app.shortcuts.video_open` | `"Ctrl+Alt+V"` |
| `video.engine` | `"stream"` (B); `"embed"` = luôn dùng A |
| `video.max_height` | `1080` |
| `video.default_width` | `320` |
| `video.auto_update_tools` | `true` |

## 8. Phụ thuộc

- Core: settings, storage (kv), media hub (`register_control`, `update_source`), shortcuts.
- Platform:
  - gọi tiến trình con không cửa sổ;
  - WebView2 `WebResourceResponseReceived` (dò trang);
  - `window` (DPI, màn hình theo chuột).
- Crate: `reqwest` (đã có), `sha2` (mới, kiểm tải yt-dlp).
- Frontend: hls.js (mới, đóng gói).
- Ngoài: yt-dlp.exe (tải khi cần, xem B1).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Chưa có yt-dlp, người dùng từ chối tải | Mời dùng trình phát nhúng (A) |
| Tải yt-dlp sai SHA-256 / mất mạng | Xóa file, báo lỗi, không chạy |
| yt-dlp lỗi / timeout | Thử cập nhật 1 lần rồi giải lại; vẫn lỗi thì A + Mở trình duyệt |
| Video có DRM / HLS mã hóa | Báo không phát được bằng B, mời A |
| URL luồng hết hạn (403/410) giữa chừng | `video_refresh`: lấy link mới từ URL gốc (yt-dlp / dò lại trang / đăng ký lại link trực tiếp), phát tiếp từ vị trí cũ; thất bại thì báo lỗi |
| Mở lại video đang xem dở | Phát tiếp từ vị trí đã lưu, có nút "Xem từ đầu" |
| Video live | Dùng HLS; ẩn thanh tua, hiện nhãn LIVE |
| Video riêng tư / cần đăng nhập | Báo cần đăng nhập; mời Mở trình duyệt (§12) |
| Trang dò chỉ có `blob:` | Báo trang dùng luồng đóng gói, gợi ý dán link gốc |
| Lệch tiếng-hình liên tục > 400 ms | Đặt lại vị trí; lặp lại 3 lần trong 30 s thì chuyển sang luồng progressive |
| Dán link khi đang có video | Thay nguồn trong cửa sổ hiện tại, giữ vị trí/kích thước/ghim |
| Màn hình đã lưu bị rút | Mở ở góc dưới phải màn hình có chuột |

## 10. Hiệu năng

- Proxy stream theo khối, không đệm cả file. Mỗi luồng giữ tối đa 2 request song song.
- Đóng cửa sổ là hủy webview, dừng proxy, hủy token. Không giữ gì ngầm.
- yt-dlp chỉ chạy lúc giải link, không chạy thường trú.
- Ngân sách: ≤ 150 MB RAM thêm khi phát 1080p; 0 MB khi tắt; CPU giải mã do WebView2 dùng tăng tốc phần cứng.

## 11. Kiểm thử

- **Unit:**
  - `link.rs`: mọi dạng link; từ chối `javascript:`, user:pass, ID sai; tính `t=`.
  - `resolve.rs`: chọn luồng từ JSON mẫu (YouTube nhiều format, Facebook sd/hd, live, có DRM).
  - Proxy: chỉ chuyển tiếp URL của token; chuyển tiếp Range; viết lại m3u8.
  - Kiểm SHA-256 của file tải.
- **Integration:** `video_open` là async và tạo đúng 1 cửa sổ; lệnh từ island tới được trang video; đóng cửa sổ gỡ nguồn media.
- **Spike:**
  - [x] **S2:** đạt; xem [báo cáo](../reports/video-s2-2026-11-01.md). Chưa thử Facebook và WebView2 thật của app.
  - [ ] **S1 (trước khi bật A):** YouTube embed từ origin của app
- **Đã chạy thật trên bản release (2026-11-01, CDP):**
  - [x] Link YouTube: chưa có công cụ → `needsTools`. Cài yt-dlp qua app mất 5,5 s, SHA-256 khớp. Phát sau 3,5 s, đúng `t=30`, HLS 1080p (giới hạn theo cài đặt), rớt 1/847 khung.
  - [x] Island: nguồn `video` có ảnh bìa, ẩn phiên WebView2 trùng; dừng / phát / tắt tiếng / tua +10 s / ghim / về mặc định / tắt cửa sổ (gỡ nguồn khỏi media hub).
  - [x] Cặp tách rời qua proxy Rust (Range cắt 4 MB): 1080p, lệch ổn định 40 ms với ngưỡng 80 ms → hạ ngưỡng xuống 35 ms.
  - [x] Thu nhỏ cửa sổ: cả hình và tiếng vẫn phát (nhờ cờ WebView2 riêng).
  - [x] Peek island với tiêu đề YouTube dài: tiêu đề cắt "…", thanh tiến trình đủ hai đầu; giao diện cửa sổ 320×180 kiểm bằng ảnh chụp.
- **Còn phải thử tay:**
  - [ ] Facebook video công khai, reel
  - [ ] Link `.mp4`, `.m3u8`; một trang tin tức có video (dò trang)
  - [ ] Để quá 6 giờ hoặc giả lập 403 → tự giải lại link
  - [ ] Trình phát nhúng (đường lui A, spike S1: lỗi 153 của YouTube)
  - [ ] Phím tắt `Ctrl+Alt+V`; phím media; DPI 150%
  - [ ] RAM khi phát 1080p và sau khi tắt

## 12. Câu hỏi mở

1. ~~Một hay nhiều cửa sổ~~ → **Tối đa 1** (chốt 2026-11-01).
2. ~~Phím tắt~~ → **Có**, mặc định `Ctrl+Alt+V`.
3. ~~Island điều khiển video~~ → **Có**, kể cả khi không kích hoạt cửa sổ video.
4. ~~Có làm B không~~ → **Làm toàn bộ B**, A chỉ làm đường lui.
5. Video cần đăng nhập (YouTube giới hạn tuổi, Facebook riêng tư): có cho yt-dlp dùng cookie của hồ sơ web player không? Mặc định: **không** (chỉ nội dung công khai), vì phải đọc cookie đăng nhập của người dùng.
6. Khi không còn dùng B (ví dụ tắt module), có tự xóa `yt-dlp.exe` không? Đề xuất: xóa khi chạy `--cleanup` và khi gỡ bộ cài.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-11-01 | Tạo đặc tả: hai phương thức A (nhúng) và B (lấy stream) |
| 2026-11-01 | Chủ dự án chốt: tối đa 1 cửa sổ, có phím tắt, island điều khiển được, làm toàn bộ B; A thành đường lui. Viết lại §1–§12 theo B |
| 2026-11-01 | Spike S2: YouTube mặc định HLS (hls.js), cặp tách rời làm đường thứ hai (lệch 16 ms); thêm cờ WebView2 chống dừng nền và thư mục WebView2 riêng; sửa nhận diện playlist trong proxy |
| 2026-11-01 | Code đủ module: link/resolve/tools(yt-dlp)/proxy vstream/sniff/window/trang phát (hls.js, cặp, đơn, nhúng)/panel island/trang Video/cài đặt/phím tắt; nút video nằm cùng hàng nút phát trên peek. Kết quả chạy thật ở §11 |
| 2026-11-01 | Phát tiếp từ đoạn dở (`progress.rs`, thông báo + "Xem từ đầu"); làm mới link hết hạn cho cả trang dò và link trực tiếp, luôn từ URL gốc |
