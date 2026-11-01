# Web player · kiểm chứng ngày 2026-11-01

## Phạm vi và bằng chứng

QA chạy bằng exe riêng trong `src-tauri/target/qa-0.1.1`, SQLite riêng. WebView nhạc vẫn dùng profile bền vững của PCisland để kiểm tra phiên. Không lấy cookie/token của Chrome. Không tải URL audio. Các helper và metadata trace nằm trong thư mục target được git ignore.

| Nguồn | Đã quan sát/kiểm chứng | Chưa xác nhận |
|---|---|---|
| Zing MP3 v1.22.4 | API metadata playlist/song; queue lưu `zmp3_np_queue.v1.1`; native queue 24 bài; chọn bài từ IPC; restart app rồi chạy tiếp trong cửa sổ ẩn; tìm từ khóa trả bài thật và chọn phát | Thay đổi cuối về chỉ kích hoạt một lần và sự kiện audio tách DOM chỉ unit test sau sự cố SoundCloud |
| SoundCloud | Chrome phát playlist; DOM `.trackItem` và `.playButton`; metadata `/tracks`; app tìm được 10 kết quả và ghi nhận source playing khi chọn bài | Sau đó xuất hiện “You have been blocked”; không coi là đạt. Đã tắt search nền, không kiểm tra lại resume/queue |
| Spotify | Chrome phát album; queue dùng nhãn không có track link; link now-playing trỏ album. Native search nhận 20 bài thật | Native phát kết quả/DRM/khôi phục phiên và chọn queue chưa kiểm chứng đầy đủ |
| YouTube, YouTube Music, NhacCuaTui | Giữ hỗ trợ bridge cơ bản có sẵn | Không có bằng chứng mới trong phiên này |

## Các lỗi xác định được

- Zing album title là span, selector link cũ không có kết quả. Queue sidebar chưa mount khi đóng. Link tên bài trên thanh phát thực ra trỏ album.
- Chuyển từ Zing đang phát sang nguồn khác làm hiện hộp thoại “Leave site?” trong cửa sổ ẩn, khiến điều hướng bị treo. Đã sửa lệnh điều hướng chủ động và kiểm chứng Zing→SoundCloud trong QA.
- Audio tạo ngoài DOM không gửi timeupdate về document. Đã đăng ký listener lên audio được bridge theo dõi để cập nhật vị trí mỗi 5 giây; vị trí chưa có trong metadata không ghi đè lịch sử bằng 0.
- Resume trước đây click phát/chọn bài lặp mỗi vài giây. Đã đổi sang một lần kích hoạt, chờ sẵn sàng hữu hạn; dừng khi nhìn thấy thông báo chặn/challenge.

## Sự cố SoundCloud

Người dùng cung cấp ảnh chặn với lý do khả dĩ: thao tác nhanh, JavaScript, automation hoặc developer/inspection tools. Không có bằng chứng xác định nguyên nhân duy nhất, thời hạn, hay tài khoản bị khóa. Các lượt QA tự động/inspection có thể đã góp phần kích hoạt.

Đã thoát QA, ngừng thử online, tắt native search SoundCloud và gỡ quyền domain này khỏi search capability. Không đổi IP/profile, không xóa cookie, không giải/bỏ qua challenge. Chỉ tiếp tục test cục bộ. Không được ghi “SoundCloud đã hỗ trợ đầy đủ” từ lần nhận state playing trước sự cố.

## Trạng thái máy sau QA

- QA đã thoát, Vite dev server đã dừng; không để vòng truy cập website chạy nền.
- HKCU Run PCisland đã trở về exe release chính, quote đúng và thêm `--autostart`; trạng thái StartupApproved được giữ theo backup. Chưa thử reboot.
- Không xóa thư viện/cache build hoặc dữ liệu cá nhân. Giữ hồ sơ đăng nhập website.
- Tại thời điểm QA, bản 0.1.1 mới build cục bộ. Sau đó chủ dự án yêu cầu phát hành; xem [release v0.1.1](https://github.com/zeusato/PCIsland/releases/tag/v0.1.1).
