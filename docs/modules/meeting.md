# Module: Cuộc họp (`meeting`)

> Trạng thái: Có source + test, chưa nghiệm thu cuộc họp thật · Phase: Bổ sung theo yêu cầu chủ dự án · Cập nhật: 2026-11-01

## 1. Mục đích

Ghi lại cuộc họp, tạo đoạn quan trọng ngay khi đang ghi và đặt bản chép nháp cạnh ghi chú cá nhân, không cài model AI lớn.

## 2. Phạm vi

Ghi mic và/hoặc âm thanh máy tính; chọn thiết bị; đánh dấu clip; lịch sử, nghe lại, ghi chú, transcript tiếng Việt tùy chọn và xuất ZIP. Chưa có diarization, AI tóm tắt, video, auto-join cuộc họp hoặc biên bản tự suy diễn.

## 3. Hành vi & luồng người dùng

1. Mở Cuộc họp trên island/main, đặt tên, chọn nguồn và thiết bị. Dropdown có cuộn, tên đầy đủ trong option, điều khiển bằng bàn phím.
2. Bắt đầu ghi; worker native tiếp tục khi người dùng mở tính năng khác. Không dừng player.
3. `Ctrl+Alt+M` hoặc nút đánh dấu mở đoạn từ tối đa 15 giây trước. Bấm lại chốt đoạn, xuất file nền trong khi bản ghi tiếp tục. Hotkey đổi được trong Cài đặt.
4. Main có timeline và editor notes cạnh nhau; notes lưu sau 800 ms, giữ bản nháp khi xung đột. Transcript máy và bản sửa là hai trường riêng.
5. Gửi Groq mặc định tắt. Bật công tắc cho phép gửi các clip đánh dấu; nút transcript/retry là yêu cầu gửi clip tương ứng. Không tự gửi toàn cuộc họp.
6. Dừng chốt đoạn đang mở, xuất bản ghi đầy đủ. Có nghe clip/bản đầy đủ và xuất thư mục cuộc họp ra ZIP.

## 4. Activity

`meeting.recording` là chỉ báo phụ, không chiếm player chính. Mở nhật ký bằng nút rõ ràng. Sự kiện `meeting://state` đồng bộ main/island.

## 5. Thiết kế kỹ thuật

`platform/meeting_audio.rs` quản lý COM/WASAPI shared mode, thiết bị mặc định hoặc ID được chọn, readiness và lỗi capture. Mic/system trộn mono 48 kHz, khung 20 ms. Hàng đợi giới hạn; mất dữ liệu/stall phải báo lỗi thay vì tiếp tục im lặng.

`codec.rs` encode Opus 48 kbps vào packet journal, flush/sync mỗi 5 giây. Clip remux Ogg với 80 ms decoder preroll và granule/pre-skip, không encode lại. Bản full đóng gói khi dừng. Journal xóa sau khi full và tất cả clip xuất thành công; lỗi giữ journal để phục hồi. Có phục hồi packet bền khi khởi động, chưa chứng minh toàn bộ cửa sổ crash giữa publish và cập nhật DB là idempotent.

`transcript.rs` dùng một worker, giãn request 4 giây, timeout 120 giây, tối đa 5 retry cho lỗi tạm thời/429; cache SHA256 + model + ngôn ngữ. Job bị gián đoạn cần retry rõ ràng. Upload multipart file tối đa 24 MB, Ogg Opus dùng tên `.ogg`, MIME `audio/ogg`; model `whisper-large-v3`, `language=vi`. Không lưu key vào frontend/log/DB. Build native đọc `GROQ_API_KEY` từ môi trường hoặc `.env` và nhúng vào binary theo yêu cầu chủ dự án.

Protocol `meetingaudio` chỉ giải file từ meeting/clip ID đã lưu, đọc từng phần phục vụ audio. ZIP dùng thư viện shared, không theo reparse point.

## 6. IPC

`meeting_state`, `meeting_devices`, `meeting_start`, `meeting_stop`, `meeting_mark`, `meeting_notes(id,text,revision)`, `meeting_retry(id,clipId)`, `meeting_transcript_edit(id,clipId,text)`, `meeting_export(id,destination)`.

DTO chính: `Meeting`, `Highlight`, `MeetingSnapshot`, `MeetingDevice`. Các command chỉ qua wrapper IPC; quyền ở capability của UI local.

## 7. Dữ liệu & cài đặt

SQLite `kv`: `meeting.session.<id>` và cache transcript. File nằm ở `data/meetings/<id>/`: journal, full Opus và các clip; khi xuất thêm `notes.md`, `meeting.json`. Ghi chú có revision, không ghi đè draft cũ. Thiết bị/nguồn được nhớ trong localStorage mỗi cửa sổ; chưa đồng bộ tùy chọn nguồn giữa main/island. Phím mặc định `Ctrl+Alt+M`.

## 8. Phụ thuộc

Core storage, ActivityHub; adapter WASAPI ở platform; `opus`, `ogg`; reqwest multipart sẵn có; `shared/archive`. Không FFmpeg, Python runtime hoặc model local. Xem ADR-020.

## 9. Trường hợp biên & lỗi

- Windows từ chối mic: trả lỗi quyền, hướng dẫn kiểm tra quyền hoặc dùng nguồn system; không tự đổi thiết lập Windows.
- Đổi/rút thiết bị, queue quá tải, disk lỗi: dừng có trạng thái lỗi và giữ phần đã ghi bền.
- Transcript hết quota/mất mạng: audio vẫn local, báo trạng thái và cho retry.
- File xuất đã tồn tại: không ghi đè. Khôi phục crash đúng lúc file đã publish nhưng DB chưa cập nhật còn cần hoàn thiện.
- Không coi transcript là biên bản đã xác minh hoặc tự gán người nói.

## 10. Hiệu năng

Audio payload mục tiêu 48 kbps tương đương khoảng 21,6 MB/giờ, cộng container; Opus VBR có thể lệch. Trong lúc finalize tạm giữ cả journal và full, cộng các clip. RAM capture dùng hàng đợi giới hạn, không nạp 60 phút PCM vào RAM. Chưa đo RAM/CPU/độ tăng kích thước release thực tế.

## 11. Kiểm thử

Test decode clip/preroll và độ dài; state/notes revision; capture hệ thống + liệt kê endpoint native; Groq thật dùng mẫu FLEURS công khai, cả WAV và pipeline Opus đạt. UI dropdown đã xem bằng fixture rộng 400 px, không tràn ngang.

Còn nghiệm thu: mic (máy QA trả Access denied), 60 phút, nghe/seek native, rút thiết bị/sleep, crash/recovery, âm thanh hỗn hợp/echo, quota retry và 20 clip họp có quyền sử dụng. Không gửi audio cá nhân trong QA.

## 12. Câu hỏi mở

Hạn mức key dùng chung và chất lượng nhận dạng thực tế trước khi phát hành. Backend giữ key là hướng nâng cấp, không nằm trong bản thử đã được chủ dự án chốt.

## 13. Lịch sử thay đổi

| Ngày | Thay đổi |
|---|---|
| 2026-11-01 | Triển khai local recording/highlight/notes/transcript, custom dropdown; QA debug và test pipeline Opus → Groq |
