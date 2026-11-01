# Modules

> Mỗi module có một tài liệu đặc tả theo cùng template. Đặc tả là hợp đồng: code làm khác đặc tả là lỗi, trừ khi đặc tả được sửa trước.

## 1. Danh sách

| Tài liệu | ID | Tính năng | Phase | Trạng thái |
|---|---|---|---|---|
| [island-window](island-window.md) | `island` | F01, F04, F05 | P0–P3 | Đang làm |
| [app-lifecycle](app-lifecycle.md) | `app` | F03, F08, F45 | P1–P3 | Đang làm |
| [activity-hub](activity-hub.md) | `activity` | F06 | P1 | Đang làm |
| [settings-storage](settings-storage.md) | `settings`, `storage` | Hạ tầng | P1 | Đang làm |
| [maintenance](maintenance.md) | `maintenance` | RAM toàn PC, dọn rác một chạm | Bổ sung theo yêu cầu | Đóng gói v0.1.2, còn nghiệm thu thực tế |
| [full-window](full-window.md) | `ui-main` | F02, F07, F09 | P1–P3 | Đang làm |
| [media-hub](media-hub.md) | `media` | F10 | P1 | Đang làm |
| [local-player](local-player.md) | `local_player` | F11, F13 | P2–P3 | Đang làm |
| [web-player](web-player.md) | `web_player` | F12 | P2 | Đang làm |
| [lyrics-visualizer](lyrics-visualizer.md) | `lyrics`, `visualizer` | F14, F15 | P3 | Đang làm |
| [timer](timer.md) | `timer` | F20–F23 | P1–P3 | Đang làm |
| [reminders](reminders.md) | `reminders` | F24, F25 | P2–P3 | Đang làm |
| [system-indicators](system-indicators.md) | `indicators` | F30–F34 | P1–P4 | Đang làm |
| [quick-notes](quick-notes.md) | `notes` | F26 | P3 | Đang làm |
| [file-shelf](file-shelf.md) | `shelf` | F27 | P3 | Đang làm |
| [meeting](meeting.md) | `meeting` | Ghi họp, highlight, transcript | Bổ sung theo yêu cầu | Có source/test, còn nghiệm thu native |
| [video-player](video-player.md) | `video` | F46 | Backlog | Đã code, chờ test tay |
| [future-modules](future-modules.md) | — | F40–F44 | P4 | Phác thảo |

Trạng thái: **Phác thảo** → **Thiết kế** → **Đang làm** → **Hoàn thành** (kèm phiên bản) → **Bảo trì**.

## 2. Bản đồ phụ thuộc

```
                      ┌──────────── platform/ (trait + impl Windows) ────────────┐
                      │ window · foreground · fullscreen · gsmtc · smtc · audio  │
                      │ endpoint · loopback · power · privacy · shell            │
                      └──────────────────────────┬────────────────────────────────┘
                                                 │ (qua trait)
┌───────────────────────── core/ ────────────────▼──────────────────────────────┐
│ registry · ActivityHub · EventBus · settings · storage · audio · artwork · clock│
└───▲─────────▲─────────▲──────────▲──────────▲──────────▲──────────▲───────────┘
    │         │         │          │          │          │          │
 island     media ◀── local_player, web_player (impl MediaSource)   │
    │         │                                                     │
    │         └─ AppEvent::NowPlayingChanged ──▶ lyrics, visualizer │
    │                                                               │
  timer · reminders · indicators · notes · shelf ───────────────────┘
                     │        │
                     └── shared/nlp_vi ──┘
```

- Mũi tên đặc duy nhất giữa hai module là `local_player`/`web_player` → `media`, qua trait `MediaSource`.
- `lyrics` và `visualizer` chỉ nghe `AppEvent::NowPlayingChanged`, không import `media`.
- `reminders` và `notes` dùng chung thư viện thuần `shared/nlp_vi`.
- `timer` ảnh hưởng tới hub qua `AppEvent::FocusModeChanged`; hub tự áp bộ lọc.

## 3. Template

Mọi module doc mới dùng đúng khung dưới đây. Mục nào chưa áp dụng thì ghi "Không áp dụng", không xóa mục.

```markdown
# Module: <Tên> (`<id>`)

> Trạng thái: Thiết kế · Phase: Px · Tính năng: Fxx · Cập nhật: YYYY-MM-DD

## 1. Mục đích
Một đoạn: module giải quyết việc gì cho người dùng.

## 2. Phạm vi
**Trong phạm vi:** …
**Ngoài phạm vi:** …

## 3. Hành vi & luồng người dùng
Các luồng chính, đánh số bước.

## 4. Activity
| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |

## 5. Thiết kế kỹ thuật
Thành phần, API nền tảng, luồng xử lý, thuật toán chính.

## 6. IPC
Commands (chữ ký) · Events (payload) · DTO chính.

## 7. Dữ liệu & cài đặt
Bảng DB (tiền tố module) · Khóa cài đặt + mặc định.

## 8. Phụ thuộc
Core, platform trait, crate.

## 9. Trường hợp biên & lỗi
| Tình huống | Xử lý |

## 10. Hiệu năng
Ngân sách riêng, kỹ thuật tiết kiệm.

## 11. Kiểm thử
Unit · Integration · Checklist thủ công.

## 12. Câu hỏi mở

## 13. Lịch sử thay đổi
| Ngày | Thay đổi |
```
