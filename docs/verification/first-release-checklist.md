# First Release Verification Checklist

## Desktop Shell

- [x] `npm run test` succeeds.
- [x] `npm run build` succeeds.
- [x] `cargo test` succeeds in `src-tauri`.
- [x] `cargo check` succeeds in `src-tauri`.
- [x] `cargo clippy -- -D warnings` succeeds in `src-tauri`.
- [x] `npm run tauri:dev` opens the app window.

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
- [x] Installer-managed `ffmpeg` and `ffprobe` status is visible.
- [x] FFmpeg license profile is recorded before public distribution.

## Login

- [x] Bilibili QR generation returns a `qrcode_key`.
- [x] Bilibili QR poll maps pending, scanned, expired, and confirmed states.
- [x] Confirmed QR poll stores cookies in the encrypted local session file.
- [x] Stored login state is verified on startup.

## Known Gaps

- The real frontend exposes QR-code login controls, renders a scannable QR image, and automatically polls login status after QR generation.
- Native media download now has a tested core path from playurl fetch through stream download and `ffmpeg` merge, a storage-backed single-task executor can persist progress/results, configured or installer-managed `ffmpeg` / `ffprobe` paths can be detected from Settings and shown in the UI, and the frontend calls the backend `run_task` command after task creation with configured concurrency. The desktop runtime still lacks backend event streaming and durable backend scheduling across app restarts.
- `create_task` uses real native bilibili probe by default and persists created records; the frontend triggers `run_task`, reloads persisted history on startup, and polls persisted task groups while a task is running so visible progress is not stale.
- Startup login verification now checks stored bilibili cookies during the existing login status load, clears confirmed-invalid cookies, and preserves sessions as `待验证` when the network is unavailable.
- `ffmpeg` / `ffprobe` are installer-managed on Windows: the full NSIS installer bundles the pinned LGPL FFmpeg archive and installs without downloading; the slim app-update installer skips work when both tools already exist and downloads plus verifies the same release asset only when either required tool is missing. The license profile is recorded in `docs/distribution/ffmpeg-license-profile.md`.
