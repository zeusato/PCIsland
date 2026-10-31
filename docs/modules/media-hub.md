# Module: Media hub (`media`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P0 (spike C), P1 · Tính năng: F10 · Cập nhật: 2026-11-01

## 1. Mục đích

Một điểm duy nhất cho "đang phát gì". Module gom mọi nguồn (app khác qua GSMTC, trình phát file, web player) thành một `NowPlaying`, hiển thị trên island và cho điều khiển từ island. Chính module này cũng hiện thực F10: điều khiển Spotify, YouTube trên Chrome và các app khác mà không cần web player.

## 2. Phạm vi

**Trong phạm vi:**
- Trait `MediaSource`, `ExternalSource` (GSMTC), chọn nguồn hiện hành, `NowPlaying`.
- Activity media, ảnh bìa qua `ArtworkCache`.
- Đăng ký SMTC của PCisland khi nguồn hiện hành là local/web.

**Ngoài phạm vi:**
- Engine phát file: thuộc [local-player](local-player.md).
- Trình duyệt nhúng: thuộc [web-player](web-player.md).
- Lời bài hát, visualizer: thuộc [lyrics-visualizer](lyrics-visualizer.md).

## 3. Hành vi & luồng người dùng

1. **Bật Spotify phát nhạc:** island compact hiện ảnh bìa + sóng nhạc. Hover thì thấy tên bài, ca sĩ, thanh tiến độ, các nút ⏮ ⏯ ⏭.
2. **Đổi bài:** alert nhỏ "▶ Tên bài – Ca sĩ" trong 2,5 s (tắt được).
3. **Vừa có Spotify vừa có YouTube trên Chrome:** dùng nguồn vừa bắt đầu phát gần nhất. Peek có hàng chip nguồn (icon app) để đổi; chọn tay là ghim nguồn đó.
4. **Tạm dừng:** sau 30 phút không phát, activity rơi khỏi island.
   - Compact có thanh tiến trình khi nguồn báo vị trí/thời lượng, nội suy trong frontend; khi tạm dừng thì giữ nguyên. Minimal cũng hiển thị tiến trình. Ảnh tải lỗi dùng icon nhạc thay cho hình ảnh hỏng.
5. **Bấm ảnh bìa trong peek:** mở `main` tại `#/media`. Nếu nguồn là app ngoài thì có thêm nút "Mở app" (đưa cửa sổ app đó lên trước).

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `media.now_playing` | Media 50 (đang phát) / Ambient 30 (tạm dừng ≤ 30 phút) | Ảnh bìa 20 px trái · sóng nhạc phải (khi tạm dừng: icon ⏸ xám) | Ảnh bìa 64 · tên bài · ca sĩ · nguồn · thanh tiến độ (tua được nếu nguồn cho phép) · ⏮ ⏯ ⏭ · chip nguồn | — | `prev`, `play_pause`, `next`, `open_source` |
| `media.track_changed` | Alert (ttl 2,5 s) | — | — | Ảnh bìa + tên bài | — |
| Bubble | — | Ảnh bìa tròn 24 px, viền cam khi đang phát | — | — | Bấm để ghim làm activity chính |

## 5. Thiết kế kỹ thuật

**Trait nguồn:**

```rust
pub trait MediaSource: Send + Sync {
    fn id(&self) -> SourceId;                       // "external:Spotify.exe", "local", "web"
    fn subscribe(&self) -> watch::Receiver<Option<SourceState>>;
    fn command(&self, cmd: MediaCommand) -> BoxFuture<'_, Result<(), MediaError>>;
}

pub struct SourceState {
    pub app_name: String, pub app_icon: Option<ArtworkRef>,
    pub title: String, pub artist: Option<String>, pub album: Option<String>,
    pub artwork: Option<ArtworkRef>,
    pub status: PlaybackStatus,                     // Playing | Paused | Stopped | Changing
    pub position_ms: Option<u64>, pub position_at: DateTime<Utc>, pub rate: f32,
    pub duration_ms: Option<u64>,
    pub caps: MediaCaps,                            // play, pause, next, prev, seek, shuffle, repeat
    pub is_ad: bool,
}
```

**ExternalSource (GSMTC):**
- Luồng `gsmtc` (MTA) gọi `GlobalSystemMediaTransportControlsSessionManager::RequestAsync()` và nghe `SessionsChanged`, `CurrentSessionChanged`.
- Mỗi phiên nghe `MediaPropertiesChanged`, `PlaybackInfoChanged`, `TimelinePropertiesChanged`.
- Mỗi phiên GSMTC thành một `SourceId` riêng (`external:<SourceAppUserModelId>`).
- Thumbnail: `TryGetMediaPropertiesAsync().Thumbnail` → đọc stream → `ArtworkCache::put(bytes)`.
- Sự kiện GSMTC có thể tới dồn dập nên debounce 150 ms cho mỗi phiên.
- **Lọc:** bỏ phiên có AUMID của chính PCisland (do SMTC của app tự đăng ký).

**Chọn nguồn hiện hành** (hàm thuần `select_source(states, pinned, now)`):
1. `pinned` còn tồn tại → chọn nó.
2. Nguồn có `status == Playing` và chuyển sang Playing gần nhất.
3. Nguồn `Paused` có `last_playing_at` gần nhất, nếu còn trong `activity.paused_media_ambient_min`.
4. Không có gì → `None`.

**NowPlaying & phát sự kiện:**
- Mỗi khi nguồn hiện hành hoặc state của nó đổi:
  - `hub.upsert(media.now_playing)`
  - `emit media://now-playing`
  - `bus.send(AppEvent::NowPlayingChanged)`
- Tiêu đề bài đổi (cùng nguồn) thì phát thêm alert `media.track_changed` nếu `media.alert_on_track_change` bật.

**SMTC của PCisland:**
- Khi nguồn hiện hành là `local` hoặc `web`, module đăng ký `SystemMediaTransportControls` của app (qua `souvlaki` với HWND của `island`). Việc này làm cho phím media trên bàn phím và flyout âm lượng của Windows hoạt động.
- Cập nhật metadata, trạng thái và timeline. Nhận nút bấm từ SMTC thì chuyển thành `MediaCommand` gửi tới nguồn.
- Nguồn hiện hành là app ngoài thì gỡ trạng thái SMTC của app (đặt Stopped), để không giành phím media với app đó.

**Yêu cầu từ module khác:** nghe `AppEvent::MediaIntent(Pause)` (ví dụ Focus mode vào pha nghỉ) rồi gửi `Pause` tới nguồn hiện hành. Không có nguồn nào thì bỏ qua.

**Ảnh bìa:** xem [02 §11](../02-system-design.md#11-kiến-trúc-media). `ArtworkRef` = `{ hash, width?, height? }`; frontend dùng `http://artwork.localhost/<hash>`.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `media_get_state` | `() → MediaSnapshot { nowPlaying?, sources: SourceSummary[], pinned?, rev }` |
| `media_play_pause` | `() → ()` |
| `media_next` / `media_previous` | `() → ()` |
| `media_seek` | `(positionMs: u64) → ()` |
| `media_select_source` | `(sourceId: string) → ()` (ghim) |
| `media_unpin_source` | `() → ()` |
| `media_focus_source_app` | `(sourceId: string) → ()` (đưa cửa sổ app ngoài lên trước) |

| Event | Payload |
|---|---|
| `media://now-playing` | `NowPlayingDto` (có `positionMs`, `positionAt`, `rate` để nội suy) |
| `media://sources` | `SourceSummary[]` |

## 7. Dữ liệu & cài đặt

Không có bảng DB.

| Khóa | Mặc định | Ghi chú |
|---|---|---|
| `media.external_enabled` | `true` | Tắt thì không đọc GSMTC |
| `media.alert_on_track_change` | `true` | |
| `media.ignored_apps` | `[]` | AUMID của các app không muốn hiện (ví dụ app họp trực tuyến) |
| `media.show_source_chips` | `true` | |

## 8. Phụ thuộc

- **Core:** ActivityHub, EventBus, ArtworkCache, settings.
- **Platform trait:** `MediaSessions` (GSMTC), `AppSmtc` (SMTC), `WindowActivator`.
- **Crate:** `windows` (`Media_Control`, `Storage_Streams`, `Foundation`), `souvlaki`.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| App không báo timeline (một số app) | Ẩn thanh tiến độ, chỉ hiện nút điều khiển |
| Không có thumbnail | Placeholder theo icon app |
| Chrome có nhiều tab đều phát | Mỗi tab là một phiên; chip nguồn cho chọn |
| App không hỗ trợ next/prev | Nút bị mờ theo `caps` |
| Lệnh GSMTC thất bại (`TrySkipNextAsync` trả false) | Rung nhẹ nút, không alert |
| GSMTC không khả dụng (Windows bản lạ) | Module media vẫn chạy với nguồn local/web; ghi `warn` |
| Quảng cáo (nguồn báo `is_ad`) | Peek ghi "Quảng cáo", ẩn nút next nếu nguồn không cho bỏ qua |

## 10. Hiệu năng

- Không poll GSMTC; chỉ dùng event.
- Frontend nội suy vị trí; backend chỉ gửi khi trạng thái hoặc vị trí thực sự đổi (tua, đổi bài, play/pause).
- Ảnh bìa cache theo hash, không gửi bytes qua IPC.

## 11. Kiểm thử

**Unit:**
- `select_source`: các tổ hợp ghim / đang phát / tạm dừng / hết hạn (≥ 12 case).
- Lọc phiên của chính app.
- Debounce sự kiện.

**Integration:** `FakeMediaSessions` phát chuỗi sự kiện, kiểm tra activity và event sinh ra.

**Spike C (P0.5):**
- [ ] Đọc được tên bài, ca sĩ, ảnh bìa, timeline từ Spotify và YouTube trên Chrome
- [ ] Play/pause/next hoạt động
- [ ] Độ trễ từ lúc đổi bài tới lúc island cập nhật < 500 ms

**Checklist thủ công:**
- [ ] Spotify, YouTube (Chrome/Edge), YouTube Music, Zing MP3 web, Windows Media Player
- [ ] Hai nguồn cùng phát → đổi nguồn bằng chip
- [ ] Phím media trên bàn phím khi nguồn là app ngoài → app ngoài nhận, PCisland không giành

## 12. Câu hỏi mở

- Có cần "ghép nguồn" khi web player và Chrome cùng mở một trang không? Đề xuất: không. Chúng là hai nguồn khác nhau.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Phát `NowPlayingChanged` qua EventBus; chip chọn nguồn trên peek khi có nhiều trình phát; `MediaIntent::Pause` |
| 2026-11-01 | Ẩn phiên GSMTC `msedgewebview2` khi web player đang báo qua bridge, tránh trùng nguồn và mất nút next/previous |
| 2026-11-01 | Tiến trình compact; fallback ảnh hỏng; ảnh nguồn web qua cache nội bộ. |
