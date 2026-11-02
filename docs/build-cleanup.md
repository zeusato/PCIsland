# Dọn cache build

Theo yêu cầu chủ dự án:
- **2026-11-01:** ưu tiên dung lượng đĩa, dọn sau mỗi lần build.
- **2026-11-02:** đổi lại. Dọn hết sau mỗi lần build khiến lần build sau phải biên dịch lại toàn bộ (5–8 phút). Chủ dự án chỉ cần cache không phình dần theo số lần build, nên giữ cache và chỉ dọn khi vượt trần.

## Lệnh dùng hằng ngày

- `pnpm build:exe`: build exe release. Xong thì đo `src-tauri/target`; chỉ dọn khi vượt trần (mặc định 20 GiB).
- `pnpm build:app`: build exe + bộ cài, cùng quy tắc trần.
- `pnpm build:app -- -Clean`: build rồi dọn hết, bất kể kích thước (ví dụ trước khi cần trả lại dung lượng đĩa).
- `pnpm build:app -- -MaxCacheGiB 15`: đổi trần cho một lần build.
- `pnpm clean:build`: dọn riêng bất kỳ lúc nào.
- `pnpm clean:build -- -WhatIf`: xem danh sách và dung lượng dự kiến; không xóa.

Build lỗi không dọn để còn chẩn đoán. `pnpm build` vẫn chỉ build frontend, phục vụ Tauri và CI.

## Vì sao cache không phình theo số lần build

Cargo build lại trên cùng `target/` thì ghi đè artifact cùng tên; cache tăng mạnh ở lần đầu (đo được khoảng 11–14 GiB cho cả debug và release) rồi đứng quanh mức đó. Phần có thể tích lũy dần:
- artifact của phiên bản dependency cũ sau khi đổi `Cargo.lock`;
- thư mục `incremental` của debug.

Trần 20 GiB chặn phần tích lũy này: vượt trần thì lần build kế tiếp tự dọn và lần sau đó biên dịch lại từ đầu.

Từ 0.1.2, codec Opus cần CMake lúc build. Wrapper tự tìm CMake đi kèm Visual Studio nếu PATH chưa có; cài workload C++/CMake của Visual Studio Build Tools khi chưa có. Build xong chạy `powershell -NoProfile -File scripts/package-release.ps1`: lấy đúng phiên bản trong Tauri config, kiểm ProductVersion exe, tạo NSIS/portable và SHA256 trong `release/<version>/`. ZIP chỉ chứa exe, README và marker `data/.portable`; không đưa dữ liệu local hay `.env` vào gói. Script từ chối ghi đè gói đã tồn tại.

Nếu nhiều agent cùng làm trên checkout, chỉ dọn cache dùng chung sau khi tất cả lượt build/verify đã kết thúc; ghi việc dọn còn chờ vào Handoff. Có thể dọn ngay fixture và server QA do chính phiên mình tạo, không dừng tác vụ của agent khác.

## Giữ và xóa

Script `scripts/clean-build.ps1` chỉ xóa danh sách cố định trong checkout hiện tại:

| Dọn | Giữ |
|---|---|
| Rust debug/release: `deps`, `build`, `.fingerprint`, `incremental`, `examples` | `src-tauri/target/release/pcisland.exe` mới nhất |
| Binary/debug symbols/rlib trung gian của debug; `.pdb`, `.d`, rlib của release | Bộ cài trong `target/release/bundle`, mọi gói trong `release/` |
| Thư mục NSIS tạm, binary QA cũ `target/qa-0.1.1/pcisland-qa.exe` | Dữ liệu và báo cáo/script kiểm thử trong thư mục QA |
| `node_modules/.vite`, `.vite-temp`, `.vitest-cache` | `node_modules` cùng dependencies, pnpm store và Cargo registry toàn cục |
| | `dist/` (rất nhỏ, cần trước khi chạy Cargo), source, `.git`, `.codex`, mọi dữ liệu portable/APPDATA/LOCALAPPDATA |

Xóa cache không ảnh hưởng việc chạy exe đã build, nhưng lần build/test kế tiếp sẽ lâu hơn do phải biên dịch lại dependencies. Script giữ nguyên hash SHA256 của exe release và báo cả tổng kích thước file đã xóa lẫn thay đổi dung lượng trống thực tế; hai số có thể khác do hardlink và hoạt động khác trên ổ đĩa.

Thư mục `release/` giữ mọi gói đã phát hành; mỗi bản khoảng 14 MB, không tính vào trần cache.

Nếu bản release đang chạy, Windows có thể khóa cả exe và hardlink `release/deps/pcisland.exe`. Không kill app để dọn. Trong phiên này hai link của bản cũ được đổi tên trong cùng thư mục release thành `pcisland-running-previous.exe` và `pcisland-running-previous-link.exe` để có chỗ cho binary mới. Script thử dọn hai file này; nếu còn khóa thì giữ lại và báo. Sau khi thoát bản cũ, chạy `pnpm clean:build` để dọn nốt. Hai tên có thể cùng trỏ tới một file vật lý; không cộng dung lượng hai lần khi báo không gian đĩa.

## Quy tắc an toàn

1. Dừng build/dev server/test trước khi dọn. Script từ chối khi Cargo/Rust đang chạy, phát hiện Node dev/test của checkout hoặc app debug/QA; không tự tắt tiến trình.
2. Xác minh đường dẫn tuyệt đối nằm trong checkout, không đi qua junction/symlink. Duyệt cây không theo link; nếu gặp thư mục `data` hoặc file dữ liệu app trong mục dự định xóa thì dừng.
3. Chỉ dùng PowerShell `Remove-Item -LiteralPath`; không xóa nguyên `target/`, không dùng `cargo clean` vì có thể xóa exe được giao hoặc dữ liệu portable đặt cạnh exe.
4. Không gọi `PCisland.exe --cleanup`: đó là dọn dữ liệu/cấu hình ứng dụng, không phải rác build.
5. Không tự xóa gói đã phát hành hoặc dữ liệu người dùng để tiết kiệm thêm vài MB. Thư mục mới phát sinh phải được kiểm tra rồi mới thêm vào danh sách cố định.
6. Khi có dọn, ghi dung lượng đã giải phóng và vị trí exe còn giữ vào Handoff.
