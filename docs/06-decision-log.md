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

## Template

```markdown
## ADR-NNN · <Tiêu đề>

- **Ngày:** YYYY-MM-DD · **Trạng thái:** Proposed | Accepted | Superseded by ADR-xxx | Rejected
- **Bối cảnh:** vấn đề, ràng buộc, lực tác động.
- **Quyết định:** chọn gì, cụ thể.
- **Lý do:** vì sao chọn.
- **Phương án đã cân nhắc:** các lựa chọn khác và lý do loại.
- **Hệ quả:** điều trở nên dễ hơn/khó hơn, việc phải làm thêm.
```
