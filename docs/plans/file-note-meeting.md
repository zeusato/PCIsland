# Kế hoạch nâng cấp kệ file ghi chú desktop và meeting

Ngày: 2026-11-01. Trạng thái: thiết kế triển khai; chưa code hoặc build theo kế hoạch này.

Mục tiêu là biến island thành nơi thao tác nhanh với file, theo dõi ghi chú trên desktop và ghi lại phần quan trọng của cuộc họp. App vẫn nhẹ, không kèm model AI hàng GB. Groq Large V3 đã qua phép thử tiếng Việt cơ bản và được chọn làm ứng viên transcript nháp; chất lượng họp thật và backend dùng chung còn là điều kiện trước phát hành. Xem [báo cáo thử Groq](../reports/groq-vietnamese-2026-11-01.md).

## Những yêu cầu đã chốt

| Tính năng | Kết quả người dùng cần |
|---|---|
| Kệ file | Bỏ giới hạn 3 mục; thao tác một/nhiều/toàn bộ; nén; kéo cả nhóm vào chat thành file đính kèm thực; chuyển đổi định dạng có ích |
| Note desktop | Ghim một ghi chú đã lưu ra cửa sổ riêng; always-on-top tùy chọn; sửa, xóa, đóng; giữ countdown, giờ hẹn, chuông; tăng giảm cỡ chữ |
| Meeting | Ghi cả cuộc họp; hotkey bắt đầu đánh dấu lùi 15 giây; hotkey lần hai kết thúc và tạo file clip ngay khi có thể, không ngắt ghi; timeline dọc và editor cạnh bên; transcript clip nếu dịch vụ sẵn sàng |
| Tính liên tục | Mở kệ/note/meeting không tự dừng nhạc hay hủy player; drag file luôn ưu tiên vùng nhận thả; có đường quay lại player/meeting đang chạy |

Đề xuất mặc định trong tài liệu là lựa chọn triển khai, không phải yêu cầu mới của chủ dự án. Không tự mở full app bằng nhấp đúp; chỉ nút hoặc phím tắt được chỉ định mới mở.

## Hiện trạng và điểm cần sửa

- Kệ đang giới hạn 3 ở cả `ShelfHandle::add`, câu SQL `COUNT(*)<3` trong `src-tauri/src/modules/shelf/mod.rs` và `Slots.svelte`. Phải gỡ đồng bộ cả ba chỗ; không chỉ tăng số slot giao diện. `shelf_remove` hiện nhận một ID, cần thao tác batch.
- Kệ hiện lưu đường dẫn tham chiếu và dùng plugin drag. Một số nội dung trong module doc là thiết kế, không được mặc định coi copy mode hay mọi IPC trong doc đã triển khai đủ.
- Ghi chú đã có `note_tasks`, deadline và scheduler ở Rust. `notes://state` hiện chỉ phát tới `island` và `main`; cửa sổ note mới phải nhận snapshot/event tương ứng. Ghim island và ghim desktop là hai trạng thái khác nhau.
- Repo đang có thay đổi của agent video ở Cargo, media hub, settings, shortcuts, IPC và cửa sổ. Kế hoạch này thêm module độc lập và gom sửa file dùng chung vào bước tích hợp cuối.
- Migration hiện đã có 0008. Khi thực thi, lấy số tiếp theo từ checkout lúc đó; không giữ trước số migration gây trùng với agent khác.

## Kệ file

### Giao diện và thao tác

1. Nút Kệ file luôn có trên launcher, kèm tổng số mục. Drag vào mở vùng nhận thả từ mọi panel, giữ nhạc/meeting chạy nền và nhớ panel trước đó.
2. Island dùng danh sách cuộn có chiều cao tối đa theo vùng làm việc màn hình. Mỗi dòng có thumbnail/icon, tên, loại và kích thước; tên dài có tooltip. Không kéo cao island theo tổng số file, không ép dòng co lại như lỗi danh sách tìm nhạc cũ.
3. Click chọn; Ctrl chọn rời, Shift chọn dải, Ctrl+A chọn tất cả khi panel kệ có focus. Checkbox hiện rõ khi chọn nhiều. Click vùng trống hoặc Escape lần đầu bỏ chọn; Escape tiếp theo trở về panel trước.
4. Thanh thao tác cố định hiển thị “Đã chọn N” và **Kéo N file**, **Nén ZIP**, menu **Thêm**. Không chọn gì thì hiện thao tác **Tất cả** với số lượng rõ ràng. Đổi định dạng hiện khi tập chọn có loại được hỗ trợ.
5. Kéo dòng đã chọn kéo cả nhóm; kéo dòng chưa chọn chỉ kéo dòng đó. Nút tay nắm “Kéo tất cả” cho phép gom và kéo một động tác. Nút click không tự gửi tin nhắn hoặc tự bấm Send ở ứng dụng chat.
6. Thao tác một mục: mở, mở thư mục chứa, sao chép file, bỏ khỏi kệ. Sao chép đường dẫn là lệnh riêng, không dùng thay cho sao chép file. Không xóa file gốc khi bỏ mục/dọn kệ.

### File thực khi kéo ra

Backend giải ID đã chọn thành những file đang tồn tại, rồi dùng OLE drag với danh sách file thực (`CF_HDROP` hoặc cơ chế Shell tương đương của plugin). Dù nội bộ payload chứa đường dẫn theo giao thức Windows, app nhận phải thấy đối tượng file để upload, không phải chuỗi text đường dẫn, URL hoặc shortcut `.lnk`.

Kiểm chứng bắt buộc: thả nhiều file vào Zalo Web/ứng dụng chat phải hiện nhiều thẻ đính kèm; tải lại và so hash byte với file nguồn. Chỉ hiện tên đường dẫn trong ô chat là thất bại. Đích không hỗ trợ nhiều file có thể từ chối; không báo gửi thành công chỉ vì drag kết thúc.

Thư mục được kéo nguyên dạng thư mục vào Explorer. Với chat không hỗ trợ folder, cung cấp **Nén rồi kéo**; không âm thầm biến thành đường dẫn. File missing/không đọc được được liệt kê rõ trước thao tác; cho bỏ qua các mục đó một cách tường minh, không lặng lẽ gửi thiếu.

### Nén và chuyển đổi

- ZIP một, nhiều hoặc tất cả mục: job nền có tiến độ, hủy được; giữ cấu trúc tương đối, xử lý trùng tên bằng quy tắc ổn định. Không theo junction/symlink ra ngoài cây đã chọn. Stream dữ liệu, không nạp cả tập file vào RAM. Output `.partial` chỉ đổi tên khi hoàn tất; không đưa file dở vào kệ.
- ZIP hoàn tất xuất hiện trong kệ và có nút kéo ngay; giữ mục nguồn. Dừng job chỉ xóa output tạm do chính job sở hữu.
- Chuyển đổi giai đoạn đầu giới hạn ảnh PNG/JPEG/WebP và resize có giữ tỉ lệ, theo codec thực sự hỗ trợ sau spike. Không quảng cáo đổi mọi định dạng; không chỉ đổi đuôi file. Ảnh có alpha sang JPEG cần lựa chọn nền rõ ràng; ảnh động không âm thầm mất animation. File gốc giữ nguyên, tên output không ghi đè.
- PDF/Office/video/audio conversion để giai đoạn sau; không kéo thêm bộ công cụ nặng chỉ để đổi định dạng ở bản đầu.

### Lưu trữ và hiệu năng

Không có trần 3 hoặc thay bằng một trần nhỏ tùy ý. Lưu tham chiếu mặc định; lazy thumbnail, phân trang/virtualize khi lớn. Thêm hàng nghìn mục phải có tiến độ/hủy và guard chống cạn tài nguyên, không đóng băng UI. Kết quả thêm trả số nhận/trùng/missing/từ chối theo nguyên nhân.

File do app tạo thuộc kho output riêng, có metadata ownership và thời điểm dùng cuối. Chỉ dọn output không còn được kệ/meeting/job/drag tham chiếu; có thời gian lưu đệm sau drag để app đích đọc xong. Cho “Lưu bản sao…” để chuyển output thành file người dùng quản lý. Tự hết hạn chỉ bỏ tham chiếu, không đụng bản gốc.

### Công việc và nghiệm thu

- Mở rộng backend shelf với thao tác batch, lựa chọn ID, job ZIP/conversion và kết quả lỗi từng mục; frontend selection chỉ là state hiển thị.
- Thêm adapter clipboard file/native drag tại `platform/`, dịch vụ tạo output và quản lý vòng đời. DTO dùng ts-rs; không tin đường dẫn tùy ý từ cửa sổ remote.
- Test: nhận 4/50/500 mục, nhiều file trùng tên, Unicode, missing, batch hủy; ZIP giải nén khớp hash; chống thoát cây thư mục; ảnh đúng kích thước/alpha.
- Test tay: kéo một/nhiều/toàn bộ ra Explorer và chat; drag vào khi nhạc/meeting/video đang hoạt động; thao tác nhiều mục với màn hình nhỏ, DPI 100–200%.

## Ghi chú ghim desktop

### Giao diện và hành vi

Từ danh sách ghi chú trên island/main, lệnh **Ghim ra desktop** mở cửa sổ note. Bản đầu có tối đa một cửa sổ note desktop, chọn note khác chuyển cửa sổ đang có sau khi lưu bản nháp. Ghim island hiện tại vẫn độc lập.

Thanh đầu có tên ghi chú, always-on-top, **A− / A+**, sửa, menu xóa và đóng; kéo thanh đầu để di chuyển, resize từ cạnh. Đề xuất cỡ chữ 18 px logic, điều chỉnh 12–40 px theo bước 2; Ctrl+wheel thay cỡ chữ khi chuột ở trong note, có nút về mặc định. Cỡ chữ lưu riêng cho cửa sổ desktop, không đổi chữ toàn app. Nút và đồng hồ vẫn dễ bấm/đọc ở mức chữ lớn.

- Nội dung dùng renderer checklist hiện tại; countdown hoặc giờ hẹn nằm cuối từng dòng, có xuống dòng hợp lý khi thiếu chỗ.
- Sửa/tick/đặt giờ từ desktop phản ánh ngay sang island và main. Khi hai nơi cùng sửa, dùng revision và giữ bản nháp xung đột, không dùng last-write-wins làm mất nội dung.
- Đóng cửa sổ chỉ bỏ ghim desktop; note và các chuông đang hẹn vẫn tồn tại. Xóa note dùng soft delete/hoàn tác như hiện tại, đóng cửa sổ và hủy nhắc của note đó. Phân biệt nút Đóng với Xóa.
- Chuông phát đúng một lần bởi scheduler backend, không nhân đôi khi mở nhiều view. Đồng hồ UI chỉ nội suy từ deadline; không ghi DB mỗi giây.
- Nhớ vị trí, kích thước, cỡ chữ và trạng thái topmost; clamp vào màn hình còn tồn tại sau đổi màn hình/DPI. Khôi phục note đang ghim khi mở app; không hồi sinh note đã xóa.
- Cửa sổ thường có thể nhận focus để gõ tiếng Việt. Không tái sử dụng no-activate của island cho editor desktop.

### Công việc và nghiệm thu

- Thêm entry frontend `src/apps/note-desktop/` và window label/capability riêng; chỉ tạo WebView khi được ghim, giải phóng khi đóng.
- Backend window controller quản lý note ID, bounds/topmost/font trong `kv` hoặc bảng chuyên biệt. Mở rộng đồng bộ note/task bằng snapshot có revision và event tới đúng cửa sổ.
- Giữ task ID/deadline khi sửa nội dung; quy tắc checklist phụ thuộc dòng trước tiếp tục theo module notes hiện có.
- Test: update hai cửa sổ, bản nháp xung đột, delete/undo, đóng vẫn báo giờ, không báo trùng, restart/sleep-resume, thay note ghim.
- Test tay: Unikey/EVKey, Ctrl+wheel, cỡ chữ 12/18/40, DPI 100/150/200%, rút màn hình, bật/tắt topmost và focus khi nhạc đang chạy.

## Meeting

### Luồng chính

1. Lần đầu chọn mic và nguồn âm thanh máy tính, nhìn meter kiểm tra. Hiển thị rõ đang thu mic, âm hệ thống hay cả hai; nhớ lựa chọn để lần sau **Bắt đầu họp** là một lần bấm. Không tự tham gia cuộc gọi hay bật camera.
2. Ghi toàn bộ cuộc họp ở nền. Island hiện thời gian đã thu, trạng thái mic/âm hệ thống, nút đánh dấu và nút mở lại giao diện họp. Đóng cửa sổ chính không dừng ghi; nút **Kết thúc họp** mới kết thúc.
3. Phím tắt đề xuất `Ctrl+Alt+M`, đổi được và kiểm tra xung đột với shortcut hiện tại. Trong cuộc họp, lần nhấn đầu tạo marker tại `max(0, t_nhan - 15 giây)`; lần tiếp theo đóng tại `t_nhan`. Debounce key-repeat; chỉ một đoạn mở tại một thời điểm.
4. Card xuất hiện trong log ngay khi đánh dấu, hiển thị thời điểm thực bắt đầu và “Đang đánh dấu”. Đóng đoạn chuyển thành “Đang tạo clip”, rồi có file audio độc lập, nghe/kéo/lưu được; không chờ kết thúc cả cuộc họp hoặc chờ AI.
5. Tạo clip và transcript chạy nền. Capture luôn độc lập với các job này. Chuyển panel, drag file và nhạc tiếp tục hoạt động; quay lại meeting bằng nút chỉ báo đang ghi.
6. Kết thúc họp tự đóng marker còn mở tại thời điểm dừng, hoàn tất audio toàn cuộc họp và giữ job clip/transcript hợp lệ. Thoát hẳn app khi đang ghi cần xử lý dừng/finalize rõ ràng; không âm thầm mất bản ghi.

### Giao diện chính

Main có hai vùng co giãn: bên trái meeting log theo thứ tự trên xuống, bên phải ghi chú cá nhân autosave. Card clip có mốc thời gian, thời lượng, tên sửa được, nghe lại/tua, kéo file thực, transcript và trạng thái lỗi/thử lại. Click mốc thời gian để nghe đúng vị trí trong bản ghi.

Log tự cuộn khi đang ở cuối; nếu người dùng cuộn lên thì giữ vị trí và hiện báo có mục mới. Transcript trả muộn cập nhật đúng card, không tạo card lặp hoặc đổi thứ tự. Ghi chú cá nhân và transcript là hai trường riêng, AI không ghi đè editor. Cho người dùng chủ động chèn một đoạn transcript vào note, kèm liên kết mốc thời gian.

Transcript có nhãn “Bản chép tự động”, sửa được; giữ bản máy riêng để retry không ghi đè bản người dùng sửa. Xuất buổi họp thành thư mục hoặc ZIP gồm audio toàn cuộc họp, các clip, note Markdown và metadata mốc thời gian. Không cần LLM tóm tắt trong bản đầu.

### Thu âm và dung lượng

Windows capture ở `platform/`: WASAPI mic + loopback của output device, timestamp theo clock đơn điệu/sample count; đồng bộ sample rate, drift và gain trước trộn. Loopback thiết bị thu cả âm thanh khác đang phát, gồm nhạc của PCisland; UI phải nói rõ. Không tự pause nhạc. Thu riêng tiến trình họp là bước sau nếu chưa có adapter đáng tin cậy. Kiểm thử echo khi dùng loa, không hứa có khử vọng hoàn chỉnh ở bản đầu. [WASAPI loopback](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)

Đề xuất codec chính: Opus mono 48 kbps trong Ogg, encoder nhẹ chạy tại máy, không phải model AI. Spike phải đo kích thước binary/phụ thuộc/license và thử playback trong WebView2. Nếu không đáp ứng ngân sách app, đánh giá AAC native Media Foundation 96 kbps; tài liệu Microsoft liệt kê bitrate thấp nhất 96 kbps, không mặc định hứa AAC native 48 kbps. [AAC encoder](https://learn.microsoft.com/en-us/windows/win32/medfound/aac-encoder)

| Cấu hình ước lượng | 60 phút | Clip 2 phút |
|---|---:|---:|
| Một luồng Opus 48 kbps | 21,6 MB | 0,72 MB |
| Một luồng AAC 96 kbps | 43,2 MB | 1,44 MB |

MB thập phân, chưa tính container/metadata và biến thiên bitrate. Các clip độc lập chiếm thêm dung lượng: 10 clip × 2 phút ở 48 kbps thêm khoảng 7,2 MB. Lưu hai luồng riêng sẽ tăng gần gấp đôi; mặc định chỉ lưu một bản trộn. Không để WAV cả cuộc họp chiếm hàng trăm MB khi đã có codec nén.

### Ghi liên tục và cắt clip

- Capture thread chỉ thu và đẩy buffer hữu hạn; encoder/disk writer/transcript dùng worker riêng. Không giữ PCM cả cuộc họp trong RAM. Ring buffer và queue có trần, đo peak RAM bằng phiên 60 phút.
- Ghi các chunk nén có thể phục hồi (đề xuất 5 giây), lưu manifest với sample range, codec config và trạng thái finalize. Encoder chạy liên tục qua ranh giới chunk; mọi chunk đã hoàn tất được bảo toàn khi crash.
- Marker lưu khoảng thời gian trên timeline audio, không dựa vào đồng hồ UI. Khi đóng marker, flush dữ liệu cần thiết tới thời điểm đóng sang worker export, không ngắt capture. Worker lấy những chunk giao khoảng, xử lý ranh giới frame và tạo clip độc lập bằng tên tạm rồi rename atomic.
- Với Opus phải xử lý pre-skip/preroll, granule position và timestamp khi remux; không nối byte các Ogg một cách mù quáng. Sai số cắt mục tiêu tối đa một frame codec, không mất 15 giây lùi. Nếu cần encode lại biên clip thì làm nền, không encode lại toàn cuộc họp cho mỗi marker.
- Khi dừng, tạo một file toàn cuộc họp từ manifest/chunk, giữ chunk cho tới khi output được xác nhận hợp lệ. Khi đang ghi có thể nghe đoạn đã finalize. Chỉ cleanup sau khi không còn job nào tham chiếu.
- Marker thứ hai có thể lùi vào đoạn trước; giữ đúng 15 giây và cho phép overlap, không cắt bớt ngữ cảnh âm thầm.
- Disk đầy/lỗi encoder: báo ngay, lưu các chunk hợp lệ và chuyển trạng thái lỗi có thể khôi phục; không tiếp tục hiển thị đang thu nếu đã mất dữ liệu. Rút mic/đổi device hoặc sleep tạo sự kiện gap trong log; không giả vờ ghi được trong lúc máy ngủ.

Mục tiêu thử nghiệm: đóng marker xuất hiện card tức thì; clip thường sẵn sàng trong 2 giây sau flush, nhưng đây là mục tiêu cần đo, không phải cam kết đã đạt. M1 phải chứng minh không có audio gap do export.

### Transcript không yêu cầu người dùng nhập key

Kiến trúc đề xuất: Rust client → backend PCisland → Groq `audio/transcriptions`. Key nhà phát triển nằm trên backend, không đóng gói trong exe/frontend. Không điều khiển Playground hay lấy cookie trình duyệt trong sản phẩm. Người dùng không cần tài khoản Groq và không thấy ô nhập key.

- Chỉ gửi clip đã finalize khi bật transcript online; lần bật có mô tả audio sẽ được gửi tới dịch vụ. Toàn bộ cuộc họp mặc định lưu local. Không gửi nội dung note cá nhân làm prompt ngầm.
- Large V3, `language=vi`, `temperature=0`, verbose JSON. Prompt từ vựng là tùy chọn ngắn do người dùng cung cấp và cần kiểm thử; không đưa bản chuẩn câu thử vào prompt để làm đẹp kết quả.
- Worker bền vững: `queued → uploading → transcribing → ready / retry_wait / failed / quota_wait`; event có session/clip/job/revision để kết quả cũ không ghi vào clip mới hoặc clip đã xóa.
- Timeout hữu hạn; retry 429 theo Retry-After và 5xx có backoff/jitter, trần retry. Lỗi xác thực không retry vòng lặp. Khi offline/hết quota, vẫn nghe và sửa note bình thường; cho gửi lại sau.
- Dedupe theo clip hash + model + ngôn ngữ + cấu hình; idempotency tại backend PCisland, không giả định Groq có exactly-once. Timeout không rõ trạng thái có thể tạo request lặp upstream; phải tính vào ngân sách dự phòng.
- Free tier công bố 20 request/phút, 2.000/ngày, 2 giờ audio/giờ, 8 giờ/ngày theo tổ chức. Mọi máy dùng chung quota. Backend quản lý ngân sách audio và request; app hiện quota/lỗi thật. Không đổi IP, xoay key hoặc tổ chức để lách hạn mức. [Groq limits](https://console.groq.com/docs/rate-limits)
- Token cài đặt/phiên ngắn hạn, rate limit từng client và tổng; token ẩn danh không phải cơ chế chống lạm dụng tuyệt đối. Trước phát hành rộng phải chốt cách cấp quyền sử dụng và kiểm thử abuse; không mở proxy vô hạn tới Groq.
- Backend giới hạn loại/kích thước/thời lượng audio, giới hạn đồng thời, không nhận URL tùy ý. Request streaming hoặc vùng tạm TTL ngắn; không ghi audio/transcript/key vào log. Chốt chính sách lưu dữ liệu của Groq và backend trước phát hành.
- Clip dài vượt 25 MB phải chia nhỏ có overlap và hợp nhất theo offset; file clip gốc người dùng vẫn là một file. Không cắt mất nội dung để vừa quota. Nguồn [Groq STT](https://console.groq.com/docs/speech-to-text).

Backend, hosting, API key sản phẩm, quota thực tế và chi phí vận hành chưa được thiết lập. Free tier của Groq không đồng nghĩa hosting miễn phí hoặc transcript không giới hạn. Đây là phần cần quyết định triển khai khi tới milestone online, không chặn kệ/note hoặc ghi âm local.

### Dữ liệu và phân chia module

| Dữ liệu | Trường chính dự kiến |
|---|---|
| Meeting session | ID, tiêu đề, trạng thái, wall time bắt đầu, thời lượng audio, cấu hình input/codec, đường dẫn manifest/output, note và revision |
| Audio chunk | Session ID, sequence, sample range, path, finalized, checksum |
| Highlight | ID, session ID, start/end sample, tiêu đề, clip path, trạng thái export, revision |
| Transcript job | Clip ID/hash, provider/model/config, trạng thái, số retry, next retry, mã lỗi, request ID |
| Transcript | Text máy, text người dùng sửa, segments/offset, revision; không lưu key |

Thêm `src-tauri/src/modules/meeting/` cho state machine, marker, persistence, export job, transcript provider adapter; `src-tauri/src/platform/meeting_audio/` cho Windows capture/codec. Frontend `src/features/meeting/` và trang main đăng ký qua registry. Hoạt động `meeting.recording` tồn tại độc lập với media, ưu tiên báo lỗi/critical theo ActivityHub hiện tại.

IPC dự kiến nhóm start/stop/state, toggle marker, list/get session, save note, retry transcript, export clip/session. Dùng DTO có revision; timer gửi mốc thời gian, không emit tick liên tục. Backend là nguồn sự thật; UI reload không dừng capture.

Meeting đưa clip vào kệ qua app orchestration hoặc event với asset ID/ownership, không import chéo module. Đường dẫn phục vụ nghe lại phải nằm trong kho meeting đã xác thực và chỉ được cấp cho cửa sổ local có capability phù hợp.

## Thứ tự thực hiện

| Mốc | Công việc | Điều kiện hoàn tất |
|---|---|---|
| S0 | Spike native multi-file drag, codec/capture/chunk export, note window; chuẩn bị corpus họp | File vào chat là attachment thực; codec có size/license chấp nhận; capture/export không ngắt audio |
| F1 | Bỏ giới hạn 3, list cuộn, selection, batch drag/copy/remove | 50 file dùng được; kiểm tra lại nội dung bằng hash; không ảnh hưởng nhạc |
| F2 | ZIP, output lifecycle, chuyển đổi ảnh hỗ trợ | Hủy job an toàn, không mất file gốc, không phát sinh output rác không có chủ |
| N1 | Note desktop, font, topmost, đồng bộ và alarms | Countdown/chuông một nguồn; sửa nhiều cửa sổ không mất dữ liệu; DPI/multi-monitor đạt |
| M1 | Ghi mic/loopback, marker lùi 15 giây, tạo clip, recovery | Test 60 phút, nhiều marker liên tiếp, stop/crash/device change/disk full |
| M2 | Timeline, editor, nghe lại và export | Đóng UI vẫn ghi; note autosave; transcript và note tách biệt |
| M3 | Backend hạn mức + Groq + retry + sửa transcript | 20 clip họp có quyền sử dụng được đánh giá; không upload ngầm; quota/offline không làm hỏng bản ghi |
| R1 | Tích hợp, regression, đo app size/RAM/disk, cập nhật docs | Checklist qua; build khi có yêu cầu, dùng quy trình cleanup hiện hành |

Kệ và note có thể hoàn thành trước phần cloud. Meeting local phải hữu ích kể cả khi Groq ngừng free tier. Không coi ba mẫu FLEURS là điều kiện đủ để qua M3.

## Kiểm thử phát hành

- Logic: marker tại 5 giây phải bắt đầu ở 0, tại 40 giây bắt đầu ở 25; nhấn lặp không mở/đóng nhiều lần; overlap, stop khi đang đánh dấu, event tới muộn, retry và delete cạnh tranh.
- Audio: test 60 phút với tín hiệu và lời nói có mốc tham chiếu; đối chiếu cả bản toàn cuộc họp và từng clip; không drift tăng dần hoặc mất mẫu ở ranh giới chunk. Kiểm tra mic/system có đủ tiếng, gap được báo thật.
- ASR: corpus ít nhất 20 clip, ghi nhận lỗi tên/ngày/số/đầu việc và text tự sinh ở im lặng; lưu bản máy/bản sửa. Tiêu chí sản phẩm là transcript nháp giúp giảm công sửa, không tự quyết định nội dung cuộc họp. Nếu lỗi làm đổi nghĩa quá nhiều thì giữ online ở thử nghiệm, không bật rộng.
- Tài nguyên: đo baseline khi tắt tính năng và delta khi ghi 60 phút; RAM không tăng theo độ dài cuộc họp, disk gần bitrate thiết kế cộng clip/cache có chủ; upload/ZIP không chặn UI. Đề xuất mục tiêu delta RAM capture/export dưới 100 MB, cần đo riêng WebView và codec trước chốt.
- Regression: kéo file luôn mở receiver trong lúc nhạc/meeting/video; không tự pause nhạc khi đổi panel; nút quay lại player/meeting; note task alarm, search list và shortcut video vẫn chạy.
- Không tải model AI; không gọi trình cài ffmpeg/Python ngầm. Phụ thuộc mới tăng exe hơn 1 MB có ADR về kích thước/license theo nguyên tắc repo.

## Phối hợp code dùng chung và tài liệu

Thực thi từng module trong phạm vi file riêng. Trước sửa file chung, đọc lại diff mới nhất và thống nhất với agent đang sở hữu phần đó; không reset/checkout toàn file, không format diện rộng hoặc ghi đè thay đổi ngoài phạm vi.

Các điểm tích hợp cần gom thành patch nhỏ: `src-tauri/src/lib.rs`, modules registry, `core/settings.rs`, `core/storage.rs`, `app/shortcuts.rs`, Cargo/lockfile, capability/permission, `vite.config.ts`, IPC commands/events, i18n, main navigation và island registry. Không sửa player video để phục vụ meeting nếu adapter/event đã đủ.

Sau mỗi milestone cập nhật module doc tương ứng và handoff theo trạng thái thực. Plan này không thay thế doc hiện trạng trước khi code được triển khai. Khi build, dùng [quy trình cleanup](../build-cleanup.md); chỉ xóa output/cache do build sở hữu, không đụng dữ liệu note/meeting/file gốc hoặc cache agent khác đang dùng.
