# Module: Kệ file (`shelf`)

> Trạng thái: Đã mở rộng source/test, còn nghiệm thu native và batch lớn · Phase: P3 + yêu cầu bổ sung · Tính năng: F27 · Cập nhật: 2026-11-01

## 1. Mục đích

Một chỗ để file tạm thời khi kéo thả giữa các cửa sổ: kéo file từ Explorer thả vào island, chuyển sang app khác, rồi kéo từ island ra. Không phải mở hai cửa sổ cạnh nhau.

## 2. Phạm vi

**Trong phạm vi:**
- Thả file/thư mục vào island.
- Danh sách mục trên peek và trong `main`.
- Chọn một/nhiều/tất cả; kéo file thực ra ngoài, sao chép file vào clipboard, mở/reveal, bỏ khỏi kệ.
- ZIP nền có tiến trình/hủy; chuyển một ảnh tĩnh PNG/JPEG sang PNG/JPEG/WebP và thu nhỏ giữ tỷ lệ.
- Tự dọn mục quá hạn.

**Ngoài phạm vi:**
- Đồng bộ đám mây, chia sẻ.
- Thả văn bản/URL: backlog.
- Chuyển đổi Office/PDF/video, đổi ảnh hàng loạt, đồng bộ hoặc gửi tin nhắn tự động.

## 3. Hành vi & luồng người dùng

1. **Thả vào:** kéo bất kỳ file/thư mục nào (kể cả nhạc) chạm island → bung vùng thả 400×220, gồm các thumbnail đang có và một ô vuông dấu +. Thả vào thêm tham chiếu vào kệ, giữ giao diện này để thấy kết quả; không có alert thay thế thumbnail.
2. **Xem:** mở **Tính năng → Kệ file** trên island. Không còn trần 3 mục ở backend hoặc UI. Danh sách dùng chung main/island, 50 mục/trang, vùng cuộn tối đa 340 px. Click chọn, Ctrl chọn rời, Shift chọn dải, Ctrl+A chọn tất cả, Escape bỏ chọn. Mở kệ không dừng nhạc.
3. **Kéo ra:** dùng tay nắm kéo mục/nhóm hoặc Kéo tất cả. Plugin native gửi danh sách file thực; clipboard dùng `CF_HDROP`, không dán text đường dẫn. Không tự bấm Send; chưa nghiệm thu attachment trong chat thật.
4. **Thao tác:** thanh công cụ chọn nhiều/tất cả, sao chép file, ZIP, bỏ khỏi kệ; một mục có mở/reveal và chuyển ảnh. ZIP/ảnh chọn vị trí qua hộp Save, thêm kết quả thành công vào kệ; giữ file nguồn. Không ghi đè file đã có. Không mở full app qua double-click.
5. **Tự dọn:** mục quá `shelf.expire_hours` (24 giờ) tự rời kệ. File gốc không bị đụng tới.

## 4. Activity

| Kind | Ưu tiên | Compact | Peek | Alert | Hành động |
|---|---|---|---|---|---|
| `shelf.items` | Ambient | Icon kệ + số mục | Danh sách phân trang, chọn và kéo nhóm | — | `clear` |
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
- Lưu **tham chiếu**: đường dẫn, tên, kích thước, loại thư mục và thời điểm thêm. Không có copy mode trong implementation hiện tại.
- ZIP/ảnh được lưu ở nơi người dùng chọn; không có kho output tự dọn. Bỏ khỏi kệ chỉ xóa tham chiếu.

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
| `shelf_list` | `() → ShelfItem[]` |
| `shelf_add` | `(paths: string[]) → number` (số nhận) |
| `shelf_remove` / `shelf_remove_many` | `(id)` / `(ids) → ()` |
| `shelf_clear` | `() → ()` |
| `shelf_open` | `(path,reveal) → ()` |
| `shelf_copy` | `(ids) → ()` (clipboard file thực) |
| `shelf_drag_icon` | `() → string`, drag qua plugin native |
| `shelf_zip` / `shelf_cancel` | `(ids,destination,progress:Channel<ShelfProgress>)` / `()` |
| `shelf_convert` | `(id,destination,format,maxEdge?) → string` |

| Event | Payload |
|---|---|
| `shelf://state` | `ShelfItem[]` |

## 7. Dữ liệu & cài đặt

**Bảng sử dụng:** `shelf_items` với id, path, name, size, is_dir, added_at; `missing` tính khi đọc.

| Khóa | Mặc định |
|---|---|
| `shelf.expire_hours` | `24` (0 = không tự dọn) |
| `shelf.max_items` | `0`, trường tương thích cũ, không áp trần nhận mới |
| `shelf.sound_on_drop` | `false` |

## 8. Phụ thuộc

- **Core:** ActivityHub, storage, settings, ArtworkCache, SoundService.
- **Platform trait:** `ShellThumbnails`, `ShellOpen` (mở file / mở thư mục chứa).
- **Crate:** `tauri-plugin-drag`, `windows`, `zip`, `image` (PNG/JPEG/WebP). ZIP dùng `shared/archive`; clipboard/publish file nằm trong platform, ADR-020.

## 9. Trường hợp biên & lỗi

| Tình huống | Xử lý |
|---|---|
| Thả thư mục rất lớn | Chỉ lưu tham chiếu thư mục, không duyệt nội dung |
| Thả nhiều file | Không trần 3; bỏ trùng/missing, trả số nhận. Chưa có báo cáo từng lỗi/progress/hủy khi thêm hàng nghìn mục |
| ZIP có junction/symlink hoặc output nằm trong thư mục nguồn | Từ chối, không theo link ra ngoài cây |
| Ảnh động/ảnh alpha sang JPEG | Từ chối, giữ nguồn; chọn PNG/WebP để giữ alpha |
| File gốc bị xóa/đổi tên | `missing`, thumbnail mờ, chặn kéo ra |
| Thả từ app chạy quyền admin (UIPI) | Windows chặn kéo từ tiến trình quyền cao sang tiến trình thường; ghi trong hướng dẫn |
| Đang fullscreen | Island ẩn nên không thả được; chấp nhận |

## 10. Hiệu năng

Thumbnail cache trong phiên, tạo tối đa 50 thumbnail chưa có mỗi lần đọc. ZIP stream theo khối 64 KB, chạy nền, một job/hủy được. Decode ảnh có giới hạn 128 MB; cạnh resize 64–8192 px. Backend vẫn trả toàn bộ danh sách, chưa virtualize dữ liệu hoặc benchmark hàng nghìn mục.

## 11. Kiểm thử

**Unit:** thêm vượt 3, chọn dải/Ctrl+A/Escape với 75 mục; ZIP trùng tên, nội dung, hủy và không ghi đè output. Không suy ra native drag/chat đã đạt từ test mock.

**Checklist thủ công:**
- [ ] Thả 4/50/500 file, file trùng và thư mục; giữ thứ tự, không trần 3
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
| 2026-11-01 | Bỏ trần 3; chọn nhiều/tất cả, clipboard file, ZIP có hủy, đổi ảnh và phân trang. Đặc tả trên thay hành vi 3 slot cũ. |
