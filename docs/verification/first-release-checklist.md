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
- [ ] QR-code login controls are available in expanded bilibili details.

## Persistence

- [x] Config persists through SQLite storage.
- [ ] Task history persists after app restart.
- [x] Encrypted bilibili session file persists through the session store.
- [x] Clearing bilibili login removes the managed session file.

## Engines And Tools

- [x] Native engine parses BV links.
- [x] Native engine fetches BV playurl streams and merges a single task through a configured `ffmpeg`.
- [x] Native engine expands multi-part bilibili videos during probe.
- [x] Native engine returns `unsupported_content` for unsupported links.
- [x] Missing `yt-dlp` reports `engine_missing`.
- [x] `yt-dlp` adapter can execute a configured local binary.
- [ ] Bundled `ffmpeg` and `ffprobe` status is visible.
- [ ] FFmpeg license profile is recorded before public distribution.

## Login

- [x] Bilibili QR generation returns a `qrcode_key`.
- [x] Bilibili QR poll maps pending, scanned, expired, and confirmed states.
- [x] Confirmed QR poll stores cookies in the encrypted local session file.
- [ ] Stored login state is verified on startup.

## Known Gaps

- The real frontend still does not expose QR-code login controls. The backend commands exist, but the expanded bilibili login detail is still placeholder text.
- Native media download now has a tested core path from playurl fetch through stream download and `ffmpeg` merge, a storage-backed single-task executor can persist progress/results, configured `ffmpeg` / `ffprobe` paths can be stored and reported, and the backend exposes a single-task `run_task` command. The desktop runtime still lacks bundled binaries and no background scheduler invokes the executor from the UI yet.
- `create_task` uses real native bilibili probe by default and persists created records, but it still only creates queued tasks; the frontend does not yet call `run_task` or poll/reload execution progress.
- Created task records are queryable from storage, but the frontend does not reload persisted task history on startup yet.
- Startup login verification is not implemented; stored sessions are loaded by presence, not revalidated against Bilibili.
- Bundled `ffmpeg` / `ffprobe` binaries, `externalBin` entries, and license profile are not configured for distribution; `externalBin` must wait until target-triple binaries are present because Tauri validates them during build.
