# Đề xuất tiện ích nhanh: bộ nhớ và dung lượng

Ngày nghiên cứu: 2026-11-01. **Đã triển khai source theo phạm vi chủ dự án chỉnh lại:** RAM toàn PC và dọn rác một chạm cho Windows/app/browser theo allowlist; xem [đặc tả và giới hạn](../modules/maintenance.md). Chưa đóng gói exe mới. Bảng dưới lưu đề xuất ban đầu; hành vi hiện tại theo đặc tả mới, không còn chỉ mở Storage hoặc thu hồi riêng PCisland.

## Ưu tiên thực hiện

| Ưu tiên | Tiện ích | Hành vi đề xuất | Công sức ước tính |
|---|---|---|---|
| 1 | Dọn cache PCisland | Hiện dung lượng thumbnail/artwork/log cũ; một lần bấm dọn các mục hết hạn, giữ ảnh đang dùng, nhạc, ghi chú, SQLite, cookie và phiên đăng nhập | 1–2 ngày gồm kiểm thử |
| 2 | Dọn dung lượng Windows | Nút mở Storage/Temporary files của Windows để người dùng chọn nhóm; hiển thị dung lượng trống trước/sau. Bản đầu không tự xóa Downloads, Thùng rác hoặc Windows.old | Khoảng 1 ngày |
| 3 | Thu hồi bộ nhớ PCisland | Dừng webview tìm kiếm đã xong, giải phóng cache trong RAM không dùng, suspend hoặc unload web player chỉ khi đã dừng phát và đang ẩn; có thể chủ động bấm thay vì chờ timeout | 2–3 ngày, cần QA WebView2 |
| 4 | Xem RAM/ổ đĩa nhanh | Panel RAM khả dụng, dung lượng ổ hệ thống, nút tới Task Manager/Storage; cập nhật khi panel mở | 1–2 ngày |

Các mốc là ước lượng triển khai, chưa phải cam kết. Nên làm 1 và 2 trước, rồi đo bộ nhớ thực tế cho 3.

## Vì sao không dùng “ép sạch RAM toàn máy”

`EmptyWorkingSet` loại trang khỏi working set, không tương đương giải phóng cấp phát của ứng dụng. Trang có thể còn cached trong RAM; lần truy cập sau gây page fault để đưa lại vào working set. Do đó suy luận kỹ thuật: con số RAM giảm tức thời không chứng minh ứng dụng chạy nhanh hơn, thậm chí có thể làm thao tác sau chậm đi. Không đề xuất trim mọi tiến trình hoặc xóa standby list làm mặc định. [Microsoft: Working Set](https://learn.microsoft.com/en-us/windows/win32/memory/working-set).

WebView2 có `TrySuspend` cho webview đang ẩn để giảm tài nguyên, và `Resume` khi quay lại. Với PCisland, điều kiện bổ sung bắt buộc là không có nhạc đang phát, không restore/tìm kiếm đang chờ; không suspend renderer của nguồn nhạc chỉ vì người dùng chuyển sang Ghi chú/Hẹn giờ. Repo hiện đã có unload web player ẩn và paused sau timeout; tiện ích mới nên tận dụng cơ chế này. [Microsoft: ICoreWebView2_3](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_3).

Storage Sense hỗ trợ dọn file tạm và Thùng rác theo cài đặt; Downloads/OneDrive tùy cấu hình của người dùng. Nó chủ yếu xử lý ổ hệ thống. Bản đầu nên dẫn tới UI Windows để thấy rõ từng nhóm trước khi xóa. [Microsoft: Storage Sense](https://support.microsoft.com/en-us/windows/experience/storage-filemanagement/manage-drive-space-with-storage-sense).

## Điều kiện nghiệm thu nếu triển khai

- Báo byte thực sự dọn được; file đang khóa thì bỏ qua và đếm riêng, không kill app.
- Dọn disk và thu hồi RAM là hai thao tác có nhãn riêng.
- Không chạm file cá nhân, database, bản nhạc, hồ sơ web hay phiên đăng nhập.
- Không thao tác registry cleaner, Prefetch, WinSxS, pagefile hoặc dịch vụ Windows.
- Đo working set/private bytes trước, ngay sau và sau 30 giây; đo độ trễ mở lại panel và kiểm tra nhạc liên tục. Chưa có số đo thì không hứa mức MB tiết kiệm.
- Mọi thao tác xóa phải kiểm tra đường dẫn canonical, không theo junction/symlink, allowlist nguồn do ứng dụng quản lý.
