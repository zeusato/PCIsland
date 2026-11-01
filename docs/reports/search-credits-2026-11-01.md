# Trace metadata kết quả tìm kiếm

Ngày: 2026-11-01. Chủ dự án cho phép trace dữ liệu thật bằng browser. Chỉ đọc kết quả công khai, không đăng nhập/phát nhạc; không truy cập lại SoundCloud.

| Nguồn | Dữ liệu quan sát | Selector đã kiểm chứng |
|---|---|---|
| [Zing: đại thiên bồng](https://zingmp3.vn/tim-kiem/bai-hat?q=%C4%91%E1%BA%A1i%20thi%C3%AAn%20b%E1%BB%93ng) | Đại Thiên Bồng – Lee Phú Quý; bản Chu Thúy Quỳnh, Ciray, Hàn Dung. Album riêng: Lan Nhân Nhứ Quả. Không thấy người đăng trong dòng kết quả. | `.media-item .card-info .subtitle a`; tên bài lấy figure title hoặc link title |
| [Spotify: đại thiên bồng](https://open.spotify.com/search/%C4%91%E1%BA%A1i%20thi%C3%AAn%20b%E1%BB%93ng/tracks) | Đại Thiên Bồng – Caokazi, Tran Ngoc Thien; Đại Thiên Bồng 3 – Trương Tiến Giáp, DG MEDIA. Có nhiều ca sĩ trong cùng dòng; không thấy người đăng riêng. | `a[href*="/track/"]` → ancestor `[role="row"]` → `a[href*="/artist/"]`; row hiện không có data-testid |

`WebQueueItem` bổ sung `artist?`, `uploader?`, serde default để queue/session cũ đọc được. Backend chuẩn hóa khoảng trắng và giới hạn credit 256 ký tự. Adapter hiện trích ca sĩ; uploader không được cung cấp nên không suy diễn từ album/artist. UI hiện tên bài, Ca sĩ; Người đăng chỉ khi có dữ liệu. Thiếu ca sĩ hiện “Chưa có thông tin”.

Ba test adapter dùng DOM fixture dựa trên cấu trúc quan sát; kiểm tra nhiều ca sĩ, không lẫn album và không lấy ca sĩ dòng kế tiếp. Một test backend kiểm tra tương thích queue cũ và giới hạn dữ liệu. UI search test kiểm tra nhãn artist/uploader độc lập. QA hình ảnh offline dùng tên/ca sĩ công khai vừa quan sát; chưa xác minh end-to-end tìm→phát qua exe mới.
