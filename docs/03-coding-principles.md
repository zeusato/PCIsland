# 03 · Nguyên tắc code

> Cập nhật: 2026-10-31 · Áp dụng cho mọi code trong repo, kể cả code do AI viết.

## 1. Mười nguyên tắc chung

1. **Ranh giới module là luật.** Module không import module khác (ngoại lệ duy nhất: `MediaSource`, xem [02 §3](02-system-design.md#3-kiến-trúc-backend)). Muốn nói chuyện với module khác thì phát `AppEvent` hoặc khai báo Activity.
2. **Backend giữ sự thật, frontend phản chiếu.** State nghiệp vụ nằm ở Rust. Frontend chỉ được giữ state thuần hiển thị (đang hover, đang mở menu…).
3. **Gửi trạng thái + mốc thời gian, không gửi tick.** Frontend tự nội suy đồng hồ, tiến độ bài hát và đếm ngược.
4. **Rảnh là không làm gì.** Không có vòng lặp hay timer chạy khi không cần. Polling phải thích ứng và dừng được.
5. **Lỗi mềm.** Lỗi trong một module không được làm sập island. Không `unwrap` trên dữ liệu ngoài.
6. **Windows nằm trong `platform/`.** Win32/WinRT/`unsafe` chỉ có ở đó, và bị che sau trait để test được.
7. **Thời gian là dependency.** Không gọi `Utc::now()` hay `Instant::now()` trực tiếp trong logic; dùng `Clock`.
8. **Mọi chuỗi hiển thị qua i18n, mọi màu/kích thước qua token.**
9. **Đơn giản trước, tổng quát sau.** Chưa có trường hợp sử dụng thứ hai thì chưa trừu tượng hóa (ngoại lệ: các trait platform phục vụ test).
10. **Code và tài liệu đi cùng nhau.** Một thay đổi là chưa xong nếu module doc chưa được cập nhật.

## 2. Cấu trúc repo

```
PCisland/
├── CLAUDE.md · README.md · CHANGELOG.md
├── docs/                         # tài liệu (xem README)
│   ├── modules/                  # đặc tả module
│   └── reports/                  # báo cáo gate, hiệu năng
├── package.json · pnpm-lock.yaml · vite.config.ts · svelte.config.js · tsconfig.json
├── island.html · main.html · web-player-toolbar.html     # điểm vào Vite
├── src/                          # frontend
│   ├── apps/
│   │   ├── island/               # IslandRoot.svelte, islandMachine.ts
│   │   ├── main/                 # khung cửa sổ đầy đủ, router, layout
│   │   └── web-player-toolbar/
│   ├── features/                 # một thư mục cho mỗi module backend
│   │   └── timer/
│   │       ├── views/            # Compact, Peek, Alert, Bubble, Page, Settings
│   │       ├── store.svelte.ts
│   │       └── index.ts          # đăng ký view vào registry
│   ├── lib/
│   │   ├── ipc/                  # commands.ts, events.ts, bindings/ (ts-rs sinh, không sửa tay)
│   │   ├── motion/               # preset spring, transition
│   │   ├── i18n/                 # vi.json, en.json, t()
│   │   ├── styles/               # tokens.css, base.css
│   │   ├── components/           # UI dùng chung: Button, IconButton, Slider, Toggle…
│   │   └── registry/             # view registry (ActivityKind) + quick action registry (launcher)
│   └── test/                     # tiện ích test
├── src-tauri/
│   ├── Cargo.toml · tauri.conf.json · build.rs
│   ├── capabilities/             # island.json, main.json, web-player-*.json
│   ├── migrations/               # NNNN_<module>_<tên>.sql
│   └── src/
│       ├── main.rs               # chỉ gọi lib::run()
│       ├── lib.rs                # dựng Tauri builder, plugin, state, command
│       ├── app/                  # vòng đời, tray, autostart, phím tắt, cập nhật
│       ├── core/                 # registry, activity, events, settings, storage, audio, artwork, clock, paths, error
│       ├── shared/               # thư viện thuần: nlp_vi, lrc, fuzzy
│       ├── platform/             # Win32/WinRT (unsafe chỉ ở đây)
│       ├── modules/
│       │   └── timer/
│       │       ├── mod.rs        # impl Module + API công khai
│       │       ├── service.rs    # actor sở hữu state
│       │       ├── commands.rs   # #[tauri::command]
│       │       ├── dto.rs        # struct gửi qua IPC (derive TS)
│       │       ├── store.rs      # truy cập DB của module
│       │       └── tests.rs
│       └── ipc/                  # đăng ký command, IpcError, tên event
├── assets/                       # âm thanh, icon
└── scripts/                      # gen-bindings, bump-version, release
```

## 3. Rust

**Công cụ:**
- Rust stable, edition 2024. `cargo fmt` mặc định.
- `cargo clippy --all-targets -- -D warnings`, bật thêm nhóm `clippy::pedantic` ở mức `warn`. Lint nào cần tắt thì `#[allow]` tại chỗ, kèm lý do.
- `lib.rs` có `#![deny(unsafe_op_in_unsafe_fn)]`. Các module ngoài `platform` có `#![forbid(unsafe_code)]` ở file `mod.rs`.

**Đặt tên:** module/hàm `snake_case`, kiểu `PascalCase`, hằng `SCREAMING_SNAKE_CASE`. Tên command và event theo [02 §8](02-system-design.md#8-giao-tiếp-ipc). ID module là hằng `ModuleId::Timer`, chuỗi `"timer"`.

**Lỗi:**
- Mỗi module có một enum lỗi dùng `thiserror`. Không dùng `anyhow` trong module; `anyhow` chỉ được phép trong `app/` lúc khởi động.
- Cấm `unwrap()`. `expect("<lý do bất biến>")` chỉ được dùng cho bất biến lúc khởi động và trong test.
- Lỗi được đổi sang `IpcError` tại `commands.rs` qua `impl From<TimerError> for IpcError`, với `code` ổn định dạng `timer.not_found`.

**Async & đồng thời:**
- Service theo mẫu actor ([02 §4](02-system-design.md#4-hệ-thống-module)): state riêng tư trong task, lệnh qua `mpsc`, phản hồi qua `oneshot`.
- Không giữ guard `std::sync::Mutex` qua `.await`. `tokio::sync::Mutex` chỉ dùng khi thật cần, và phải có comment giải thích.
- Việc chặn (I/O file lớn, WinRT `.get()`) đưa vào `spawn_blocking` hoặc luồng riêng của platform.
- Mọi task nhận `CancellationToken` và thoát sạch.
- Command Tauri nào tạo cửa sổ (`WebviewWindowBuilder::build`) **phải là `async fn`**. Command đồng bộ chạy trên main thread; tạo cửa sổ WebView2 ở đó bị deadlock trên Windows và làm treo toàn bộ IPC (lỗi đã gặp 2026-11-01).

**Thời gian:** lưu `DateTime<Utc>` (chrono), đo thời lượng bằng `Instant`. Hẹn giờ dựa trên **deadline tuyệt đối** để chịu được sleep/resume ([timer](modules/timer.md)).

**unsafe:** chỉ trong `platform/`. Mỗi khối có comment `// SAFETY:` nêu bất biến. Mọi handle Win32 được bọc RAII (`Drop` gọi `CloseHandle`/`UnhookWinEvent`…).

**Comment:** viết tiếng Anh, giải thích *tại sao*, không lặp lại *cái gì*. Mọi `pub` item có doc comment `///` một dòng trở lên.

## 4. Frontend (Svelte 5 + TypeScript)

**TypeScript:** `strict`, `noUncheckedIndexedAccess`, không dùng `any` (dùng `unknown` rồi thu hẹp kiểu). Type IPC lấy từ `lib/ipc/bindings`, không tự khai lại.

**Svelte:**
- Chỉ dùng rune (`$state`, `$derived`, `$effect`, `$props`); không dùng cú pháp store cũ, trừ khi thư viện bắt buộc.
- `$effect` chỉ dùng cho tác dụng phụ thật (đăng ký event, đo DOM) và luôn trả hàm dọn dẹp.
- Component tối đa khoảng 200 dòng; dài hơn thì tách.
- Component nhận dữ liệu qua props và phát hành động qua callback props. Component không tự gọi IPC; việc đó thuộc store của feature.

**IPC:** chỉ `lib/ipc/commands.ts` và `lib/ipc/events.ts` được gọi `invoke`/`listen`. Store của feature dùng hai file này.

**Style:**
- Chỉ dùng biến trong `tokens.css`, không hex trần, không số đo trần (ngoại lệ: `0`, `1px` cho viền).
- Đơn vị là px logic: island có kích thước cố định nên không dùng `rem` ở island.
- CSS scoped theo component, không dùng CSS framework.

**Chuyển động:**
- Chỉ dùng preset trong `lib/motion` ([04 §6](04-design-system.md#6-chuyển-động)).
- Ưu tiên animate `transform`, `opacity`, `filter`. Riêng container island được animate `width`/`height`/`border-radius`, vì nó là một phần tử đơn có kích thước nhỏ.
- Luôn tôn trọng `prefers-reduced-motion` và cài đặt "Giảm chuyển động".

**Truy cập:**
- Cửa sổ `main` dùng được hoàn toàn bằng bàn phím: focus ring nhìn thấy được, `aria-label` cho nút chỉ có icon.
- Island: các nút trong peek có `aria-label`. Thao tác bằng bàn phím trên island đi qua phím tắt toàn cục.

**Mạng:** frontend không gọi mạng (trừ trang trong `web-player-site`). Mọi dữ liệu ngoài đi qua backend.

## 5. Lỗi & log

| Mức | Dùng khi |
|---|---|
| `error` | Thao tác thất bại và người dùng bị ảnh hưởng; module chuyển sang Failed |
| `warn` | Có lỗi nhưng đã tự phục hồi (thử lại, dùng giá trị mặc định) |
| `info` | Mốc vòng đời: khởi động, module start/stop, đổi visibility mode, đổi nguồn media |
| `debug` | Chi tiết để chẩn đoán, chỉ bật khi chạy `--debug` |
| `trace` | Dữ liệu cá nhân (tiêu đề bài, nội dung nhắc). Không bao giờ bật mặc định |

- Dùng field có cấu trúc: `info!(module = "timer", timer_id = %id, "timer started")`. Mỗi module có span riêng.
- Không log cookie, URL có query nhạy cảm, nội dung ghi chú.
- Lỗi hiển thị cho người dùng phải có hướng xử lý ("Thử lại", "Mở cài đặt"), không đưa stack trace lên UI.

## 6. Kiểm thử

- **Bắt buộc có unit test** cho mọi logic thuần mới: arbitration, policy, scheduler, parser, migration, hàng đợi phát.
- Test Rust đặt trong `tests.rs` của module (`#[cfg(test)]`). Test tích hợp dùng `FakeXxx` của platform và `TestClock`.
- Đặt tên test theo hành vi: `snoozed_reminder_fires_after_resume`, không đặt `test_1`.
- Test frontend (vitest) đặt cạnh file: `islandMachine.test.ts`.
- Bug sửa xong phải có test tái hiện, nếu tái hiện được bằng test.
- Hành vi chỉ kiểm được bằng tay (z-order, fullscreen, DPI) ghi thành checklist trong module doc, và phải chạy trước mỗi bản phát hành.

## 7. Git & review

- **Nhánh:** trunk-based. `main` luôn build được. Nhánh ngắn: `feat/<module>-<mô-tả>`, `fix/<module>-<mô-tả>`, `docs/…`, `spike/…`.
- **Commit:** Conventional Commits, viết tiếng Anh, scope là ID module:
  - `feat(timer): add lap support to stopwatch`
  - `fix(island): keep topmost after explorer restart`
  - `docs(reminders): specify missed reminder flow`
- **PR:**
  - Nhỏ, một mục đích.
  - Mô tả liên kết Feature ID (F-xx) và mục roadmap (P1.6…).
  - Có ảnh chụp hoặc GIF nếu đổi UI.
  - Đánh dấu checklist DoD (§10).
- **Checklist review:**
  - [ ] Đúng ranh giới module và lớp (§1, [02 §3](02-system-design.md#3-kiến-trúc-backend))
  - [ ] Không `unwrap`; lỗi có `code` ổn định
  - [ ] Không có polling/animation chạy khi rảnh
  - [ ] Thời gian qua `Clock`; DTO có mốc thời gian thay vì tick
  - [ ] Chuỗi qua i18n (đủ vi + en); màu/kích thước qua token
  - [ ] Có test cho logic mới
  - [ ] Module doc + handoff đã cập nhật

## 8. Phụ thuộc

- Thêm crate hoặc gói npm phải nêu lý do trong PR: kích thước, mức độ bảo trì, license.
- **License chấp nhận:** MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0. Không dùng GPL/AGPL.
- Phụ thuộc lớn (tăng exe > 1 MB) hoặc khó thay thế phải có ADR.
- Phụ thuộc dự kiến:

| Lĩnh vực | Crate / gói |
|---|---|
| Khung app | `tauri` 2, `tauri-plugin-single-instance`, `tauri-plugin-autostart`, `tauri-plugin-global-shortcut`, `tauri-plugin-dialog` |
| Windows | `windows` (Win32 + WinRT) |
| Async & tiện ích | `tokio`, `tokio-util`, `futures`, `serde`, `serde_json`, `thiserror`, `chrono`, `uuid` (v7) |
| Lưu trữ | `rusqlite` (bundled), `rusqlite_migration` |
| Âm thanh | `rodio` (symphonia), `lofty`, `souvlaki`, `realfft` |
| Mạng | `reqwest` (rustls) |
| Log | `tracing`, `tracing-subscriber`, `tracing-appender` |
| Type sharing | `ts-rs` |
| Frontend | `svelte` 5, `vite`, `typescript`, `lucide-svelte`, `vitest`, `eslint`, `prettier`, `svelte-check` |

## 9. Quy trình phát hành

1. Đóng băng tính năng; cập nhật `CHANGELOG.md`.
2. Chạy `scripts/bump-version <x.y.z>`.
3. **Release checklist** trên một máy Windows 11 sạch (hoặc VM):
   - [ ] Chạy exe từ zip tải về: SmartScreen hiện như hướng dẫn, app khởi động được
   - [ ] Checklist thủ công của mọi module thuộc bản phát hành
   - [ ] Fullscreen: video trình duyệt, game borderless, trình chiếu
   - [ ] Sleep/resume: nhắc việc và hẹn giờ vẫn đúng
   - [ ] Đa màn hình + DPI 100/125/150%
   - [ ] Đo hiệu năng theo [02 §13](02-system-design.md#13-hiệu-năng), ghi `docs/reports/perf-<ver>.md`
   - [ ] Bộ cài: cài không cần admin, có shortcut Start menu, dữ liệu ở `%APPDATA%`; gỡ cài đặt xóa sạch cả khóa autostart
   - [ ] Portable: dữ liệu nằm trong `data\` cạnh exe; chạy từ USB khóa ghi thì chuyển sang `%APPDATA%`
   - [ ] Gỡ: xóa thư mục + `--cleanup` thì không còn dấu vết (trừ khóa autostart nếu người dùng đã tắt app trước)
4. Tag `v<x.y.z>`, CI build zip + SHA256, rồi duyệt bản nháp release.

## 10. Definition of Done

Một việc chỉ xong khi:

- [ ] Code đúng các nguyên tắc ở tài liệu này; fmt/clippy/svelte-check/eslint đều sạch
- [ ] Test cho logic mới đã viết và đều qua
- [ ] Checklist thủ công liên quan trong module doc đã chạy
- [ ] Chuỗi đủ `vi` + `en`
- [ ] Module doc cập nhật: trạng thái, IPC, cài đặt, lịch sử thay đổi
- [ ] Handoff cập nhật: snapshot, việc tiếp theo, nhật ký phiên
- [ ] Không làm hụt ngân sách hiệu năng (nếu đụng tới island hoặc đường render)
