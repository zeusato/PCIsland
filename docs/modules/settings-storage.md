# Module: Settings & storage (`settings`, `storage`)

> Trạng thái: Thiết kế · Phase: P1 · Tính năng: hạ tầng · Cập nhật: 2026-10-31

## 1. Mục đích

Cho mọi module một chỗ đọc/ghi cấu hình có kiểu và một nơi lưu dữ liệu có cấu trúc. Hai thứ này phải bền: không mất dữ liệu khi crash hay mất điện, nâng cấp phiên bản an toàn, và portable đúng [ADR-005](../06-decision-log.md#adr-005--lưu-trữ--portable).

## 2. Phạm vi

**Trong phạm vi:**
- Xác định đường dẫn và portable mode.
- `settings.json`: schema, migration, ghi nguyên tử, thông báo thay đổi.
- SQLite: kết nối, migration, luồng `db`.
- Xuất gói chẩn đoán; xóa dữ liệu WebView2.

**Ngoài phạm vi:**
- Ý nghĩa từng khóa cài đặt: thuộc module sở hữu.
- Giao diện trang Cài đặt: thuộc [full-window](full-window.md).

## 3. Hành vi & luồng người dùng

1. **Lần chạy đầu:** tạo `data/` cạnh exe (hoặc `%APPDATA%\PCisland`), ghi `settings.json` mặc định, tạo `pcisland.db`.
2. **Đổi một cài đặt trong `main`:** ghi ngay vào bộ nhớ, module nhận `SettingsChanged` và áp dụng tức thì, file được ghi sau 500 ms.
3. **File settings hỏng (sửa tay sai cú pháp):** nạp `settings.json.bak`; không có backup thì dùng mặc định. Island hiện alert "Cài đặt bị lỗi, đã khôi phục" kèm hành động mở thư mục.
4. **Nâng cấp lên bản mới:** migration settings và DB chạy tự động. Trước migration DB có tạo bản sao `pcisland.db.pre-<ver>`.
5. **Hạ cấp xuống bản cũ:** DB có `user_version` cao hơn bản app hiểu thì không ghi đè. App báo lỗi rõ ràng và chạy ở chế độ chỉ đọc cho module bị ảnh hưởng.

## 4. Activity

| Kind | Ưu tiên | Alert | Hành động |
|---|---|---|---|
| `settings.recovered` | Alert (ttl 10 s) | "Cài đặt bị lỗi, đã khôi phục" | Mở thư mục |
| `storage.error` | Alert (ttl 10 s) | "Không ghi được dữ liệu" | Mở chẩn đoán |

## 5. Thiết kế kỹ thuật

**Xác định đường dẫn** (`core/paths.rs`):
1. Có `--data-dir` thì dùng nó.
2. Ngược lại, nếu thư mục `<exe_dir>\data\` tồn tại thì đây là **bản portable** (zip phát hành có sẵn thư mục `data\` rỗng) và dùng thư mục đó. Nếu thư mục này không ghi được (USB khóa ghi…) thì dùng `%APPDATA%\PCisland\` kèm alert.
3. Còn lại là **bản cài đặt** và dùng `%APPDATA%\PCisland\`. Bản cài đặt nằm ở `%LOCALAPPDATA%\Programs\PCisland` là thư mục ghi được, nên **không** được dựa vào việc thư mục có ghi được hay không để đoán bản portable ([ADR-009](../06-decision-log.md#adr-009--phân-phối--ký-số)).
4. WebView2 luôn ở `%LOCALAPPDATA%\PCisland\WebView2\{app,web-player}`.

**Settings** (`core/settings/`):

```rust
#[derive(Serialize, Deserialize, Default, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    pub app: AppSettings,
    pub island: IslandSettings,
    pub activity: ActivitySettings,
    pub modules: ModulesSettings,     // { <id>: { enabled: bool } }
    pub media: MediaSettings,
    pub local_player: LocalPlayerSettings,
    // ... one section per module
    #[serde(flatten)]
    pub unknown: serde_json::Map<String, serde_json::Value>, // preserved for forward compatibility
}
```

- **Nạp:** đọc file → `serde_json::Value` → chạy chuỗi `migrate_vN_to_vN+1(&mut Value)` tới phiên bản hiện tại → deserialize thành struct. Lỗi thì thử `.bak`, rồi tới mặc định.
- **Ghi:** serialize ra `settings.json.tmp` → `fsync` → `rename` đè `settings.json`. Lần đọc thành công đầu tiên của mỗi phiên sao chép file thành `.bak`.
- **Debounce:** ghi 500 ms sau thay đổi cuối; tắt app thì flush ngay.
- **Phạm vi truy cập:** `SettingsHandle` của module chỉ đọc/ghi section của module đó (`ctx.settings.get::<TimerSettings>()`, `ctx.settings.update(|s: &mut TimerSettings| …)`).
- **Thông báo:** đổi section nào thì phát `AppEvent::SettingsChanged { section }` và event `settings://changed { section, value, rev }` tới `main` và `island`.
- **Kiểm tra giá trị:** mỗi struct có `fn validate(&mut self)` để kẹp giá trị vào miền hợp lệ (ví dụ `hover_delay_ms` trong 100–800).

**Storage** (`core/storage/`):
- `rusqlite` với feature `bundled`; một kết nối duy nhất trên luồng `db`. `PRAGMA journal_mode=WAL; synchronous=NORMAL; foreign_keys=ON`.
- `StorageHandle::run(|conn| …) -> Result<T>` gửi closure sang luồng `db` qua channel và chờ kết quả qua `oneshot`. Module bọc các truy vấn của mình trong `store.rs`.
- **Migration:** `rusqlite_migration` với các file `src-tauri/migrations/NNNN_<module>_<name>.sql` nhúng bằng `include_str!`, đánh số toàn cục tăng dần. `PRAGMA user_version` lưu phiên bản.
- **Quy ước bảng:**
  - Tên bảng `<module>_<thực thể>`, khóa chính `id TEXT` (UUID v7).
  - Thời gian lưu cột `*_at INTEGER` (epoch ms UTC).
  - Có cột `created_at` và `updated_at`.
- **Bảng dùng chung:** `kv (key TEXT PRIMARY KEY, value TEXT, updated_at INTEGER)` cho state nhỏ không thuộc settings (ví dụ phiên bản đã bỏ qua, lần kiểm tra cập nhật gần nhất).

**Gói chẩn đoán:** zip gồm `logs/` (7 ngày), `settings.json` (đã lọc các khóa được đánh dấu `#[sensitive]`), `system.txt` (phiên bản Windows/WebView2/app, danh sách màn hình + DPI, module đang bật). Lưu ra Desktop.

**Xóa dữ liệu web:** đóng web player rồi xóa `WebView2\web-player`. "Dọn toàn bộ" thì xóa cả `%LOCALAPPDATA%\PCisland`, áp dụng ở lần chạy sau.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `settings_get` | `() → Settings` |
| `settings_update` | `(section: string, patch: JsonValue) → Settings` (merge sâu, rồi validate) |
| `settings_reset_section` | `(section: string) → Settings` |
| `storage_open_data_folder` | `() → ()` |
| `storage_export_diagnostics` | `() → string` (đường dẫn file zip) |
| `storage_clear_web_data` | `() → ()` |

| Event | Payload |
|---|---|
| `settings://changed` | `{ section, value, rev }` |

## 7. Dữ liệu & cài đặt

- **File:** `settings.json`, `settings.json.bak`, `pcisland.db` (+ `-wal`, `-shm`), `logs/`.
- **Bảng:** `kv`.
- **Khóa cài đặt:** mỗi module tự khai báo trong tài liệu của mình. Tài liệu này chỉ sở hữu `schemaVersion` và `modules.<id>.enabled` (mặc định `true` cho mọi module của phase đã phát hành, `false` cho module thử nghiệm).

## 8. Phụ thuộc

- **Crate:** `serde`, `serde_json`, `rusqlite` (bundled), `rusqlite_migration`, `zip`, `directories` (hoặc `windows` `SHGetKnownFolderPath`).
- **Platform:** `KnownFolders`.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Mất điện khi đang ghi settings | Ghi nguyên tử, nên file cũ vẫn nguyên |
| Đĩa đầy | Ghi thất bại → alert `storage.error`, giữ trong bộ nhớ, thử lại sau 30 s |
| DB khóa (mở bằng tool khác) | Thử lại với `busy_timeout = 2000`; vẫn lỗi thì alert |
| Migration DB lỗi giữa chừng | Mỗi migration chạy trong transaction; lỗi thì rollback, khôi phục từ `.pre-<ver>`, chạy chế độ chỉ đọc cho module liên quan |
| Settings có khóa lạ (từ bản mới hơn) | Giữ nguyên trong `unknown`, ghi lại không mất |
| Thư mục app đổi từ ghi được sang chỉ đọc | Lần chạy sau chuyển sang `%APPDATA%`. **Không** tự sao chép dữ liệu cũ; báo người dùng và có nút "Nhập dữ liệu từ thư mục cũ" |

## 10. Hiệu năng

- Đọc settings từ bộ nhớ, O(1). Ghi file tối đa 2 lần/giây.
- Truy vấn DB của module không được vượt 50 ms trên luồng `db`. Truy vấn nặng (quét thư viện nhạc) phải chia lô.

## 11. Kiểm thử

**Unit:**
- Mọi hàm migrate settings, với file mẫu của từng phiên bản đặt trong `src-tauri/tests/fixtures/settings/`.
- `validate()` kẹp giá trị.
- Giữ khóa lạ khi ghi lại.
- Chọn đường dẫn: portable, chỉ đọc, `--data-dir`, Program Files.

**Integration:** migration DB từ 0 lên mới nhất trên DB tạm; giả lập lỗi migration rồi kiểm tra rollback.

**Checklist thủ công:**
- [ ] Sửa hỏng `settings.json` → khôi phục từ `.bak`, có alert
- [ ] Chạy từ thư mục chỉ đọc → dữ liệu sang `%APPDATA%`
- [ ] Xuất gói chẩn đoán → zip đủ nội dung, không có dữ liệu nhạy cảm

## 12. Câu hỏi mở

Không có.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
