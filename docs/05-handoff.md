# 05 · Handoff

> **Tài liệu sống.** Đây là điểm bắt đầu của mọi phiên làm việc, dù là người hay AI. Cuối mỗi phiên phải cập nhật §2, §3, §4 và thêm một dòng vào §7.

## 1. Cách dùng

- **Người mới:** đọc theo thứ tự tài liệu này → [00 Tầm nhìn](00-vision-scope.md) → [02 System design](02-system-design.md) → [03 Nguyên tắc code](03-coding-principles.md) → module doc liên quan.
- **Bắt đầu phiên:** đọc §2 và §3, chọn việc đầu tiên chưa xong ở §3, rồi mở module doc tương ứng.
- **Kết thúc phiên:** cập nhật snapshot (§2), sắp lại việc tiếp theo (§3), cập nhật câu hỏi mở (§4), thêm dòng nhật ký (§7). Nếu đã có quyết định kiến trúc thì thêm ADR.

## 2. Snapshot

| Mục | Giá trị |
|---|---|
| Cập nhật lần cuối | 2026-10-31 |
| Phase | **P1–P3 đã có code cho phần lớn tính năng**; chưa qua kiểm thử tay, Gate G0 chưa đóng |
| Phiên bản | 0.0.1 (chưa phát hành) |
| Code | Nhánh `main`, commit `17a8cc6` (P1–P3) |
| Kiểm tra | 118 test Rust + 15 test frontend xanh; clippy `-D warnings`, svelte-check, eslint, i18n sạch |
| Bản build gần nhất | `pnpm tauri dev` chạy OK 2026-10-31 (island, launcher, bấm giờ, cửa sổ đầy đủ đã thử bằng ảnh chụp) |
| Đang chặn | Không có |

**Trạng thái tính năng** (✅ có code + test logic · 🟡 có code, còn thiếu phần trong đặc tả · ⬜ chưa làm):

| ID | Tính năng | Trạng thái | Còn thiếu so với đặc tả |
|---|---|---|---|
| F01 | Island 4 trạng thái, click xuyên, morph | ✅ | — |
| F02 | Cửa sổ đầy đủ | ✅ | Toast lỗi chung; nhớ kích thước/vị trí cửa sổ |
| F03 | Tray, autostart, single instance, phím tắt, `--cleanup` | ✅ | Hiện lỗi phím tắt trong trang Cài đặt (đang chỉ ghi log) |
| F04 | Fullscreen / phóng to / khóa máy | ✅ | Chưa test tay; virtual desktop chưa kiểm chứng |
| F05 | Đa màn hình | ⬜ | Đang cố định màn hình chính (đúng ADR-008), chưa có tùy chọn khác |
| F06 | ActivityHub | ✅ | Payload là JSON thay vì enum có kiểu (xem nợ kỹ thuật) |
| F07 | Theme / màu nhấn | 🟡 | Có màu nhấn; chưa có glow intensity, island scale |
| F08 | Bộ âm báo | 🟡 | 4 âm tổng hợp; chưa chọn âm cho từng loại |
| F09 | i18n vi/en | ✅ | — |
| F10 | Media app khác (GSMTC) | ✅ | Chip chọn nguồn trên peek |
| F11 | Trình phát file | 🟡 | Có thư viện, quét, tìm kiếm, hàng đợi, shuffle/repeat, playlist, khôi phục phiên; thiếu SMTC riêng (phím media), m3u, theo dõi đổi thiết bị |
| F12 | Web player | 🟡 | Phương án 1 webview; chưa có toolbar, tự hủy khi rảnh, xóa dữ liệu |
| F13 | Radio / stream | ⬜ | |
| F14 | Lời bài hát | 🟡 | LRCLIB + cache; thiếu `.lrc` cạnh file, chỉnh lệch |
| F15 | Visualizer thật | ⬜ | Đang dùng sóng giả bằng CSS |
| F20–F22 | Hẹn giờ, bấm giờ, Pomodoro | ✅ | — |
| F23 | Focus mode | 🟡 | Chặn cảnh báo đúng; chưa có `MediaIntent` (dừng nhạc khi nghỉ) |
| F24 | Nhắc việc | ✅ | Toast Windows dự phòng |
| F25 | Câu tiếng Việt tự nhiên | ✅ | 55 test case (đặc tả yêu cầu ≥ 80) |
| F26 | Ghi chú nhanh | ✅ | Xuất `.md` |
| F27 | Kệ file | 🟡 | Thả vào, mở, mở thư mục; chưa kéo ra ngoài, chưa thumbnail |
| F30–F31 | Âm lượng, pin/sạc | ✅ | — |
| F32–F34 | Mic/cam, theo dõi hệ thống | ⬜ | P4 |
| F33 | Màn hình chờ đôi mắt / đồng hồ | 🟡 | Chớp mắt có; biểu cảm & nhìn theo chuột (P3) chưa |
| F45 | Kiểm tra cập nhật | ⬜ | |

**Nợ kỹ thuật so với system design** (cần xử lý hoặc ghi ADR):
1. Chưa có `Module` trait + supervisor (§4): mỗi module là hàm `setup` + task actor; panic trong task chưa được bắt và khởi động lại.
2. Chưa có `EventBus`/`AppEvent` (§3): các module nói chuyện qua handle (`media.subscribe()`, `register_control`). Đây đúng tinh thần ranh giới module nhưng khác đặc tả.
3. Payload Activity là `serde_json::Value` (TS: `unknown`), view tự ép kiểu theo `kind`. ADR-002 yêu cầu enum có kiểu.
4. Chưa có trait platform giả + `Clock` cho test tích hợp; logic thời gian test bằng tham số `now` truyền vào hàm thuần.
5. Bản dev đo RAM 222 MB; chưa đo bản release.
6. Trong lúc test có bật một bộ bấm giờ: nếu còn chạy, vào Hẹn giờ › Bấm giờ › Đặt lại.

## 3. Việc tiếp theo

Theo thứ tự. Đánh dấu `[x]` khi xong và thêm việc mới phát sinh vào đúng chỗ.

- [x] **P0.1** git init, scaffold (viết tay, không dùng create-tauri-app), lint, ts-rs, i18n. Lệnh trong `CLAUDE.md` đã chạy thật.
- [x] **P0.2** CI GitHub Actions viết xong; **chưa chạy** (repo chưa có remote GitHub).
- [x] **P0.3** Spike A chạy được (xem [báo cáo G0](reports/g0-poc.md)).
- [~] **P0.4** Spike B đã code; **chờ test tay**: fullscreen, phóng to, virtual desktop.
- [~] **P0.5** Spike C đã code; **chờ test tay** với Spotify/Chrome.
- [~] **P0.6** Spike D đã code (phương án 1 webview); **chờ chủ dự án** đăng nhập và điền test matrix.
- [ ] **P0.7** Đo bản release; hoàn thiện `docs/reports/g0-poc.md`; họp Gate G0.
- [x] Commit đầu tiên `506dba9` trên `main`.
- [x] Commit phần P1–P3 (`17a8cc6`).
- [ ] Tạo repo GitHub và push (để CI chạy lần đầu).
- [ ] **Test tay toàn bộ** theo checklist trong từng module doc (đặc biệt: nhắc việc qua sleep, fullscreen, kéo thả file, phát nhạc file, web player 5 trang).
- [ ] Xử lý nợ kỹ thuật 1–3 ở §2 trước khi làm thêm tính năng.
- [ ] Phần còn thiếu theo bảng §2, ưu tiên: SMTC cho trình phát file (phím media) → toolbar web player → visualizer thật → radio → kiểm tra cập nhật.

## 4. Câu hỏi mở

| # | Câu hỏi | Đề xuất mặc định | Cần chốt trước | Trạng thái |
|---|---|---|---|---|
| Q1 | Island hiện gì khi rảnh? | Đôi mắt chớp (mặc định); đồng hồ / pill trống / ẩn là tùy chọn | P1.3 | **Đã chốt** 2026-10-31 |
| Q2 | Island ở màn hình nào? | Luôn hiện, trên mọi virtual desktop; nhiều màn hình thì ở màn hình chính | P1.3 | **Đã chốt** 2026-10-31 |
| Q3 | Island nổi cách mép trên hay dính mép trên? | Dính chặt mép trên (kiểu notch) | P0.3 | **Đã chốt** 2026-10-31 |
| Q4 | Phím tắt mặc định có trùng với app bạn đang dùng không? | Giữ mặc định ADR-008, đổi sau trong Cài đặt | P1.4 | **Đã chốt** 2026-10-31 |
| Q5 | Phát hành thế nào; có ký số không? | GitHub Releases, có bộ cài + portable; không mua chứng chỉ, xin SignPath miễn phí nếu đủ điều kiện (ADR-009) | P2.7 | **Đã chốt** 2026-10-31 |
| Q6 | Trang nhạc nào cần test ở G0? | Zing MP3, SoundCloud, YouTube, Spotify Web, NhacCuaTui | P0.6 | **Đã chốt** 2026-10-31 |
| Q7 | Có cần hỗ trợ Windows 10 không? | Không. Chỉ hỗ trợ chính thức Windows 11 | — | **Đã chốt** 2026-10-31 |

## 5. Môi trường phát triển

**Yêu cầu:**
- Windows 11 x64.
- Visual Studio 2022 Build Tools với workload "Desktop development with C++" (MSVC + Windows SDK).
- Rust stable qua `rustup`, toolchain `x86_64-pc-windows-msvc`.
- Node.js LTS (22.x) và pnpm.
- WebView2 Runtime (có sẵn trên Windows 11).
- Tauri CLI 2 (cài như devDependency `@tauri-apps/cli`).

**Khuyến nghị:** VS Code với các extension rust-analyzer, Svelte for VS Code, Tauri, Even Better TOML, ESLint, Prettier.

**Lệnh:** xem bảng trong [CLAUDE.md](../CLAUDE.md#lệnh). Bảng sẽ được kiểm tra lại sau P0.1.

**Mẹo debug:**
- Mở DevTools của webview `island` bằng chuột phải, chỉ ở bản dev.
- Log nằm ở `data/logs/`; chạy với `--debug` để có log chi tiết.
- Theo dõi RAM bằng Process Explorer: lọc `PCisland.exe` và `msedgewebview2.exe` (cột Command Line có đường dẫn user data của PCisland).

## 6. Rủi ro đang theo dõi

Sổ rủi ro đầy đủ ở [00 §8](00-vision-scope.md#8-sổ-rủi-ro). Những rủi ro cần kiểm chứng ngay trong P0:

| Rủi ro | Kiểm chứng ở |
|---|---|
| R1 Island đè vùng giữa mép trên | P0.3 + P0.4 |
| R2/R3 Đăng nhập Google & DRM trong WebView2 | P0.6 |
| R9 Multiwebview `unstable` | P0.6 |
| R10 Virtual desktop | P0.4 |

## 7. Nhật ký phiên

| Ngày | Ai | Đã làm | Tiếp theo |
|---|---|---|---|
| 2026-10-31 | Claude + chủ dự án | Brainstorm, đánh giá khả thi, bàn phương án nhạc online; viết toàn bộ tài liệu thiết kế (00–06, modules) | P0.1 scaffold repo |
| 2026-10-31 | Claude + chủ dự án | Chốt Q1 (màn hình chờ đôi mắt, đặc tả ở 04 §11), Q4, Q6 (5 trang test), Q7 (chỉ Win11) | Chốt Q2, Q3, Q5; P0.1 |
| 2026-10-31 | Claude + chủ dự án | Chốt Q2 (màn hình chính, mọi virtual desktop), Q3 (dính mép, 04 §5), Q5 (ADR-009: bộ cài + portable, không mua chứng chỉ). ADR-008 chuyển Accepted | P0.1 scaffold repo |
| 2026-10-31 | Claude | Cài toolchain (Rust 1.99, MSVC Build Tools, pnpm 9); P0.1–P0.6: scaffold, CI, spike island/fullscreen/GSMTC/web player; 33 test Rust + 15 test frontend xanh; app chạy, CPU rảnh 0,18% | Test tay checklist G0; commit đầu tiên |
| 2026-10-31 | Claude | P1–P3: core (settings, SQLite, log, ActivityHub, audio), timer/stopwatch/Pomodoro/Focus, nhắc việc + NLP tiếng Việt, âm lượng/pin, trình phát file, ghi chú, kệ file, lời bài hát, toàn bộ UI island + cửa sổ đầy đủ; 118 test Rust + 15 test frontend | Test tay; commit; xử lý nợ kỹ thuật §2 |

## 8. Thuật ngữ

| Thuật ngữ | Nghĩa |
|---|---|
| Island | Vùng hiển thị chính ở giữa mép trên màn hình |
| Compact | Trạng thái thu gọn mặc định của island |
| Peek | Trạng thái mở rộng nhỏ khi hover hoặc click |
| Alert | Trạng thái tự bung khi có sự kiện (âm lượng, nhắc việc…) |
| Minimal | Vạch mảnh khi cửa sổ khác đang phóng to và island rảnh |
| Hidden | Ẩn hẳn (fullscreen, khóa máy, người dùng ẩn) |
| Full window / `main` | Cửa sổ đầy đủ với dashboard và cài đặt |
| Activity | Đơn vị nội dung một module khai báo để hiển thị trên island |
| Primary / Secondary | Activity chính (pill) / phụ (bong bóng bên cạnh) |
| ActivityHub | Thành phần lõi chọn activity nào hiện ở đâu |
| Hit region | Vùng trên cửa sổ island nhận chuột; ngoài vùng này click đi xuyên qua |
| GSMTC | `GlobalSystemMediaTransportControlsSessionManager`: API đọc và điều khiển media của app khác |
| SMTC | `SystemMediaTransportControls`: API để chính app nhận phím media và hiện trên flyout của Windows |
| Web player | Trình duyệt nhúng chạy trang nhạc online, thu gọn được để chạy ngầm |
| Bridge script | Script chèn vào trang trong web player để đọc và điều khiển media |
| Portable mode | Bản zip có thư mục `data\` cạnh exe; dữ liệu nằm trong đó, không cần cài |
| Gate | Điểm quyết định bắt buộc giữa các phase |
