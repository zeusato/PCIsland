# Module: Kệ file (`shelf`)

> Trạng thái: Thiết kế · Phase: P3 · Tính năng: F27 · Cập nhật: 2026-10-31

## 1. Mục đích

Một chỗ để file tạm thời khi kéo thả giữa các cửa sổ: kéo file từ Explorer thả vào island, chuyển sang app khác, rồi kéo từ island ra. Không phải mở hai cửa sổ cạnh nhau.

## 2. Phạm vi

**Trong phạm vi:**
- Thả file/thư mục vào island.
- Danh sách mục trên peek và trong `main`.
- Kéo mục ra ngoài; mở, mở thư mục chứa, sao chép đường dẫn, xóa.
- Tự dọn mục quá hạn.

**Ngoài phạm vi:**
- Đồng bộ đám mây, chia sẻ.
- Thả văn bản/URL: backlog.
- Nén/chuyển đổi file.

## 3. Hành vi & luồng người dùng

1. **Thả vào:** kéo file từ Explorer lên island. Khi chuột vào pill, island bung thành vùng thả lớn. Nếu có file âm thanh thì chia hai vùng "Phát ngay" / "Cất vào kệ" ([local-player](local-player.md)). Thả xong: âm `pop` (nếu bật), alert "Đã cất 3 mục".
2. **Xem:** compact (khi rảnh) hiện icon kệ + số mục. Peek hiện lưới thumbnail (tối đa 8, cuộn ngang nếu nhiều hơn).
3. **Kéo ra:** kéo thumbnail từ peek thả vào Explorer, Zalo, Gmail… File được kéo dưới dạng **tham chiếu tới file gốc**, không phải bản sao.
4. **Thao tác:** chuột phải một mục → Mở · Mở thư mục chứa · Sao chép đường dẫn · Xóa khỏi kệ. Nút "Dọn kệ".
5. **Tự dọn:** mục quá `shelf.expire_hours` (24 giờ) tự rời kệ. File gốc không bị đụng tới.

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `shelf.items` | Ambient | Icon kệ + số mục | Lưới thumbnail, kéo ra được | — | `clear`, `open_main` |
| `shelf.dropped` | Alert (ttl 1,5 s) | — | — | "Đã cất N mục" | — |
| Trạng thái thả (frontend) | — | — | Vùng thả lớn, viền cam đứt nét | — | — |

## 5. Thiết kế kỹ thuật

**Nhận thả:**
- Cửa sổ island đang ở chế độ click xuyên. [island-window](island-window.md) xử lý như sau: khi chuột (đang giữ nút trái) đi vào hit region, poller tắt ignore để cửa sổ nhận `DragEnter`.
- Frontend dùng `onDragDropEvent` của Tauri (`enter`/`over`/`drop`/`leave`) để lấy danh sách đường dẫn.
- Khi `enter`: frontend chuyển sang trạng thái `dropTarget` (bung pill thành vùng thả) và gửi hit region mới, rộng hơn.
- **Hạn chế đã biết:** island không biết khi nào người dùng *bắt đầu* kéo ở nơi khác, nên chỉ bung khi chuột đã chạm pill. Chấp nhận ở v1.

**Lưu mục:**
- Mặc định lưu **tham chiếu**: đường dẫn, kích thước, `mtime`.
- Tùy chọn `shelf.copy_mode = "copy"`: sao chép vào `data/shelf/<id>/`, dùng khi muốn giữ bản tạm dù file gốc bị xóa. Giới hạn 500 MB tổng; vượt thì từ chối và báo.

**Thumbnail:**
- Platform `ShellThumbnails`: `IShellItemImageFactory::GetImage` 96×96 → PNG → `ArtworkCache`.
- Không có thumbnail thì dùng icon theo loại file (`SHGetFileInfo`).

**Kéo ra:**
- Dùng `tauri-plugin-drag` (hoặc crate `drag`), bắt đầu thao tác kéo OLE với danh sách đường dẫn từ webview.
- **Kiểm chứng ở P3**: kéo ra từ cửa sổ `WS_EX_NOACTIVATE` vào Explorer và trình duyệt.

**Dọn dẹp:** kiểm tra mục quá hạn khi khởi động và mỗi giờ (chỉ khi kệ có mục). Mục có file gốc biến mất thì đánh dấu `missing` (thumbnail mờ); kéo ra bị chặn.

## 6. IPC

| Command | Chữ ký |
|---|---|
| `shelf_get_state` | `() → ShelfSnapshot { items: ShelfItemDto[], rev }` |
| `shelf_add` | `(paths: string[]) → ShelfItemDto[]` |
| `shelf_remove` | `(ids: string[]) → ()` |
| `shelf_clear` | `() → ()` |
| `shelf_open` / `shelf_reveal` | `(id) → ()` |
| `shelf_copy_paths` | `(ids) → ()` (ghi clipboard) |
| `shelf_start_drag` | `(ids) → ()` |

| Event | Payload |
|---|---|
| `shelf://state` | `ShelfSnapshot` |

## 7. Dữ liệu & cài đặt

**Bảng:** `shelf_items (id, path, name, size, mtime, is_dir, copy_path, added_at, missing)`

| Khóa | Mặc định |
|---|---|
| `shelf.expire_hours` | `24` (0 = không tự dọn) |
| `shelf.copy_mode` | `"reference"` (`"reference"` · `"copy"`) |
| `shelf.max_items` | `30` |
| `shelf.sound_on_drop` | `false` |

## 8. Phụ thuộc

- **Core:** ActivityHub, storage, settings, ArtworkCache, SoundService.
- **Platform trait:** `ShellThumbnails`, `ShellOpen` (mở file / mở thư mục chứa).
- **Crate:** `tauri-plugin-drag` (hoặc `drag`), `windows` (`Win32_UI_Shell`).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Thả thư mục rất lớn | Chỉ lưu tham chiếu thư mục, không duyệt nội dung |
| Thả hơn `max_items` | Nhận tới giới hạn; alert "Kệ đầy" |
| File gốc bị xóa/đổi tên | `missing`, thumbnail mờ, chặn kéo ra |
| Thả từ app chạy quyền admin (UIPI) | Windows chặn kéo từ tiến trình quyền cao sang tiến trình thường; ghi trong hướng dẫn |
| Đang fullscreen | Island ẩn nên không thả được; chấp nhận |

## 10. Hiệu năng

Thumbnail tạo một lần khi thả và cache theo hash. Công việc dọn dẹp chỉ chạy khi kệ có mục.

## 11. Kiểm thử

**Unit:** hết hạn theo TestClock; giới hạn số mục và dung lượng chế độ copy; đánh dấu `missing`.

**Checklist thủ công:**
- [ ] Thả 1, 5, 30 file từ Explorer; thả thư mục
- [ ] Kéo ra Explorer (di chuyển/sao chép theo phím giữ), Chrome (upload), Zalo/Telegram
- [ ] Thả file nhạc: hiện hai vùng "Phát ngay" / "Cất vào kệ"
- [ ] Xóa file gốc → mục chuyển `missing`

## 12. Câu hỏi mở

- Có nên phát hiện "đang kéo file ở đâu đó" để bung island sẵn không? Hiện chưa có API sạch; để backlog.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
