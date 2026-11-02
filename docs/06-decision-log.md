# 06 · Nhật ký quyết định (ADR)

> Mỗi quyết định kiến trúc hoặc sản phẩm có tác động lâu dài được ghi thành một ADR. Không sửa nội dung một ADR đã **Accepted**. Muốn đổi thì viết ADR mới và đánh dấu ADR cũ là **Superseded by ADR-xxx**.
>
> Trạng thái: **Proposed** (đề xuất, chờ chốt) · **Accepted** · **Superseded** · **Rejected**

| ADR | Tiêu đề | Trạng thái |
|---|---|---|
| [001](#adr-001--stack-tauri-2--rust--svelte-5) | Stack: Tauri 2 + Rust + Svelte 5 | Accepted, xác nhận lại ở Gate G0 |
| [002](#adr-002--kiến-trúc-lấy-activity-làm-trung-tâm) | Kiến trúc lấy Activity làm trung tâm | Accepted |
| [003](#adr-003--cơ-chế-cửa-sổ-island) | Cơ chế cửa sổ island | Accepted, kiểm chứng ở G0 |
| [004](#adr-004--nhạc-online-theo-mô-hình-lai) | Nhạc online theo mô hình lai | Accepted |
| [005](#adr-005--lưu-trữ--portable) | Lưu trữ & portable | Accepted; phần nhận biết portable được ADR-009 thay |
| [006](#adr-006--type-sharing--quy-ước-ipc) | Type sharing & quy ước IPC | Accepted |
| [007](#adr-007--chính-sách-ngôn-ngữ) | Chính sách ngôn ngữ | Accepted |
| [008](#adr-008--mặc-định-sản-phẩm) | Mặc định sản phẩm | Accepted |
| [009](#adr-009--phân-phối--ký-số) | Phân phối & ký số | Accepted |

---

## ADR-001 · Stack: Tauri 2 + Rust + Svelte 5

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted, xác nhận lại ở Gate G0
- **Bối cảnh:** cần UI đẹp (glow, blur, morph spring), gói portable nhỏ, truy cập sâu Win32/WinRT, RAM hợp lý.
- **Quyết định:** Tauri 2 (Rust backend, WebView2), frontend Svelte 5 + TypeScript + Vite, quản lý gói bằng pnpm.
- **Lý do:**
  - Exe nhỏ nhất (~5–15 MB) trong nhóm làm được UI đẹp.
  - CSS/JS làm hiệu ứng nhanh hơn mọi stack native.
  - Crate `windows` gọi thẳng được Win32/WinRT.
  - Svelte biên dịch ra bundle nhỏ, có sẵn `Spring`.
- **Phương án đã cân nhắc:**
  - Electron: quá nặng.
  - WPF: gói self-contained 60–150 MB, spring phải tự viết.
  - Avalonia + NativeAOT: nhẹ RAM hơn nhưng làm UI tốn công hơn. **Đây là phương án dự phòng nếu trượt G0.**
  - C++/Rust thuần + Direct2D: tốn công gấp nhiều lần.
- **Hệ quả:**
  - RAM ~100 MB vì WebView2.
  - Phụ thuộc WebView2 Evergreen.
  - Web player dùng multiwebview của Tauri (đang gắn cờ `unstable`, xem R9).

## ADR-002 · Kiến trúc lấy Activity làm trung tâm

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Bối cảnh:** nhiều tính năng tranh nhau một chỗ hiển thị nhỏ. Nếu mỗi tính năng tự vẽ lên island thì code sẽ rối và khó thêm tính năng mới.
- **Quyết định:**
  - Module chỉ khai báo `Activity` (kind, priority, payload, actions).
  - `ActivityHub` quyết định activity chính, activity phụ và alert.
  - Frontend render qua view registry theo `ActivityKind`.
  - Hành động nhanh đi qua `activity_invoke`.
- **Lý do:** thêm module không phải sửa island; luật ưu tiên tập trung một chỗ và test được; mở đường cho plugin API (F44).
- **Hệ quả:**
  - Payload phải có kiểu rõ (enum), dùng chung Rust/TS qua ts-rs.
  - Mọi module phải mô tả activity trong module doc mục 4.

## ADR-003 · Cơ chế cửa sổ island

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted, kiểm chứng ở G0
- **Bối cảnh:** island cần bung/thu mượt, click xuyên ở vùng trong suốt, không cướp focus, và không bị antivirus nghi ngờ.
- **Quyết định:**
  - Một cửa sổ trong suốt **kích thước cố định** 480×264.
  - Click xuyên theo **hit region** do frontend gửi lên.
  - **Cursor poller** (`GetCursorPos`, 60/10 Hz) bật/tắt `ignore_cursor_events`.
  - Cửa sổ có `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW`.
  - Đặt lại topmost khi foreground đổi.
- **Phương án đã loại:**
  - Đổi kích thước cửa sổ theo từng frame: giật và nháy với WebView2.
  - Global mouse hook `WH_MOUSE_LL`: dễ bị antivirus gắn cờ, và mọi sự kiện chuột toàn hệ thống phải đi qua app.
  - `SetWindowRgn`: không hợp với cửa sổ trong suốt có glow.
- **Hệ quả:** có độ trễ tối đa ~16 ms khi chuột đi vào island, chấp nhận được. Cần xử lý riêng kéo thả file (xem [file-shelf](modules/file-shelf.md)).

## ADR-004 · Nhạc online theo mô hình lai

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Bối cảnh:** người dùng muốn mở trang nhạc, đăng nhập, chọn playlist, rồi thu gọn cho chạy ngầm như app trên điện thoại. Có các giới hạn: Google chặn đăng nhập trong trình duyệt nhúng; DRM (Spotify) có thể không chạy trong WebView2; điều khoản dịch vụ cấm tách luồng âm thanh.
- **Quyết định:** hai nguồn online dùng chung một giao diện điều khiển:
  1. **Web player nhúng** (F12) cho các trang qua được test matrix ở G0.
  2. **Điều khiển nguồn ngoài** qua GSMTC (F10) cho phần còn lại (app Spotify, trang đã đăng nhập trên Chrome…).
- **Ràng buộc cứng:**
  - Không tách luồng âm thanh, không tải nhạc.
  - Không chặn quảng cáo.
  - Không giả user agent để vượt chặn đăng nhập.
  - Trang web chạy trong webview cô lập, chỉ có một command `web_player_report`.
- **Hệ quả:** danh sách trang hỗ trợ chính thức chỉ chốt sau G0; RAM tăng khi web player mở; phải bảo trì adapter cho trang không dùng Media Session.

## ADR-005 · Lưu trữ & portable

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Quyết định:**
  - **Vị trí dữ liệu:** dữ liệu app (settings, DB, log) để ở `<exe>\data\` nếu ghi được, ngược lại ở `%APPDATA%\PCisland\`. Dữ liệu WebView2 (gồm cookie của web player) luôn ở `%LOCALAPPDATA%\PCisland\WebView2\`.
  - **Định dạng:** settings là JSON có schema version; dữ liệu có cấu trúc là SQLite (WAL) với migration nhúng; mỗi module sở hữu bảng có tiền tố tên module.
- **Lý do:**
  - Portable đúng nghĩa với dữ liệu người dùng.
  - Không vô tình gửi phiên đăng nhập khi nén thư mục chia sẻ.
  - Cache WebView2 lớn, không nên nằm cạnh exe.
- **Hệ quả:**
  - Gỡ sạch cần thêm `--cleanup` (hoặc nút trong Cài đặt).
  - Khóa autostart trong registry trỏ tới đường dẫn exe, nên phải đồng bộ lại khi người dùng di chuyển thư mục ([app-lifecycle](modules/app-lifecycle.md)).

## ADR-006 · Type sharing & quy ước IPC

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Quyết định:**
  - **Type:** dùng `ts-rs` sinh type TS từ DTO Rust vào `src/lib/ipc/bindings/`. Wrapper command được viết tay trong `src/lib/ipc/commands.ts`.
  - **Tên gọi:** command `<module>_<verb>`, event `<module>://<kebab>`. Lỗi có dạng `IpcError { code, message }`.
  - **State:** DTO trạng thái có `rev`. Backend gửi mốc thời gian, không gửi tick.
- **Phương án đã cân nhắc:** `tauri-specta` sinh được cả wrapper, nhưng độ ổn định trên Tauri 2 chưa chắc chắn. Xét lại sau v1.0.
- **Hệ quả:** CI phải kiểm tra bindings luôn khớp với Rust.

## ADR-007 · Chính sách ngôn ngữ

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Quyết định:**
  - Tài liệu viết tiếng Việt.
  - Tên định danh, comment code và commit message viết tiếng Anh.
  - UI có i18n từ ngày đầu, mặc định tiếng Việt, kèm tiếng Anh.
- **Lý do:** người phát triển và người dùng đầu tiên nói tiếng Việt; code tiếng Anh hợp với hệ sinh thái và công cụ; i18n làm từ đầu rẻ hơn nhiều so với thêm sau.

## ADR-008 · Mặc định sản phẩm

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted (chủ dự án chốt ngày 2026-10-31)
- **Quyết định:**

| Vấn đề | Mặc định đề xuất | Tùy chọn trong Cài đặt |
|---|---|---|
| Island hiện gì khi rảnh | **Đôi mắt thỉnh thoảng chớp** ([04 §11](04-design-system.md#11-đôi-mắt-màn-hình-chờ)), *đã chốt 2026-10-31* | Đồng hồ · Pill trống · Ẩn khi rảnh |
| Màn hình | Luôn hiện trên cùng, trên **mọi virtual desktop**; nhiều màn hình thì ở **màn hình chính** | Theo màn hình có chuột · Chọn cố định |
| Kiểu island | **Dính chặt mép trên** (kiểu notch MacBook, [04 §5](04-design-system.md#5-hình-học-island)) | Nổi cách mép trên (kiểu Dynamic Island) |
| Khi cửa sổ khác phóng to | Thu thành vạch mảnh nếu không có activity ≥ Live | Tắt |
| Alert khi đang fullscreen | Chỉ Critical (nhắc việc, hết giờ) | Không bao giờ · Mọi alert |
| Phím tắt | `Ctrl+Alt+Space` mở peek · `Ctrl+Alt+I` mở cửa sổ đầy đủ · `Ctrl+Alt+H` ẩn/hiện · `Ctrl+Alt+N` ghi chú nhanh. *Đã chốt: giữ mặc định, chỉnh sau được* | Đổi tùy ý |
| Hệ điều hành | Chỉ hỗ trợ chính thức Windows 11; không test và không sửa lỗi riêng cho Win10. *Đã chốt 2026-10-31* | — |
| Trang test web player (G0) | Zing MP3, SoundCloud, YouTube, Spotify Web, NhacCuaTui. *Đã chốt 2026-10-31* | Người dùng thêm URL khác |
| Phát hành | GitHub Releases: bộ cài + bản portable. Cá nhân & bạn bè ở v0.x; công khai từ v1.0 | — |
| Ký số | Xem ADR-009 | — |

## ADR-009 · Phân phối & ký số

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted. Bổ sung cho ADR-005 ở cách nhận biết bản portable.
- **Bối cảnh:** chủ dự án muốn phát hành qua GitHub Releases, cho người dùng chọn bộ cài hoặc bản portable, và giao cho kỹ thuật quyết định có ký số hay không.
- **Quyết định:**
  1. **Hai gói cho mỗi bản phát hành**, mỗi gói có SHA256:
     - `setup.exe`: NSIS, cài theo user, không cần admin.
     - `portable.zip`: exe + thư mục `data\` rỗng.
  2. **Nhận biết bản portable:** dựa vào thư mục `data\` cạnh exe, không dựa vào việc thư mục có ghi được hay không. Lý do: thư mục cài theo user (`%LOCALAPPDATA%\Programs`) cũng ghi được. Phần này thay cho quy tắc đoán theo quyền ghi trong ADR-005.
  3. **Ký số:**
     - Không mua chứng chỉ trả phí.
     - v0.x phát hành không ký.
     - Trước v1.0: nếu repo công khai mã nguồn mở thì đăng ký ký miễn phí qua **SignPath Foundation**. Điều kiện xét duyệt cần kiểm tra lại lúc đăng ký.
     - Không được duyệt thì tiếp tục không ký, kèm các biện pháp giảm thiểu ở dưới.
  4. **Giảm thiểu khi chưa ký:**
     - Phát hành kèm SHA256.
     - README có hướng dẫn "More info → Run anyway" kèm ảnh.
     - Bị antivirus báo nhầm thì gửi mẫu cho Microsoft Defender (Submit a file for analysis).
     - Không dùng global hook, không tự tải/thực thi code.
- **Lý do không mua chứng chỉ:**
  - Chứng chỉ OV/EV tốn khoảng 200–400 USD/năm, cần xác minh danh tính và token phần cứng.
  - Theo thay đổi của Microsoft từ 2024, ngay cả chứng chỉ EV cũng không còn được qua SmartScreen ngay lập tức; app vẫn phải tích lũy uy tín theo lượt tải.
  - Azure Trusted Signing rẻ hơn nhưng, theo hiểu biết hiện tại, chỉ mở cho một số quốc gia.
  - Với dự án cá nhân, chi phí lớn hơn lợi ích.
- **Xét lại khi:** antivirus báo nhầm thường xuyên; nhiều người dùng phàn nàn về cảnh báo; hoặc dự án có nguồn thu.
- **Hệ quả:**
  - CI build cả hai gói.
  - Bản cài có shortcut Start menu, nên dễ đăng ký AUMID cho toast ([reminders](modules/reminders.md)).
  - Gỡ bản cài bằng trình gỡ cài đặt (cũng xóa khóa autostart). Gỡ bản portable bằng `--cleanup` rồi xóa thư mục.

## ADR-010 · Payload Activity: JSON ở core, có kiểu theo từng module

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted (thay một phần hệ quả của ADR-002)
- **Bối cảnh:** ADR-002 yêu cầu payload là enum có kiểu dùng chung. Làm vậy thì `core::activity` phải biết mọi module, trái với ranh giới module (03 §1) và chặn plugin (F44).
- **Quyết định:**
  - Trong core, `Activity.payload` vẫn là `serde_json::Value`.
  - Mỗi module khai báo struct payload riêng có `#[derive(TS)]` (ví dụ `VolumePayload`, `PhaseEndPayload`, `CountPayload`) và tạo payload bằng `activity::payload(&T)`.
  - Frontend có một bảng `ActivityPayloads` (`src/lib/registry/payloads.ts`) ánh xạ `kind → kiểu`; view lấy payload qua `payloadOf(activity, kind)`, không ép kiểu tự do.
- **Lý do:** giữ core không phụ thuộc module mà vẫn có kiểu ở hai đầu; thêm module chỉ thêm một dòng vào bảng.
- **Phương án đã cân nhắc:** enum tổng trong core (vỡ ranh giới module); `unknown` + ép kiểu trong view (tình trạng cũ, dễ lệch).
- **Hệ quả:** khi đổi struct payload phải chạy lại `pnpm gen:bindings`; CI đã kiểm tra bindings không lệch.

## ADR-011 · EventBus và supervisor cho module

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Bối cảnh:** các module cần phản ứng với sự kiện chung (máy thức dậy, khóa máy, island bị ẩn, đổi bài, đổi thiết bị âm thanh). Panic trong task actor trước đây làm module chết im lặng.
- **Quyết định:**
  - `core::events::EventBus` là `tokio::sync::broadcast` của `AppEvent` (`Resumed`, `SessionLocked`, `IslandHidden`, `NowPlayingChanged`, `MediaIntent`, `AudioDeviceChanged`…). Lệnh có địa chỉ vẫn đi qua handle của module.
  - `core::supervisor::Supervisor::spawn(module, factory)` chạy service, bắt panic và khởi động lại sau 1 s / 5 s / 30 s; lỗi liên tiếp quá ngưỡng thì chuyển `disabled`. Trạng thái phát qua `app://modules` và hiện trong Cài đặt.
  - Kênh nhận của service bọc trong `Arc<tokio::Mutex<_>>` để lần khởi động lại dùng tiếp, không mất lệnh.
- **Lý do:** đúng 02 §3–§4 mà không cần trait `Module` nặng; mỗi module vẫn chỉ có `setup`.
- **Hệ quả:** sự kiện broadcast có thể bị `Lagged` → mọi vòng nhận phải bỏ qua lỗi này; module không được giả định nhận đủ mọi sự kiện cũ.

## ADR-012 · Manifest Common-Controls v6 nhúng qua linker

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Bối cảnh:** plugin dialog dùng `TaskDialogIndirect` (comctl32 v6). Binary test của Cargo không có manifest nên chết với `STATUS_ENTRYPOINT_NOT_FOUND`.
- **Quyết định:** `build.rs` nhúng `windows-app-manifest.xml` bằng `/MANIFEST:EMBED /MANIFESTINPUT` cho mọi binary (app và test), và tắt manifest mặc định của `tauri-build`.
- **Hệ quả:** sửa manifest ở một chỗ; `cargo test` chạy được trên máy sạch và CI.

## ADR-013 · Toolbar web player nằm ở cửa sổ đầy đủ

- **Ngày:** 2026-10-31 · **Trạng thái:** Accepted
- **Bối cảnh:** đặc tả web player có thanh công cụ (về trang chủ, đổi trang, xóa dữ liệu) trong chính cửa sổ web. Cửa sổ đó chạy trang từ xa; muốn vẽ toolbar cục bộ phải dùng multiwebview `unstable` (R9) hoặc mở thêm quyền IPC cho trang từ xa.
- **Quyết định:** trang từ xa chỉ được gọi `web_player_report`. Toolbar đặt ở tab **Nhạc › Online** của cửa sổ đầy đủ (`web_player_navigate`, `web_player_clear_data`, hiện/ẩn cửa sổ). Điều khiển phát vẫn qua island và phím media (SMTC).
- **Lý do:** giữ bề mặt tấn công nhỏ nhất; không phụ thuộc tính năng `unstable`.
- **Xét lại khi:** multiwebview của Tauri ổn định.

---

## ADR-014 · Giữ island tối giản khi còn tác vụ

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo yêu cầu chủ dự án; thay phần mặc định ẩn tác vụ trong ADR-008.
- **Bối cảnh:** nhạc/tác vụ đang chạy bị mất island khi vào fullscreen; người dùng muốn luôn theo dõi và dùng tính năng khác ngay trên island.
- **Quyết định:** thêm `island.hide_while_active = false`. Khi fullscreen hoặc dùng lệnh ẩn, còn activity không phải Idle/alert thì giữ minimal; bật công tắc mới mới cho phép ẩn hẳn. Khóa máy vẫn ẩn. Quy tắc alert fullscreen hiện có được giữ.
- **Giao diện:** compact media và minimal có tiến trình khi biết thời lượng; peek có nút mở launcher và quay lại tác vụ mà không mở main.
- **Hệ quả:** settings cũ tự nhận giá trị mặc định false qua serde; không thay lựa chọn hide_in_fullscreen cũ. Không áp dụng cam kết hiển thị trên màn hình khóa/secure desktop.

## ADR-015 · Tương tác island và lưu phiên theo yêu cầu sử dụng thực tế

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo yêu cầu chủ dự án.
- **Quyết định:** Nhạc và Online dùng chung một menu; lịch sử/cookie của web player được giữ, từng site có Chạy tiếp hoặc Mở trang gốc. Danh sách phát dùng dữ liệu local hoặc DOM do website cung cấp, không tách/tải luồng nhạc.
- **Thay thế hành vi cũ:** Pomodoro bấm lần hai dừng hẳn (không pause), thông báo native Windows 3 giây. Bấm giờ chỉ mở panel. Mọi file kéo vào island vào kệ tối đa 3 slot, bỏ lựa chọn Phát ngay tại vùng drop.
- **Ghi chú:** quy tắc nhắc riêng từng dòng; chủ dự án đã chọn dòng checklist ngay phía trên làm điều kiện. Lưu ID và deadline trong SQLite, không dùng timer trong component. Thuộc mở rộng F26/F24 được yêu cầu trực tiếp.
- **Hệ quả:** thêm migration 0008; không hạ về bản schema 7 trên dữ liệu đã nâng. Việc tự phát và queue online phụ thuộc trang; có đường lui mở trang sau 30 giây. Kiểm thử reboot và tài khoản thật vẫn cần thực hiện.

## ADR-016 · Tìm bài trong app và giới hạn tự động hóa website nhạc

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo yêu cầu tìm kiếm/chọn nguồn của chủ dự án; SoundCloud bị hạn chế sau sự cố QA.
- **Quyết định:** tìm kiếm bằng cửa sổ nền riêng, chung profile, không phát âm thanh và chỉ có IPC trả kết quả. Player giữ phiên hiện tại trong lúc tìm. Island mặc định hiển thị nguồn đang mở và Mở trang/Đổi trang/Tìm bài.
- **Bằng chứng:** Zing trả metadata playlist qua response của trang và queue qua một khóa localStorage xác định; không tiếp tục dùng selector album phỏng đoán. Đã thử native chạy tiếp sau restart, chọn queue và tìm→phát.
- **Giới hạn:** SoundCloud chặn unusual activity khi QA; dừng truy cập tự động, tắt tìm kiếm nền của nguồn này, không vượt chặn. Restore bỏ click lặp; các provider chưa kiểm chứng đầy đủ không được ghi là đã hỗ trợ trọn vẹn. Spotify chỉ xác nhận kết quả tìm kiếm trong WebView; phát trên Chrome không chứng minh phát trong app.
- **Hệ quả:** thêm capability riêng cho search; không giữ credential/stream URL trong kết quả. Có thể phải chuyển sang tích hợp được nhà cung cấp hỗ trợ trước khi bật lại tìm kiếm SoundCloud.

## ADR-017 · Island mở rộng theo nội dung tương tác

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo yêu cầu chủ dự án.
- **Bối cảnh:** chooser nhạc cắt các nút mới hiện, playlist bị ép trong chiều cao player. Chạy tiếp quay về lưới tính năng do dùng chung callback quay lại.
- **Quyết định:** thay phần kích thước cố định trong ADR-003: giữ ngang 480, cao native 264–616 theo hit region; peek đo nội dung tối đa 600. Resize canvas trước morph, thu sau morph; click xuyên vẫn theo vùng island, poller tính vùng gần theo chiều cao mới. Queue có tối đa 340 px cho các dòng bài trước khi cuộn.
- **Điều hướng:** callback phát thành công chuyển về player của nguồn đã chọn, độc lập Quay lại của launcher. Hiện trạng thái tải khi chờ metadata, lỗi IPC giữ panel.
- **Kiểm chứng:** test tích hợp nội dung/điều hướng, test Rust canvas và QA trình duyệt với fixture offline. Không truy cập lại website nhạc.

## ADR-018 · Trình phát video nổi: tự lấy stream và phát (phương thức B), trình phát nhúng làm đường lui

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo quyết định của chủ dự án.
- **Bối cảnh:** F46: dán link YouTube/Facebook/trang có video → một cửa sổ nhỏ chỉ có video + nút. Có hai cách:
  - (A) trình phát nhúng chính thức;
  - (B) giải link ra luồng thật rồi phát bằng trình phát của app, giống addon stream-pocket bắt request mạng.
- **Quyết định:**
  - Làm **B** làm phương thức chính:
    - yt-dlp (tải khi người dùng đồng ý, kiểm SHA-256) giải link YouTube/Facebook;
    - WebView ẩn + `WebResourceResponseReceived` dò luồng trang bất kỳ;
    - proxy `vstream://` gắn header + Range, chỉ chuyển tiếp URL đã giải;
    - trang cục bộ phát bằng cặp `<video>` + `<audio>` đồng bộ (MSE nếu spike S2 không đạt) hoặc hls.js.
  - **A** (trình phát nhúng) chỉ làm đường lui khi B không phát được.
  - Tối đa 1 cửa sổ, có phím tắt, island điều khiển được qua media hub (nguồn `video`).
- **Lý do:** chủ dự án muốn trình phát không bị trang gốc can thiệp (quảng cáo, giao diện thừa) và có toàn quyền điều khiển (island, phím, chất lượng). Chỉ B làm được việc này.
- **Phương án đã cân nhắc:**
  - A làm chính: ít code nhưng vẫn là trình phát của trang gốc.
  - Nhúng libmpv: mạnh nhưng nặng (~30 MB), khó vẽ nút đè lên.
  - Tự viết bộ giải chữ ký YouTube: không làm, dùng yt-dlp được cộng đồng bảo trì.
- **Hệ quả:**
  - Phụ thuộc yt-dlp và tốc độ cập nhật của nó; phải có cơ chế tự cập nhật và đường lui A.
  - App chạy một exe ngoài, có thể bị antivirus cảnh báo.
  - Cần spike S2 (đồng bộ tiếng-hình) trước khi code trình phát, và S1 trước khi bật A.
  - Rủi ro về điều khoản của YouTube/Facebook do chủ dự án chấp nhận.

## ADR-020 · File theo nhóm, note desktop và meeting nhẹ

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo yêu cầu trực tiếp của chủ dự án; nghiệm thu release còn mở.
- **Quyết định:** bỏ giới hạn 3 của ADR-015; phân trang UI 50 mục, native drag/CF_HDROP file thực, ZIP stream và đổi ảnh một mục. Kết quả lưu tại vị trí người dùng chọn; chưa có kho output tự dọn. Tạo một note WebView theo yêu cầu, giữ draft và CAS bằng updated_at tăng đơn điệu.
- **Meeting:** WASAPI native, Opus mono 48 kHz/48 kbps, journal packet bền và remux highlight lùi 15 giây; hàng đợi giới hạn, không giữ PCM cả buổi trong RAM. Không kèm model AI/FFmpeg. Ghi chú cá nhân độc lập transcript; chỉ clip có yêu cầu mới được gửi.
- **Groq:** chủ dự án chọn nhúng key từ `.env` lúc build vào native, thay đề xuất backend trung gian trong plan. Không coi đây là cách giấu key khỏi người có binary; mọi người dùng chung hạn mức. Thiếu key vẫn dùng phần local. Không đưa key vào Git/frontend/log.
- **Dependencies mới:** `opus 0.3.1` (MIT, native libopus BSD-3-Clause), `ogg 0.9.2` (BSD-3-Clause), `zip 2.6.1` (MIT), `image =0.25.5` (MIT OR Apache-2.0, chỉ PNG/JPEG/WebP); bật multipart của reqwest hiện có. Cargo.lock là nguồn phiên bản chính xác. Native libopus cần CMake lúc build, không cần cài runtime riêng. Chưa đo delta exe release nên không công bố số MB tăng.
- **QA:** flag debug-only `--qa` bắt buộc data-dir riêng, bỏ đăng ký autostart/tray/hotkey để không thay phiên người dùng. Không áp dụng vào release.
- **Hệ quả:** thêm IPC/capability local, protocol audio kiểm tra ID từ storage; shared archive phục vụ shelf/meeting. Mất thiết bị, crash, DPI, attachment chat và họp dài cần nghiệm thu trước phát hành.

## Template

## ADR-019 · Dọn rác một chạm và thu hồi RAM của app nền toàn PC

- **Ngày:** 2026-11-01 · **Trạng thái:** Accepted theo yêu cầu trực tiếp của chủ dự án; chính sách disk và "không kill" được thay bằng ADR-022.
- **Bối cảnh:** chủ dự án yêu cầu triển khai quick wins, làm rõ RAM phải ở phạm vi PC và dọn rác giống CCleaner: một lần bấm, có tiến trình/kết quả, không chọn từng nhóm.
- **Quyết định:** thêm service `app/maintenance` với adapter Win32 ở `platform`, chỉ chạy khi bấm. Disk theo allowlist nguồn/file cache và file tạm quá 7 ngày, bỏ app đang chạy/file đang dùng, không theo reparse point, giữ dữ liệu đăng nhập/cá nhân; worker báo tiến trình qua channel. RAM dùng bộ lọc bảo thủ và PID + thời điểm khởi chạy, trim working set các app nền được người dùng chọn, đo trước/sau/30 giây; không kill, không purge standby.
- **Lý do:** không có phép suy luận chung rằng mọi cache/bộ nhớ chiếm dụng đều vô giá trị. Giới hạn rõ recipe và đo kết quả thực tế để tránh ảnh hưởng dữ liệu/phiên đang dùng.
- **Hệ quả:** chưa bao phủ mọi app/profile tùy chỉnh; không hứa mức MB hoặc tăng tốc. Không triển khai mới unload/suspend web player vì yêu cầu đã mở rộng sang PC. [Đặc tả, phạm vi và nghiệm thu](modules/maintenance.md).

```markdown
## ADR-021 · Spotify/SoundCloud: đọc trạng thái từ response của trang, repeat bằng nút của trang

- **Ngày:** 2026-11-02 · **Trạng thái:** Accepted theo yêu cầu chủ dự án. Thay phần giới hạn SoundCloud trong ADR-016.
- **Bối cảnh:** chủ dự án đã đăng nhập SoundCloud, phát được, và yêu cầu làm tracklist, tìm kiếm, repeat cho SoundCloud và Spotify; cho phép trace API và tự nghiệm thu để tránh hoạt động giống bot.
- **Quyết định:**
  - Spotify: bridge móc `fetch`/`WebSocket` của trang, đọc `player_state` từ connect-state (PUT/GET + payload dealer) và tên bài từ `decorateContextTracks`. Không gọi API riêng, không giữ token/device ID/URL audio.
  - SoundCloud: đọc queue từ panel “Next up” của trang; bật lại tìm kiếm bằng một lượt tải trang `/search/sounds` cho mỗi lần người dùng bấm tìm.
  - Repeat của hai trang dùng nút repeat của chính trang, không dùng `media.loop` hay chặn sự kiện `ended`.
- **Giới hạn:** không tự thử lại, không tìm nền, không vượt chặn; gặp trang chặn thì dừng như ADR-016. Trace Spotify được làm trên profile thật với một lần tải lại và một lần phát/dừng; SoundCloud chỉ đọc DOM và mở panel queue một lần. Trang đổi cấu trúc thì repeat/queue báo lỗi thay vì đoán.
- **Hệ quả:** phụ thuộc vào cấu trúc nội bộ của Spotify (endpoint/field có thể đổi). Nhãn `aria-label` của Spotify phụ thuộc ngôn ngữ, chỉ dùng làm dự phòng khi chưa có `options` từ cluster.

## ADR-022 · Mở rộng Dọn dẹp: nguồn rác lớn, Thùng rác, Windows Update và đóng app ngốn RAM

- **Ngày:** 2026-11-02 · **Trạng thái:** Accepted theo yêu cầu trực tiếp của chủ dự án. Thay thế phần chính sách disk (tuổi 7 ngày, chỉ đuôi `.tmp/.temp/.log/.etl`, danh sách nguồn) và phần "không kill" của ADR-019; các phần khác của ADR-019 giữ nguyên.
- **Bối cảnh:** chủ dự án nghiệm thu thấy Dọn dẹp "không có ý nghĩa", dọn quá ít. Chủ dự án chọn: thêm nguồn rác lớn, Thùng rác + Windows Update, và đóng app nền ngốn RAM thay vì chỉ trim working set (trim gần như không giảm cấp phát).
- **Quyết định:**
  - Disk một chạm: file tạm (LocalAppData/Temp, Windows/Temp) mọi loại file quá 1 ngày, vẫn giữ tên `~`, autosave/recovery/backup và file đang khóa; xóa thư mục con rỗng trong root temp/cache bằng handle (Windows từ chối nếu còn entry), không bao giờ xóa root, không remove_dir_all. Thêm CrashDumps, WER (LocalAppData và ProgramData khi tài khoản có quyền), cache shader DirectX/NVIDIA/AMD, `thumbcache_*/iconcache_*.db`, INetCache (trừ Content.Outlook), npm/pip/Yarn (quá 7 ngày), Teams mới/cũ, Spotify `Data`/`Storage`. Cache trình duyệt/Electron/Teams/Spotify giảm tuổi xuống 1 ngày nhưng vẫn bỏ cả app khi tiến trình chạy; nguồn dùng chung (shader, thumbnail, INetCache, WER, npm…) không chặn theo tiến trình vì file đang dùng bị khóa và được bỏ qua. Kết quả theo nhóm: ước tính khi quét và byte thực xóa.
  - Thùng rác: thao tác riêng, xác nhận trong UI, `SHEmptyRecycleBinW`; không nằm trong một chạm.
  - Windows Update/Delivery Optimization: thao tác riêng, xác nhận, chạy lại chính `pcisland.exe` với đúng một đối số cố định qua `ShellExecuteExW` "runas"; helper chỉ chạy recipe cố định rồi thoát với mã trạng thái + KiB đã xóa. Từ chối UAC báo "chưa dọn".
  - RAM: danh sách app của cùng người dùng/phiên gom theo gốc cây tiến trình, loại hệ thống và cây PCisland; nút Đóng gửi `WM_CLOSE` tới cửa sổ cấp cao sau xác nhận, chờ 5 giây. Chỉ khi app chưa thoát mới cho "Buộc đóng" với xác nhận thứ hai; backend chỉ terminate đúng PID + thời điểm tạo mà bước một để lại. App đang phát âm thanh cần xác nhận riêng. Trim working set giữ lại như mục nâng cao.
- **Lý do:** đây là các nguồn Storage Sense/Disk Cleanup cũng coi là tái tạo được; quy tắc an toàn cốt lõi (allowlist cố định, handle, không theo reparse point, không dữ liệu đăng nhập/cá nhân, không đường dẫn hay lệnh từ UI) vẫn giữ. Thao tác không hoàn tác hoặc cần quyền admin tách riêng và luôn hỏi lại.
- **Phương án đã cân nhắc:** PowerShell/cleanmgr nâng quyền (khó báo kết quả trung thực, phụ thuộc cấu hình sageset); dừng dịch vụ wuauserv/BITS trước khi xóa (rủi ro hỏng cập nhật đang chạy, bỏ); kill thẳng app (mất dữ liệu chưa lưu, bỏ); gom app theo tên exe (gộp WebView2 của nhiều app, bỏ).
- **Hệ quả:** dọn được nhiều hơn rõ rệt nhưng cache tạo lại khi dùng; Spotify có thể phải tải lại bài offline; dev cache có thể phải tải lại gói. Cần nghiệm thu UAC thật, đóng app có hộp thoại lưu, app ẩn khay. [Đặc tả](modules/maintenance.md).

## ADR-023 · Nhạc tập trung tải theo yêu cầu từ repo phát hành riêng

- **Ngày:** 2026-11-02 · **Trạng thái:** Accepted theo yêu cầu chủ dự án.
- **Bối cảnh:** Pomodoro cần nhạc tập trung/thư giãn có sẵn, nhưng nhúng vào bộ cài sẽ làm gói nặng thêm hàng chục đến hàng trăm MB.
- **Quyết định:** nhạc nằm ở release của repo public riêng `zeusato/pcisland-sounds` kèm `manifest.json`. App chỉ tải khi người dùng bấm, ghép URL file từ repo cố định, kiểm tra kích thước + SHA-256, lưu trong `data_dir/focus-sounds`. Timer báo pha qua EventBus; module `focus_sounds` yêu cầu local_player phát file theo vai trò `focus`/`break`. Định dạng MP3 vì rodio không giải mã Opus.
- **Phương án đã cân nhắc:** nhúng vào bộ cài (nặng); release trong repo dự án (lẫn với `releases/latest` của kiểm tra cập nhật); Git LFS (giới hạn băng thông); app tự sinh tiếng ồn nền (không đáp ứng nhu cầu nhạc).
- **Hệ quả:** phụ thuộc GitHub Releases. Chủ dự án chịu trách nhiệm quyền phân phối nhạc (Suno gói trả phí hoặc giấy phép tương đương) và ghi trong manifest. Đổi repo cần sửa hằng số trong `modules/focus_sounds`.

## ADR-NNN · <Tiêu đề>

- **Ngày:** YYYY-MM-DD · **Trạng thái:** Proposed | Accepted | Superseded by ADR-xxx | Rejected
- **Bối cảnh:** vấn đề, ràng buộc, lực tác động.
- **Quyết định:** chọn gì, cụ thể.
- **Lý do:** vì sao chọn.
- **Phương án đã cân nhắc:** các lựa chọn khác và lý do loại.
- **Hệ quả:** điều trở nên dễ hơn/khó hơn, việc phải làm thêm.
```
