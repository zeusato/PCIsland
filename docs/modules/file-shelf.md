# Module: Kệ file (`shelf`)

> Trạng thái: Đã code đủ đặc tả, chờ test tay (xem Handoff §2) · Phase: P3 · Tính năng: F27 · Cập nhật: 2026-10-31

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

1. **Thả vào:** kéo bất kỳ file/thư mục nào (kể cả nhạc) chạm island → bung vùng thả 400×220, gồm các thumbnail đang có và một ô vuông dấu +. Thả vào thêm tham chiếu vào kệ, giữ giao diện này để thấy kết quả; không có alert thay thế thumbnail.
2. **Xem:** mở **Tính năng → Kệ file** trên island, ngay trước nút Home; nút luôn có và kèm số file đang giữ. Compact hiện icon + số mục; peek tối đa 3 slot theo thứ tự thêm, một ô + khi còn chỗ. Đủ 3 thì hiện thông báo kệ đầy. Mục cũ vượt 3 được giữ trong main, không tự xóa dữ liệu. Mở kệ không dừng nhạc; Escape quay lại menu.
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
- Poller theo dõi nhấn trái ngoài island và dịch chuyển ≥8 px; đến vùng 400×64 sát đỉnh thì phát `island://drag-hover` và mở receiver, kể cả khi nhạc đang primary hoặc island minimal. Giữ vùng 400×220 trong lúc kéo. Đây là preview kéo chuột (có thể hiện khi kéo đối tượng khác); chỉ sự kiện native drop có paths mới thêm file. Không dùng DOM hover làm điều kiện nhận file.
- Preview và native drag cùng giữ kệ mở để tránh `leave` do đổi kích thước làm tắt vùng thả; rời ngoài cả hai thì trả lại panel trước. Không pause nhạc; lỗi thêm file hiển thị ngay tại kệ.

**Lưu mục:**
- Frontend nạp danh sách đã lưu khi khởi động, sau khi đăng ký sự kiện; snapshot khởi động không ghi đè sự kiện mới hơn.
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
| `shelf.max_items` | `3` (giới hạn nhận mới cố định 3, kể cả cấu hình cũ lớn hơn) |
| `shelf.sound_on_drop` | `false` |

## 8. Phụ thuộc

- **Core:** ActivityHub, storage, settings, ArtworkCache, SoundService.
- **Platform trait:** `ShellThumbnails`, `ShellOpen` (mở file / mở thư mục chứa).
- **Crate:** `tauri-plugin-drag` (hoặc `drag`), `windows` (`Win32_UI_Shell`).

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Thả thư mục rất lớn | Chỉ lưu tham chiếu thư mục, không duyệt nội dung |
| Thả vượt 3 slot | Nhận tới giới hạn; vùng kệ hiện hướng dẫn bỏ một file |
| File gốc bị xóa/đổi tên | `missing`, thumbnail mờ, chặn kéo ra |
| Thả từ app chạy quyền admin (UIPI) | Windows chặn kéo từ tiến trình quyền cao sang tiến trình thường; ghi trong hướng dẫn |
| Đang fullscreen | Island ẩn nên không thả được; chấp nhận |

## 10. Hiệu năng

Thumbnail tạo một lần khi thả và cache theo hash. Công việc dọn dẹp chỉ chạy khi kệ có mục.

## 11. Kiểm thử

**Unit:** hết hạn theo TestClock; giới hạn số mục và dung lượng chế độ copy; đánh dấu `missing`.

**Checklist thủ công:**
- [ ] Thả 1, 2, 3, 4 file, file trùng và thư mục; chỉ nhận tới 3, giữ thứ tự
- [ ] Kéo ra Explorer (di chuyển/sao chép theo phím giữ), Chrome (upload), Zalo/Telegram
- [ ] Thả file nhạc cũng vào kệ; phát file qua Nhạc › Nhạc trên máy › Chọn thư mục
- [ ] Xóa file gốc → mục chuyển `missing`

## 12. Câu hỏi mở

- Preview dựa trên kéo chuột ngoài island, chưa xác định kiểu dữ liệu OLE trước DragEnter; native drop vẫn là nguồn đường dẫn duy nhất.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-10-31 | Tạo đặc tả |
| 2026-10-31 | Thumbnail qua IShellItemImageFactory (cache theo đường dẫn); kéo file từ kệ ra app khác (plugin drag) |
| 2026-11-01 | Mọi file thả vào kệ, 3 slot, thumbnail + một ô chờ, không thay bằng alert. |
| 2026-11-01 | Preview kéo từ ngoài vùng đỉnh, ưu tiên kệ trước media/minimal; giữ qua morph, báo lỗi thêm file; 3 test frontend và 2 test backend DPI. |
| 2026-11-01 | Nút Kệ file trong launcher, số mục và nạp lại danh sách lúc khởi động; regression test mở kệ và gọi kéo file gốc ra. |
