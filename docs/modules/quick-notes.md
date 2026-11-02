# Module: Ghi chú nhanh (`notes`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P3 · Tính năng: F26 · Cập nhật: 2026-11-01

## 1. Mục đích

Ưu tiên xem và tương tác với ghi chú đã lưu ngay trên island: mở ghi chú gần nhất, tick checklist, thêm mục; có nút Tạo mới riêng. Có thể ghim một ghi chú lên island để luôn nhìn thấy, và đổi một ghi chú thành nhắc việc.

## 2. Phạm vi

**Trong phạm vi:**
- Tạo nhanh từ island, danh sách và tìm kiếm trong `main`.
- Ghim lên island, đổi thành nhắc việc, xuất `.md`.
- Nhắc việc riêng cho từng dòng checklist: giờ cố định, đếm từ hiện tại hoặc từ khi dòng checklist ngay phía trên hoàn thành.
- Định dạng nhẹ: xuống dòng, danh sách `-`, checkbox `[ ]`, link tự nhận.

**Ngoài phạm vi:**
- Trình soạn thảo phong phú, ảnh, đồng bộ, thư mục, thẻ phức tạp.

## 3. Hành vi & luồng người dùng

1. **Ghi chú:** `Ctrl+Alt+N` hoặc launcher › 📝 → hiện ghi chú cập nhật gần nhất, tick trực tiếp; chọn ghi chú khác từ danh sách. Không tự mở editor, kể cả chưa có ghi chú. **Tạo mới** mở ô nhập → `Enter` lưu (`Shift+Enter` xuống dòng). **+ Ô tick** mở một ô nhập tên mục, lưu nối vào ghi chú đang xem; **Sửa** mới mở editor toàn bộ. Island chỉ xin focus bàn phím lúc nhập. Nội dung dài cuộn được; thay đổi từ main cập nhật qua event.
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
- Cả trang đầy đủ và peek dùng chung renderer checklist: nhận `[]`, `[ ]`, `[x]`, `[X]` và dạng Markdown `- [ ]` / `- [x]`; tick giữ đúng chỉ số dòng, thụt lề và dòng trống. Ghi chú cũ không cần migration. Xuất Markdown chuẩn hóa về `- [ ]` / `- [x]`.
- Form tạo ghi chú có nút **+ Ô tick** chèn `[ ] ` trên dòng mới; Enter lưu, Shift+Enter xuống dòng (không bắt Enter đang composition của bộ gõ). Trang đầy đủ có nút **Sửa**, tự focus editor và nút **Lưu / Hủy**; Ctrl+Enter lưu, Escape hủy. Bấm dòng checklist chỉ đổi trạng thái, không mở editor.
- Trang đầy đủ mặc định hiện danh sách mới cập nhật trước, không mở ô soạn; từng ghi chú có **+ Ô tick**, **Sửa**, ghim và xóa. Bấm nội dung thường không mở editor. Nút **Tạo mới** mở form riêng.
- Form tạo/sửa ghi chú và thêm mục checklist trên island giữ peek khi rời chuột cho tới khi editor đóng; dùng cơ chế chung của island, không dựa vào focus hiện tại.
- Tìm kiếm: FTS5 nếu có, nếu không thì `LIKE` trên cột `normalized` (bỏ dấu), giống [local-player](local-player.md).
- Chỉ ghim được **một** ghi chú (v1).
- Đổi thành nhắc việc: frontend gọi `reminders_parse` với nội dung để điền sẵn rồi mở form nhắc việc. Không gọi chéo module ở backend.
- Thùng rác: `deleted_at`; công việc dọn dẹp chạy mỗi lần khởi động.

### Nhắc từng dòng checklist (2026-11-01)

- Nút chuông cạnh từng ô tick mở form đặt giờ; mở form trên island xin focus và giữ peek khi rời chuột.
- Thời gian nằm ở cuối chính dòng checklist, trước nút chuông: `afterNow` và `afterPrevious` đã kích hoạt hiện đếm ngược cập nhật mỗi giây, `at` hiện ngày/giờ hẹn; phụ thuộc chưa kích hoạt hiện “Chờ dòng trước · thời lượng”. Đã báo thì hiện “Đã đến giờ nhắc”; dòng đã tick không hiện thời gian. Timer chỉ tồn tại khi view có đếm ngược chưa hoàn thành.
- Ba chế độ: `at` (giờ địa phương chuyển sang epoch), `afterNow` (1 giây–365 ngày), `afterPrevious` (đếm từ lúc hoàn thành dòng checklist ngay phía trên, bỏ qua dòng văn bản thường). Dòng checklist đầu không có chế độ phụ thuộc.
- Bỏ tick dòng trước hủy deadline chưa báo của dòng sau; tick lại bắt đầu lại từ lần hoàn thành mới. Tick xong task thì bỏ cảnh báo của chính task. Xóa ghi chú/dòng task không phát nhắc; bỏ giờ nhắc xóa quy tắc.
- Migration 0008 thêm `note_tasks`: ID riêng, dòng, nhãn, trạng thái hoàn thành, quy tắc, deadline, thời điểm đã báo, ID dòng trước. Nhãn khớp giữ ID qua chèn/đổi thứ tự; đổi tên tại cùng vị trí giữ ID. Với nhãn trùng nhau, ghép theo thứ tự xuất hiện.
- Lịch được lưu SQLite và nạp lại khi mở app; đến hạn khi app đóng thì báo khi mở lại. `notes.task_due` là Critical, có hành động `complete`; dùng âm nhắc việc và toast dự phòng nếu island bị ẩn.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `notes_list` | `(query?: string) → Note[]` |
| `notes_save` | `(id?: string, text: string) → Note` |
| `notes_delete` / `notes_restore` | `(id) → ()` |
| `notes_pin` / `notes_unpin` | `(id?) → ()` |
| `notes_toggle_check` | `(id, lineIndex: u32) → NoteDto` |
| `notes_export` | `(path: string) → number` |
| `notes_tasks` | `(id: string) → NoteTask[]` |
| `notes_set_task_alarm` | `(taskId: string, input: TaskAlarmInput|null) → ()` |

| Event | Payload |
|---|---|
| `notes://state` | `Note|null` (báo thay đổi, client đọc lại danh sách/task) |

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

Scheduler ngủ tới deadline gần nhất (tối đa 60 giây dự phòng), thức ngay khi ghi chú/lịch nhắc thay đổi hoặc có sự kiện hệ thống. Không ghi DB theo tick giao diện.

## 11. Kiểm thử

**Unit:** sửa checkbox theo dòng; chuẩn hóa tìm kiếm; dọn thùng rác theo TestClock.

**Checklist thủ công:**
- [ ] `Ctrl+Alt+N` → ghi chú gần nhất, tick / thêm ô tick / chọn ghi chú khác; Tạo mới → gõ tiếng Việt bằng Unikey/EVKey → `Enter`
- [ ] Ghim → compact/peek hiện đúng; tick checkbox trên peek
- [ ] Đổi thành nhắc việc với "gọi lại cho khách 4h chiều"

## 12. Câu hỏi mở

Còn nghiệm thu desktop với bộ gõ, DPI 100/150/200%, rút màn hình và báo chuông khi đóng view. Chưa tự mở lại cửa sổ note sau restart; geometry dùng helper hiện có, chưa clamp đầy đủ theo work area/DPI.

### Mở rộng desktop (2026-11-01)

Nút Ghim ra desktop trên main/island mở tối đa một WebView `note-desktop`, chỉ tạo khi dùng. Có topmost, đóng, sửa/lưu/hủy, xóa/hoàn tác và A−/A+ (12–40 px, mặc định 18, bước 2), Ctrl+wheel đổi chữ. Countdown/giờ hẹn dùng renderer và scheduler chung, đóng view không hủy alarm. Ghim desktop độc lập ghim island.

IPC: `notes_desktop_open(id)`, `notes_desktop_state()`, `notes_desktop_options(font,topmost,id?)`. Lưu `notes.desktop.id/font/topmost/geometry` trong kv. Events notes được phát thêm tới desktop. Chọn note khác hoặc đóng khi có draft yêu cầu Lưu/Bỏ/Hủy; draft lưu localStorage để phục hồi.

`notes_desktop_open` là async command: tạo WebView2 ngoài main thread IPC để tránh deadlock Windows, theo coding principles §3.

`notes_save` nhận `expectedUpdatedAt`; CAS nguyên tử dùng updated_at tăng đơn điệu, từ chối ghi đè bản mới hoặc hồi sinh note đã xóa. Main/island/desktop truyền version khi sửa. Có test hai bản sửa, timestamp cùng mili giây và xóa trước lưu.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Xuất toàn bộ ghi chú ra một file Markdown (checkbox thành task list) |
| 2026-11-01 | Checklist tương tác dùng chung main/peek; nhận []/[ ]/[x]/[X]/Markdown, nút thêm ô tick, sửa với Lưu/Hủy và Enter lưu ghi chú mới. |
| 2026-11-01 | Theo yêu cầu chủ dự án: mặc định xem/tương tác ghi chú gần nhất; Tạo mới và thêm mục checklist riêng trên island/main, chọn ghi chú và cuộn nội dung trong island. |
| 2026-11-01 | Form tạo/sửa/thêm checklist giữ mở khi rời chuột, gồm ô trống và mất focus; 3 test tích hợp trên IslandRoot. |
| 2026-11-01 | Nhắc riêng từng task với ba kiểu giờ, phụ thuộc dòng checklist ngay trên, lưu bền và kiểm thử undo/đổi thứ tự/xóa. |
| 2026-11-01 | Thêm nút Danh sách ghi chú với list cuộn và chọn trực tiếp; ghi chú ghim cũng có nút mở trình chọn. |
| 2026-11-01 | Cửa sổ desktop, cỡ chữ/topmost, giữ draft và CAS chống ghi đè; dùng lại alarm/timer backend. |
| 2026-11-02 | Sửa nút đóng note desktop: `onCloseRequested` của Tauri cần quyền `core:window:allow-destroy`, capability thiếu nên đóng thất bại âm thầm. Ô tick co giãn theo cỡ chữ (tối thiểu 16 px, 1.1em), canh giữa dòng đầu, dấu tick là icon SVG 75% ô thay cho ký tự ✓ cỡ cố định. Chưa thử trên exe. |
