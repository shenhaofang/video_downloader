# Video Downloader Tauri Design

## Scope

Build a local desktop video downloader. The first release focuses on bilibili.

The app is a Rust + Tauri desktop client. The frontend provides a simple local UI. The Rust core handles platform parsing, login, task scheduling, retries, persistence, logging, file output, and media merging.

First release goals:

- Download bilibili single videos.
- Download bilibili multi-part videos, collections, or list-like links when supported by the selected engine.
- Save final `.mp4` files directly to a user-selected local directory.
- Allow choosing an output directory when creating a download task, prefilled from the default download root.
- Use the highest available quality by default.
- Support bilibili QR-code login.
- Persist login state in a local encrypted file.
- Persist task history, logs, settings, output paths, and failure reasons.
- Provide two download engines:
  - `native`: Rust native bilibili subset implementation. This is the default.
  - `yt-dlp`: optional fallback adapter, downloaded or installed on demand.
- Bundle `ffmpeg` and `ffprobe` with the app.

Out of scope for the first release:

- User accounts or multi-user isolation.
- Scheduled subscription downloads by uploader, keyword, or category.
- Cover image, subtitle, or metadata JSON output.
- Full-featured admin console.
- Native coverage of every bilibili content type.
- DRM, captcha, platform access-control bypass, or entitlement bypass.

## Architecture

The app is a single local desktop application.

Recommended module layout:

- `src-tauri/src/commands`: Tauri commands exposed to the frontend.
- `src-tauri/src/task`: task queue, concurrency control, state machine, retry handling.
- `src-tauri/src/platform`: shared platform downloader traits.
- `src-tauri/src/platform/bilibili`: bilibili engines.
- `src-tauri/src/platform/bilibili/native`: Rust native implementation.
- `src-tauri/src/platform/bilibili/yt_dlp`: optional `yt-dlp` adapter.
- `src-tauri/src/auth/bilibili`: QR-code login, cookie verification, encrypted session storage.
- `src-tauri/src/storage`: SQLite persistence for tasks, settings, and logs.
- `src-tauri/src/media`: path generation, filename sanitization, bundled `ffmpeg` and `ffprobe` invocation.
- `src-tauri/src/events`: task status and log events sent to the frontend.
- `frontend`: single-page UI.

The UI and task system must not depend on bilibili API details or `yt-dlp` command syntax. They depend only on a shared downloader interface.

Example interface shape:

```rust
trait PlatformDownloader {
    async fn probe(&self, input: ProbeInput) -> Result<ProbeResult, DownloadError>;
    async fn expand(&self, input: ExpandInput) -> Result<Vec<DownloadItem>, DownloadError>;
    async fn download(&self, input: DownloadInput, events: EventSink) -> Result<DownloadOutput, DownloadError>;
}
```

## Download Engines

The app ships with two bilibili engines.

`native` is the default engine. It should support the core bilibili subset needed for the first release:

- BV video links.
- Multi-part videos.
- Anonymous probing.
- Logged-in probing.
- Highest available quality selection.
- DASH audio/video download when needed.
- `ffmpeg` merge to `.mp4`.

`yt-dlp` is an optional fallback engine. It is not bundled by default. When a user chooses `yt-dlp` in Settings or retries a failed task with `yt-dlp`, the app checks whether the tool is available. If missing, the app prompts the user to download or install it. The app stores the resolved path and version after setup.

Engine selection rules:

- The default engine is configured only in the Settings tab.
- The download task creation UI does not show an engine selector.
- New tasks use the current global default engine.
- Each task records the actual engine used.
- Failed tasks may offer "retry with the other engine" as an explicit recovery action.
- `native` may return `unsupported_content` for content types it does not yet cover. The UI should suggest `yt-dlp` fallback in that case.

## Media Tooling

`ffmpeg` and `ffprobe` are required app components.

The app bundles platform-specific binaries and invokes them from app-managed paths. The user should not need to install them or add them to `PATH`.

The Settings tab shows:

- bundled `ffmpeg` status,
- bundled `ffprobe` status,
- detected versions,
- diagnostic error if a binary is missing or cannot run.

Before public distribution, the build pipeline must verify the license profile of the bundled `ffmpeg` build, especially LGPL/GPL options.

## Login And Session Storage

The bilibili login flow uses QR-code login.

Flow:

1. The user opens the login area and clicks login.
2. Rust requests a bilibili QR-code login token.
3. The frontend displays the QR code.
4. Rust polls login status.
5. On success, Rust captures the necessary cookies.
6. Rust stores session data in a local encrypted file.
7. The UI updates the login state.

Session storage:

- Store cookies in an encrypted local file under the Tauri app data directory.
- Do not store account passwords.
- Do not store cookies as plaintext in SQLite.
- Save platform name, necessary cookies, expiration data when available, and last verification time.
- Provide "clear login state" and "re-login" actions.

Startup behavior:

- Try to decrypt and load the stored bilibili session.
- Verify it through a lightweight bilibili login-status endpoint.
- If valid, mark bilibili as logged in.
- If invalid or decrypt fails, mark as logged out and allow re-login.

Task behavior:

- Probe anonymously first.
- If logged-in probing exposes higher quality or is required for access, use the session.
- Record whether the final task used login state.

## Task Model

Use SQLite for durable task data.

Core entities:

- `TaskGroup`: one user submission, including the output directory chosen for that submission.
- `DownloadTask`: one actual downloadable item, such as one video or one part.
- `TaskLog`: append-only task logs.
- `AppConfig`: settings such as default download root, concurrency, and default engine.
- `SessionState`: encrypted file-backed session data, not a SQLite cookie record.

Task states:

- `pending`: created.
- `probing`: resolving metadata, quality, and login requirements.
- `queued`: ready and waiting for a concurrency slot.
- `downloading`: downloading media data.
- `merging`: producing final `.mp4`.
- `completed`: success.
- `failed`: failed with structured error data.
- `interrupted`: the app exited or crashed while work was in progress.
- `cancelled`: user cancelled the task.

Batch behavior:

- A submitted collection expands into a `TaskGroup` with multiple `DownloadTask` records.
- Concurrency applies to `DownloadTask`, not `TaskGroup`.
- Default concurrency is 2.
- One failed child task does not block the rest unless the failure is a global condition such as expired login state.
- Expanded `TaskGroup` details must show each child `DownloadTask` with video name, output file, download progress, and retry count.

Restart behavior:

- Persist task history, config, logs, actual quality, output path, selected engine, and login usage.
- On app startup, tasks left in `probing`, `downloading`, or `merging` become `interrupted`.
- Tasks left in `queued` can remain queued if they had not started external work.
- Interrupted tasks can be manually retried.
- Persisted login state can be reused after successful verification.

## File Output

The user configures a default download root directory in Settings.

When creating a task, the Downloads tab also shows an output directory picker. It is prefilled from the default download root, but the user can override it for that task. The selected directory is saved on the `TaskGroup` and is used for every child `DownloadTask` created from that submission.

Default output layout:

```text
<selected-output-directory>/bilibili/<collection-or-video-title>/<index> - <video-title>.mp4
```

Rules:

- Single videos still get a title directory.
- Multi-part or collection tasks use the collection/main title as the directory.
- Preserve platform order for numbering.
- Sanitize invalid filesystem characters.
- Limit excessive filename length.
- Do not overwrite existing files by default.
- On conflict, append a suffix such as `(1)`.
- Store the final output path on the task.

## UI

The first release uses a simple single-page desktop UI with tabs or clearly separated sections.

The UI design must be reviewed against the interactive prototype at:

```text
docs/superpowers/prototypes/video-downloader-ui/index.html
```

The prototype is a low-fidelity design artifact. It confirms layout, task interactions, login-state controls, Settings-only engine selection, and fallback retry behavior. It is not production frontend code.

Downloads tab:

- Link input.
- Output directory picker, prefilled from the Settings default download root.
- Add download button.
- Task groups and child tasks.
- Status, title, quality, login usage, output path, failure reason, and actual engine used.
- For collection or multi-part tasks, expanded details show each child video name, output file, download progress, and retry count.
- Cancel.
- Retry.
- Retry with the other engine when useful.
- Open output folder.
- Expand logs.

The Downloads tab must not show an engine selector for task creation. Engine selection is centralized in Settings.

Login tab:

- Platform login status list.
- Each platform row is a single line with platform name on the left and a short login status on the right.
- Clicking a platform row expands platform-specific details below it.
- For bilibili, expanded details include QR-code login, re-validate login state, and clear login state.
- The first release only enables bilibili, but the UI structure must support adding more platform rows later.

Settings tab:

- Default download root directory picker.
- Concurrency setting, default 2.
- Default engine setting, default `native`.
- `yt-dlp` status, version, path, and download/install action.
- Bundled `ffmpeg` and `ffprobe` status and versions.

The UI should stay compact and operational. Do not build a marketing landing page or a full admin console.

Navigation buttons must keep their labels on one line across desktop and narrow layouts. The narrow top-nav layout should allocate enough button width for four Chinese characters such as `下载任务`, and it should wrap the top bar instead of showing a horizontal scrollbar inside the nav.

## Error Handling

Errors are structured and user-readable.

Required categories:

- `network_error`: timeout, connection failure, CDN interruption. Auto-retry allowed.
- `login_required`: login is needed. Prompt QR-code login.
- `login_expired`: stored login state no longer works. Prompt re-login.
- `permission_denied`: account lacks permission, such as membership, region, or copyright restrictions. No auto-retry.
- `unsupported_content`: the selected engine does not support this content. Suggest the other engine when applicable.
- `engine_missing`: selected `yt-dlp` engine is unavailable.
- `ffmpeg_error`: bundled media merge or probe failed.
- `filesystem_error`: missing directory, permission denied, disk full, or path conflict failure.
- `platform_changed`: native parser no longer matches bilibili behavior.
- `unknown_error`: unclassified failure with logs preserved.

Retry rules:

- Network errors auto-retry up to 3 times.
- Login, permission, filesystem, unsupported content, and merge errors do not auto-retry.
- Manual retry is always available when the task is not running.
- Retry with the other engine is available for supported failure cases.

## Verification

Before calling the first release complete, verify:

- A BV single-video link downloads as `.mp4` using the default `native` engine.
- A multi-part bilibili video expands and downloads as multiple `.mp4` files.
- A logged-in sample shows whether login changes available quality.
- The app chooses the highest available quality.
- The app records actual quality and login usage.
- The task list persists after restart.
- The encrypted bilibili session persists after restart and is verified on startup.
- Interrupted tasks are marked `interrupted` and can be retried.
- The Downloads tab does not expose a task-level engine selector.
- The Downloads tab exposes an output directory picker for the task being created.
- Changing the default engine in Settings affects newly created tasks.
- Changing the default download root in Settings updates the prefilled task output directory.
- A failed task can be retried with the other engine when applicable.
- Missing `yt-dlp` triggers the on-demand install/download path.
- Bundled `ffmpeg` and `ffprobe` are detected and used.
- Task logs can be expanded in the UI.
- Collection task details show every child video's name, output file, download progress, and retry count.
- Login tab shows a platform login status list, with bilibili details available only after expanding that platform row.
- Files are written under the configured root directory using the expected naming scheme.
- Invalid filenames and conflicts are handled without overwriting existing files.

## Open Risks

- bilibili login and media APIs can change. The native engine must surface `platform_changed` rather than failing opaquely.
- Native support may lag behind `yt-dlp` for complex collections. The fallback path must stay visible.
- Bundled `ffmpeg` distribution requires license review before public release.
- Encrypted local session storage improves over plaintext, but anyone with local machine access and app data access may still be in the trust boundary unless stronger user-provided secrets are added later.
