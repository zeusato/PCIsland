# 05 · Handoff

> **Tài liệu sống.** Đây là điểm bắt đầu của mọi phiên làm việc, dù là người hay AI. Cuối mỗi phiên phải cập nhật §2, §3, §4 và thêm một dòng vào §7.

## 1. Cách dùng

- **Người mới:** đọc theo thứ tự tài liệu này → [00 Tầm nhìn](00-vision-scope.md) → [02 System design](02-system-design.md) → [03 Nguyên tắc code](03-coding-principles.md) → module doc liên quan.
- **Bắt đầu phiên:** đọc §2 và §3, chọn việc đầu tiên chưa xong ở §3, rồi mở module doc tương ứng.
- **Kết thúc phiên:** cập nhật snapshot (§2), sắp lại việc tiếp theo (§3), cập nhật câu hỏi mở (§4), thêm dòng nhật ký (§7). Nếu đã có quyết định kiến trúc thì thêm ADR.

## 2. Snapshot

**Phiên 2026-11-02 (Claude):** máy dev mới cài lại toolchain (rustup stable, VS 2022 Build Tools + Windows 11 SDK + CMake qua winget, `pnpm install`); `.env` chưa có nên build local không nhúng key Groq. CI của commit v0.1.2 đỏ vì CMake 4 từ chối `audiopus_sys` (`cmake_minimum_required` < 3.5) → đặt `CMAKE_POLICY_VERSION_MINIMUM=3.5` trong `src-tauri/.cargo/config.toml`. Sửa note desktop: nút đóng (thiếu `core:window:allow-destroy`) và ô tick lệch khi đổi cỡ chữ. Spotify: queue/bài hiện tại/repeat từ connect-state + dealer WebSocket (trace thật), SoundCloud: queue panel Next up, repeat nút trang, bật lại tìm kiếm (ADR-021). Dọn dẹp mở rộng nguồn rác, thùng rác/Windows Update và đóng app nền ngốn RAM (ADR-022). Kiểm tra: 251 Rust đạt/4 ignored, 87 frontend/27 file, fmt, clippy all-targets, svelte-check/eslint, i18n 542 khóa đạt. `.env` đã có `GROQ_API_KEY`. Đã build exe chạy thử bằng `pnpm build:exe` (10:27, 21.948.416 byte, vẫn ProductVersion 0.1.2, SHA256 `25F17A7696EBEEF848E8E3E80F5B7850C5292A5DE7A0EE232ED52A609DD912A4`), wrapper dọn 9,616 GiB file sinh ra (ổ trống tăng 8,005 GiB); chưa tạo installer/release. Chưa chạy Dọn dẹp/UAC/đóng app thật; chủ dự án tự nghiệm thu Spotify/SoundCloud.

**Bản phát hành v0.1.2:** theo yêu cầu cập nhật docs, push code và release của chủ dự án. Exe và bộ cài ProductVersion 0.1.2; portable/NSIS và hai SHA256 trong `release/0.1.2/`, [trang release](https://github.com/zeusato/PCIsland/releases/tag/v0.1.2). Đã kiểm tra ZIP chỉ có exe, README, marker `data/.portable` và hash exe khớp bản build. Key transcript nhúng native theo yêu cầu; `.env` không nằm trong Git/gói source. Sửa thêm `notes_desktop_open` sang async. Build wrapper đã dọn 8,140 GiB file sinh ra, ổ đĩa tăng thực tế **7,458 GiB**; giữ exe, installer, portable và dữ liệu. Những đoạn bên dưới ghi lại từng phiên trước đó; bảng hiện hành đã cập nhật.

**Mới nhất — File / note desktop / meeting (2026-11-01):** đã triển khai bản thử theo yêu cầu trực tiếp trên nền commit `f4d320a`, giữ module video. Bỏ trần 3 file, chọn/kéo/copy/ZIP nhóm, đổi ảnh; note desktop có cỡ chữ/topmost/CAS; meeting WASAPI + Opus + đánh dấu 15 giây + notes/transcript Groq. Dropdown meeting đã sửa theo phản hồi, có fixture 400 px không tràn. Key native từ `.env` theo quyết định chủ dự án, ADR-020. [Báo cáo và giới hạn](reports/file-note-meeting-implementation-2026-11-01.md), [đặc tả meeting](modules/meeting.md). Đã compile/debug QA, **chưa rebuild release/installer**. User yêu cầu đóng QA và dọn cache; giữ dữ liệu QA/app.

Kiểm tra cuối: 238 Rust đạt/4 ignored; 81 frontend đạt; check/i18n 497 khóa, frontend build, fmt và clippy all-targets đạt. Native system capture/device enum đạt, mic bị Windows Access denied. Groq thật nhận WAV và Opus của mẫu công khai; chưa thu/upload âm thanh cá nhân. Các snapshot cũ bên dưới là lịch sử, không phải số test hiện tại.

**Bổ sung module Dọn dẹp (2026-11-01):** đã có source RAM toàn PC, dọn rác một chạm Windows/app/browser theo allowlist và log cũ; có trên launcher island/Cài đặt. [Phạm vi, API và giới hạn](modules/maintenance.md), ADR-019. Chưa đóng gói exe mới; không xóa rác thật/trim app thật trong phiên phát triển. Agent khác đang verify các module còn lại, không reset/format hàng loạt hoặc xóa cache target dùng chung khi họ còn chạy.

Kiểm tra bổ sung Dọn dẹp: 194 Rust đạt (1 smoke native mặc định ignored đã chạy riêng đạt), 77 frontend đạt, check/i18n 365 khóa, frontend build, fmt và clippy `--lib` đạt. Clippy `--all-targets` còn `items_after_test_module` tại `web_player/mod.rs:660`; không sửa chồng phần agent khác. Preview dùng số liệu giả, server và fixture đã dọn. Bản exe release cũ không đổi.

**Bổ sung Trình phát video nổi F46 (2026-11-01, Claude):** dán link YouTube/Facebook/trang có video → app tự lấy stream (yt-dlp tải khi người dùng đồng ý, proxy `vstream://`) và phát trong 1 cửa sổ nhỏ không quảng cáo; island điều khiển được, phím tắt `Ctrl+Alt+V`, trình phát nhúng làm đường lui. [Đặc tả](modules/video-player.md), ADR-018, [spike S2](reports/video-s2-2026-11-01.md). Kiểm tra: 227 Rust + 81 frontend đạt, clippy/svelte-check/eslint/i18n 418 khóa sạch; YouTube đã chạy thật trên exe 22:28. Exe chạy thử hiện tại **chưa** có bản sửa peek và giao diện cửa sổ video cuối cùng (cần build lại).

| Mục | Giá trị |
|---|---|
| Cập nhật lần cuối | 2026-11-02 |
| Phase | **P1–P3 đã code đủ đặc tả** (trừ P4: F32–F34, F40–F44); chưa qua kiểm thử tay, Gate G0 chưa đóng |
| Phiên bản | [v0.1.4](https://github.com/zeusato/PCIsland/releases/tag/v0.1.4): NSIS + portable + hai SHA256, theo yêu cầu chủ dự án. Nhạc tập trung: [focus-sounds-v1](https://github.com/zeusato/pcisland-sounds/releases/tag/focus-sounds-v1) |
| Code | Nhánh `main` trên https://github.com/zeusato/PCIsland |
| Kiểm tra | (2026-11-02) 251 Rust đạt/4 ignored; 87 frontend/27 file; i18n 542 khóa; trước đó 238 Rust/81 frontend; fmt, clippy all-targets -D warnings, Svelte/eslint, i18n 497 khóa, frontend và Tauri NSIS build đạt. Giới hạn native/meeting ghi trong release notes |
| Bản build gần nhất | `src-tauri/target/release/pcisland.exe`: 22.190.592 byte, ProductVersion 0.1.4, SHA256 `8D281E5433EB8E2B62C722590ACC93DC7B1147F1078C47B9B9A7BAD1E1A84516`. NSIS 5.990.122 byte; portable 7.906.394 byte. Build này còn dọn hết cache (9,4 GiB); từ nay wrapper giữ cache, dọn khi target > 20 GiB |
| Đang chặn | Không. SoundCloud đã đăng nhập/phát lại được (2026-11-02), tìm kiếm bật lại theo ADR-021; vẫn không tự thử lặp hoặc vượt challenge |

UI launcher mới (2026-11-01): nội dung rảnh lên 28 px, header ở y=16, tổng chiều cao 226 px với đủ 7 nút; chữ ký “by Zeusato” nét mảnh nghiêng neon cam ở đáy phải. Đã QA localhost, panel Hẹn giờ mở đúng; `pnpm check`, 51 test frontend và `pnpm build` đạt (296 khóa i18n). Đã build exe release bằng `pnpm tauri build --no-bundle` theo yêu cầu chạy thử, đầu ra `src-tauri/target/release/pcisland.exe`; chưa tạo lại installer hay phát hành.

Bổ sung sau phản hồi chạy thử (2026-11-01): chữ ký weight 400, opacity 0.8–1; launcher theo thứ tự Nhạc/Ghi chú/Nhắc việc/Pomodoro/Hẹn giờ/Bấm giờ/Home. Nhạc trên máy chọn thư mục (gồm thư mục con, không lẫn thư mục khác; đã cấp `dialog:allow-open` cho island). Nút repeat ở island và trang Nhạc chọn 1/list/off cho local và web; online chờ bridge xác nhận, báo lỗi khi không đọc được queue. Ghi chú hiển thị đếm ngược/giờ hẹn ngay cuối từng dòng. Đã QA bố cục bằng fixture offline; 61 test frontend + 168 Rust, check/i18n 301 khóa và clippy đạt. Đã build lại exe release 0.1.1 bằng `pnpm tauri build --no-bundle` tại `src-tauri/target/release/pcisland.exe`, có quyền mở hộp thoại. Website thật và hộp thoại native cần chủ dự án kiểm tra trên exe mới. Dev server và fixture QA đã dọn.

**Trạng thái tính năng** (✅ có code + test logic · 🟡 có code, còn thiếu phần trong đặc tả · ⬜ chưa làm):

Quy trình build mới: dùng `pnpm build:exe` / `pnpm build:app` để tự dọn khi thành công, hoặc `pnpm clean:build` sau build thủ công; [chi tiết](build-cleanup.md). Build phiên này đã thành công; wrapper ban đầu lỗi Get-FileHash sau build, đã đổi sang SHA256 .NET và chạy cleanup riêng thành công, không rebuild không cần thiết. Các bản phát hành trong `release/` vẫn là bản cũ, không thay bằng exe thử nghiệm. SHA256 exe mới: `5C44DF75F0F9EA2C0C49F43D5F858A37A691B2F6E088FF03143D9F05E40B602E`.

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
| F12 | Web player | 🟡 | Zing resume/queue/tìm→phát đã thử native. Spotify/SoundCloud có queue, repeat, tìm kiếm theo trace 2026-11-02 (ADR-021), mới có test logic; chủ dự án tự nghiệm thu |
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
| F46 | Trình phát video nổi | 🟡 | Code đủ B + A; YouTube đã chạy thật. Chưa thử tay Facebook, dò trang, link hết hạn, trình phát nhúng (S1) |

**Nợ kỹ thuật so với system design:**
1. ~~Module trait + supervisor~~ → đã có `Supervisor` (bắt panic, khởi động lại 1/5/30 s), ADR-011.
2. ~~EventBus/AppEvent~~ → đã có `core::events`, ADR-011.
3. ~~Payload Activity không có kiểu~~ → struct payload theo module + bảng `ActivityPayloads`, ADR-010.
4. Chưa có trait platform giả + `Clock` cho test tích hợp; logic thời gian test bằng tham số `now` truyền vào hàm thuần.
5. RAM release 189 MB > ngân sách 150 MB (phần lớn là nền WebView2). Cần thử `--disable-gpu` hoặc chốt nới ngân sách bằng ADR.
6. Visualizer, toast, thumbnail, kéo file ra ngoài, nhớ vị trí cửa sổ mới chỉ chạy qua test logic + build; chưa test tay.

## 3. Việc tiếp theo

Ưu tiên mới: nghiệm thu ba module theo [plan cập nhật](plans/file-note-meeting.md), đặc biệt kéo nhiều file thành attachment thật, note desktop nhiều DPI/chuông, quyền mic, meeting 60 phút và crash đúng lúc publish file. Chưa đóng các tiêu chí này bằng unit test. Bản release tiếp theo cần đo kích thước/RAM rồi dọn theo `pnpm clean:build`.

Theo thứ tự. Đánh dấu `[x]` khi xong và thêm việc mới phát sinh vào đúng chỗ.

- [ ] Nghiệm thu trên exe mới (2026-11-02): Spotify queue/chọn bài/repeat 3 chế độ/tìm→phát; SoundCloud queue/repeat/tìm kiếm (selector trang tìm kiếm chưa kiểm trên WebView thật); note desktop đóng được, ô tick ở cỡ 12–40; Dọn dẹp mới (UAC Windows Update, thùng rác, đóng app).
- [ ] Đẩy commit sửa CMake rồi xem CI xanh lại.
- [ ] Nhạc tập trung (ADR-023): release `focus-sounds-v1` đã có trên `zeusato/pcisland-sounds` (lofi 7, ambient 9, rest 4; 97 MiB). Còn nghiệm thu tải/hủy/xóa, hộp chào lần đầu và phát theo pha Pomodoro trên exe.
- [ ] Nghiệm thu đợt 2 (2026-11-02): nút chia sẻ link trên player island (web/video), ô launcher đồng cỡ + Cài đặt › Nút trên island, Dọn dẹp giữ tiến trình khi rời island, kéo file Explorer lên island (đã sửa, thử giả lập đạt trên bản debug).

- [ ] Sửa Zing trang bài hát riêng (`.song-header button.btn-play-all`) được đưa vào bản v0.1.2; còn nghiệm thu đổi từ Lee Phú Quý sang Chu Thúy Quỳnh. Yêu cầu dừng build cũ đã được thay bằng yêu cầu phát hành hiện tại.

- [ ] Kiểm tra native kéo file lên island đang phát nhạc ở compact/peek/minimal; kéo ra/hủy/thả file, kiểm tra đa DPI. Có test logic, chưa chạy end-to-end trên exe mới.
- [ ] Nghiệm thu [Dọn dẹp](modules/maintenance.md) trên exe khi build bản tiếp theo: RAM trước/sau/30 giây, nhạc không gián đoạn, dọn cache profile đã đóng và file tạm cũ; source/test đã có. Không chạy cleanup build dùng chung khi agent verify còn hoạt động.
- [ ] Khi đóng gói bản tiếp theo, kiểm tra launcher gọn và chữ ký “by Zeusato” trên WebView2 ở các mức DPI; frontend đã QA localhost.
- [ ] Bản thử mới: chọn/hủy thư mục trên island; repeat 1/list/off với local và từng website có queue (không thử lại SoundCloud đang bị chặn); ghi chú đếm ngược và giờ cố định, gồm phụ thuộc dòng trước.

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
- [ ] Kiểm tra bổ sung: mở Ghi chú thấy bản gần nhất, tick/thêm mục/chọn ghi chú cũ/Tạo mới; Pomodoro đổi màu theo pha, bấm lại **dừng hẳn**, toast native Windows 3 giây và bộ gõ tiếng Việt trên island.
- [ ] Hover mở island → soạn ghi chú mới/sửa/thêm ô tick/nhắc việc → rời chuột: editor phải giữ mở và giữ nội dung, kể cả ô trống; hủy rồi rời chuột thì thu lại bình thường.
- [x] Xử lý nợ kỹ thuật 1–3 ở §2 (ADR-010, ADR-011).
- [x] Phần còn thiếu theo bảng §2 (SMTC, web player, visualizer, radio, cập nhật, lyrics `.lrc`, toast, xuất `.md`, chip nguồn, âm theo loại, lỗi phím tắt, nhớ cửa sổ).
- [x] **P0.7 (một phần)** Đo bản release: CPU đạt, RAM chưa đạt.
- [ ] Giảm RAM WebView2 hoặc ra ADR nới ngân sách (nợ kỹ thuật 5).
- [x] Phát hành v0.1.0 trên GitHub Releases theo yêu cầu trực tiếp của chủ dự án (bộ cài + portable + SHA256); release notes nêu rõ phần test tay còn thiếu, Gate G0 chưa đóng.
- [ ] P4: mic/cam, theo dõi hệ thống (F32, F34), plugin API (F44).
- [ ] F46 video: build lại exe (có bản sửa peek + giao diện 320×180), thử tay theo [video-player §11](modules/video-player.md#11-kiểm-thử): Facebook, dò trang, link hết hạn, spike S1 trình phát nhúng, phím tắt, DPI 150%, RAM.
- [x] Yêu cầu 9 mục: autostart có xác minh registry, gộp menu Nhạc, nhãn Nhạc trên máy, phiên nghe, queue, stopwatch panel, Pomodoro start/stop, kệ 3 slot, alarm từng dòng. Các phần chưa QA tay đầy đủ giữ rõ ở module docs.
- [x] Island Online hiện nguồn/bài đang phát, Mở trang, Đổi trang, Tìm bài; danh sách site chỉ mở theo yêu cầu.
- [x] Chooser tự nới để thấy nút; Chạy tiếp về player; playlist nới theo nội dung rồi mới cuộn. Scrollbar mặc định cam neon/nền tối dùng chung toàn app. Đã test tích hợp và xem layout fixture offline; resize native có unit test, còn kiểm tra tay đa DPI/màn hình.
- [ ] Không tiếp tục test online tự động trong phiên đang có block. Trước khi bật lại search SoundCloud cần phương án tích hợp phù hợp; không suy ra Chrome phát được thì WebView cũng được.
- [ ] Reboot thực để kiểm chứng autostart; kéo thả OS lên island; xác minh toast Windows hiện 3 giây trong cấu hình thông báo của người dùng. QA đã khôi phục HKCU Run về exe release chính, có quote và `--autostart`.

- [x] Đóng gói v0.1.1: bộ cài, portable chỉ chứa exe/README/data marker, hai file SHA256; cập nhật README và release notes theo yêu cầu push + release.

## 4. Câu hỏi mở

Chủ dự án đã cho phép push toàn bộ thay đổi và phát hành phiên bản mới; không chờ xác nhận phát hành thêm. Những hạn chế QA được ghi công khai trong release notes.

Ba module đã được chủ dự án cho triển khai; `.env` là nguồn key Groq và key được nhúng native theo yêu cầu, không cần hỏi lại về backend. Tài khoản dùng chung quota; không đổi IP/vượt hạn mức. Mic QA bị Windows từ chối quyền: chưa đổi privacy settings. Ghi âm/timeline có code nhưng chất lượng 60 phút và phục hồi mọi cửa sổ crash còn chưa nghiệm thu.

Đã chốt phạm vi Dọn dẹp: RAM toàn PC, không riêng PCisland; disk một chạm tự quét/xóa/báo kết quả, không chọn nhóm. Cache mới/đang dùng và dữ liệu đăng nhập/cá nhân được giữ. Đây là yêu cầu triển khai bổ sung được chủ dự án cho phép, không chờ P4.

Yêu cầu giao diện đã chốt 2026-11-01: launcher lúc rảnh sát mép trên hơn; thêm chữ “by Zeusato” mảnh, neon cam, hiệu ứng nhẹ ở góc dưới phải của island.

Phản hồi tiếp theo: chữ ký đậm thêm; thứ tự launcher Nhạc/Ghi chú/Nhắc việc/Pomo/Hẹn giờ/Bấm giờ/Home; nhạc trên máy chọn thư mục; repeat áp dụng cả local và online (đã hỏi và xác nhận); giờ/đếm ngược ở cuối từng dòng ghi chú.

Bổ sung: cần nút mở lại file đã giữ và chọn bài tìm kiếm phải phát đúng bài. Đã thêm Kệ file trước Home; tìm→phát xác nhận đúng URL đang Playing, không chỉ thành công điều hướng. Xem [báo cáo](reports/search-playback-2026-11-01.md).

Quyết định bổ sung của chủ dự án 2026-11-01: khi còn tác vụ mặc định không ẩn hoàn toàn island, có tùy chọn cho phép ẩn (ADR-014); phát hành bản mới nhất lên GitHub sau khi dọn output/cache build. Các giới hạn kiểm thử tay được giữ rõ trong release notes.

Yêu cầu mới đã chốt: alarm phụ thuộc **dòng checklist ngay phía trên**; Pomodoro bấm lần hai dừng hẳn; tìm bài/chọn phát trong app; Online trên island ưu tiên thẻ nguồn đang mở; island được kéo dài khi mở tùy chọn/playlist; tay kéo scrollbar **cam neon mặc định**, không xám hoặc trắng. Sự cố SoundCloud chưa có bằng chứng khóa tài khoản, chỉ có thông báo chặn thiết bị/mạng; không khẳng định thời gian tự mở chặn. Chủ dự án đã yêu cầu push và release v0.1.1; giữ rõ giới hạn kiểm thử trong ghi chú phát hành.

| # | Câu hỏi | Đề xuất mặc định | Cần chốt trước | Trạng thái |
|---|---|---|---|---|
| Q1 | Island hiện gì khi rảnh? | Đôi mắt chớp (mặc định); đồng hồ / pill trống / ẩn là tùy chọn | P1.3 | **Đã chốt** 2026-10-31 |
| Q2 | Island ở màn hình nào? | Luôn hiện, trên mọi virtual desktop; nhiều màn hình thì ở màn hình chính | P1.3 | **Đã chốt** 2026-10-31 |
| Q3 | Island nổi cách mép trên hay dính mép trên? | Dính chặt mép trên (kiểu notch) | P0.3 | **Đã chốt** 2026-10-31 |
| Q4 | Phím tắt mặc định có trùng với app bạn đang dùng không? | Giữ mặc định ADR-008, đổi sau trong Cài đặt | P1.4 | **Đã chốt** 2026-10-31 |
| Q5 | Phát hành thế nào; có ký số không? | GitHub Releases, có bộ cài + portable; không mua chứng chỉ, xin SignPath miễn phí nếu đủ điều kiện (ADR-009) | P2.7 | **Đã chốt** 2026-10-31 |
| Q6 | Trang nhạc nào cần test ở G0? | Zing MP3, SoundCloud, YouTube, Spotify Web, NhacCuaTui | P0.6 | **Đã chốt** 2026-10-31 |
| Q8 | Tiện ích dọn cache/RAM/Windows triển khai mục nào trước? | — | — | **Đã chốt** 2026-11-02: thêm nguồn rác lớn, thùng rác + Windows Update, đóng app nền ngốn RAM (ADR-022) |
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

- 2026-11-02 · Claude: sau v0.1.3, thêm trang Pomodoro và Dọn dẹp trong cửa sổ chính, nút Pomodoro trên island mở panel có tải gói nhạc; build + đóng gói v0.1.4 (269 Rust + 92 frontend + check đạt). Đổi quy tắc dọn build theo chủ dự án: giữ cache, dọn khi target > 20 GiB. Đã push + release v0.1.4. Sau release: thêm nhãn i18n còn thiếu cho phím tắt `meetingMark` (Cài đặt hiện key thô), chưa commit.

- 2026-11-02 · Claude: phát hành nhạc `focus-sounds-v1` (3 gói, 20 bài) lên `zeusato/pcisland-sounds`, rồi đóng gói v0.1.3 theo yêu cầu build + release. 269 Rust + vitest + check đạt; exe/NSIS/portable đúng version, hash ở snapshot; đã đối chiếu URL manifest trong app với release nhạc.

- 2026-11-02 · Claude: cài toolchain, sửa CI CMake, note desktop đóng/ô tick, Spotify + SoundCloud queue/repeat/tìm kiếm theo trace thật (ADR-021), mở rộng Dọn dẹp (ADR-022, subagent). Kết quả kiểm tra ghi ở snapshot.

- 2026-11-01 · Codex: đóng gói v0.1.2 theo yêu cầu push + release; đồng bộ README/CHANGELOG/roadmap/kiến trúc/module/handoff, thêm script package allowlist và tự tìm CMake. 238 Rust + 81 frontend đạt; exe/NSIS/portable đúng version, hash và nội dung ZIP được kiểm tra; dọn cache giải phóng thực tế 7,458 GiB. Link/gói tại snapshot.

- 2026-11-01 · Codex: triển khai file/note/meeting, sửa dropdown theo ảnh phản hồi, QA codec/Groq bằng mẫu công khai và system capture. 238 Rust + 81 frontend đạt, chưa release; đóng QA và dọn cache theo lựa chọn chủ dự án. Chi tiết tại báo cáo triển khai.

- 2026-11-01 · Codex: triển khai module maintenance tách biệt, tích hợp additive island/Cài đặt/IPC, RAM theo tiến trình và cleanup một chạm; test fixture filesystem và UI, native RAM smoke chỉ đọc, preview localhost. Chưa build exe; giữ cache dùng chung cho agent khác đang verify.

| Ngày | Ai | Đã làm | Tiếp theo |
|---|---|---|---|
| 2026-11-01 | Codex | Thêm Kệ file + nạp lúc khởi động; sửa tìm→phát ngoài queue cũ và xác nhận URL. Native album Lee Phú Quý đạt; phát hiện và sửa thêm trang bài hát riêng, tổng 72 frontend + 171 Rust đạt. Hủy build tiếp theo theo yêu cầu tắt máy | Chỉ tiếp tục build/QA khi chủ dự án yêu cầu |
| 2026-11-01 | Codex | Sửa ưu tiên kéo file/minimal; giữ player + quay lại; chọn ghi chú đã lưu; tìm trong player, sửa co dòng và thêm ca sĩ từ DOM Zing/Spotify; bỏ double-click mở main. 68 frontend + 171 Rust, check/clippy đạt. Thêm build wrapper tự dọn và nghiên cứu quick wins | QA native và quyết định tiện ích mới |
| 2026-11-01 | Codex | Tăng chữ ký, đổi thứ tự launcher, chọn thư mục nhạc, repeat local/web có xác nhận bridge và thời gian cuối dòng ghi chú. 61 frontend + 168 Rust; check, clippy, QA bố cục offline và build exe release đạt | Chủ dự án thử native/website trên exe mới |
| 2026-11-01 | Codex | Build exe release có launcher gọn và chữ ký “by Zeusato” theo yêu cầu chạy thử; `pnpm tauri build --no-bundle` hoàn tất | Chủ dự án chạy thử exe mới |
| 2026-11-01 | Codex | Bỏ khoảng điều hướng rỗng 28 px, đồng bộ chiều cao/hit region với nút chuyển tác vụ; chữ ký neon cam ở launcher, hiệu ứng hữu hạn và giảm chuyển động. QA localhost: 400×226, header y=16; check, 51 test frontend và build đạt | Kiểm tra native trong lần đóng gói tiếp theo |
| 2026-11-01 | Codex + chủ dự án | Phát hành v0.1.1 theo yêu cầu: push main, tag và release với bộ cài/portable/SHA256. Rà và giữ đủ test cũ, test nhạc mới tách riêng: 51 frontend + 167 Rust | Kiểm tra tay các giới hạn nêu trong release notes |
| 2026-11-01 | Codex + chủ dự án | Island đo nội dung và nới native canvas (ADR-017); resume về player, playlist mở rộng; scrollbar cam neon theo yêu cầu. 167 Rust + 46 frontend trong lần chạy toàn bộ; QA giao diện bằng fixture offline; exe/NSIS đã build 10:55 | Kiểm tra tay native đa DPI, không truy cập website đang bị chặn |
| 2026-11-01 | Codex + chủ dự án | Sửa 9 mục tương tác/khởi động/alarm; trace Zing thật, xác minh resume sau restart và chọn queue; thêm tìm kiếm nền/thẻ nguồn. SoundCloud báo anti-bot: dừng QA online, tắt search SoundCloud, bỏ click phát lặp. 166 Rust + 50 frontend test xanh | Build local đã xong; kiểm chứng các mục tay còn thiếu khi phù hợp, không thử vượt chặn |
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
| 2026-11-01 | Claude | Review nhạc/island theo yêu cầu: sửa web player tự hủy sai, không lưu quảng cáo làm bài cuối, "Tiếp tục" giữ cửa sổ, queue tô bài hiện tại, island kẹt player, kéo nhầm mở kệ, mất focus ghi chú, nhắc việc lưu trùng. Làm F46 video nổi (ADR-018, spike S2, module `video`), sửa peek bị tiêu đề dài kéo rộng | Build lại exe; thử tay F46 |

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
