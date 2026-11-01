# 01 · Lộ trình

> Cập nhật: 2026-11-01 · P1–P3 đã có source; Gate G0 và nghiệm thu Windows còn mở. Các mốc phiên bản trong bảng dưới là kế hoạch ban đầu; lịch phát hành thực tế xem [CHANGELOG](../CHANGELOG.md).

Bổ sung được chủ dự án yêu cầu trực tiếp: video nổi, maintenance, mở rộng kệ file, note desktop và meeting. Source được gom vào v0.1.2; phần chưa nghiệm thu không được coi là đã đóng gate. [Trạng thái hiện hành](05-handoff.md), [phạm vi ba module](plans/file-note-meeting.md).

## 1. Tổng quan

| Phase | Tên | Bản phát hành | Ước lượng* | Tính năng |
|---|---|---|---|---|
| P0 | Nền móng & PoC | (nội bộ) | 1,5–2 tuần | Hạ tầng repo, spike F01/F04/F10/F12 |
| P1 | Lõi MVP | v0.1 "dogfood" | 3 tuần | F01, F02 (khung), F03, F04, F06, F09, F10, F20, F21, F33 |
| P2 | Tính năng chính | v0.5 "beta" | 4 tuần | F02 (đủ), F11, F12, F22, F24, F30, F31 |
| P3 | Hoàn thiện | v1.0 | 3–4 tuần | F05, F07, F08, F13, F14, F15, F23, F25, F26, F27, F45 |
| P4 | Mở rộng | v1.x | liên tục | F32, F34, F40–F44 |

\*Ước lượng cho một người làm gần như toàn thời gian. Có AI hỗ trợ thì phần code nhanh hơn, nhưng phần tinh chỉnh chuyển động và trải nghiệm khó rút ngắn.

## 2. Quy tắc vận hành lộ trình

- **Tài liệu trước, code sau.** Mỗi tính năng bắt đầu bằng việc rà lại module doc. Đặc tả chưa đủ thì bổ sung trước.
- **Không chen tính năng.** Ý tưởng mới vào backlog ([00 §9](00-vision-scope.md#9-backlog-ý-tưởng)). Muốn kéo vào phase đang làm thì phải đổi ra một tính năng có độ lớn tương đương và ghi lý do vào handoff.
- **Phase kết thúc bằng tiêu chí, không bằng ngày.** Hết thời gian mà chưa đạt tiêu chí thì cắt phạm vi, không hạ tiêu chí.
- **Cổng quyết định (Gate)** là điểm dừng bắt buộc: viết báo cáo vào `docs/reports/`, rồi cập nhật ADR nếu có thay đổi.

## 3. P0 · Nền móng & PoC

**Mục tiêu:** chứng minh những phần rủi ro nhất chạy được trên Tauri/WebView2 trước khi viết code thật.

| Mã | Việc | Kết quả |
|---|---|---|
| P0.1 | Khởi tạo repo: `git init`, scaffold Tauri 2 + Svelte 5 + TS (pnpm), cấu trúc thư mục theo [03 §2](03-coding-principles.md#2-cấu-trúc-repo), cấu hình fmt/clippy/eslint/prettier, pipeline ts-rs, khung i18n | Repo build được, `pnpm tauri dev` chạy |
| P0.2 | CI GitHub Actions (`windows-latest`): fmt, clippy, test, svelte-check, vitest, build | Badge xanh |
| P0.3 | **Spike A – cửa sổ island:** trong suốt, topmost, `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW`, kích thước cố định, click xuyên theo vùng hit region + cursor poller, morph compact ⇄ peek bằng spring | Bản demo + số đo fps/CPU/RAM |
| P0.4 | **Spike B – fullscreen & phóng to:** `SHQueryUserNotificationState` + so khung cửa sổ, `SetWinEventHook` foreground, đặt lại topmost, kiểm tra virtual desktop | Bảng kết quả theo kịch bản |
| P0.5 | **Spike C – Media Session:** đọc phiên GSMTC (tên bài, ảnh bìa, timeline) và điều khiển play/pause/next | Demo với Spotify + Chrome |
| P0.6 | **Spike D – web player:** cửa sổ 2 webview (multiwebview), bridge script móc `navigator.mediaSession`, chạy test matrix 5 trang | Bảng test matrix điền đủ |
| P0.7 | Đo hiệu năng theo [02 §13](02-system-design.md#13-hiệu-năng), viết báo cáo `docs/reports/g0-poc.md` | Báo cáo Gate G0 |

**Gate G0, tiêu chí đi tiếp với Tauri:**

- [ ] Morph compact ⇄ peek đạt 60 fps, không giật thấy được trên máy có GPU tích hợp.
- [ ] Click xuyên đúng: không có click nào bị nuốt khi thao tác 5 phút với app bên dưới.
- [ ] Click vào island không làm mất focus của app đang gõ.
- [ ] Khi rảnh: CPU < 0,5%, RAM lõi (PCisland.exe + các tiến trình WebView2 của app) < 150 MB.
- [ ] Tự ẩn đúng với: YouTube toàn màn hình trong trình duyệt, một game borderless, PowerPoint trình chiếu.
- [ ] Web player: ít nhất 3/5 trang qua test matrix.

Trượt hai tiêu chí đầu thì đánh giá phương án dự phòng Avalonia + NativeAOT ([ADR-001](06-decision-log.md#adr-001--stack-tauri-2--rust--svelte-5)). Trượt tiêu chí web player thì thu hẹp danh sách trang hỗ trợ, không đổi stack.

**Quy tắc spike:** spike nằm trong nhánh `spike/*`. Code spike chỉ được đưa vào `main` khi đã viết lại cho đạt [nguyên tắc code](03-coding-principles.md).

## 4. P1 · Lõi MVP (v0.1 "dogfood")

**Mục tiêu:** người phát triển dùng được hằng ngày.

| Mã | Việc | Module |
|---|---|---|
| P1.1 | Hạ tầng: paths & portable mode, settings (schema + migration), storage (SQLite + migration), logging, EventBus, Clock | [settings-storage](modules/settings-storage.md) |
| P1.2 | Module registry + supervisor, ActivityHub (chính/phụ/cảnh báo/hành động) | [activity](modules/activity-hub.md) |
| P1.3 | Island: 4 trạng thái hiển thị, máy trạng thái frontend, visibility policy, hit region | [island](modules/island-window.md) |
| P1.4 | Vòng đời app: tray, autostart, chạy một bản, phím tắt, âm báo cơ bản, khởi động/tắt có thứ tự | [app](modules/app-lifecycle.md) |
| P1.5 | Media hub + nguồn ngoài GSMTC + đường ống ảnh bìa | [media](modules/media-hub.md) |
| P1.6 | Timer + stopwatch | [timer](modules/timer.md) |
| P1.7 | Màn hình chờ: đôi mắt chớp + đồng hồ | [indicators](modules/system-indicators.md) |
| P1.8 | Khung cửa sổ đầy đủ: title bar, điều hướng, trang Cài đặt chung, i18n vi/en | [ui-main](modules/full-window.md) |

**Tiêu chí hoàn thành:**

- [ ] Người phát triển dùng 5 ngày liên tục không crash, không lỗi nghiêm trọng.
- [ ] Đạt ngân sách hiệu năng của island và lõi.
- [ ] Unit test cho arbitration của ActivityHub, visibility policy, migration settings, logic timer.
- [ ] Checklist thủ công của các module P1 đều qua.
- [ ] Module doc và handoff được cập nhật.

## 5. P2 · Tính năng chính (v0.5 "beta")

**Mục tiêu:** 3–5 người dùng thử được một tuần.

| Mã | Việc | Module |
|---|---|---|
| P2.1 | Trình phát file trên máy: engine, thư viện, playlist, hàng đợi, SMTC của app, khôi phục phiên | [local_player](modules/local-player.md) |
| P2.2 | Web player: danh sách trang đã qua G0, toolbar, bridge, vòng đời ẩn/hủy, xóa dữ liệu | [web_player](modules/web-player.md) |
| P2.3 | Pomodoro + thống kê ngày | [timer](modules/timer.md) |
| P2.4 | Nhắc việc: CRUD, lặp lại, hoãn, nhắc bị lỡ, scheduler chịu được sleep/resume | [reminders](modules/reminders.md) |
| P2.5 | Chỉ báo âm lượng, pin & sạc | [indicators](modules/system-indicators.md) |
| P2.6 | Cửa sổ đầy đủ: trang Nhạc, Online, Hẹn giờ, Nhắc việc, cài đặt từng module | [ui-main](modules/full-window.md) |
| P2.7 | Bản phát hành đầu tiên trên GitHub Releases: bộ cài NSIS + zip portable + SHA256 + hướng dẫn SmartScreen ([ADR-009](06-decision-log.md#adr-009--phân-phối--ký-số)) | [system design §15](02-system-design.md#15-build-đóng-gói--phát-hành) |

**Tiêu chí hoàn thành:**

- [ ] 3–5 người dùng thử 1 tuần, không còn lỗi mức nghiêm trọng.
- [ ] Nhắc việc không bao giờ mất: đã kiểm thử sleep, tắt máy, đổi giờ hệ thống.
- [ ] Ngân sách RAM của web player được đo và ghi lại.
- [ ] Có hướng dẫn sử dụng ngắn (`docs/user-guide.md`).

## 6. P3 · Hoàn thiện (v1.0)

| Mã | Việc | Module |
|---|---|---|
| P3.1 | Đa màn hình & DPI | [island](modules/island-window.md) |
| P3.2 | Lời bài hát + visualizer | [lyrics/visualizer](modules/lyrics-visualizer.md) |
| P3.3 | Focus mode | [timer](modules/timer.md) |
| P3.4 | Nhắc việc bằng câu tiếng Việt (≥ 80 test case) | [reminders](modules/reminders.md) |
| P3.5 | Ghi chú nhanh | [notes](modules/quick-notes.md) |
| P3.6 | Kệ file | [shelf](modules/file-shelf.md) |
| P3.7 | Radio / link stream | [local_player](modules/local-player.md) |
| P3.8 | Theme, bộ âm báo, kiểm tra cập nhật, biểu cảm đôi mắt | [ui-main](modules/full-window.md), [app](modules/app-lifecycle.md) |
| P3.9 | Tối ưu hiệu năng tổng thể, hoàn thiện quy trình phát hành, đăng ký SignPath (nếu repo mã nguồn mở) | [system design §15](02-system-design.md#15-build-đóng-gói--phát-hành) |

**Tiêu chí hoàn thành:**

- [ ] Toàn bộ tính năng ưu tiên M và S đã xong, hoặc đã có quyết định ghi rõ việc dời lại.
- [ ] Đạt mọi ngân sách hiệu năng, có báo cáo `docs/reports/perf-1.0.md`.
- [ ] Release checklist ([03 §9](03-coding-principles.md#9-quy-trình-phát-hành)) qua toàn bộ.
- [ ] Đã có kết quả đăng ký SignPath, hoặc README đã có hướng dẫn SmartScreen cho bản không ký (ADR-009).

## 7. P4 · Mở rộng (v1.x)

Thứ tự đề xuất, xét lại sau v1.0 dựa trên phản hồi: F32 (mic/cam) → F40 (thời tiết) → F34 (theo dõi hệ thống) → F41 (lịch) → F44 (plugin API) → F42, F43 (cần nghiên cứu). Đặc tả sơ bộ ở [future-modules](modules/future-modules.md).

## 8. Quy ước phiên bản

- SemVer. Bản `0.x` trong P1–P2, `1.0.0` khi xong P3.
- `CHANGELOG.md` theo định dạng Keep a Changelog, có từ v0.1.
- Phiên bản được đồng bộ giữa `package.json`, `src-tauri/Cargo.toml` và `src-tauri/tauri.conf.json` bằng script `scripts/bump-version`.
