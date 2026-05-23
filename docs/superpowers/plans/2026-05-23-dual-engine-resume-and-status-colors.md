# Dual Engine Resume And Status Colors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add resumable paused downloads for both `native` and `yt-dlp`, and color-code visible task states in the downloads UI.

**Architecture:** Keep task lifecycle commands unchanged and improve the engine internals behind `start_task` / group `continue`. `native` owns Range-based `.part` files under a deterministic task workspace; `yt-dlp` owns its own `--continue` partials while the app owns process lifecycle, persistence, and conservative cleanup. UI state coloring is a render/CSS-only change driven by existing task state values.

**Tech Stack:** Rust/Tauri, SQLite via `sqlx`, async `reqwest`, `tokio::process`, Vitest, vanilla TypeScript renderer, CSS.

---

## File Map

- Modify `src-tauri/src/platform/bilibili/media.rs`: add Range-aware stream download behavior and tests.
- Modify `src-tauri/src/platform/bilibili/native.rs`: use deterministic task resume workspaces, preserve partials on pause/failure, cleanup on success.
- Modify `src-tauri/src/platform/bilibili/yt_dlp.rs`: add `--continue`, JSON probe parsing, task execution adapter, and `kill_on_drop` process execution.
- Modify `src-tauri/src/platform/mod.rs`: allow generic metadata for `yt-dlp` probe results without forcing Bilibili `bvid/cid/page`.
- Modify `src-tauri/src/task/executor.rs`: pass task id/group metadata into `DownloadInput`, persist events as before.
- Modify `src-tauri/src/commands.rs`: wire `yt-dlp` create/run paths and cleanup on delete.
- Modify `src-tauri/src/models.rs` and `src-tauri/src/storage.rs`: persist task source URL only if needed for `yt-dlp` execution.
- Modify `frontend/src/render.ts`: add state class names to group and child state elements.
- Modify `frontend/src/styles.css`: add color tokens for `queued`, `downloading`, `completed`, `failed`, and related states.
- Modify `frontend/src/render.test.ts`: assert state classes are rendered.
- Modify `tasks/todo.md`: track execution and final review.

## Task 1: Add UI State Color Classes

**Files:**
- Modify: `frontend/src/render.ts`
- Modify: `frontend/src/styles.css`
- Test: `frontend/src/render.test.ts`

- [ ] **Step 1: Write failing render tests**

Add tests that assert task group and child state elements receive semantic classes:

```ts
expect(root.querySelector<HTMLElement>(".state-pill")?.classList.contains("state-failed")).toBe(true);
expect(root.querySelector<HTMLElement>(".child-state")?.classList.contains("state-failed")).toBe(true);
```

Run: `npx vitest run frontend/src/render.test.ts --pool=threads -t "state color"`

Expected: FAIL because no `state-*` classes are rendered.

- [ ] **Step 2: Add minimal render helper**

Add:

```ts
function stateClass(state: string): string {
  return `state-${state}`;
}
```

Use it where `.state-pill` and `.child-state` are created:

```ts
element("span", `state-pill ${stateClass(taskGroupState(created))}`, stateLabel(taskGroupState(created)))
element("div", `child-state ${stateClass(task.state)}`, stateLabel(task.state))
```

- [ ] **Step 3: Add CSS colors**

Add colors for:

```css
.state-queued,
.state-pending { ... }
.state-downloading,
.state-probing,
.state-merging { ... }
.state-completed { ... }
.state-failed,
.state-interrupted,
.state-cancelled { ... }
.state-paused { ... }
```

- [ ] **Step 4: Verify frontend**

Run:

```powershell
npx vitest run frontend/src/render.test.ts --pool=threads -t "state color"
npm run test -- --pool=threads
npm run build
```

Expected: all tests and build pass.

## Task 2: Make Native Stream Downloads Range-Aware

**Files:**
- Modify: `src-tauri/src/platform/bilibili/media.rs`

- [ ] **Step 1: Write failing Range tests**

Add tests for:

- Existing partial plus `206 Partial Content` appends missing bytes.
- Existing partial plus `200 OK` restarts that stream.
- Existing complete partial plus `416 Range Not Satisfiable` returns existing size.

Run: `cargo test bilibili::media::tests::resumes_stream_from_existing_partial -- --nocapture`

Expected: FAIL because `download_to_file` currently uses `File::create` and does not send `Range`.

- [ ] **Step 2: Implement `download_to_file_resumable`**

Add a new helper with this behavior:

```rust
pub async fn download_to_file_resumable(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    sink: &dyn EventSink,
) -> AppResult<u64>
```

It reads `metadata(path).len()`, sends `Range` when length is greater than zero, appends on `206`, restarts on `200`, and treats matching `416` as complete.

- [ ] **Step 3: Keep old helper stable**

Either keep `download_to_file` as a wrapper around the resumable helper or update native call sites directly. Existing non-resume tests must still pass.

- [ ] **Step 4: Verify media tests**

Run:

```powershell
cargo test bilibili::media -- --nocapture
```

Expected: pass.

## Task 3: Persist Native Resume Workspaces

**Files:**
- Modify: `src-tauri/src/platform/bilibili/native.rs`

- [ ] **Step 1: Write failing native workspace tests**

Add tests asserting:

- `native_download_temp_dir` is deterministic for the same output path and task id.
- Successful completion deletes the workspace.
- A simulated failure leaves the workspace for resume.

Expected: FAIL because the current workspace uses a fresh UUID.

- [ ] **Step 2: Extend `DownloadInput` with `task_id`**

Modify `src-tauri/src/platform/mod.rs`:

```rust
pub struct DownloadInput {
    pub task_id: String,
    pub item: DownloadItem,
    pub output_path: String,
}
```

Update `src-tauri/src/task/executor.rs` to pass `task.id.to_string()`.

- [ ] **Step 3: Use deterministic workspace**

Change native workspace to:

```rust
fn native_download_temp_dir(output_path: &Path, task_id: &str) -> PathBuf {
    output_path.parent().unwrap_or_else(|| Path::new(".")).join(".video-downloader").join(task_id)
}
```

Use `video.part` and `audio.part` instead of page-specific random files.

- [ ] **Step 4: Cleanup only on success**

Remove workspace only after merge success. If download/merge returns an error, leave it for future start/retry.

- [ ] **Step 5: Verify native tests**

Run:

```powershell
cargo test bilibili::native -- --nocapture
cargo test task::executor -- --nocapture
```

Expected: pass.

## Task 4: Wire `yt-dlp` Probe And Download Execution

**Files:**
- Modify: `src-tauri/src/platform/mod.rs`
- Modify: `src-tauri/src/platform/bilibili/yt_dlp.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Write failing yt-dlp args test**

Update `builds_download_args_with_mp4_merge_and_output_template` to require `--continue` and reject `--no-continue`.

Run: `cargo test ytdlp_download_args -- --nocapture`

Expected: FAIL until args include `--continue`.

- [ ] **Step 2: Add `--continue`**

Update `ytdlp_download_args` so the first args include:

```rust
"--newline",
"--continue",
"--merge-output-format",
"mp4",
```

- [ ] **Step 3: Persist task source URL**

Add `source_url: Option<String>` to `DownloadTask`, add nullable `source_url` column to `download_tasks`, bind/load it in storage, and populate it from the task group URL when creating tasks.

- [ ] **Step 4: Add yt-dlp probe adapter**

Implement a `YtDlpDownloader` that implements `PlatformDownloader`. `probe` runs `yt-dlp --dump-json` and maps the JSON title/output into a single `DownloadItem` with no Bilibili metadata. Initial collection expansion can stay out of scope; Bilibili native already owns collection page selection.

- [ ] **Step 5: Add yt-dlp download adapter**

`download` builds a stable output template from `input.output_path`, calls `run_ytdlp`, emits raw output lines to logs, and returns `DownloadOutput`.

- [ ] **Step 6: Wire commands**

In `create_task_from_state`, if default engine is `YtDlp`, require configured `yt-dlp.exe`, construct `YtDlpDownloader`, and call `create_task_with_downloader_from_state`.

In `run_task_from_state_mode`, if task engine is `YtDlp`, require configured `yt-dlp.exe`, construct `YtDlpDownloader`, and call `run_task_with_downloader_from_state_mode`.

- [ ] **Step 7: Verify yt-dlp targeted tests**

Run:

```powershell
cargo test yt_dlp -- --nocapture
cargo test commands::tests -- --nocapture
cargo test storage::tests -- --nocapture
```

Expected: pass.

## Task 5: Cleanup Task-Owned Resume Files

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/platform/bilibili/native.rs`
- Modify: `src-tauri/src/platform/bilibili/yt_dlp.rs`

- [ ] **Step 1: Write failing cleanup tests**

Add tests for:

- `delete_task` removes native task workspace.
- `delete_task` removes only output-path-derived `yt-dlp` partial files.

Expected: FAIL because `delete_task` currently deletes only database rows.

- [ ] **Step 2: Implement conservative cleanup helpers**

Native cleanup removes:

```text
<output-dir>/.video-downloader/<task-id>/
```

yt-dlp cleanup removes only exact output-derived paths such as:

```text
<output>.part
<output>.ytdl
<output>.temp
<output>.frag
```

Do not glob broad directories.

- [ ] **Step 3: Call cleanup from `delete_task_from_state`**

Load the task first, abort active run, cleanup by engine, then delete DB rows.

- [ ] **Step 4: Verify cleanup tests**

Run:

```powershell
cargo test delete_task -- --nocapture
```

Expected: pass.

## Task 6: Full Verification And Package Build

**Files:**
- Verify all changed files.
- Update: `tasks/todo.md`

- [ ] **Step 1: Run full gates**

Run:

```powershell
cargo fmt --check
cargo test
cargo check
cargo clippy -- -D warnings
npm run test -- --pool=threads
npm run build
git diff --check
npm run tauri:build
```

Expected: all pass.

- [ ] **Step 2: Browser check**

Start the dev server and inspect the downloads UI. Verify queued, downloading, failed, completed, and paused states have visually different colors and text remains readable.

- [ ] **Step 3: Update task review**

Update `tasks/todo.md` with the commands that passed, the package path, and any remaining limitations.
