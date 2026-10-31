# Module: Lời bài hát & visualizer (`lyrics`, `visualizer`)

> Trạng thái: Thiết kế · Phase: P3 · Tính năng: F14, F15 · Cập nhật: 2026-10-31

Hai module nhỏ cùng "tiêu thụ" `NowPlaying` và không phụ thuộc vào nhau, nên dùng chung một tài liệu.

---

## A. Lời bài hát (`lyrics`)

### A1. Mục đích

Hiện lời bài hát chạy theo nhạc: dòng hiện tại trên peek, toàn bộ lời cuộn theo trong `main › Nhạc`.

### A2. Phạm vi

**Trong phạm vi:**
- Tìm lời theo thứ tự: file `.lrc` cạnh file nhạc → lời nhúng trong tag (USLT/SYLT) → LRCLIB.
- Đồng bộ theo vị trí phát; chỉnh lệch thời gian cho từng bài.

**Ngoài phạm vi:**
- Dịch lời, karaoke từng chữ, gửi lời lên LRCLIB.

### A3. Hành vi

1. Bài mới bắt đầu → tìm lời ở nền. Có lời đồng bộ thì peek hiện dòng hiện tại (cam) và dòng kế (xám).
2. Chỉ có lời thường (không timestamp) → chỉ hiện trong `main`, không hiện trên peek.
3. Lệch thời gian → trong `main` có nút ±0,5 s; giá trị được lưu cho bài đó.

### A4. Activity

Không tạo activity riêng. Lời được gửi qua event, view `Peek` của media hiện thêm một hàng lời khi `lyrics` bật và có dữ liệu.

### A5. Thiết kế kỹ thuật

- Nghe `AppEvent::NowPlayingChanged`. Khóa bài = `normalize(artist) | normalize(title) | round(duration_s)`.
- **LRCLIB:**
  - Gọi `GET https://lrclib.net/api/get?track_name=&artist_name=&album_name=&duration=`.
  - 404 thì thử `GET /api/search?q=` và chọn kết quả có `duration` lệch ≤ 2 s.
  - Header `User-Agent: PCisland/<ver> (+repo url)`; timeout 5 s.
- Cache trong `lyrics_cache`, kể cả kết quả "không có lời" (TTL 7 ngày) để không gọi lại.
- Parser LRC (`shared/lrc`): `[mm:ss.xx]` (nhiều timestamp một dòng), `[offset:]`, bỏ qua thẻ meta.
- Gửi `lyrics://current { trackKey, lines: [{ timeMs, text }], synced, offsetMs }` một lần mỗi bài. Frontend tự tìm dòng hiện tại bằng vị trí nội suy, nên **không gửi theo từng dòng**.

### A6. IPC

| Command | Chữ ký |
|---|---|
| `lyrics_get_current` | `() → LyricsDto?` |
| `lyrics_set_offset` | `(trackKey: string, offsetMs: i32) → ()` |
| `lyrics_retry` | `() → ()` |

| Event | Payload |
|---|---|
| `lyrics://current` | `LyricsDto?` |

### A7. Dữ liệu & cài đặt

**Bảng:** `lyrics_cache (track_key PRIMARY KEY, source, synced, content, offset_ms, fetched_at)`

| Khóa | Mặc định |
|---|---|
| `lyrics.enabled` | `true` |
| `lyrics.online_enabled` | `true` (tắt thì chỉ dùng `.lrc`/tag) |
| `lyrics.show_on_peek` | `true` |

### A8. Trường hợp biên

| Tình huống | Xử lý |
|---|---|
| Bài tiếng Việt ít có trên LRCLIB | Chấp nhận; hiện "Chưa có lời" chỉ trong `main` |
| Tên bài có "(Official MV)", "[Lyrics]"… (nguồn web) | Chuẩn hóa: bỏ ngoặc và hậu tố phổ biến trước khi tìm |
| Nguồn không có duration | Dùng `search`, chọn kết quả đầu nếu khớp tên chính xác |
| Không mạng | Dùng cache; không có thì im lặng |

### A9. Kiểm thử

**Unit:** parser LRC (nhiều timestamp, offset, dòng trống); chuẩn hóa tên bài (≥ 20 case gồm tiếng Việt và tên YouTube); chọn kết quả search.

**Thủ công:** 10 bài quốc tế + 10 bài Việt, ghi tỉ lệ tìm thấy.

---

## B. Visualizer (`visualizer`)

### B1. Mục đích

Sóng nhạc nhảy theo âm thanh thật trên island: 4–5 thanh ở compact, 16 thanh ở peek.

### B2. Phạm vi

**Trong phạm vi:**
- Thu âm toàn hệ thống bằng WASAPI loopback, FFT, chia dải, làm mượt.
- Gửi khung hình tới island; chế độ giả lập khi không thu được.

**Ngoài phạm vi:**
- Thu âm riêng theo tiến trình (process loopback): backlog P4.
- Ghi âm.

### B3. Hành vi

- Chỉ chạy khi: có `NowPlaying` đang phát, island `normal`, và activity media đang hiện (chính hoặc phụ).
- Im lặng > 2 s thì thanh hạ dần về mức nghỉ, không giật.
- `visualizer.mode`:
  - `real`: âm thanh thật.
  - `fake`: sóng ngẫu nhiên mềm theo nhịp, không thu âm.
  - `off`: icon tĩnh.

### B4. Activity

Không tạo activity. View `Compact`/`Peek` của media nhận khung hình qua `Channel`.

### B5. Thiết kế kỹ thuật

- **Capture:** luồng `loopback` (platform `LoopbackCapture`): `IAudioClient` shared mode, `AUDCLNT_STREAMFLAGS_LOOPBACK | EVENTCALLBACK` trên thiết bị render mặc định. Trộn về mono `f32`.
- **Phân tích:** cửa sổ Hann 1024 mẫu, bước 512; `realfft`; gộp thành 16 dải theo thang log 60 Hz–12 kHz; đổi sang dB rồi chuẩn hóa về 0–1.
- **Làm mượt:** attack 0,6 / decay 0,15 cho mỗi khung; compact dùng 5 dải gộp từ 16.
- **Gửi:** 30 khung/giây qua `tauri::ipc::Channel<[u8; 16]>` (lượng tử 0–255). Frontend vẽ bằng `transform: scaleY`, không dùng canvas.
- Đổi thiết bị mặc định (`AudioDevices`) thì khởi động lại capture.
- **Bật/tắt theo điều kiện** (§B3): dừng hẳn capture khi không cần để CPU về 0.

### B6. IPC

| Command | Chữ ký |
|---|---|
| `visualizer_subscribe` | `(channel: Channel<Frame>) → ()` (island gọi một lần; backend chỉ đẩy khi đang chạy) |

### B7. Dữ liệu & cài đặt

| Khóa | Mặc định |
|---|---|
| `visualizer.mode` | `"real"` |
| `visualizer.fps` | `30` (15–60) |

### B8. Trường hợp biên

| Tình huống | Xử lý |
|---|---|
| Thiết bị không cho loopback (một số driver) | Tự chuyển `fake`, ghi `warn` |
| Âm thanh từ app khác (thông báo, game) cũng làm nhảy sóng | Chấp nhận (thu toàn hệ thống); process loopback để P4 |
| Thiết bị exclusive mode | Capture lỗi → `fake` cho tới khi có thể thu lại |

### B9. Hiệu năng

CPU khi chạy < 1% (FFT 1024 điểm × ~94 lần/giây là rất nhẹ); 0% khi dừng. Không cấp phát bộ nhớ trong vòng lặp.

### B10. Kiểm thử

**Unit:** chia dải log; làm mượt; lượng tử. Đầu vào là sóng sin tổng hợp ở các tần số đã biết.

**Thủ công:**
- [ ] Đo CPU khi chạy và khi dừng
- [ ] Đổi thiết bị âm thanh khi đang chạy
- [ ] Island ẩn (fullscreen) thì capture dừng

---

## Câu hỏi mở (chung)

Không có.

## Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
