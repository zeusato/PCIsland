# Module: Trình phát file trên máy (`local_player`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P2 (F11), P3 (F13 radio) · Tính năng: F11, F13 · Cập nhật: 2026-10-31

## 1. Mục đích

Phát nhạc từ ổ đĩa ngay trong PCisland: mở file/thư mục, thư viện có tag và ảnh bìa, playlist, hàng đợi, shuffle/repeat, phím media. Đến P3 thêm radio online và link stream trực tiếp. Mục tiêu là "đủ dùng hằng ngày", không cạnh tranh với foobar2000 ([00 §3](../00-vision-scope.md#3-phi-mục-tiêu-ít-nhất-tới-v10)).

## 2. Phạm vi

**Trong phạm vi:**
- Engine phát (qua AudioEngine).
- Thư viện: thư mục theo dõi, quét, tag, ảnh bìa.
- Playlist người dùng, nhập/xuất m3u.
- Hàng đợi, shuffle, repeat, âm lượng riêng.
- Khôi phục phiên; theo thiết bị âm thanh mặc định.
- Radio / link stream HTTP (P3).

**Ngoài phạm vi:**
- Equalizer, ReplayGain, crossfade (backlog).
- Chỉnh sửa tag; tải nhạc; stream HLS (dùng web player).

## 3. Hành vi & luồng người dùng

1. **Phát nhanh:** kéo file nhạc thả vào island → hai vùng thả hiện ra: "Phát ngay" và "Cất vào kệ". Thả vào "Phát ngay" thì file được thêm vào đầu hàng đợi và phát.
2. **Thư viện:** `main › Nhạc › Thư viện`: thêm thư mục → quét nền với tiến độ → duyệt theo Bài hát / Album / Nghệ sĩ / Thư mục; tìm kiếm tức thì.
3. **Playlist:** tạo, đổi tên, thêm bài (chuột phải hoặc kéo), sắp xếp bằng kéo thả; nhập/xuất `.m3u8`.
4. **Hàng đợi:** "Phát tiếp theo" / "Thêm vào hàng đợi"; xem và sắp xếp lại trong `main`.
5. **Mở lại app:** khôi phục hàng đợi, bài và vị trí cuối, ở trạng thái **tạm dừng**.
6. **Rút tai nghe:** tạm dừng (giống điện thoại), tùy chọn.
7. **Radio (P3):** `main › Nhạc › Radio`: thêm đài (tên + URL hoặc nhập `.m3u`/`.pls`), phát; island hiện tên đài và tên bài từ metadata ICY.

## 4. Activity

Module không tự khai báo activity. Nó là một `MediaSource` (`SourceId = "local"`); [media-hub](media-hub.md) tạo `media.now_playing`. Riêng:

| Kind | Ưu tiên | Alert | Hành động |
|---|---|---|---|
| `local_player.scan_done` | Alert (ttl 4 s) | "Đã thêm 1.240 bài" | Mở thư viện |
| `local_player.file_error` | Alert (ttl 4 s) | "Không phát được: tên file" | Bỏ qua |

## 5. Thiết kế kỹ thuật

**Engine** (`modules/local_player/engine.rs`, chạy trên luồng `audio` của `core::audio`):
- `rodio` + `symphonia`. Định dạng v1: MP3, FLAC, WAV, OGG Vorbis, M4A (AAC-LC, ALAC). **Không hỗ trợ v1:** Opus, WMA, APE; gặp thì báo lỗi rõ ràng.
- Mỗi bài là một `Source` có lớp bọc đếm vị trí (`position_ms`) để trả về chính xác.
- Thêm sẵn bài tiếp theo vào `Sink` 5 s trước khi bài hiện tại hết, cho chuyển bài gần như liền mạch.
- Tua (seek) dùng `Sink::try_seek`; nếu định dạng không hỗ trợ thì giải mã lại tới vị trí mới.
- Âm lượng riêng `local_player.volume` (0–1, đường cong lũy thừa cho tai), độc lập âm lượng hệ thống.

**Thiết bị âm thanh:** platform `AudioDevices` (IMMNotificationClient) báo khi thiết bị mặc định đổi hoặc bị rút. Đổi thiết bị thì dựng lại `OutputStream` trên luồng `audio` và phát tiếp từ vị trí cũ. Bị rút và `pause_on_device_removed` bật thì tạm dừng.

**Thư viện** (`library.rs`):
- Thư mục theo dõi lưu trong `local_player_folders`.
- Quét bằng `walkdir` trên `spawn_blocking`: so `mtime` và `size` với DB, chỉ đọc tag của file mới hoặc đã đổi bằng `lofty`. Chèn theo lô 500 bài mỗi transaction.
- Tiến độ quét gửi qua `Channel<ScanProgress>`.
- Ảnh bìa không lưu DB, chỉ lưu cờ `has_artwork`. Khi cần thì đọc lại bằng `lofty`, hoặc lấy `cover.jpg`/`folder.jpg` cùng thư mục, rồi đưa vào `ArtworkCache`.
- Tìm kiếm: SQLite FTS5 trên title/artist/album. Nếu bản build SQLite không có FTS5 thì dùng `LIKE` với cột đã chuẩn hóa bỏ dấu (`normalized`).
- Theo dõi thay đổi thư mục (crate `notify`): P3, tùy chọn.

**Hàng đợi** (`queue.rs`, logic thuần):
- `Vec<QueueItem>` + `current_index`.
- Shuffle bằng Fisher–Yates trên phần chưa phát, giữ lịch sử để ⏮ quay lại đúng bài trước.
- Repeat: `off` · `all` · `one`.
- Bấm ⏮ khi vị trí > 3 s thì về đầu bài; ≤ 3 s thì về bài trước.

**Khôi phục phiên:** lúc `stop()` và mỗi 30 s khi đang phát, lưu `{queue, index, position_ms, shuffle, repeat}` vào `kv` với khóa `local_player.session`.

**Radio (F13, P3):**
- Tải HTTP bằng `reqwest` dạng stream, qua bộ đệm (`stream-download`) cho `symphonia` (MP3/AAC/OGG).
- Gửi header `Icy-MetaData: 1`, tách `StreamTitle` → cập nhật `title`.
- Mất kết nối thì thử lại 3 lần (2/5/10 s) rồi dừng và báo.
- Không hỗ trợ HLS (`.m3u8` dạng segment); gặp thì gợi ý mở bằng web player.

**MediaSource:** trả `SourceState` với `caps` đầy đủ (radio: không có seek/next/prev, trừ khi người dùng có danh sách đài).

**Yêu cầu từ module khác:** nghe `AppEvent::MediaIntent(PlayLocalPlaylist(id))` (Focus mode bắt đầu) rồi phát playlist đó từ đầu. Playlist không còn tồn tại thì ghi `warn` và bỏ qua.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `local_player_get_state` | `() → LocalPlayerSnapshot { queue, index, status, positionMs, positionAt, volume, shuffle, repeat, rev }` |
| `local_player_open_files` | `(paths?: string[]) → ()` (không có `paths` thì mở hộp thoại) |
| `local_player_play` | `(target: PlayTarget, startIndex?: u32) → ()` (`PlayTarget` = tracks · playlist · album · folder · station) |
| `local_player_enqueue` | `(trackIds: string[], mode: "next" \| "end") → ()` |
| `local_player_queue_move` / `local_player_queue_remove` / `local_player_queue_clear` | … |
| `local_player_set_volume` | `(volume: f32) → ()` |
| `local_player_set_shuffle` / `local_player_set_repeat` | … |
| `local_player_library_add_folder` | `(path?: string) → ()` |
| `local_player_library_remove_folder` | `(folderId: string) → ()` |
| `local_player_library_rescan` | `(onProgress: Channel<ScanProgress>) → ScanSummary` |
| `local_player_library_query` | `(query: LibraryQuery) → Page<TrackDto>` (phân trang, sắp xếp, lọc) |
| `local_player_playlist_*` | `create`, `rename`, `delete`, `add`, `remove`, `reorder`, `import_m3u`, `export_m3u` |
| `local_player_station_*` (P3) | `add`, `update`, `delete`, `list`, `import` |

Điều khiển cơ bản (play/pause/next/prev/seek) đi qua `media_*` khi `local` là nguồn hiện hành; command riêng chỉ dùng trong trang Nhạc.

| Event | Payload |
|---|---|
| `local_player://state` | `LocalPlayerSnapshot` (đổi bài, đổi hàng đợi, play/pause, tua) |
| `local_player://library-changed` | `{ added, updated, removed }` |

## 7. Dữ liệu & cài đặt

**Bảng:**
- `local_player_folders (id, path, added_at, last_scan_at)`
- `local_player_tracks (id, path UNIQUE, folder_id, title, artist, album, album_artist, track_no, disc_no, year, genre, duration_ms, has_artwork, size, mtime, normalized, created_at, updated_at)`
- `local_player_tracks_fts` (FTS5, nếu có)
- `local_player_playlists (id, name, created_at, updated_at)`
- `local_player_playlist_items (playlist_id, position, track_id)`
- `local_player_stations (id, name, url, favicon_url, created_at)` (P3)

| Khóa | Mặc định |
|---|---|
| `local_player.volume` | `0.8` |
| `local_player.resume_on_start` | `"paused"` (`"off"` · `"paused"` · `"play"`) |
| `local_player.pause_on_device_removed` | `true` |
| `local_player.watch_folders` | `false` (P3) |
| `local_player.prefer_folder_artwork` | `true` |

## 8. Phụ thuộc

- **Core:** AudioEngine, ArtworkCache, storage, settings, ActivityHub (alert).
- **Module:** `media` (trait `MediaSource`).
- **Platform trait:** `AudioDevices`.
- **Crate:** `rodio` (symphonia), `symphonia`, `lofty`, `walkdir`, `notify` (P3), `stream-download` (P3), `reqwest`.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| File bị xóa/di chuyển | Bỏ qua, phát bài kế, alert `file_error`; lần quét sau đánh dấu `missing` |
| File hỏng / định dạng không hỗ trợ | Như trên |
| Ổ mạng chậm hoặc mất kết nối | Quét có timeout cho mỗi file 5 s; bỏ qua và ghi `warn` |
| Thư viện 50.000+ bài | Quét theo lô; truy vấn có phân trang; UI dùng virtual list |
| Tên file Unicode / đường dẫn > 260 ký tự | Dùng `\\?\` prefix trong platform khi cần |
| Rút tai nghe | Tạm dừng theo cài đặt; dựng lại output khi có thiết bị mặc định mới |
| Đổi thiết bị mặc định khi đang phát | Dựng lại stream, phát tiếp không mất vị trí |
| Radio đổi định dạng giữa chừng / mất mạng | Thử lại 3 lần rồi dừng, báo người dùng |

## 10. Hiệu năng

- Khi đang phát: CPU < 1% (giải mã MP3/FLAC).
- Quét: không chặn UI; chạy ở mức ưu tiên luồng thấp.
- Không giữ ảnh bìa trong bộ nhớ ngoài LRU của ArtworkCache.

## 11. Kiểm thử

**Unit:**
- Hàng đợi: shuffle giữ lịch sử, repeat, ⏮ theo ngưỡng 3 s, chèn "phát tiếp theo".
- Phân tích m3u/pls.
- Chuẩn hóa bỏ dấu tiếng Việt để tìm kiếm.
- Tách ICY metadata.

**Integration:** quét thư mục fixture (`tests/fixtures/audio/` gồm mp3, flac, m4a, ogg, có tag tiếng Việt) và kiểm tra DB.

**Checklist thủ công:**
- [ ] Phát đủ 5 định dạng; tua; next/prev; phím media trên bàn phím
- [ ] Chuyển bài gần như liền mạch trong một album
- [ ] Rút/cắm tai nghe USB và Bluetooth
- [ ] Mở lại app: khôi phục đúng bài và vị trí
- [ ] Thư viện lớn (≥ 10.000 bài): quét, tìm kiếm, cuộn mượt

## 12. Câu hỏi mở

- Có cần hỗ trợ Opus (`.opus`) ở v1 không? Đề xuất: không, xét lại khi symphonia hỗ trợ.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | SMTC riêng (phím media, flyout Windows); nhập/xuất m3u; radio/stream HTTP; tự mở lại thiết bị âm thanh khi đổi thiết bị mặc định; nhận `MediaIntent` từ Pomodoro |
