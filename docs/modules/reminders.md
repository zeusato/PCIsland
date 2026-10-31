# Module: Nhắc việc (`reminders`)

> Trạng thái: Thiết kế · Phase: P2 (F24), P3 (F25) · Tính năng: F24, F25 · Cập nhật: 2026-10-31

## 1. Mục đích

Nhắc đúng giờ và **không bao giờ mất nhắc**: tạo nhanh, lặp lại, hoãn, và xử lý nhắc bị lỡ khi máy tắt. Ở P3 có thêm nhập bằng câu tiếng Việt tự nhiên: "họp team 3h chiều mai", "uống nước mỗi 2 tiếng"…

## 2. Phạm vi

**Trong phạm vi:**
- CRUD nhắc việc, lặp lại (ngày, ngày làm việc, tuần, tháng, mỗi N phút/giờ trong khung giờ).
- Nhắc trước, hoãn, nhắc bị lỡ.
- Thông báo trên island, tùy chọn toast Windows.
- Bộ phân tích câu tiếng Việt `shared/nlp_vi` (P3).

**Ngoài phạm vi:**
- Đồng bộ lịch Google/Outlook: F41, P4.
- Công việc nhiều bước, dự án, thẻ: không phải app todo.

## 3. Hành vi & luồng người dùng

1. **Tạo nhanh:** launcher › 🔔 Nhắc việc → ô nhập (island xin focus) → gõ "nộp báo cáo 17h" → chip xem trước "Nộp báo cáo · Hôm nay 17:00" → `Enter` lưu, `Esc` hủy.
   - Trước P3 (chưa có NLP): ô nhập tiêu đề + chip giờ nhanh (`+15 phút`, `+1 giờ`, `Chiều nay 17:00`, `Sáng mai 9:00`, Tùy chỉnh…).
2. **Sắp đến giờ:** trong 15 phút trước giờ nhắc, island có activity Ambient chuông nhỏ + `16:45` (tùy chọn).
3. **Đến giờ:** alert lớn có glow nhịp và âm `chime`: "Nộp báo cáo · 17:00". Hành động: Xong · Hoãn 5 / 10 / 30 phút · Mở.
4. **Bỏ qua (không bấm gì):** âm dừng sau 60 s; nhắc chuyển trạng thái "Đã lỡ" và còn trong danh sách Đã lỡ.
5. **Máy tắt lúc 17:00, bật lại 18:30:** alert "Bạn đã lỡ 1 nhắc việc" → bấm thì mở `main › Nhắc việc › Đã lỡ`.
6. **Lặp lại:** "Uống nước" mỗi 2 giờ từ 9:00 tới 18:00 các ngày làm việc. Bấm Xong ở một lần chỉ hoàn thành lần đó.
7. **Quản lý:** `main › Nhắc việc` có các tab Hôm nay · Sắp tới · Đã lỡ · Hoàn thành; form sửa đầy đủ; xóa có thể hoàn tác 5 s.

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `reminders.upcoming` | Ambient | Icon chuông + giờ | Danh sách nhắc trong 2 giờ tới (tối đa 3) | — | `open`, `done` |
| `reminders.due` | Critical | — | — | Lớn: tiêu đề, giờ, ghi chú (1 dòng) | `done`, `snooze_5`, `snooze_10`, `snooze_30`, `open` |
| `reminders.missed_summary` | Alert (ttl 10 s) | — | — | "Bạn đã lỡ N nhắc việc" | `open_missed` |

**Hành động nhanh** (launcher): 🔔 Nhắc việc (mở ô nhập).

## 5. Thiết kế kỹ thuật

**Mô hình:**

```rust
pub struct Reminder {
    pub id: Uuid, pub title: String, pub note: Option<String>,
    pub due_at: DateTime<Utc>,            // next occurrence
    pub repeat: Repeat,
    pub lead_min: u16,                    // 0 = no early notice
    pub snoozed_until: Option<DateTime<Utc>>,
    pub status: ReminderStatus,           // Pending | Ringing | Missed | Done
    pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
}
pub enum Repeat {
    None, Daily, Weekdays, Weekly { days: WeekdaySet }, Monthly { day: u8 },
    Every { minutes: u32, window: Option<(NaiveTime, NaiveTime)>, days: WeekdaySet },
}
```

- `due_at` lưu UTC, nhưng lặp lại được tính theo **giờ địa phương**: 9:00 sáng vẫn là 9:00 khi đổi múi giờ hoặc đổi DST. Hàm thuần `next_occurrence(repeat, after_local, tz)`.

**Scheduler:** giống [timer](timer.md): một `sleep_until` tới mốc gần nhất trong số `due_at`, `snoozed_until` và `due_at - lead`. Tính lại khi nhận `Resumed`, `SystemTimeChanged`, hoặc khi có thay đổi.

**Phát nhắc:**
1. `status = Ringing`; `hub.upsert(reminders.due, Critical)`; `SoundService::play(chime, 60 s)`.
2. Xong → nếu có lặp lại: `due_at = next_occurrence`, `status = Pending`; nếu không: `Done`.
3. Hoãn → `snoozed_until = now + n`, `status = Pending`.
4. Hết 60 s không phản hồi → `Missed`. Nếu có lặp lại thì vẫn tạo lần kế tiếp.

**Nhắc bị lỡ khi khởi động hoặc wake:**
- Mọi `Pending` có mốc < now − 2 phút chuyển thành `Missed`; nhắc lặp lại được nhảy tới lần kế tiếp trong tương lai.
- Có ≥ 1 nhắc bị lỡ thì phát alert `missed_summary`.
- Mốc vừa qua < 2 phút thì vẫn phát bình thường (máy vừa wake).

**Toast Windows (tùy chọn, P3):** khi island đang `hidden` và `reminders.toast_fallback` bật, phát thêm một toast Windows qua `tauri-plugin-notification`. Cần AUMID: app đăng ký `HKCU\Software\Classes\AppUserModelId\PCisland` lúc bật tùy chọn này. **Kiểm chứng** với app không đóng gói trước khi đưa vào P3.

**NLP tiếng Việt (F25, `shared/nlp_vi`):**
- Thư viện thuần, không phụ thuộc app. API: `parse(input, now_local) -> Parsed { title, due: Option<LocalDateTime>, repeat: Option<Repeat>, confidence, spans }`.
- **Chuẩn hóa:** viết thường; bản bỏ dấu dùng để so khớp ("chieu" ≡ "chiều"); gộp khoảng trắng; "h"/"giờ"/"g"/"tiếng" đều hiểu là giờ.
- **Thành phần nhận diện:**

| Loại | Ví dụ |
|---|---|
| Tương đối | "sau 15 phút", "15p nữa", "1 tiếng nữa", "nửa tiếng nữa", "2h30p nữa" |
| Giờ | "3h", "15h30", "15:30", "3 giờ 15", "3h chiều", "9h sáng", "8h tối", "12h trưa", "nửa đêm" |
| Ngày | "hôm nay", "tối nay", "mai", "ngày mai", "sáng mai", "mốt", "ngày kia", "thứ 2", "thứ hai tuần sau", "chủ nhật", "25/12", "ngày 5" |
| Lặp | "mỗi ngày", "hằng ngày", "hàng ngày", "mỗi sáng 7h", "thứ 2 hàng tuần", "các ngày trong tuần", "mỗi 2 tiếng", "ngày 1 hàng tháng" |

- **Gỡ mơ hồ:**
  - Giờ 1–11 không có buổi → chọn mốc tương lai gần nhất trong {h, h+12}.
  - "mai" khi đang 0:00–4:00 → hỏi lại bằng chip ("Sáng nay" / "Ngày mai").
  - Có ngày mà không có giờ → 9:00.
- **Tiêu đề:** chuỗi gốc sau khi bỏ các đoạn đã nhận diện và các từ nối thừa ("lúc", "vào", "nhắc tôi", "nhớ").
- `confidence < 0.6` hoặc không nhận ra giờ → chip xem trước hiện "Chưa rõ giờ" và mở chọn giờ.
- Có hỗ trợ tiếng Anh cơ bản ("in 10 min", "at 5pm", "tomorrow 9am") vì chi phí thấp.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `reminders_get_state` | `() → RemindersSnapshot { upcoming, ringing, missedCount, rev }` |
| `reminders_list` | `(filter: "today" \| "upcoming" \| "missed" \| "done", page) → Page<ReminderDto>` |
| `reminders_create` | `(input: ReminderInput) → ReminderDto` |
| `reminders_update` | `(id, patch: ReminderPatch) → ReminderDto` |
| `reminders_delete` | `(id) → ()` (frontend giữ hoàn tác 5 s trước khi gọi) |
| `reminders_complete` | `(id) → ()` |
| `reminders_snooze` | `(id, minutes: u32) → ()` |
| `reminders_parse` (P3) | `(text: string) → ParsedPreview` (gọi khi gõ, debounce 150 ms) |

| Event | Payload |
|---|---|
| `reminders://state` | `RemindersSnapshot` |

## 7. Dữ liệu & cài đặt

**Bảng:** `reminders_items (id, title, note, due_at, repeat_json, lead_min, snoozed_until, status, created_at, updated_at, completed_at)`, có index trên `(status, due_at)`.

| Khóa | Mặc định |
|---|---|
| `reminders.show_upcoming_min` | `15` (0 = tắt) |
| `reminders.default_lead_min` | `0` |
| `reminders.snooze_options_min` | `[5, 10, 30]` |
| `reminders.default_time_when_date_only` | `"09:00"` |
| `reminders.toast_fallback` | `false` (P3) |

## 8. Phụ thuộc

- **Core:** ActivityHub, EventBus, storage, settings, Clock, SoundService.
- **Shared:** `nlp_vi` (P3).
- **Crate:** `chrono`, `chrono-tz` (hoặc lấy múi giờ hệ thống qua `windows`).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Máy tắt / sleep qua giờ nhắc | Luồng nhắc bị lỡ (§5) |
| Đổi múi giờ / DST | Lặp lại tính theo giờ địa phương; nhắc một lần giữ mốc UTC (đúng "thời điểm thật") |
| Hai nhắc trùng giờ | Hai Critical; hub hiện "+1" |
| Đang fullscreen | Theo `island.allow_alert_over_fullscreen`; tùy chọn toast |
| Ghi DB thất bại khi tạo | Báo lỗi, giữ nguyên ô nhập để không mất nội dung |
| Xóa nhầm | Hoàn tác trong 5 s |

## 10. Hiệu năng

Scheduler chỉ thức dậy đúng mốc. Hàm `parse` < 1 ms cho câu ≤ 100 ký tự.

## 11. Kiểm thử

**Unit (bắt buộc):**
- `next_occurrence` cho mọi kiểu lặp, gồm DST, cuối tháng (ngày 31), năm nhuận.
- Scheduler với TestClock: hoãn, lỡ, wake, nhắc trước.
- `nlp_vi`: **≥ 80 case** trong `shared/nlp_vi/tests/cases.toml` (đầu vào, `now`, kết quả mong đợi), gồm câu có dấu, không dấu, viết tắt và tiếng Anh.

**Checklist thủ công:**
- [ ] Tạo nhắc 1 phút, đợi đến giờ: alert, âm, hoãn, xong
- [ ] Tắt app qua giờ nhắc rồi mở lại: có nhắc bị lỡ
- [ ] Sleep qua giờ nhắc rồi wake
- [ ] Nhắc lặp "mỗi 2 tiếng 9h–18h ngày làm việc" chạy đúng 2 ngày liền

## 12. Câu hỏi mở

- Có cần tích hợp với Microsoft To Do / Google Tasks không? Đề xuất: không, xét cùng F41.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
