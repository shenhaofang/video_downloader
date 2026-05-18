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
- [ ] Native engine downloads a BV single video to `.mp4`.
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
- Native media download is not wired end to end. The parser, streaming download helper, and ffmpeg merge boundary exist, but `NativeBilibiliDownloader::download()` still returns a staged error until task metadata carries `cid` / stream URLs and bundled ffmpeg is configured.
- `create_task` still uses the mock downloader path in the Tauri command layer, so the visible task creation flow is not yet a real bilibili download.
- Startup login verification is not implemented; stored sessions are loaded by presence, not revalidated against Bilibili.
- Bundled `ffmpeg` / `ffprobe` binaries and license profile are not configured for distribution.
