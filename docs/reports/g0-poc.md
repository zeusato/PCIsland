# Báo cáo Gate G0 · PoC (bản nháp)

> Cập nhật: 2026-10-31 · Trạng thái: **đang đo**. Các mục ghi "Cần chủ dự án test" phải chạy tay trước khi họp G0.

## 1. Đã làm

| Mã | Việc | Kết quả |
|---|---|---|
| P0.1 | Repo, scaffold Tauri 2.12 + Svelte 5.57 + TS 5.9 (pnpm), lint/format, ts-rs, i18n | Xong. `cargo fmt/clippy/test`, `pnpm check/test/build` đều sạch |
| P0.2 | CI GitHub Actions (`.github/workflows/ci.yml`) | Đã viết; chưa chạy vì repo chưa có remote |
| P0.3 | Spike A: cửa sổ island | Chạy được: trong suốt, dính mép trên, `NOACTIVATE + TOOLWINDOW`, click xuyên theo hit region, morph spring compact ⇄ peek, đôi mắt chớp |
| P0.4 | Spike B: fullscreen / phóng to | Đã code: `SetWinEventHook` foreground + `SHQueryUserNotificationState` + so khung; tự ẩn/minimal theo `VisibilityPolicy` |
| P0.5 | Spike C: GSMTC | Đã code: đọc phiên, ảnh bìa (qua `artwork://`), timeline, play/pause/next/prev |
| P0.6 | Spike D: web player | Đã code theo **phương án dự phòng** (1 webview, chưa multiwebview): hồ sơ cookie riêng, bridge Media Session, IPC remote chỉ `web_player_report` |

## 2. Số đo (máy dev, bản `tauri dev`, debug)

| Chỉ số | Ngân sách | Đo được | Ghi chú |
|---|---|---|---|
| CPU khi rảnh (20 s, island compact, đôi mắt chớp) | < 0,5% | **0,18%** | Đạt |
| RAM lõi (private bytes, exe + 6 tiến trình WebView2) | < 150 MB | **222 MB** | Bản debug + Vite HMR. Phải đo lại bằng bản release |
| Hover → peek | 250 ms trễ + morph | Chạy đúng (ảnh chụp) | Chưa đo fps bằng DevTools |

**Bản release** (`pnpm tauri build`, 2026-10-31, đã có đủ module P1–P3):

| Chỉ số | Ngân sách | Đo được | Ghi chú |
|---|---|---|---|
| Kích thước exe | < 20 MB | **15,5 MB** | Đạt. Bộ cài NSIS 4,3 MB |
| CPU khi rảnh (30 s) | < 0,5% | **0,37%** | Đạt, nhưng lúc đo có một bộ bấm giờ đang chạy (cập nhật mỗi giây) |
| RAM lõi (private, exe + 6 tiến trình WebView2) | < 150 MB | **189 MB** | **Chưa đạt.** Hướng xử lý: import icon theo từng file thay vì cả gói `@lucide/svelte`, gộp tiến trình WebView2, đo lại khi không có activity |

**Đo lại** (bản release có đủ module + EventBus/supervisor, visualizer, toast, thumbnail; import icon theo file; `--renderer-process-limit=1`; 2026-10-31, rảnh, không có activity, cửa sổ đầy đủ đóng):

| Chỉ số | Ngân sách | Đo được | Ghi chú |
|---|---|---|---|
| Kích thước exe | < 20 MB | **17,1 MB** | Đạt. Bộ cài NSIS 4,66 MB |
| CPU khi rảnh (20 s) | < 0,5% | **0,25%** | Đạt |
| RAM lõi (private, exe + 6 tiến trình WebView2) | < 150 MB | **189 MB** | **Vẫn chưa đạt.** Working set tổng 420 MB (phần lớn dùng chung). Riêng `pcisland.exe` ~46 MB; phần còn lại là WebView2 (browser, GPU, network, storage, renderer). Import icon theo file và giới hạn renderer không làm giảm đáng kể → nền WebView2 là phần chính. Hướng tiếp: thử `--disable-gpu` (cần kiểm tra nền trong suốt), cân nhắc nới ngân sách lên 200 MB trong ADR nếu không giảm được |

## 3. Lỗi đã gặp và sửa

- `$effect` morph đọc `size.current` nên chạy lại mỗi frame của spring (vòng lặp `Spring.set` bị Aborted). Đã sửa bằng `untrack`. Bài học thêm vào checklist review: effect không được đọc giá trị spring đang chạy.
- `lucide-svelte` dùng cú pháp Svelte 4 → chuyển sang `@lucide/svelte`.

## 4. Cần chủ dự án test (checklist G0)

- [ ] Morph mượt 60 fps (DevTools Performance; mở DevTools của island bằng chuột phải trong bản dev)
- [ ] Thao tác 5 phút với app bên dưới vùng island: không mất click
- [ ] Đang gõ trong Notepad, click island: Notepad không mất focus
- [ ] Tự ẩn với: YouTube toàn màn hình, game borderless, PowerPoint trình chiếu
- [ ] Cửa sổ phóng to → island thu thành vạch mảnh; hover vào thì bung lại
- [ ] Win+Tab đổi virtual desktop: island có hiện không (R10, **bắt buộc**)
- [ ] Spotify / YouTube trên Chrome: island hiện ảnh bìa, tên bài; ⏮ ⏯ ⏭ hoạt động
- [ ] Web player: điền test matrix trong [web-player §11](../modules/web-player.md#11-kiểm-thử) cho 5 trang
- [x] Đo RAM/CPU bằng bản release (`pnpm tauri build`) — xem §2; RAM chưa đạt ngân sách

## 5. Kết luận tạm

Chưa đủ dữ liệu để đóng G0. Các phần rủi ro nhất về kỹ thuật (cửa sổ trong suốt + click xuyên + morph, đọc GSMTC, IPC remote bị giới hạn) đã chạy được trên Tauri, chưa thấy lý do phải đổi stack.
