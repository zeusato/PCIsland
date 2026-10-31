# Module: Hẹn giờ, bấm giờ, Pomodoro, Focus (`timer`)

> Trạng thái: Thiết kế · Phase: P1 (F20, F21), P2 (F22), P3 (F23) · Tính năng: F20–F23 · Cập nhật: 2026-10-31

## 1. Mục đích

Mọi thứ liên quan tới đếm thời gian: hẹn giờ đếm ngược (nhiều bộ), bấm giờ, Pomodoro và Focus mode. Đặt được từ island trong ≤ 2 click, không cần gõ phím, và **không bao giờ sai giờ** kể cả khi máy sleep hay app khởi động lại.

## 2. Phạm vi

**Trong phạm vi:**
- Countdown (nhiều bộ, nhãn, preset, +1 phút).
- Stopwatch (một bộ, lap).
- Pomodoro (chu kỳ, thống kê ngày).
- Focus mode (bật theo Pomodoro hoặc bật tay).

**Ngoài phạm vi:**
- Báo thức theo giờ cố định trong ngày: đó là nhắc việc ([reminders](reminders.md)).
- Đồng bộ đa thiết bị.

## 3. Hành vi & luồng người dùng

1. **Đặt hẹn giờ từ island:** hover island rảnh → peek "Khởi chạy nhanh" → ⏱ Hẹn giờ → chip preset `1 · 3 · 5 · 10 · 15 · 25 · 45 · 60` phút (cuộn chuột trên số để chỉnh ±1 phút) → Bắt đầu.
2. **Đang chạy:** compact hiện vòng tiến độ + `mm:ss`. Peek có số lớn, nhãn, ⏸, +1 phút, ✕.
3. **Hết giờ:** alert lớn "Hết giờ · Luộc trứng", glow nhịp, âm `bell` lặp tối đa 60 s. Hành động: Dừng · +1 phút · Lặp lại.
4. **Nhiều hẹn giờ:** activity chính là bộ sắp hết nhất; peek liệt kê tối đa 3 bộ và "+N" (bấm vào để mở `main`).
5. **Bấm giờ:** compact hiện giây; peek hiện phần mười giây, nút Lap, ⏸, Đặt lại. Danh sách lap xem trong `main`.
6. **Pomodoro:** Bắt đầu → làm 25 phút (cam) → hết pha: âm `soft-tick` + alert → nghỉ 5 phút (xanh lá) → … → sau 4 lần làm thì nghỉ dài 15 phút. Peek hiện tên pha, chấm chu kỳ `●●○○`, ⏸, Bỏ qua pha, Dừng.
7. **Focus mode:** bật khi vào pha làm (nếu `focus.with_pomodoro`), hoặc bật tay từ peek/`main`. Trong Focus, alert không quan trọng bị chặn ([activity-hub](activity-hub.md)); island có viền cam mảnh để báo đang Focus.
8. **Máy sleep 10 phút trong lúc hẹn 5 phút:** khi wake, alert hết giờ hiện ngay, ghi chú "đã hết lúc 14:05".

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `timer.running` | Live | Vòng tiến độ + `mm:ss` | Số 40 px, nhãn, danh sách ≤ 3 bộ | — | `pause`, `resume`, `add_minute`, `cancel` |
| `timer.finished` | Critical | — | — | Lớn: "Hết giờ · nhãn", thời điểm hết | `stop`, `add_minute`, `repeat` |
| `stopwatch.running` | Live | Icon + `mm:ss` | `mm:ss.t` (10 Hz, chỉ khi đang hiện peek), lap gần nhất | — | `lap`, `pause`, `resume`, `reset` |
| `pomodoro.running` | Live | Vòng (cam: làm / xanh: nghỉ) + `mm:ss` | Tên pha, chấm chu kỳ, thời gian | — | `pause`, `resume`, `skip`, `stop` |
| `pomodoro.phase_end` | Alert (tự chuyển pha) / Critical (chờ người dùng) | — | — | "Hết pha làm, nghỉ 5 phút nhé" | `start_next`, `stop` |
| Bubble | — | Vòng tiến độ 24 px | — | — | Bấm để ghim |

**Hành động nhanh** (đăng ký vào launcher của island): ⏱ Hẹn giờ · ⏲ Bấm giờ · 🍅 Pomodoro (icon lucide tương ứng; emoji ở đây chỉ để minh họa).

## 5. Thiết kế kỹ thuật

**Mô hình thời gian, dựa trên deadline tuyệt đối:**

```rust
pub enum TimerState {
    Running { ends_at: DateTime<Utc> },
    Paused { remaining_ms: u64 },
    Ringing { ended_at: DateTime<Utc> },
}
pub enum StopwatchState {
    Running { started_at: DateTime<Utc>, accumulated_ms: u64 },
    Paused { accumulated_ms: u64 },
}
```

Hiển thị và kiểm tra hết giờ đều tính từ `ends_at - clock.now_utc()`, nên không tích lũy sai số và chịu được sleep.

**Scheduler (trong actor):**
- Giữ một `sleep_until` cho deadline gần nhất trong số mọi timer và pha Pomodoro.
- Khi có thay đổi (thêm, tạm dừng, xóa) thì tính lại.
- Khi nhận `AppEvent::Resumed` hoặc `SystemTimeChanged` thì kiểm tra mọi deadline đã qua và tính lại.
- Có kiểm tra dự phòng mỗi 30 s, chỉ khi có timer đang chạy.

**Ringing:**
- Gọi `hub.upsert(timer.finished, Critical)`, `SoundService::play(bell, loop_until = now + 60 s)`.
- Hết 60 s mà chưa xác nhận: dừng âm, activity giữ nguyên nhưng hết glow nhịp và ghi "Đã hết lúc …".

**Pomodoro:**
- Máy trạng thái `Work → ShortBreak → Work → … → LongBreak` theo `long_every`.
- `auto_start_breaks`, `auto_start_work` quyết định chờ người dùng (Critical) hay tự chuyển (Alert).
- Mỗi pha xong (hoàn tất hoặc bị bỏ qua) ghi một dòng `timer_pomodoro_sessions`.

**Focus mode:**
- Trạng thái `focus_active` trong actor; khi đổi thì phát `AppEvent::FocusModeChanged`.
- Tùy chọn trong `focus.*` đều đi qua EventBus, không gọi trực tiếp module khác:
  - `pause_media_on_break`: khi vào pha nghỉ, phát `AppEvent::MediaIntent(Pause)`; `media` chuyển lệnh tới nguồn hiện hành.
  - `start_playlist_id`: khi bắt đầu Focus, phát `AppEvent::MediaIntent(PlayLocalPlaylist(id))`; `local_player` nghe sự kiện này và phát playlist.

**Lưu trữ:** mọi timer, stopwatch và trạng thái Pomodoro được ghi vào DB mỗi khi đổi trạng thái (không ghi theo tick). Khởi động thì nạp lại; deadline đã qua thì chuyển sang Ringing (hiện "đã hết lúc …").

## 6. IPC

| Command | Chữ ký |
|---|---|
| `timer_get_state` | `() → TimerSnapshot { timers, stopwatch?, pomodoro?, focusActive, rev }` |
| `timer_start` | `(durationMs: u64, label?: string) → TimerDto` |
| `timer_pause` / `timer_resume` / `timer_cancel` | `(id: string) → ()` |
| `timer_add` | `(id: string, deltaMs: i64) → ()` |
| `timer_dismiss` | `(id: string) → ()` (tắt chuông) |
| `stopwatch_start` / `stopwatch_pause` / `stopwatch_reset` / `stopwatch_lap` | `() → ()` |
| `pomodoro_start` / `pomodoro_pause` / `pomodoro_resume` / `pomodoro_skip` / `pomodoro_stop` | `() → ()` |
| `pomodoro_stats` | `(range: DateRange) → PomodoroStats` |
| `focus_set` | `(active: bool) → ()` |

Lệnh nhanh trên island đi qua `activity_invoke`.

| Event | Payload |
|---|---|
| `timer://state` | `TimerSnapshot` |

## 7. Dữ liệu & cài đặt

**Bảng:**
- `timer_items (id, kind, label, duration_ms, state_json, created_at, updated_at)` (`kind`: countdown | stopwatch)
- `timer_laps (stopwatch_id, idx, at_ms)`
- `timer_pomodoro_state (id = 'current', state_json, updated_at)`
- `timer_pomodoro_sessions (id, phase, planned_ms, started_at, ended_at, completed)`

| Khóa | Mặc định |
|---|---|
| `timer.presets_min` | `[1, 3, 5, 10, 15, 25, 45, 60]` |
| `timer.ring_max_sec` | `60` |
| `pomodoro.work_min` / `short_break_min` / `long_break_min` | `25` / `5` / `15` |
| `pomodoro.long_every` | `4` |
| `pomodoro.auto_start_breaks` / `auto_start_work` | `true` / `false` |
| `focus.with_pomodoro` | `true` |
| `focus.pause_media_on_break` | `false` |
| `focus.start_playlist_id` | `null` |

## 8. Phụ thuộc

- **Core:** ActivityHub, EventBus, storage, settings, Clock, SoundService.
- Không phụ thuộc platform trực tiếp.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Sleep / tắt máy khi đang chạy | Deadline tuyệt đối → wake hoặc khởi động lại thì hiện "đã hết lúc …" |
| Đổi giờ hệ thống | Đếm ngược tính theo UTC, không bị ảnh hưởng bởi múi giờ. Đổi giờ UTC thật (đồng bộ NTP) gây sai lệch nhỏ; chấp nhận |
| Hai timer hết cùng lúc | Hai Critical; hub hiện cái mới nhất, kèm "+1" |
| Đang fullscreen khi hết giờ | Theo `island.allow_alert_over_fullscreen`; âm vẫn phát |
| Tạm dừng cảnh báo (`app_pause_alerts`) | Critical vẫn hiện nhưng không phát âm |

## 10. Hiệu năng

- Không có tick ở backend. Frontend: một `requestAnimationFrame` cho vòng tiến độ khi đang hiện; `mm:ss` cập nhật bằng timer căn theo đầu giây.
- Stopwatch hiện phần mười giây chỉ khi peek đang mở.

## 11. Kiểm thử

**Unit (TestClock):**
- Start/pause/resume/add với deadline chính xác.
- Hết giờ khi đang sleep (nhảy đồng hồ).
- Khôi phục từ DB với deadline đã qua.
- Pomodoro: chuỗi pha với `long_every`; auto start bật/tắt; bỏ qua pha.
- Focus bật/tắt theo pha.

**Checklist thủ công:**
- [ ] Đặt 1 phút từ island trong ≤ 2 click
- [ ] Sleep máy giữa chừng rồi wake
- [ ] Ba timer song song
- [ ] Pomodoro đủ một vòng (dùng cấu hình 1/1/2 phút để test nhanh)
- [ ] Focus chặn alert âm lượng nhưng vẫn cho nhắc việc

## 12. Câu hỏi mở

- Màu pha nghỉ của Pomodoro dùng xanh lá `--c-success` có lạc tông neon cam không? Đề xuất thử trong P2: nghỉ dùng cam nhạt `--c-accent-bright`, viền đứt nét.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
