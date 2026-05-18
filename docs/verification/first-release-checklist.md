# First Release Verification Checklist

## Desktop Shell

- [x] `npm run test` succeeds.
- [x] `npm run build` succeeds.
- [x] `cargo test` succeeds in `src-tauri`.
- [x] `cargo check` succeeds in `src-tauri`.
- [x] `cargo clippy -- -D warnings` succeeds in `src-tauri`.
- [ ] `npm run tauri:dev` opens the app window.

## UI

- [x] Downloads/Login/Settings navigation works.
- [x] Narrow navigation has no horizontal scrollbar.
- [x] Navigation labels stay on one line.
- [x] Download task creation exposes video link and output directory.
- [x] Settings owns default engine selection.
- [x] Downloads tab does not expose task-level engine selection.
- [x] Login tab shows a flat platform list.
- [x] Clicking bilibili expands login details.
- [x] Collection task details show child video name, output file, progress, and retry count.
- [x] QR-code login controls are available in expanded bilibili details.

## Persistence

- [x] Config persists through SQLite storage.
- [x] Task history persists after app restart.
- [x] Encrypted bilibili session file persists through the session store.
- [x] Clearing bilibili login removes the managed session file.

## Engines And Tools

- [x] Native engine parses BV links.
- [x] Native engine fetches BV playurl streams and merges a single task through a configured `ffmpeg`.
- [x] Native engine expands multi-part bilibili videos during probe.
- [x] Native engine returns `unsupported_content` for unsupported links.
- [x] Missing `yt-dlp` reports `engine_missing`.
- [x] `yt-dlp` adapter can execute a configured local binary.
- [x] Bundled `ffmpeg` and `ffprobe` status is visible.
- [ ] FFmpeg license profile is recorded before public distribution.

## Login

- [x] Bilibili QR generation returns a `qrcode_key`.
- [x] Bilibili QR poll maps pending, scanned, expired, and confirmed states.
- [x] Confirmed QR poll stores cookies in the encrypted local session file.
- [x] Stored login state is verified on startup.

## Known Gaps

- The real frontend exposes manual QR-code login controls in the expanded bilibili login detail; automatic polling and rendered QR images are still deferred.
- Native media download now has a tested core path from playurl fetch through stream download and `ffmpeg` merge, a storage-backed single-task executor can persist progress/results, configured `ffmpeg` / `ffprobe` paths can be stored from Settings and shown in the UI, and the frontend calls the backend `run_task` command after task creation. The desktop runtime still lacks bundled binaries, live progress streaming, and background concurrency scheduling.
- `create_task` uses real native bilibili probe by default and persists created records; the frontend now triggers `run_task` for created child tasks and reloads persisted history on startup, but it still does not stream in-flight progress.
- Startup login verification now checks stored bilibili cookies during the existing login status load, clears confirmed-invalid cookies, and preserves sessions as `待验证` when the network is unavailable.
- Bundled `ffmpeg` / `ffprobe` binaries, `externalBin` entries, and license profile are not configured for distribution; `externalBin` must wait until target-triple binaries are present because Tauri validates them during build.
