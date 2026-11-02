# Dọn dẹp và RAM toàn máy

Ngày cập nhật: 2026-11-02. Triển khai theo yêu cầu trực tiếp của chủ dự án ([ADR-019](../06-decision-log.md), mở rộng bởi [ADR-022](../06-decision-log.md)). Có source và kiểm thử; chưa đóng gói exe cho phần mở rộng 2026-11-02, nghiệm thu thực tế còn mở.

## Hành vi

Mở **Dọn dẹp** trên launcher island hoặc trang **Dọn dẹp** riêng của cửa sổ chính (`#/maintenance`, hai cột, cùng controller nên tiến trình đang chạy hiện ở cả hai nơi). Dọn disk, Thùng rác, Windows Update, đóng app và thu hồi RAM là các thao tác riêng, chỉ chạy khi người dùng bấm. Thao tác không hoàn tác được hoặc gây gián đoạn (Thùng rác, Windows Update, đóng app, buộc đóng) luôn có bước xác nhận ngay trong panel.

**Dọn rác ngay** là một thao tác: tự quét, xóa các mục đủ điều kiện, hiển thị tiến trình và báo kết quả. Không cần chọn từng nhóm. Khi quét chưa biết tổng số file, thanh tiến trình ở chế độ vô định; khi xóa dùng số file đã xử lý/tổng ứng viên. Từ giai đoạn xóa, panel hiện từng nhóm với dung lượng tìm thấy khi quét; khi xong hiện byte thực xóa / tìm thấy của từng nhóm và số thư mục rỗng đã xóa. Báo thành công, không có rác đủ điều kiện, hoặc hoàn tất có mục được giữ lại; không báo đã dọn hết nếu gặp lỗi/thiếu quyền/app đang chạy.

| Nhóm | Nguồn | Phạm vi được xóa | Khi app chạy |
|---|---|---|---|
| File tạm | LocalAppData/Temp, Windows/Temp | Mọi loại file quá 1 ngày; bỏ tên bắt đầu `~`, tên/thư mục có autosave/recovery/backup; xóa thư mục con rỗng quá 1 ngày | Không chặn; file đang mở bị khóa và bỏ qua |
| Crash dump, báo cáo lỗi | LocalAppData/CrashDumps; `Microsoft/Windows/WER/{ReportArchive,ReportQueue,Temp}` ở LocalAppData và ProgramData | Mọi file quá 1 ngày. ProgramData chỉ tính file tài khoản hiện tại xóa được | Không chặn |
| Cache shader GPU | LocalAppData/D3DSCache, NVIDIA/{DXCache,GLCache}, AMD/{DxCache,DxcCache,GLCache} | Mọi file quá 1 ngày; driver tạo lại | Không chặn; game đang chạy giữ khóa file |
| Windows | LocalAppData/Microsoft/Windows/Explorer; LocalAppData/Microsoft/Windows/INetCache | Explorer: chỉ `thumbcache_*.db`, `iconcache_*.db`, không đệ quy, không theo tuổi. INetCache: mọi file quá 1 ngày trừ `desktop.ini` và thư mục `Content.Outlook` | Không chặn; Explorer khóa file đang dùng → bỏ qua |
| Trình duyệt | Chrome, Edge, Brave, Chromium: `Cache`, `Code Cache`, `GPUCache` trong Default, Guest Profile, Profile N; Firefox: `cache2` của profile | File quá 1 ngày | Bỏ qua cả trình duyệt |
| Ứng dụng | Discord, Slack, VS Code, Teams cũ (RoamingAppData) và Teams mới (`MSTeams_8wekyb3d8bbwe/.../EBWebView/Default`): `Cache`, `Code Cache`, `GPUCache`; Spotify (bản desktop và Store): `Data`, `Storage` | File quá 1 ngày | Bỏ qua app đó |
| Gói lập trình | LocalAppData/npm-cache, pip/cache, Yarn/Cache | File quá 7 ngày | Không chặn |
| Log PCisland | Thư mục log của PCisland | Log có tên ngày/epoch hợp lệ và mtime đều quá 7 ngày; không đệ quy | — |

Chặn theo tiến trình chỉ áp dụng cho cache Chromium/Electron/Firefox/Spotify: các app này giữ index cache trong bộ nhớ và ghi đồng thời, xóa khi đang chạy có thể làm hỏng cache của phiên. Nguồn dùng chung nhiều tiến trình (shader, thumbnail, INetCache, WER, gói lập trình, temp) không thể gắn với một app; file đang dùng bị khóa khi mở độc quyền nên tự được giữ lại. Cache gói npm/pip/Yarn theo nội dung, thiếu mục chỉ là cache miss; dùng 7 ngày để không chạm gói vừa tải. Spotify `Storage` chứa cả bài tải offline: sau khi dọn, Spotify tải lại khi cần.

Nguồn toàn máy (Windows/Temp, WER trong ProgramData) chỉ dọn phần tài khoản hiện tại có quyền xóa; từ chối quyền ở đó là bình thường với người dùng thường nên không tính vào "bỏ qua".

Thư mục rỗng: chỉ thư mục con bên trong root temp/cache có mtime đủ tuổi tại lúc quét, xử lý từ sâu ra nông sau khi xóa file. Mở thư mục bằng handle DELETE độc quyền, kiểm tra không phải reparse point, rồi đặt disposition; Windows từ chối nếu thư mục còn entry, nên thư mục còn file không bao giờ bị xóa. Không bao giờ xóa root của nguồn, không gọi remove_dir_all.

Không dọn: Cookie, Login Data, History, Local Storage, IndexedDB, Service Worker/CacheStorage, database, phiên đăng nhập, ghi chú, Downloads, Windows.old, Prefetch, WinSxS, registry. Không tự phát hiện mọi app hay profile tùy chỉnh. Không xóa toàn bộ AppData hoặc TEMP theo biến môi trường tùy ý; ProgramData lấy từ known folder của shell.

Dung lượng kết quả là tổng độ dài nội dung file có lệnh xóa thành công, không phải cam kết cùng số byte trống vật lý (nén, sparse, hard link, hoạt động đồng thời của Windows). Ước tính nhóm lấy từ lần quét; lượt xóa kiểm tra lại từng file nên có thể thấp hơn.

### Thùng rác

Panel đọc kích thước/số mục (`SHQueryRecycleBinW`, mọi ổ) khi mở và sau mỗi lần dọn. Nút **Dọn Thùng rác** chỉ bật khi có mục; bấm sẽ hiện xác nhận "xóa vĩnh viễn, không thể hoàn tác" kèm số mục/dung lượng. Chỉ khi xác nhận mới gọi `SHEmptyRecycleBinW` (không hộp thoại shell, không âm thanh). Kết quả là chênh lệch trước/sau đo ở backend. Không nằm trong Dọn rác ngay.

### Cache Windows Update

Thao tác riêng, có xác nhận. Backend chạy lại chính `pcisland.exe` bằng `ShellExecuteExW` verb `runas` với đúng một đối số cố định `--maintenance-windows-update`; Windows hiện UAC. Tiến trình nâng quyền thoát ngay ở đầu `run()` (trước log, single-instance, cửa sổ), chỉ chạy recipe cố định:

- `Windows/SoftwareDistribution/Download`
- `Windows/SoftwareDistribution/DeliveryOptimization`
- `Windows/ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/Cache`

với cùng cơ chế handle: file quá 1 ngày, file dịch vụ đang dùng bị khóa thì giữ, xóa thư mục con rỗng. Không dừng dịch vụ wuauserv/BITS. Mã thoát: bit 29 đánh dấu, bit 0 = có mục giữ lại, bit 2–28 = KiB đã xóa (giới hạn 128 GiB). Mã khác (crash, panic) báo thất bại. Từ chối UAC → "chưa dọn gì". Chờ tối đa 10 phút; quá hạn báo thất bại, không kill helper. Không có đường dẫn hay lệnh nào do UI cung cấp.

## RAM toàn máy

Hiển thị RAM khả dụng/tổng, system cache do Windows quản lý.

### Đóng app ngốn RAM (chính)

**Tìm app nặng** liệt kê tối đa 15 app của cùng người dùng/phiên dùng từ 32 MiB working set, gom theo gốc cây tiến trình (đi lên cha tới khi gặp Explorer/tiến trình hệ thống) và đặt tên theo exe gốc. Nhờ vậy WebView2 con của một app thuộc về app đó, không gộp chung. Loại tiến trình hệ thống (đường dẫn trong SystemRoot hoặc người dùng khác), cây tiến trình PCisland (kể cả WebView2 con) và bản PCisland khác. Mỗi app hiện số tiến trình, working set, nhãn "Đang phát âm thanh" và "Chạy nền, không có cửa sổ".

Bấm **Đóng** → xác nhận (app đang phát âm thanh có cảnh báo riêng) → backend kiểm tra lại PID + FILETIME tạo tiến trình, cùng người dùng, không thuộc cây PCisland, mọi PID thuộc cùng một app; nếu có phiên âm thanh đang hoạt động mà chưa được xác nhận thì trả `audio` để UI hỏi lại. Sau đó gửi `WM_CLOSE` (PostMessage) tới cửa sổ cấp cao hiển thị, không có owner, của các tiến trình đó, rồi chờ tối đa 5 giây trên handle SYNCHRONIZE đã kiểm tra danh tính.

- Tất cả thoát → "Đã đóng".
- Còn chạy (app hỏi lưu, ẩn xuống khay) hoặc không có cửa sổ để gửi → báo số tiến trình còn chạy và hiện **Buộc đóng**. Bấm cần xác nhận thứ hai ("dữ liệu chưa lưu sẽ mất"). Backend chỉ chấp nhận đúng các cặp PID + thời điểm tạo mà bước một để lại (dùng một lần), mở lại với PROCESS_TERMINATE, kiểm tra lại thời điểm tạo và người dùng trên chính handle đó rồi `TerminateProcess`, chờ tối đa 3 giây.

Không dùng debug privilege, không đóng tiến trình người dùng khác hoặc hệ thống. Tối đa 256 PID mỗi lượt.

### Thu hồi working set (nâng cao, phụ)

Giữ nguyên hành vi cũ trong mục thu gọn: quét hai lần cách nhau 1 giây, chỉ cho chọn tiến trình nền cùng người dùng/phiên, working set ít nhất 32 MiB, không dùng hơn 1% một CPU logic và không có I/O mới. Giữ nhóm PCisland, app có cửa sổ hiển thị, app có phiên âm thanh. Backend kiểm tra lại PID + FILETIME, hoạt động, cửa sổ và âm thanh trước `EmptyWorkingSet`. Tối đa 128 tiến trình/lượt. Báo working set/private bytes trước-sau, RAM khả dụng trước-sau, đo lại sau 30 giây nếu panel còn mở. Giảm working set không đồng nghĩa giảm cấp phát; app nạp lại khi dùng.

## Kỹ thuật và IPC

- `app/maintenance`: điều phối async, lock loại trừ toàn app, recipe cố định, tập PID được phép buộc đóng, helper nâng quyền; `#![forbid(unsafe_code)]`.
- `platform/maintenance`: counter Win32, tiến trình đang chạy, handle file/thư mục, known folder ProgramData, Thùng rác, `ShellExecuteExW runas` chính exe, mở Storage/Task Manager theo allowlist.
- `platform/memory`: bộ lọc tiến trình, gom app, audio session/cửa sổ, trim working set, WM_CLOSE/terminate có kiểm tra danh tính.

| Command | Kết quả | Ghi chú |
|---|---|---|
| `maintenance_resources()` | `ResourceSnapshot` | Chỉ đọc |
| `maintenance_clean(progress: Channel<CleanupProgress>)` | `CleanupProgress` | Có `groups: CleanupGroup[]` (`category`, `estimated`, `candidates`, `bytes`, `files`) và `folders`. Channel ~10 lần/giây |
| `maintenance_logs(clean)` | `CleanupResult` | Log PCisland |
| `maintenance_recycle_bin()` | `RecycleBinInfo { bytes, items }` | Chỉ đọc |
| `maintenance_empty_recycle_bin()` | `RecycleBinInfo` | Đã xóa = trước − sau |
| `maintenance_windows_update()` | `ElevatedCleanup { status, bytes }` | `done`/`partial`/`cancelled`/`failed` |
| `maintenance_apps()` | `MemoryApp[]` | `name`, `processes: MemoryTarget[]`, `workingSet`, `privateBytes`, `windows`, `audio` |
| `maintenance_close_app(targets, allowAudio)` | `AppCloseResult` | `status`: `closed`/`pending`/`no_window`/`audio`/`skipped`; `closed`, `remaining`, `skipped` |
| `maintenance_force_close(targets)` | `AppCloseResult` | Chỉ PID bước một để lại |
| `maintenance_processes()`, `maintenance_reclaim(targets)`, `maintenance_measure(targets)` | như cũ | Trim working set |
| `maintenance_open(tool)` | `()` | `storage` hoặc `task_manager` |

Không command nào nhận đường dẫn, tên app hay lệnh shell từ UI. Command chỉ được cấp cho cửa sổ local qua manifest sinh từ `lib.rs`; không mở quyền cho trang nhạc từ xa. Không thêm DB/cài đặt hay công việc nền tự chạy.

Giữ handle trên tất cả thư mục tổ tiên, từ chối reparse point/junction/symlink, không cho rename trong lúc kiểm tra/xóa. Mỗi file được kiểm tra parent canonical và mở với DELETE + không chia sẻ trước khi gọi `SetFileInformationByHandle`. Quét tối đa 250.000 entry/100.000 ứng viên, sâu tối đa 32 cấp; báo nếu đạt giới hạn. Trước xóa kiểm tra lại tuổi file và root, quét tiến trình lại tối đa mỗi 250 ms.

Mỗi cửa sổ dùng một controller chung cho mọi panel: rời island khi đang dọn rồi mở lại vẫn thấy tiến trình hoặc kết quả, không quay về trạng thái ban đầu. Panel cuối cùng đóng thì chỉ ngừng polling tài nguyên; phép đo RAM sau 30 giây vẫn chạy để có kết quả khi quay lại; tác vụ đã bắt đầu vẫn hoàn thành trên worker, khóa toàn app được trả bằng RAII kể cả lỗi. Kết quả UI thuộc panel đang mở, chưa lưu lịch sử.

## Kiểm thử và giới hạn nghiệm thu

Kết quả 2026-11-02: `cargo test` 251 đạt / 4 ignored (gồm 1 native smoke RAM chỉ đọc), `cargo fmt --check` và `cargo clippy --all-targets -- -D warnings` đạt; `pnpm test` 87 đạt (9 test panel Dọn dẹp); eslint và i18n check (542 khóa) đạt. `svelte-check` chỉ còn một lỗi ở `features/web_player/bridge.test.ts` do agent khác đang sửa, không thuộc module này.

Unit test backend dùng cây thư mục giả trong thư mục tạm của test: tuổi 1 ngày/7 ngày theo nhóm, mọi loại file temp nhưng giữ `~`/recovery/autosave, nguồn mới (CrashDumps, WER LocalAppData/ProgramData, shader, thumbnail chỉ đúng tên và không đệ quy, INetCache bỏ `desktop.ini`/Content.Outlook, npm), thứ tự nhóm và ước tính = thực xóa, Spotify/Teams bị bỏ khi chạy, cookie Teams giữ nguyên, thư mục rỗng cũ bị xóa nhưng root/thư mục mới/thư mục còn file giữ nguyên, file khóa/thumbnail đang mở/file đổi giữa hai lượt, path ngoài root, recipe Windows Update chỉ xóa payload cũ. RAM: gom app theo gốc cây, loại hệ thống và cây PCisland (kể cả WebView2 con), xác minh PID/thời điểm tạo/cùng app, buộc đóng chỉ cho PID bước một để lại và dùng một lần, mã thoát helper encode/decode và từ chối mã crash. Frontend: xác nhận Thùng rác (hủy không gọi), Windows Update xác nhận và báo UAC bị hủy, đóng app hai bước với buộc đóng chỉ sau xác nhận thứ hai, app có âm thanh gửi `allowAudio` sau cảnh báo, hiển thị nhóm.

Chưa chạy dọn rác thật, Thùng rác thật, UAC thật hay đóng app thật trong phiên phát triển. Cần nghiệm thu trên exe: UAC hiện và từ chối/đồng ý báo đúng; Windows Update chạy khi đang tải cập nhật; đóng app có hộp thoại lưu, app ẩn khay (Discord/Teams), app chỉ có cửa sổ ẩn; Spotify sau khi dọn `Storage`; nhạc tiếp tục khi mở/đóng Dọn dẹp; reparse point thực tế.

Nguồn API/chính sách: [EmptyWorkingSet](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-emptyworkingset), [SHEmptyRecycleBinW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shemptyrecyclebinw), [ShellExecuteExW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw), [Chromium user data/cache](https://chromium.googlesource.com/chromium/src.git/+/HEAD/docs/user_data_dir.md), [Firefox HTTP cache](https://firefox-source-docs.mozilla.org/networking/cache2/doc.html), [Electron data/cache directories](https://www.electronjs.org/docs/latest/api/app), [Windows Storage Sense](https://support.microsoft.com/en-us/windows/experience/storage-filemanagement/manage-drive-space-with-storage-sense).

## Lịch sử

- 2026-11-01: thêm Dọn dẹp island/Cài đặt, RAM toàn PC và dọn rác một chạm theo yêu cầu chủ dự án. Giữ phần sửa code dùng chung ở mức đăng ký module, IPC, i18n và view tích hợp vì agent khác đang verify các module còn lại.
- 2026-11-02: mở rộng theo yêu cầu chủ dự án (ADR-022): temp mọi loại file quá 1 ngày và xóa thư mục rỗng, thêm crash dump/WER/shader/thumbnail/INetCache/npm/pip/Yarn/Teams/Spotify, cache trình duyệt/app 1 ngày, kết quả theo nhóm; Thùng rác có xác nhận; cache Windows Update qua helper UAC; đóng app ngốn RAM hai bước (WM_CLOSE rồi buộc đóng có xác nhận). Trim working set chuyển thành mục nâng cao.
- 2026-11-02: tiến trình/kết quả dọn, Thùng rác, Windows Update và đóng app được giữ khi rời panel (controller dùng chung theo cửa sổ).
- 2026-11-02: theo yêu cầu chủ dự án, Dọn dẹp có trang riêng `#/maintenance` trong sidebar cửa sổ chính (bố cục hai cột), bỏ thẻ Dọn dẹp ở cuối Cài đặt.
