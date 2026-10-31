# 05 · Handoff

> **Tài liệu sống.** Đây là điểm bắt đầu của mọi phiên làm việc, dù là người hay AI. Cuối mỗi phiên phải cập nhật §2, §3, §4 và thêm một dòng vào §7.

## 1. Cách dùng

- **Người mới:** đọc theo thứ tự tài liệu này → [00 Tầm nhìn](00-vision-scope.md) → [02 System design](02-system-design.md) → [03 Nguyên tắc code](03-coding-principles.md) → module doc liên quan.
- **Bắt đầu phiên:** đọc §2 và §3, chọn việc đầu tiên chưa xong ở §3, rồi mở module doc tương ứng.
- **Kết thúc phiên:** cập nhật snapshot (§2), sắp lại việc tiếp theo (§3), cập nhật câu hỏi mở (§4), thêm dòng nhật ký (§7). Nếu đã có quyết định kiến trúc thì thêm ADR.

## 2. Snapshot

| Mục | Giá trị |
|---|---|
| Cập nhật lần cuối | 2026-11-01 |
| Phase | **P1–P3 đã code đủ đặc tả** (trừ P4: F32–F34, F40–F44); chưa qua kiểm thử tay, Gate G0 chưa đóng |
| Phiên bản | 0.1.0 · [GitHub Release](https://github.com/zeusato/PCIsland/releases/tag/v0.1.0) · [ghi chú phát hành](releases/v0.1.0.md) |
| Code | Nhánh `main` trên https://github.com/zeusato/PCIsland |
| Kiểm tra | 2026-11-01: 156 test Rust xanh; 33 test frontend xanh (thêm 4 luồng giữ mở editor khi rời chuột); clippy `-D warnings`, fmt, svelte-check, eslint, i18n (260 khóa), frontend build sạch. Chờ chạy tay bản exe mới cho các sửa trong phiên Codex |
| Bản build gần nhất | 2026-11-01: `pnpm tauri build` v0.1.0; installer 4.913.695 byte, portable ZIP 6.704.723 byte và SHA256 trong `release/`. Exe chuẩn: `src-tauri/target/release/pcisland.exe` (ProductVersion/FileVersion 0.1.0). Đã xóa các exe thử và cache debug (~17,3 GB), giữ dữ liệu app. Số đo bản 2026-10-31: CPU rảnh 0,25%, RAM private 189 MB; chưa đo lại v0.1.0 |
| Đang chặn | Không có |

**Trạng thái tính năng** (✅ có code + test logic · 🟡 có code, còn thiếu phần trong đặc tả · ⬜ chưa làm):

| ID | Tính năng | Trạng thái | Ghi chú / còn thiếu |
|---|---|---|---|
| F01 | Island 4 trạng thái, click xuyên, morph | ✅ | — |
| F02 | Cửa sổ đầy đủ | ✅ | — |
| F03 | Tray, autostart, single instance, phím tắt, `--cleanup` | ✅ | — |
| F04 | Fullscreen / phóng to / khóa máy | ✅ | Chưa test tay; virtual desktop chưa kiểm chứng |
| F05 | Đa màn hình | ✅ | Chưa test tay với 2 màn hình |
| F06 | ActivityHub | ✅ | — |
| F07 | Theme / màu nhấn | ✅ | Island scale chưa làm (để P4) |
| F08 | Bộ âm báo | ✅ | Chọn âm cho hẹn giờ / nhắc việc / Pomodoro; chưa có âm tùy chỉnh từ file (P4) |
| F09 | i18n vi/en | ✅ | — |
| F10 | Media app khác (GSMTC) | ✅ | — |
| F11 | Trình phát file | ✅ | — |
| F12 | Web player | ✅ | Toolbar đặt ở tab Nhạc › Online (ADR-013); chờ test 5 trang |
| F13 | Radio / stream | ✅ | Chỉ stream HTTP trực tiếp; HLS mở bằng trình duyệt |
| F14 | Lời bài hát | ✅ | — |
| F15 | Visualizer thật | ✅ | — |
| F20–F22 | Hẹn giờ, bấm giờ, Pomodoro | ✅ | — |
| F23 | Focus mode | ✅ | — |
| F24 | Nhắc việc | ✅ | Toast dùng AUMID của PowerShell khi chạy portable/dev |
| F25 | Câu tiếng Việt tự nhiên | ✅ | 85 test case |
| F26 | Ghi chú nhanh | ✅ | — |
| F27 | Kệ file | ✅ | — |
| F30–F31 | Âm lượng, pin/sạc | ✅ | — |
| F32–F34 | Mic/cam, theo dõi hệ thống | ⬜ | P4 |
| F33 | Màn hình chờ đôi mắt / đồng hồ | ✅ | — |
| F45 | Kiểm tra cập nhật | ✅ | Chỉ báo có bản mới và mở trang Release; không tự cập nhật |

**Nợ kỹ thuật so với system design:**
1. ~~Module trait + supervisor~~ → đã có `Supervisor` (bắt panic, khởi động lại 1/5/30 s), ADR-011.
2. ~~EventBus/AppEvent~~ → đã có `core::events`, ADR-011.
3. ~~Payload Activity không có kiểu~~ → struct payload theo module + bảng `ActivityPayloads`, ADR-010.
4. Chưa có trait platform giả + `Clock` cho test tích hợp; logic thời gian test bằng tham số `now` truyền vào hàm thuần.
5. RAM release 189 MB > ngân sách 150 MB (phần lớn là nền WebView2). Cần thử `--disable-gpu` hoặc chốt nới ngân sách bằng ADR.
6. Visualizer, toast, thumbnail, kéo file ra ngoài, nhớ vị trí cửa sổ mới chỉ chạy qua test logic + build; chưa test tay.

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
- [x] Tạo repo GitHub và push: https://github.com/zeusato/PCIsland.
- [ ] **Test tay toàn bộ** theo checklist trong từng module doc (đặc biệt: nhắc việc qua sleep, fullscreen, kéo thả file, phát nhạc file, web player 5 trang).
- [ ] Kiểm tra sửa 2026-11-01: thumbnail Zing/YouTube thực; tiến trình compact/minimal; fullscreen và công tắc `hideWhileActive`; từ peek nhạc mở Tính năng → hẹn giờ/ghi chú; “nhắc tao sau 10 giây”; tick/sửa ghi chú cũ dạng `[]`.
- [ ] Kiểm tra bổ sung: mở Ghi chú thấy bản gần nhất, tick/thêm mục/chọn ghi chú cũ/Tạo mới; Pomodoro đổi màu theo pha, bấm lại pause/resume, toast đúng tùy chọn Focus/nhạc và bộ gõ tiếng Việt trên island.
- [ ] Hover mở island → soạn ghi chú mới/sửa/thêm ô tick/nhắc việc → rời chuột: editor phải giữ mở và giữ nội dung, kể cả ô trống; hủy rồi rời chuột thì thu lại bình thường.
- [x] Xử lý nợ kỹ thuật 1–3 ở §2 (ADR-010, ADR-011).
- [x] Phần còn thiếu theo bảng §2 (SMTC, web player, visualizer, radio, cập nhật, lyrics `.lrc`, toast, xuất `.md`, chip nguồn, âm theo loại, lỗi phím tắt, nhớ cửa sổ).
- [x] **P0.7 (một phần)** Đo bản release: CPU đạt, RAM chưa đạt.
- [ ] Giảm RAM WebView2 hoặc ra ADR nới ngân sách (nợ kỹ thuật 5).
- [x] Phát hành v0.1.0 trên GitHub Releases theo yêu cầu trực tiếp của chủ dự án (bộ cài + portable + SHA256); release notes nêu rõ phần test tay còn thiếu, Gate G0 chưa đóng.
- [ ] P4: mic/cam, theo dõi hệ thống (F32, F34), plugin API (F44).

## 4. Câu hỏi mở

Quyết định bổ sung của chủ dự án 2026-11-01: khi còn tác vụ mặc định không ẩn hoàn toàn island, có tùy chọn cho phép ẩn (ADR-014); phát hành bản mới nhất lên GitHub sau khi dọn output/cache build. Các giới hạn kiểm thử tay được giữ rõ trong release notes.

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
| 2026-10-31 | Claude | Hoàn thiện phần còn lại: EventBus + supervisor, payload có kiểu, đa màn hình, glow, SMTC, m3u, radio, visualizer thật, web player tự tắt/xóa dữ liệu, MediaIntent, thumbnail + kéo file ra, biểu cảm mắt, kiểm tra cập nhật, lyrics `.lrc` + chỉnh lệch, toast nhắc việc, xuất ghi chú `.md`, chip nguồn, âm theo loại, lỗi phím tắt, nhớ cửa sổ. ADR-010..013. 148 test Rust | Test tay; giảm RAM |
| 2026-11-01 | Claude | Sửa lỗi nghiêm trọng: mở cửa sổ đầy đủ từ island làm treo toàn bộ IPC (tạo cửa sổ WebView2 trong command đồng bộ bị deadlock) → `island_open_main`, `web_player_open` chuyển sang async. Kiểm tra bản release qua CDP: 22 command + 8 trang chạy đúng. Thêm favicon rỗng | Test tay |
| 2026-11-01 | Claude | Mắt chờ về hình gốc (biểu cảm mặc định tắt). Web player: cho phép popup đăng nhập theo chính sách §5 (sửa Zing trắng màn hình, SoundCloud/Google báo chặn popup); bỏ nguồn trùng `msedgewebview2` khi phát YouTube. Đã kiểm qua CDP: popup Google/Zalo hiện đúng, island dừng/phát YouTube được | Chủ dự án thử đăng nhập thật |
| 2026-11-01 | Codex | Sửa thumbnail online qua cache, tiến trình compact/minimal, tùy chọn ẩn (ADR-014), launcher khi đang phát nhạc, NLP giây, checklist và editor ghi chú. 156 test Rust; dọn cache incremental khi ổ D đầy | Kiểm tra tay bản exe mới theo §3 |
| 2026-11-01 | Codex | Ghi chú ưu tiên xem/tương tác gần nhất, thêm checklist vào ghi chú hiện tại, Tạo mới riêng; Pomodoro có màu/nhãn trạng thái và toast mô tả phạm vi Focus/cấu hình nhạc. 29 test frontend + 156 Rust xanh | Test tay bản notes-pomodoro, không đóng phiên app cũ tự động |
| 2026-11-01 | Codex | Giữ island mở khi có editor dù chuột rời vùng: kiểm tra chung input/textarea/contenteditable ngay khi hết collapse delay. 4 test tích hợp cho tạo/sửa ghi chú, thêm checklist và nhắc việc; tổng 33 frontend + 156 Rust xanh | Test tay bản editing với bộ gõ tiếng Việt |
| 2026-11-01 | Codex | Nâng phiên bản đồng bộ lên 0.1.0, cập nhật README/release notes, build installer + portable + SHA256, dọn ~17,3 GB cache/artifact thử và đưa mã nguồn/gói phát hành lên GitHub theo yêu cầu chủ dự án | Hoàn thiện kiểm thử tay và đo RAM cho bản phát hành |

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
