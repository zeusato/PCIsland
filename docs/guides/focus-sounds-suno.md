# Tạo và phát hành nhạc tập trung

Hướng dẫn cho chủ dự án: tạo nhạc bằng Suno, sinh manifest và đẩy lên repo `zeusato/pcisland-sounds`. Đặc tả app: [focus-sounds](../modules/focus-sounds.md), [ADR-023](../06-decision-log.md#adr-023--nhạc-tập-trung-tải-theo-yêu-cầu-từ-repo-phát-hành-riêng).

## 1. Nguyên tắc khi tạo

- **Bật Instrumental.** Không dùng ô Lyrics.
- Mỗi prompt bấm tạo 2–3 lượt, chỉ giữ bản **không có cao trào, không đổi nhịp đột ngột, kết thúc êm**. App phát xáo và lặp, nên bài kết êm thì chuyển bài không bị giật.
- Độ dài 4–8 phút. Nếu Suno ra bản ngắn, dùng **Extend** cùng prompt để kéo dài.
- Xuất **MP3** (app không đọc Opus). Giữ tempo tương đương trong cùng một gói.
- Nên tạo bằng gói Suno trả phí (Pro/Premier) để được phân phối công khai; ghi vào `license` của manifest.
- Cả bộ đề xuất 17 bài, khoảng 100–120 MB ở MP3 160–192 kbps.

## 2. Bộ prompt

Mỗi dòng: **tên file** · *Title* trên Suno · Style of Music (dán nguyên văn).

### Gói `lofi` · Tập trung · 5 bài

| File | Title | Style of Music |
|---|---|---|
| `01-rainy-desk.mp3` | Rainy Desk | lofi hip hop, 72 BPM, dusty vinyl crackle, mellow Rhodes chords, soft boom bap drums, warm bass, light rain ambience, no vocals, study, steady and unobtrusive |
| `02-late-train.mp3` | Late Train | chillhop, 70 BPM, jazzy electric piano, brushed drums, muted guitar licks, tape saturation, night city mood, no vocals, calm focus, no build-up |
| `03-paper-lantern.mp3` | Paper Lantern | lofi beats, 68 BPM, koto and soft piano motif, gentle side-chained pads, lo-fi drums, warm hiss, no vocals, serene concentration |
| `04-coffee-steam.mp3` | Coffee Steam | lofi jazz, 75 BPM, upright bass, soft saxophone pads in the background, laid-back swing drums, cafe room tone, no vocals, relaxed productivity |
| `05-window-seat.mp3` | Window Seat | mellow lofi, 66 BPM, nylon guitar arpeggios, warm sub bass, soft kick and snare, distant birds, no vocals, minimal melody, even dynamics |

### Gói `ambient` · Tập trung · 5 bài

| File | Title | Style of Music |
|---|---|---|
| `01-deep-work-drift.mp3` | Deep Work Drift | ambient focus music, 60 BPM, soft analog synth pads, warm Rhodes, gentle tape hiss, slow evolving drones, no drums, no vocals, calm and steady |
| `02-glass-horizon.mp3` | Glass Horizon | ambient electronic, slow pulsing arpeggio, glassy pads, wide stereo reverb, 64 BPM, no percussion, no vocals, weightless, consistent texture |
| `03-north-light.mp3` | North Light | cinematic ambient, sustained strings, soft synth swells, very slow harmonic movement, no drums, no vocals, spacious, no crescendo |
| `04-quiet-orbit.mp3` | Quiet Orbit | space ambient, deep drone, shimmering granular textures, soft sub bass, no beat, no vocals, hypnotic, steady volume |
| `05-brown-noise-bed.mp3` | Brown Noise Bed | ambient drone over brown noise bed, warm low pads, barely moving harmony, no melody, no drums, no vocals, masking background for concentration |

### Gói `piano` · Tập trung · 3 bài

| File | Title | Style of Music |
|---|---|---|
| `01-still-water.mp3` | Still Water | minimal solo piano, sparse notes, soft felt piano, long reverb, slow tempo, no other instruments, no vocals, contemplative, gentle dynamics |
| `02-morning-notes.mp3` | Morning Notes | neo-classical piano, repeating gentle ostinato, light cello drone underneath, 70 BPM, no vocals, focused and warm, no dramatic climax |
| `03-soft-ledger.mp3` | Soft Ledger | ambient piano, muted upright piano, room noise, simple chord progression looping, no vocals, intimate, steady |

### Gói `rest` · Nghỉ · 4 bài

| File | Title | Style of Music |
|---|---|---|
| `01-breathing-room.mp3` | Breathing Room | meditative ambient, tibetan singing bowls, soft wind chimes, distant rain, airy pads, very slow, breathing pace, no beat, no vocals, peaceful reset |
| `02-forest-pause.mp3` | Forest Pause | nature soundscape with soft ambient pads, birdsong, gentle stream, light breeze, no rhythm, no vocals, restorative |
| `03-tide-out.mp3` | Tide Out | ocean waves ambient, slow swelling pads, low drone, calm and spacious, no percussion, no vocals, deep relaxation |
| `04-tea-break.mp3` | Tea Break | soft acoustic guitar and kalimba, slow tempo, warm room ambience, light and cheerful, no drums, no vocals, short restful interlude |

## 3. Đóng gói

Xếp file theo thư mục gói; mỗi gói có `pack.json`:

```
focus-sounds/
  lofi/    pack.json  01-rainy-desk.mp3 …
  ambient/ pack.json  …
  piano/   pack.json  …
  rest/    pack.json  …
```

```json
{ "role": "focus", "name": { "vi": "Lo-fi", "en": "Lo-fi" } }
```

Gói nghỉ dùng `"role": "break"`. Tên hiển thị bài lấy từ tên file (bỏ số thứ tự, gạch ngang thành dấu cách); muốn đặt tên khác thì thêm `"titles": { "01-rainy-desk.mp3": "Bàn mưa" }`.

Sinh manifest và thư mục upload:

```bash
node scripts/focus-sounds-manifest.mjs D:/music/focus-sounds --release focus-sounds-v1 --license "Suno Pro, Zeusato"
```

Script kiểm tra tên/định dạng, tính SHA-256, ghi `D:/music/focus-sounds/release/` (file đã đổi tên `<gói>-<file>` + `manifest.json`) và in lệnh tạo release.

## 4. Phát hành

```bash
gh repo create zeusato/pcisland-sounds --public --description "Focus music for PCisland"
```

```bash
gh release create focus-sounds-v1 D:/music/focus-sounds/release/* --repo zeusato/pcisland-sounds --title "Focus sounds v1" --notes "Nhạc tập trung cho PCisland"
```

Release mới (thêm/sửa bài) dùng tag mới, ví dụ `focus-sounds-v2`, và đính kèm **đủ** mọi file đang dùng: app tải theo release mới nhất. Bài không đổi giữ nguyên hash nên app không tải lại.
