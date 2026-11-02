# CLAUDE.md

Hướng dẫn cho AI agent (và người) làm việc trong repo PCisland.

## Dự án

Dynamic Island cho Windows. Stack: Tauri 2 (Rust) + Svelte 5 (TypeScript), chạy trên WebView2, đóng gói portable thành một file exe. Tổng quan ở [README.md](README.md).

## Đầu mỗi phiên

1. Đọc [docs/05-handoff.md](docs/05-handoff.md): trạng thái, việc tiếp theo, câu hỏi mở.
2. Đọc tài liệu module sắp động tới trong [docs/modules/](docs/modules/README.md).
3. Cần bức tranh kiến trúc thì đọc [docs/02-system-design.md](docs/02-system-design.md).

## Luật bắt buộc

- Code theo [docs/03-coding-principles.md](docs/03-coding-principles.md). Giao diện theo [docs/04-design-system.md](docs/04-design-system.md).
- Làm đúng đặc tả module. Muốn làm khác thì sửa đặc tả trước hoặc trong cùng commit. Không để code và tài liệu lệch nhau.
- Không đảo một quyết định trong [docs/06-decision-log.md](docs/06-decision-log.md) khi chưa có ADR mới thay thế.
- Không làm tính năng ngoài phase hiện tại của [lộ trình](docs/01-roadmap.md). Ý tưởng mới ghi vào backlog trong [docs/00-vision-scope.md](docs/00-vision-scope.md).
- Module không gọi thẳng nhau. Giao tiếp qua ActivityHub, EventBus hoặc service trait đã khai báo (system design §4).
- `unsafe` và lời gọi Win32/WinRT trực tiếp chỉ nằm trong `src-tauri/src/platform/`. Mỗi khối `unsafe` có comment `// SAFETY:`.
- Frontend:
  - Không gọi `invoke`/`listen` trong component, dùng `src/lib/ipc/`.
  - Không hardcode màu hay kích thước, dùng token.
  - Mọi chuỗi hiển thị phải qua i18n.
- Ngôn ngữ: tài liệu viết tiếng Việt; tên định danh, comment và commit message viết tiếng Anh; UI mặc định tiếng Việt.

## Cuối mỗi phiên (Definition of Done)

- Chạy đủ các lệnh kiểm tra ở bảng dưới.
- Cập nhật tài liệu module đã động tới: trạng thái, IPC, cài đặt, lịch sử thay đổi.
- Cập nhật handoff: snapshot, việc tiếp theo, thêm một dòng vào nhật ký phiên.
- Có quyết định kiến trúc mới thì thêm ADR.
- Build bằng `pnpm build:exe` / `pnpm build:app`. Wrapper giữ cache Rust để build sau nhanh, chỉ tự dọn khi `src-tauri/target` vượt trần 20 GiB, để rác không phình theo số lần build. Không dọn hết sau mỗi build trừ khi chủ dự án yêu cầu. Giữ exe mới, gói phát hành và mọi dữ liệu app. Xem [quy trình dọn build](docs/build-cleanup.md).

## Lệnh

> Đã kiểm tra ngày 2026-10-31. Trên Git Bash cần `export PATH="$HOME/.cargo/bin:$PATH"` nếu terminal mở trước khi cài Rust. `pnpm build` phải chạy trước lệnh cargo đầu tiên (thư mục `dist/` cần cho `generate_context!`). Opus (`audiopus_sys`) cần `cmake` trong PATH; bản đi kèm VS Build Tools nằm ở `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin` (`pnpm build:exe`/`build:app` tự tìm).

| Việc | Lệnh |
|---|---|
| Cài phụ thuộc | `pnpm install` |
| Chạy dev | `pnpm tauri dev` |
| Kiểm tra frontend | `pnpm check` (svelte-check + eslint) · `pnpm test` (vitest) |
| Kiểm tra backend (trong `src-tauri/`) | `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` |
| Sinh type TS từ Rust | `pnpm gen:bindings` |
| Build exe chạy thử (dọn khi cache > 20 GiB) | `pnpm build:exe` |
| Build bản phát hành (dọn khi cache > 20 GiB) | `pnpm build:app` |
| Xem trước phần sẽ dọn | `pnpm clean:build -- -WhatIf` |
| Dọn cache build | `pnpm clean:build` |
