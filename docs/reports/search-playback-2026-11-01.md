# Sửa tìm kiếm chọn bài và mở lại kệ file

Ngày: 2026-11-01.

## Lỗi tái hiện trên app thật

Cửa sổ Zing của PCisland đã mở album **Lan Nhân Nhứ Quả – Lee Phú Quý**, có bài **Đại Thiên Bồng**, nhưng player vẫn giữ **Lời Của Gió – Bằng Kiều** ở 1:40/3:07. Điều hướng thành công không đồng nghĩa bài được chọn đã phát.

Đọc DOM công khai của [album](https://zingmp3.vn/album/Lan-Nhan-Nhu-Qua-Lee-Phu-Quy/MmLmO9jReNE.html) xác nhận hàng bài nằm trong `.playlist-content`, có `.media-item`, title trên figure và nút `button.action-play`. Adapter trước đây chỉ liên kết hàng trang với queue đang phát; bài thuộc album mới không có trong queue cũ nên bị bỏ qua.

## Sửa và hồi quy

- Ánh xạ hàng trang độc lập với queue, dùng URL bài từ metadata playlist đã nhận để chọn đúng hàng. Không đổi queue chỉ vì đang xem album khác.
- Restore xác nhận URL bài, tránh nhầm hai bản cùng tên. Một lần kích hoạt; một lần điều hướng dự phòng; timeout hủy restore.
- Lệnh tìm→phát chờ tối đa 24 giây để bridge xác nhận đúng URL đang Playing. UI giữ trạng thái chờ; thất bại giữ kết quả, báo lỗi và cho mở nguồn.
- Launcher có **Kệ file**, số mục và panel để kéo file đã lưu ra. Store nạp kệ khi khởi động, bảo vệ khỏi snapshot về muộn ghi đè sự kiện mới.
- Frontend: 72 test / 24 file đạt, gồm album mới và trang bài hát riêng + queue cũ + bài trùng tên, chờ xác nhận phát và mở kệ/kéo tham chiếu file. Rust: 171 test đạt. Svelte-check, eslint, i18n 310 khóa, fmt và clippy all-targets đạt.

## Xác minh bản build

Exe 17:51: tìm **đại thiên bồng** → chọn **Lee Phú Quý** đã vào đúng player, tiến trình 0:06 → 0:17. Nút Kệ file mở đúng panel trên native. Kệ hiện 0 mục khớp số hàng đọc từ DB; không khẳng định đã phục hồi file từ phiên khác.

Kiểm tra tiếp bản **Chu Thúy Quỳnh** phát hiện trang riêng `/bai-hat/.../dsHms6HwKza2.html`, không chuyển sang album. Bổ sung nhánh bấm `.song-header button.btn-play-all` chỉ khi URL trang đúng URL được yêu cầu, kiểm chứng selector bằng DOM công khai. Test hồi quy thêm trang bài hát riêng và đảm bảo không phát/seek bài cũ cùng tên. Chủ dự án yêu cầu dừng để tắt máy: đã hủy lần build tiếp theo. Nhánh trang bài hát riêng mới ở source và test, chưa có trong exe 17:51, chưa xác minh native.

Thử kéo file QA từ Explorer sang island bị công cụ native chặn ở biên cửa sổ; chưa xác minh được drag/drop OS. Không kiểm tra lại SoundCloud đang bị chặn.
