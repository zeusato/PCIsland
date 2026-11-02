# Module `focus_sounds` · Nhạc tập trung tải về

Ngày tạo: 2026-11-02 · Trạng thái: có source và test logic; release `focus-sounds-v1` đã phát hành (3 gói, 20 bài, 97 MiB); chưa nghiệm thu trên exe. Quyết định: [ADR-023](../06-decision-log.md#adr-023--nhạc-tập-trung-tải-theo-yêu-cầu-từ-repo-phát-hành-riêng).

## 1. Mục đích

Có sẵn nhạc tập trung/thư giãn cho Pomodoro mà bộ cài không phình. Nhạc nằm ở repo phát hành riêng `zeusato/pcisland-sounds`; người dùng tự bấm tải gói mình thích trong Cài đặt.

## 2. Phạm vi

- Đọc `manifest.json` của release mới nhất trong repo nhạc, liệt kê gói, dung lượng, trạng thái đã tải / có bản mới.
- Tải gói đã chọn, kiểm tra kích thước + SHA-256 từng file, có tiến trình và hủy. File sai hash bị xóa, không dùng.
- Xóa gói đã tải.
- Chọn gói dùng cho Pomodoro: gói vai trò `focus` phát khi vào pha làm việc, gói `break` phát khi vào pha nghỉ. Phát xáo bài, lặp cả danh sách qua trình phát nhạc trên máy.
- Ngoài phạm vi: stream trực tuyến, tự tải nền, tự cập nhật không hỏi, upload nhạc của người dùng.

## 3. Hành vi

1. Cài đặt › Pomodoro › **Nhạc tập trung**: bấm **Xem các gói** mới gọi mạng. Trước đó chỉ hiện gói đã tải (đọc đĩa).
2. Mỗi gói hiện tên, vai trò (Tập trung / Nghỉ), số bài, dung lượng; nút **Tải**, **Cập nhật** (release mới đổi hash) hoặc **Xóa**.
3. Tải: một lượt tại một thời điểm, thanh tiến trình theo byte, nút **Hủy**. Lỗi mạng/hash báo rõ file lỗi; phần đã tải xong của gói vẫn giữ để lần sau chỉ tải phần thiếu.
4. Gói đã tải có công tắc **Dùng khi Pomodoro**. Có ít nhất một gói được chọn thì playlist tập trung cũ (`pomodoro.focusPlaylistId`) bị bỏ qua.
5. Lần chạy đầu, hộp chào ở cửa sổ chính liệt kê các gói (tick sẵn) cùng yt-dlp; chỉ tải khi người dùng bấm **Tải đã chọn**, gói tải xong được bật cho Pomodoro. Không lấy được manifest thì báo tải sau trong Cài đặt.
6. Pomodoro vào pha làm việc → phát gói `focus`; vào pha nghỉ → phát gói `break` nếu có, nếu không thì theo cài đặt tạm dừng nhạc khi nghỉ. Dừng Pomodoro → tạm dừng nhạc nếu chính module này đã bật.

## 4. Repo nhạc và manifest

- Repo: `zeusato/pcisland-sounds`, chỉ chứa release nhạc. Không dùng repo dự án vì kiểm tra cập nhật app đọc `releases/latest` của repo đó.
- App tải `https://github.com/zeusato/pcisland-sounds/releases/latest/download/manifest.json`. URL từng file **do app tự ghép** `…/releases/download/<release>/<file>`, manifest không được chỉ định host khác.
- Asset của release là danh sách phẳng, nên tên file phải duy nhất giữa các gói. Script `scripts/focus-sounds-manifest.mjs` tự đặt tên `<gói>-<file>` và sinh manifest.

```json
{
  "schema": 1,
  "release": "focus-sounds-v1",
  "license": "Tạo bằng Suno (gói trả phí) bởi Zeusato; dùng kèm PCisland",
  "packs": [
    { "id": "lofi", "role": "focus", "name": { "vi": "Lo-fi", "en": "Lo-fi" },
      "tracks": [ { "file": "lofi-01-rainy-desk.mp3", "title": "Rainy Desk", "size": 7340032, "sha256": "…64 hex…" } ] }
  ]
}
```

Kiểm tra khi đọc: `schema = 1`; `release`, `id`, `file` chỉ gồm `[a-z0-9._-]`, file kết thúc `.mp3`; vai trò `focus` hoặc `break`; hash 64 hex; mỗi file ≤ 50 MiB; tổng ≤ 1 GiB; tối đa 32 gói, 64 bài/gói; manifest ≤ 256 KiB. Sai một mục thì từ chối cả manifest.

Định dạng là **MP3** (khuyến nghị 160–192 kbps). Trình phát rodio của app không đọc Opus.

## 5. Lưu trữ

- `data_dir/focus-sounds/<gói>/<file>`; theo portable mode như dữ liệu khác. File tải dở là `<file>.part`, chỉ đổi tên sau khi khớp kích thước và SHA-256.
- `data_dir/focus-sounds/installed.json`: release, và mỗi gói đã tải đủ: vai trò, tên, danh sách file + hash. Trạng thái offline đọc từ đây và kiểm tra file còn tồn tại.
- Cài đặt: `pomodoro.focusSoundPacks: string[]` (ID gói dùng khi Pomodoro, tối đa 32, `[a-z0-9._-]`).

## 6. IPC

| Command | Ai gọi | Chữ ký |
|---|---|---|
| `focus_sounds_status` | main | `() → FocusSoundsStatus` (chỉ đọc đĩa) |
| `focus_sounds_catalog` | main | `() → FocusCatalog` (tải manifest) |
| `focus_sounds_download` | main | `(packs: string[], progress: Channel<FocusDownloadProgress>) → FocusSoundsStatus` |
| `focus_sounds_cancel` | main | `() → ()` |
| `focus_sounds_remove` | main | `(packs: string[]) → FocusSoundsStatus` |

Phát nhạc đi qua EventBus: timer phát `AppEvent::PomodoroPhase(Work | Break | Stopped)`; module này đổi thành `MediaIntent::PlayFiles { paths, shuffle: true }` hoặc `MediaIntent::Pause` cho local_player. Module không gọi thẳng timer hay local_player.

## 7. Kiểm thử

Unit: kiểm tra manifest (tên file, hash, giới hạn, host), ghép URL, so sánh gói đã tải với catalog, chọn file theo vai trò. Tải dùng hàm thuần nhận stream byte để test hash/size mà không gọi mạng. Frontend: Cài đặt hiện gói, tải có tiến trình, hủy, xóa, công tắc gói.

Nghiệm thu tay: tạo release `focus-sounds-v1` thật, tải một gói, ngắt mạng giữa chừng rồi tải lại, chạy Pomodoro qua pha làm việc → nghỉ → dừng.

## 8. Lịch sử

| Ngày | Thay đổi |
|---|---|
| 2026-11-02 | Phát hành `focus-sounds-v1` lên `zeusato/pcisland-sounds`: `lofi` (Lo-fi & Jazz, tập trung, 7 bài), `ambient` (tập trung, 9 bài), `rest` (Thư giãn, nghỉ, 4 bài); MP3 192 kbps. Đã kiểm tra URL manifest `latest` và hash một file tải về. |
| 2026-11-02 | Hộp chào lần đầu hỏi tải yt-dlp + gói nhạc. Repo `zeusato/pcisland-sounds` đã được chủ dự án tạo, chưa có release. 269 Rust + 92 frontend đạt. |
| 2026-11-02 | Tạo module theo yêu cầu chủ dự án: manifest từ repo nhạc riêng, tải có hash, gói theo vai trò, phát theo pha Pomodoro. |
