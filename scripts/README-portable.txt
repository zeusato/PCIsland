PCisland – bản portable
=======================

1. Giải nén cả thư mục ra nơi bạn muốn (ví dụ D:\Apps\PCisland). Giữ nguyên thư mục "data"
   cạnh PCisland.exe: đó là dấu hiệu bản portable, mọi cài đặt sẽ nằm trong đó.
2. Chạy PCisland.exe.
   Khi nâng cấp: thoát bản cũ qua tray, sao lưu dữ liệu rồi chỉ thay exe.
   Không thay thư mục data đang dùng bằng thư mục trống từ gói mới.
3. Nếu Windows hiện "Windows protected your PC" (bản chưa ký số): bấm "More info" rồi "Run anyway".
   Bạn có thể so mã SHA256 của file zip với file .sha256 trên trang GitHub Releases.

Cuộc họp: Ctrl+Alt+M đánh dấu đoạn quan trọng (lùi 15 giây), bấm lại để chốt.
Transcript là tùy chọn: chỉ clip được yêu cầu mới gửi tới Groq, cần mạng và còn quota.
Nếu ghi mic bị từ chối, kiểm tra quyền micro cho ứng dụng desktop trong Windows.
Không có model AI local. Bản chép tự động cần đối chiếu âm thanh trước khi sử dụng.

Gỡ: thoát PCisland (chuột phải icon ở khay hệ thống → Thoát), chạy
    PCisland.exe --cleanup
rồi xóa thư mục.
