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
| Phase | **P0 · Nền móng & PoC**, phần code xong, chờ test tay để đóng Gate G0 |
| Phiên bản | 0.0.1 (chưa phát hành) |
| Code | Nhánh `main`, commit đầu `506dba9`. Scaffold + 4 spike chạy được |
| Bản build gần nhất | `pnpm tauri dev` chạy OK ngày 2026-10-31; chưa build release |
| Đang chặn | Không có. Mọi câu hỏi mở đã được chốt |

**Đã có:**
- Bộ tài liệu thiết kế: tầm nhìn, lộ trình, system design, nguyên tắc code, design system, nhật ký quyết định, đặc tả 14 module và backlog.
- Các quyết định đã chốt: ADR-001 → ADR-009.

**Chưa có:** repo git, scaffold, CI, bất kỳ dòng code nào.

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
- [ ] Tạo repo GitHub và push (để CI chạy lần đầu).

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
