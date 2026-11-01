# Dọn dẹp và RAM toàn máy

Ngày cập nhật: 2026-11-01. Triển khai theo yêu cầu trực tiếp của chủ dự án, mở rộng đề xuất [quick wins](../reports/quick-wins-2026-11-01.md). Có source và kiểm thử; chưa đóng gói exe mới.

## Hành vi

Mở **Dọn dẹp** trên launcher island hoặc trong Cài đặt. Dọn disk và thu hồi RAM là hai thao tác riêng, chỉ chạy khi người dùng bấm.

**Dọn rác ngay** là một thao tác: tự quét, xóa các mục đủ điều kiện, hiển thị tiến trình và báo kết quả. Không cần chọn từng nhóm. Khi quét chưa biết tổng số file, thanh tiến trình ở chế độ vô định; khi xóa dùng số file đã xử lý/tổng ứng viên. Báo thành công, không có rác đủ điều kiện, hoặc hoàn tất có mục được giữ lại; không báo đã dọn hết nếu gặp lỗi/thiếu quyền/app đang chạy.

| Nguồn mặc định | Phạm vi được xóa |
|---|---|
| LocalAppData/Temp và Windows/Temp | File `.tmp`, `.temp`, `.log`, `.etl` quá 7 ngày; bỏ tên bắt đầu `~`, tên/thư mục có autosave/recovery/backup |
| Chrome, Edge, Brave, Chromium | `Cache`, `Code Cache`, `GPUCache` trong Default, Guest Profile, Profile N; file quá 7 ngày; bỏ qua cả app nếu tiến trình đang chạy |
| Firefox | `cache2` của profile trong LocalAppData/Mozilla/Firefox/Profiles; cùng điều kiện tuổi và app đã đóng |
| Discord, Slack, VS Code | Chỉ `Cache`, `Code Cache`, `GPUCache` trong thư mục app mặc định ở RoamingAppData; cùng điều kiện tuổi và app đã đóng |
| PCisland | Log có tên ngày/epoch hợp lệ và mtime đều quá 7 ngày; không đệ quy thư mục log |

Không tự phát hiện mọi ứng dụng hay profile tùy chỉnh. Không xóa thư mục cache gốc; chỉ xóa file qua handle đã kiểm tra. Không dọn Cookie, Login Data, History, Local Storage, Service Worker/CacheStorage, database, phiên đăng nhập, nhạc, ghi chú, Downloads, Recycle Bin, Windows.old, Prefetch, WinSxS, registry hoặc payload cài đặt/cập nhật. Không xóa toàn bộ AppData hoặc TEMP theo biến môi trường tùy ý. Storage Windows vẫn có nút mở riêng cho việc dọn nâng cao.

Dung lượng kết quả là tổng độ dài nội dung file có lệnh xóa thành công, không phải cam kết cùng số byte trống vật lý (nén, sparse, hard link, hoạt động đồng thời của Windows). File đang mở/đang ghi hoặc thiếu quyền được bỏ qua; không nâng quyền, đóng app, dừng dịch vụ hay hẹn xóa khi khởi động lại. Cache mới tạo trong 7 ngày được giữ lại; đây là chính sách mặc định, không cần người dùng cấu hình.

## RAM toàn máy

Hiển thị RAM khả dụng/tổng, system cache do Windows quản lý, working set và private bytes theo tiến trình trong phiên hiện tại. Tiến trình đã thoát được Windows thu hồi cấp phát; system cache không được gắn nhãn rác.

Quét hai lần cách nhau 1 giây. Chỉ cho chọn tiến trình của cùng người dùng/phiên, working set ít nhất 32 MiB, không thuộc Windows, không dùng hơn 1% một CPU logic và không có I/O byte mới trong lần đo. Giữ nguyên nhóm tiến trình PCisland, app có cửa sổ hiển thị (kể cả thu nhỏ), app có phiên thu/phát âm thanh, các tiến trình cùng tên với nhóm được bảo vệ. Không suy ra “không có giá trị” chỉ vì không thấy cửa sổ.

Người dùng chọn app nền hoặc bấm chọn nhóm phù hợp rồi **Thu hồi RAM**. Backend kiểm tra lại PID + FILETIME tạo tiến trình, hoạt động, người dùng, cửa sổ và âm thanh trước `EmptyWorkingSet`. Tối đa 128 tiến trình/lượt. Không kill app, không dùng debug privilege, không purge standby list. Báo working set/private bytes trước-sau, RAM toàn máy khả dụng trước-sau; đo lại nhóm thực sự xử lý sau 30 giây nếu panel vẫn mở. Tổng working set có thể tính trùng trang dùng chung, giảm working set không đồng nghĩa giảm cấp phát hoặc tăng tốc. Đây là lọc bảo thủ theo mẫu đo, không chứng minh app nền không làm việc.

## Kỹ thuật và IPC

- `app/maintenance`: điều phối async, lock loại trừ toàn app, chính sách allowlist, công việc nặng qua `spawn_blocking`.
- `platform/maintenance`: counter Win32, tiến trình đang chạy ở mọi session cho dọn cache, handle filesystem, mở Storage/Task Manager theo allowlist.
- `platform/memory`: bộ lọc tiến trình, audio session/cửa sổ, trim working set và đo lại.
- `maintenance_resources() → ResourceSnapshot`; `maintenance_processes() → MemoryProcess[]`; `maintenance_reclaim(targets) → MemoryReclaim`; `maintenance_measure(targets) → MemoryMeasurement`.
- `maintenance_clean(progress: Channel<CleanupProgress>) → CleanupProgress`: không nhận đường dẫn, tên app hay lệnh shell từ UI. Channel chỉ tới cửa sổ gọi; callback được giới hạn khoảng 10 lần/giây trong vòng quét/xóa, cộng chuyển nhóm/giai đoạn.
- `maintenance_logs(clean) → CleanupResult`: thao tác riêng cho log PCisland; `maintenance_open(tool)` chỉ nhận `storage` hoặc `task_manager`.
- Command chỉ được cấp cho cửa sổ local qua manifest sinh từ `lib.rs`. Không mở quyền cho trang nhạc từ xa. Không thêm DB/cài đặt hay công việc nền tự chạy.

Giữ handle trên tất cả thư mục tổ tiên, từ chối reparse point/junction/symlink, không cho rename trong lúc kiểm tra/xóa. Mỗi file được kiểm tra parent canonical và mở với DELETE + không chia sẻ trước khi gọi `SetFileInformationByHandle`; không gọi remove_dir_all cho dữ liệu người dùng. Quét tối đa 250.000 entry/100.000 ứng viên, sâu tối đa 32 cấp; báo nếu đạt giới hạn. Trước xóa kiểm tra lại tuổi file và root, quét tiến trình lại, tối đa mỗi 250 ms trong vòng xóa. File có thể thay đổi giữa hai lượt nên số đã xóa không được lấy từ ước tính quét.

Panel đóng thì ngừng polling UI và phép đo RAM sau 30 giây; tác vụ disk đã bắt đầu vẫn hoàn thành trên worker, khóa toàn app được trả bằng RAII kể cả lỗi. Không ảnh hưởng media hub/player. Kết quả UI hiện thuộc panel đang mở, chưa lưu lịch sử dọn giữa các cửa sổ hoặc phiên app.

## Kiểm thử và giới hạn nghiệm thu

Kết quả 2026-11-01: `pnpm check` đạt (365 khóa i18n), 77 test frontend đạt, `pnpm build` đạt; `cargo test` 194 đạt/1 native smoke bỏ qua mặc định, smoke đó đã chạy riêng và đạt (chỉ đọc). 13 test backend maintenance và 5 test panel nằm trong tổng này. `cargo fmt --check`, `cargo clippy --lib -- -D warnings`, `git diff --check` đạt. `clippy --all-targets` còn lỗi `items_after_test_module` tại `modules/web_player/mod.rs:660`; giữ nguyên phần agent khác đang verify, không tính là module maintenance đã xử lý lỗi đó. Preview localhost xác minh nút một chạm, tiến trình và kết quả với số liệu giả; fixture/server đã dọn. Exe release 17:51 giữ nguyên.

Unit/integration dùng cây thư mục giả: allowlist/tuổi file, dữ liệu cá nhân/đăng nhập giữ nguyên, app đang chạy và app mở giữa quét-xóa, process scan lỗi phải bỏ cache, file bị khóa/vừa sửa, path ngoài root, khóa chống đổi thư mục, byte thực đã xóa, chạy lặp không báo byte ảo, các giai đoạn tiến trình. Frontend kiểm tra một chạm, chặn bấm lặp, tiến trình xác định/vô định, kết quả có bỏ qua, lỗi và thử lại, chọn PID chính xác và hủy timer khi panel đóng.

Native smoke chỉ đọc resource counters và danh sách tiến trình; thử identity cũ để đảm bảo không trim thật. Chưa thực hiện xóa rác thật/trim app thật trong phiên phát triển. Cần kiểm tra nghiệm thu trên exe: nhạc tiếp tục khi mở/đóng Dọn dẹp; app nền được chọn nạp lại bình thường; tiến trình/quyền khác nhau; reparse point thực tế; browser cache của profile mặc định trên máy thử.

Nguồn API/chính sách: [Microsoft EmptyWorkingSet](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-emptyworkingset), [Working set](https://learn.microsoft.com/en-us/windows/win32/memory/working-set), [Chromium user data/cache](https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/user_data_dir.md), [Firefox HTTP cache](https://firefox-source-docs.mozilla.org/networking/cache2/doc.html), [Electron data/cache directories](https://www.electronjs.org/docs/latest/api/app), [Windows Storage Sense](https://support.microsoft.com/en-us/windows/experience/storage-filemanagement/manage-drive-space-with-storage-sense).

## Lịch sử

- 2026-11-01: thêm Dọn dẹp island/Cài đặt, RAM toàn PC và dọn rác một chạm theo yêu cầu chủ dự án. Giữ phần sửa code dùng chung ở mức đăng ký module, IPC, i18n và view tích hợp vì agent khác đang verify các module còn lại.
