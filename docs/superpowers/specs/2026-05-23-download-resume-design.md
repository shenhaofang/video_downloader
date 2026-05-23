# Download Resume Design

## Goal

Paused downloads must be recoverable for both engines:

- `native` Bilibili downloads should reuse already written audio/video partial files when the CDN supports byte ranges.
- `yt-dlp` downloads should use `yt-dlp`'s own continuation behavior and preserve its partial files across pause/start.

The user-facing controls stay unchanged: child rows and task groups continue to use `pause`, `start`, `retry`, and `delete`. The behavior behind `start` and group `continue` changes from "always restart from scratch" to "resume when safe, restart only when resume is unsafe or unsupported".

## Current State

`native` currently creates a fresh UUID temporary directory for each run, downloads video and audio streams with `File::create`, and removes the temp directory after the run. Pausing aborts the active future and persists `TaskState::Paused`, but starting again performs a fresh execution.

`yt-dlp` currently has detection, installation, argument construction, and process execution helpers. However, persisted `YtDlp` task creation and task execution are not wired into `create_task` and `run_task`; those paths still return `engine_missing`.

## Requirements

- Pause must not convert a task into `failed`.
- Starting a paused task must prefer resuming partial bytes.
- If resume cannot be proven safe, the app must fall back to a clean re-download for that task or stream.
- Delete must clean task-owned partial files without deleting unrelated user files.
- Retry from `failed` should keep useful resumable partials unless the failure indicates corrupt local data.
- Completed tasks should remove task-owned resume metadata and partial files.
- The UI should continue to show existing progress, retry count, and failure detail surfaces.
- No per-task engine selector should be added to the create form or child rows.

## Native Resume Design

### Stable Resume Workspace

Each task gets a deterministic workspace under its output directory:

```text
<output-dir>/.video-downloader/<task-id>/
```

The native engine stores:

- `video.part`
- `audio.part`
- `resume.json`

`resume.json` records the task id, engine, bvid, cid, page, selected quality, stream URLs or URL fingerprints, current stream sizes, and expected total sizes when available.

### Range Download Flow

For each stream:

1. Re-fetch Bilibili playurl before every run so expired CDN URLs are refreshed.
2. Inspect existing `.part` length.
3. If the file is empty or missing, download normally.
4. If the file has bytes, request `Range: bytes=<existing-size>-`.
5. If the server returns `206 Partial Content`, append to the `.part` file.
6. If the server returns `200 OK`, delete that stream's `.part` and restart that stream from byte 0.
7. If the server returns `416 Range Not Satisfiable`, verify whether the local size already matches the expected total; if not, restart that stream.

Video and audio are independent. A task can resume video while restarting audio, or the other way around.

### Merge Flow

FFmpeg merge is not resumed. If pause happens during merge, starting the task again should reuse completed `.part` media files and run merge from the beginning.

After successful merge:

- Persist `completed`.
- Remove the task workspace.

### Safety Rules

- Never append to a partial file if bvid/cid/page/quality no longer matches the task.
- Never append when the existing file is larger than the expected total.
- Never delete outside `<output-dir>/.video-downloader/<task-id>/`.
- Use `kill_on_drop` for external processes so pause/delete can stop active work.

## yt-dlp Resume Design

### Wiring Missing Task Paths

This feature also wires `yt-dlp` task creation and execution:

- `yt-dlp --dump-json` probes metadata and creates persisted `DownloadTask` rows.
- Download execution uses the persisted task URL/output path and the configured `yt-dlp.exe`.
- The process runs through the existing no-window process helper.
- Active run registration must prevent duplicate `yt-dlp` processes for the same task.

### Continuation Behavior

The app should let `yt-dlp` manage its own resume files:

- Pass `--continue`.
- Do not pass `--no-continue`.
- Use a stable output template for each persisted task.
- Preserve `yt-dlp` partial files between pause and start.
- On pause/delete, abort the active `yt-dlp` process. Pause keeps partial files; delete removes task-owned partial files.

Expected task-owned files include the final target and adjacent files generated from the same output template, such as `.part`, `.ytdl`, `.temp`, and `.frag` variants. Cleanup must be conservative: remove only paths derived from the exact task output path or inside the task workspace.

### Progress and Logs

Use `--newline` output and parse enough progress to update:

- `bytes_downloaded`
- `bytes_total`
- task logs
- `downloading` / `merging` state

If parsing is incomplete, preserve raw lines in task logs and keep coarse state updates rather than hiding the process.

## Task Lifecycle

- `run_task`: automatic post-create runner. It should skip `paused` tasks, preserving the existing rule that paused tasks only restart manually.
- `start_task`: manually starts `queued`, `paused`, or `interrupted` tasks and enables resume behavior.
- `retry_task`: resets failure fields and starts again. It should not delete partials by default.
- `pause_task`: aborts the active run and persists `paused`.
- `delete_task`: aborts the active run, deletes the persisted task, and cleans task-owned resume files.

## Error Handling

- Network interruption during native resume remains `network_error`; the task becomes `failed` only when the active run returns an error rather than being paused.
- Missing or invalid `yt-dlp` remains `engine_missing`.
- `yt-dlp` non-zero exits map to `unknown_error` unless a clearer existing error code can be inferred.
- Filesystem cleanup failures should be logged and surfaced as `filesystem_error` when they prevent task completion or deletion.
- Platform response changes in native still map to `platform_changed`.

## Testing

Native tests:

- Existing partial file plus `206 Partial Content` appends only missing bytes.
- Existing partial file plus `200 OK` restarts that stream.
- `416` with matching expected size treats the stream as complete.
- Pause keeps the task workspace; completion deletes it.
- Delete only removes the task workspace.
- Merge restart after pause reuses complete `.part` files.

yt-dlp tests:

- `ytdlp_download_args` includes `--continue` and never includes `--no-continue`.
- A fake `yt-dlp` process can be aborted by `pause_task`, leaving partial files and persisting `paused`.
- Starting a paused `yt-dlp` task invokes the same output template.
- Deleting a `yt-dlp` task cleans only derived task-owned partial files.
- `create_task` and `run_task` no longer return `engine_missing` when configured `yt-dlp.exe` exists.

End-to-end gates:

- `cargo test`
- `cargo check`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `npm run test -- --pool=threads`
- `npm run build`
- `git diff --check`
- `npm run tauri:build`

## Out Of Scope

- True resume of an in-progress ffmpeg merge.
- Cross-engine resume file compatibility between `native` and `yt-dlp`.
- Per-task engine selection in the create form or child rows.
- Automatic retry policy changes.
- Remote/cloud sync of partial download state.

## Open Risks

- Bilibili CDN URLs may expire or change range support; the native implementation must re-fetch playurl and safely restart unsupported streams.
- Some `yt-dlp` formats may produce additional sidecar files; cleanup must be conservative to avoid deleting user files.
- Disk-space failures during append can leave partials; future starts should either resume or restart safely.
- Existing installed databases may contain paused tasks without resume metadata; those tasks should continue by restarting cleanly.
