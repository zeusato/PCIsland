# Module: Ghi chú nhanh (`notes`)

> Trạng thái: Thiết kế · Phase: P3 · Tính năng: F26 · Cập nhật: 2026-10-31

## 1. Mục đích

Ghi lại một ý trong 3 giây mà không phải mở app nào: bấm phím tắt hoặc launcher, gõ, `Enter`. Có thể ghim một ghi chú lên island để luôn nhìn thấy, và đổi một ghi chú thành nhắc việc.

## 2. Phạm vi

**Trong phạm vi:**
- Tạo nhanh từ island, danh sách và tìm kiếm trong `main`.
- Ghim lên island, đổi thành nhắc việc, xuất `.md`.
- Định dạng nhẹ: xuống dòng, danh sách `-`, checkbox `[ ]`, link tự nhận.

**Ngoài phạm vi:**
- Trình soạn thảo phong phú, ảnh, đồng bộ, thư mục, thẻ phức tạp.

## 3. Hành vi & luồng người dùng

1. **Ghi nhanh:** `Ctrl+Alt+N` hoặc launcher › 📝 → island bung peek có ô nhập (island xin focus) → gõ → `Enter` lưu (`Shift+Enter` xuống dòng) → alert nhỏ "Đã lưu ghi chú" → focus trả về app trước.
2. **Ghim:** trong `main › Ghi chú`, bấm 📌 ở một ghi chú → khi rảnh, compact hiện icon ghim + đoạn đầu (≤ 24 ký tự); peek hiện toàn bộ (≤ 6 dòng) và cho tick checkbox trực tiếp.
3. **Đổi thành nhắc việc:** menu ghi chú › "Tạo nhắc việc" → mở form nhắc việc, điền sẵn tiêu đề (và giờ, nếu NLP nhận ra trong nội dung).
4. **Tìm:** ô tìm trong `main`, so khớp không dấu.
5. **Xóa:** hoàn tác trong 5 s; ghi chú vào thùng rác 30 ngày rồi mới xóa hẳn.

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `notes.pinned` | Ambient | Icon ghim + đoạn đầu | Nội dung (≤ 6 dòng), checkbox bấm được | — | `unpin`, `open` |
| `notes.saved` | Alert (ttl 1,5 s) | — | — | "Đã lưu ghi chú" | `open` |

**Hành động nhanh** (launcher): 📝 Ghi chú.

## 5. Thiết kế kỹ thuật

- Ghi chú là chuỗi văn bản thuần, kiểu markdown rút gọn. Render trong frontend bằng bộ phân tích nhỏ tự viết (chỉ danh sách, checkbox, link), **không** dùng thư viện markdown đầy đủ và **không** render HTML thô (chống XSS).
- Tick checkbox trên peek → `notes_toggle_check(id, line_index)`; backend sửa dòng tương ứng.
- Tìm kiếm: FTS5 nếu có, nếu không thì `LIKE` trên cột `normalized` (bỏ dấu), giống [local-player](local-player.md).
- Chỉ ghim được **một** ghi chú (v1).
- Đổi thành nhắc việc: frontend gọi `reminders_parse` với nội dung để điền sẵn rồi mở form nhắc việc. Không gọi chéo module ở backend.
- Thùng rác: `deleted_at`; công việc dọn dẹp chạy mỗi lần khởi động.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `notes_get_state` | `() → NotesSnapshot { pinned?, count, rev }` |
| `notes_list` | `(query?: string, page) → Page<NoteDto>` |
| `notes_create` | `(text: string) → NoteDto` |
| `notes_update` | `(id, text) → NoteDto` |
| `notes_delete` / `notes_restore` | `(id) → ()` |
| `notes_pin` / `notes_unpin` | `(id?) → ()` |
| `notes_toggle_check` | `(id, lineIndex: u32) → NoteDto` |
| `notes_export` | `(ids?: string[]) → string` (đường dẫn file `.md` hoặc `.zip`) |

| Event | Payload |
|---|---|
| `notes://state` | `NotesSnapshot` |

## 7. Dữ liệu & cài đặt

**Bảng:** `notes_items (id, text, normalized, pinned, created_at, updated_at, deleted_at)` + `notes_items_fts` (nếu có).

| Khóa | Mặc định |
|---|---|
| `notes.trash_days` | `30` |
| `notes.show_pinned_when_idle` | `true` |

## 8. Phụ thuộc

- **Core:** ActivityHub, storage, settings.
- **Island:** `island_request_focus` / `island_release_focus` (phía frontend).
- **Shared:** chuẩn hóa bỏ dấu (`shared/fuzzy`).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Đang gõ thì mất focus (click ra ngoài) | Tự lưu bản nháp vào `kv` (`notes.draft`), lần mở sau khôi phục |
| Ghi chú rất dài (> 10.000 ký tự) | Cho phép; peek chỉ hiện 6 dòng đầu |
| Bộ gõ tiếng Việt (Unikey/EVKey) trong ô nhập của island | **Kiểm thử thủ công bắt buộc** vì island chỉ tạm nhận focus |

## 10. Hiệu năng

Không có tác vụ nền ngoài dọn thùng rác lúc khởi động.

## 11. Kiểm thử

**Unit:** sửa checkbox theo dòng; chuẩn hóa tìm kiếm; dọn thùng rác theo TestClock.

**Checklist thủ công:**
- [ ] `Ctrl+Alt+N` → gõ tiếng Việt có dấu bằng Unikey/EVKey → `Enter` → focus về đúng app cũ
- [ ] Ghim → compact/peek hiện đúng; tick checkbox trên peek
- [ ] Đổi thành nhắc việc với "gọi lại cho khách 4h chiều"

## 12. Câu hỏi mở

Không có.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
