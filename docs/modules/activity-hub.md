# Module: Activity hub (`activity`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P1 · Tính năng: F06 · Cập nhật: 2026-10-31

## 1. Mục đích

Quyết định island hiện gì. Các module khai báo activity; hub chọn activity **chính** (pill), activity **phụ** (bong bóng) và **alert** (tự bung), rồi định tuyến hành động của người dùng về đúng module. Hub là lõi của [ADR-002](../06-decision-log.md#adr-002--kiến-trúc-lấy-activity-làm-trung-tâm).

## 2. Phạm vi

**Trong phạm vi:**
- Mô hình `Activity` và thuật toán arbitration.
- Hàng đợi alert, ghim activity, bộ lọc Focus mode, tạm dừng cảnh báo.
- Định tuyến hành động.

**Ngoài phạm vi:**
- Cách vẽ từng activity: view trong `features/<module>/`.
- Ẩn/hiện cửa sổ: thuộc [island-window](island-window.md).

## 3. Hành vi & luồng người dùng

1. **Nghe nhạc rồi đặt timer 25 phút:** timer (Live 60) thành activity chính; nhạc (Media 50) lùi thành bong bóng phụ.
2. **Bấm vào bong bóng nhạc:** đổi chỗ chính và phụ (ghim nhạc), giữ cho tới khi một trong hai kết thúc.
3. **Chỉnh âm lượng:** alert nhỏ hiện 1,5 s rồi trả lại trạng thái trước đó. Chỉnh liên tục thì gộp vào cùng alert và kéo dài thời gian hiện.
4. **Nhắc việc đến giờ trong lúc có alert âm lượng:** Critical thay ngay alert âm lượng, ở lại tới khi người dùng bấm Xong hoặc Hoãn.
5. **Focus mode đang bật:** alert đổi bài và sạc pin bị chặn (không hiện, không âm). Nhắc việc vẫn hiện.

## 4. Activity

Hub không có activity riêng. Bảng mức ưu tiên dùng chung cho mọi module:

| Mức | Giá trị | Dùng bởi |
|---|---|---|
| Critical | 100 | `reminders.due`, `timer.finished`, `pomodoro.phase_end` (tùy cài đặt) |
| Alert | 80 | `indicators.volume`, `indicators.power`, `media.track_changed`, `app.update`, `shelf.dropped` |
| Live | 60 | `timer.running`, `stopwatch.running`, `pomodoro.running`, `indicators.mic_cam` (P4) |
| Media | 50 | `media.now_playing` (đang phát) |
| Ambient | 30 | `media.now_playing` (tạm dừng ≤ 30 phút), `reminders.upcoming`, `shelf.items`, `notes.pinned`, `app.alerts_paused` |
| Idle | 0 | `indicators.idle` (đôi mắt / đồng hồ) hoặc pill trống |

Danh sách đầy đủ các kind nằm ở mục 4 của từng module doc.

## 5. Thiết kế kỹ thuật

**Mô hình:** xem [02 §5](../02-system-design.md#5-hệ-thống-activity).

```rust
pub enum Lifetime { Persistent, Transient { ttl_ms: u32 } }
pub struct ActivityAction { pub id: ActionId, pub icon: IconId, pub label_key: String, pub primary: bool }
pub enum Placement { Auto, SecondaryOnly }
```

- Activity `Transient` đi vào hàng đợi alert.
- Activity `Persistent` tham gia chọn chính/phụ.
- Activity Critical luôn `Persistent` và được hiện theo kiểu alert lớn cho tới khi xác nhận.
- `Placement::SecondaryOnly` dành cho chỉ báo quyền riêng tư (mic/camera). Activity loại này:
  - Không bao giờ thành activity chính.
  - Khi có mặt, luôn chiếm vị trí phụ.
  - Không bị Focus mode hay tạm dừng cảnh báo ẩn đi.

**Arbitration** (hàm thuần `arbitrate(&Snapshot, &Filters) -> IslandState`):

```
candidates = persistent activities, sau khi lọc (focus, paused)
critical   = candidates có priority == Critical, mới nhất trước
if critical không rỗng:
    alert = critical[0] (dạng lớn); primary/secondary tính trên phần còn lại
primary = pinned nếu pinned còn tồn tại
          ngược lại max(candidates không phải SecondaryOnly) theo (priority, started_at)
secondary = activity SecondaryOnly nếu có
            ngược lại max(candidates \ {primary}, có source != primary.source)
alert (nếu chưa có Critical) = đầu hàng đợi transient
```

**Hàng đợi alert:**
- Hiện mỗi lần một alert. Thời gian hiện tối thiểu 1,2 s để không chớp.
- Cập nhật alert cùng `id` thì thay payload và đặt lại `ttl`, không xếp hàng mới.
- Tối đa 5 alert chờ; vượt quá thì bỏ alert cũ nhất cùng `source`.
- Island đang `hidden` (fullscreen): alert không phải Critical bị bỏ, không xếp hàng dồn lại.

**Bộ lọc:**
- **Focus mode** (`AppEvent::FocusModeChanged`): bỏ mọi Transient có `source != timer`; Critical vẫn qua.
- **Tạm dừng cảnh báo** (`app_pause_alerts`): bỏ mọi Transient. Critical vẫn hiện nhưng không phát âm.

**Ghim:** `activity_pin(id)` ghim một activity làm chính. Bỏ ghim khi activity đó bị gỡ hoặc khi gọi `activity_unpin`.

**Định tuyến hành động:** `activity_invoke(activity_id, action_id)` → tra `source` → `Module::handle_action`. Lỗi trả về frontend dạng `IpcError` để view hiện rung nhẹ, kèm tooltip lỗi.

**Phát state:** mỗi lần kết quả arbitration đổi (so sánh sâu), hub tăng `rev` và `emit_to(["island", "main"], "island://state", IslandState)`. Có debounce 16 ms để gộp nhiều cập nhật trong cùng một frame.

**API cho module** (`ActivityHubHandle`):

```rust
async fn upsert(&self, activity: Activity) -> Result<()>;       // insert or replace by id
async fn update_payload(&self, id: &ActivityId, payload: ActivityPayload) -> Result<()>;
async fn remove(&self, id: &ActivityId) -> Result<()>;
async fn remove_all_from(&self, source: ModuleId) -> Result<()>; // used when a module stops
async fn alert(&self, activity: Activity) -> Result<()>;         // shorthand for a Transient
async fn acknowledge(&self, id: &ActivityId) -> Result<()>;      // ends a Critical
```

## 6. IPC

| Command | Chữ ký |
|---|---|
| `activity_get_state` | `() → IslandState` |
| `activity_invoke` | `(activityId: string, actionId: string) → ()` |
| `activity_dismiss` | `(activityId: string) → ()` (đóng alert sớm hoặc xác nhận Critical) |
| `activity_pin` | `(activityId: string) → ()` |
| `activity_unpin` | `() → ()` |

| Event | Payload |
|---|---|
| `island://state` | `IslandState { primary?: ActivityDto, secondary?: ActivityDto, alert?: ActivityDto, pinnedId?: string, rev }` |

`ActivityDto` là discriminated union theo `kind` (ts-rs sinh), mỗi kind có payload riêng do module định nghĩa trong `dto.rs`.

## 7. Dữ liệu & cài đặt

Không lưu DB; state sống trong bộ nhớ. Activity bền vững (timer, nhắc việc) do module tự khôi phục khi khởi động.

| Khóa | Mặc định | Ghi chú |
|---|---|---|
| `activity.alert_min_display_ms` | `1200` | |
| `activity.secondary_enabled` | `true` | Tắt thì chỉ hiện activity chính |
| `activity.paused_media_ambient_min` | `30` | Phút giữ nhạc tạm dừng ở mức Ambient |

## 8. Phụ thuộc

- **Core:** EventBus (nghe `FocusModeChanged`, `VisibilityChanged`), registry (tra module nguồn), settings.
- Không phụ thuộc platform.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Module bị Disabled khi có activity đang hiện | `remove_all_from(source)` và tính lại |
| Hai Critical cùng lúc (2 nhắc việc trùng giờ) | Hiện cái mới nhất trước; alert hiện "+1" và xem tiếp được bằng cách vuốt hoặc bấm mũi tên |
| Activity ghim bị gỡ | Bỏ ghim, tính lại |
| `activity_invoke` cho activity đã biến mất | Trả `activity.not_found`, frontend bỏ qua im lặng |
| Bão cập nhật (nhiều payload mỗi giây) | Debounce 16 ms; module phải gửi mốc thời gian thay vì tick |

## 10. Hiệu năng

Arbitration chạy O(n) với n < 50; chỉ chạy khi có thay đổi. Event state chỉ phát khi kết quả khác lần trước.

## 11. Kiểm thử

**Unit (bắt buộc, ≥ 25 case):**
- Chọn chính/phụ theo ưu tiên và `started_at`; phụ phải khác source.
- Ghim / bỏ ghim / ghim activity đã biến mất.
- Critical chen ngang alert; hai Critical; xác nhận Critical.
- Gộp alert cùng id; thời gian hiện tối thiểu; giới hạn hàng đợi.
- Focus mode và tạm dừng cảnh báo lọc đúng.
- Island hidden thì bỏ alert thường, giữ Critical.

**Integration:** module giả phát chuỗi activity theo `TestClock`, kiểm tra chuỗi `IslandState` sinh ra.

## 12. Câu hỏi mở

- Thứ tự Live (timer) > Media (nhạc) đã hợp lý chưa? Đề xuất giữ nguyên, xem lại sau khi dùng thử ở P1.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Payload có kiểu theo từng module (ADR-010); EventBus + supervisor (ADR-011); activity rảnh không còn làm activity phụ |
