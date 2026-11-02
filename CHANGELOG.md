# Lịch sử phát hành

## 0.1.3 · 2026-11-02

- Nhạc tập trung cho Pomodoro: tải theo yêu cầu từ repo `zeusato/pcisland-sounds` (3 gói, 20 bài), kiểm SHA-256, phát xáo theo pha làm việc/nghỉ.
- Hộp chào lần đầu hỏi tải thư viện YouTube (yt-dlp) và gói nhạc; không tải gì khi chưa đồng ý.
- Spotify/SoundCloud: danh sách phát, tìm kiếm SoundCloud, repeat theo nút của trang.
- Nút chia sẻ trong trình phát: chép link bài đang phát vào clipboard.
- Nút island đều kích thước, tên một dòng; Cài đặt cho ẩn/hiện từng nút.
- Dọn dẹp: Thùng rác, cache Windows Update (UAC), đóng app nền ngốn RAM; tiến trình giữ khi rời và quay lại island.
- Sửa: kéo file vào island bị dấu cấm, kệ file khi thả bị cắt mất nút; note desktop không đóng được, ô tick lệch khi đổi cỡ chữ.

[Chi tiết, tải và giới hạn](docs/releases/v0.1.3.md).

## 0.1.2 · 2026-11-01

- Kệ file không còn trần 3: chọn/kéo/copy nhiều file thực, ZIP nền có hủy, đổi một ảnh PNG/JPEG sang PNG/JPEG/WebP.
- Ghim note ra desktop, topmost, cỡ chữ 12–40, countdown, sửa/xóa/đóng; giữ draft và chống ghi đè sửa đổi từ cửa sổ khác.
- Ghi họp mic/system bằng Opus, đánh dấu lùi 15 giây, clip nền, timeline/notes, nghe lại và ZIP; transcript Groq tiếng Việt tùy chọn.
- Dropdown cuộc họp gọn trong island, có cuộn và tên thiết bị dài; mở note desktop async để tránh deadlock WebView2.
- Gom các thay đổi sau 0.1.1: video nổi, maintenance, sửa tìm→phát, kệ file luôn truy cập được, giữ player khi dùng tính năng khác.
- Cập nhật đặc tả, ADR, hướng dẫn build/dọn cache và giới hạn nghiệm thu.

[Chi tiết, tải và giới hạn](docs/releases/v0.1.2.md).

## 0.1.1 · 2026-11-01

Island mở rộng theo nội dung, phiên nhạc/repeat, tìm kiếm nhạc, nhắc giờ từng dòng note. [Release notes](docs/releases/v0.1.1.md).

## 0.1.0 · 2026-10-31

Bản phát hành đầu: island, media, timer/Pomodoro, notes, kệ file; NSIS và portable. [GitHub release](https://github.com/zeusato/PCIsland/releases/tag/v0.1.0).
