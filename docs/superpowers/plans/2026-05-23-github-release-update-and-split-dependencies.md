# GitHub Release Update And Split Dependencies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add signed GitHub Release application updates while keeping large media dependencies outside app update packages.

**Architecture:** Tauri official updater handles app version checks, signature verification, download, install, and restart. FFmpeg and yt-dlp remain independent install-root dependencies. FFmpeg is also a required native-engine dependency, so the NSIS installer/update hook ensures it exists after install: existing complete tools are reused, missing tools are downloaded from the pinned release asset. Settings remains the explicit repair/reinstall path. The frontend talks only to local Tauri commands.

**Tech Stack:** Tauri v2, `tauri-plugin-updater`, Rust command wrappers, GitHub Releases static `latest.json`, Vitest, Rust unit tests.

---

### Task 1: Public Repository And Update Configuration

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/media.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`

- [ ] Add `tauri-plugin-updater` to Rust dependencies.
- [ ] Generate a local updater signing key outside the repo.
- [ ] Add updater `pubkey`, GitHub `latest.json` endpoint, `windows.installMode = "passive"`, and `bundle.createUpdaterArtifacts = true`.
- [ ] Remove FFmpeg zip from `bundle.resources`; keep only scripts/assets needed by the app.
- [ ] Update config guard tests so app update packages do not include `resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip`.

### Task 2: Backend Update Commands

**Files:**
- Create: `src-tauri/src/updater.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/errors.rs`

- [ ] Add `ErrorCode::UpdateError`.
- [ ] Add `AppUpdateStatus` response type.
- [ ] Add `check_app_update(app: tauri::AppHandle)`.
- [ ] Add `install_app_update(app: tauri::AppHandle)`.
- [ ] Register updater plugin and commands in `lib.rs`.
- [ ] Add Rust tests for status mapping, command registration text, and error serialization.

### Task 3: Split FFmpeg Dependency Installation

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/media.rs`
- Modify: `src-tauri/windows/hooks.nsh`
- Modify: `src-tauri/resources/install-media-tools.ps1`

- [ ] Add `install_media_tools` command that downloads `ffmpeg-win64-lgpl.zip` from GitHub Releases.
- [ ] Verify SHA256 before extraction.
- [ ] Extract into `dependencies\ffmpeg`.
- [ ] Persist `ffmpeg_path` and `ffprobe_path` in app config.
- [ ] Update NSIS hook so it conditionally ensures required FFmpeg tools after install/update: skip when `ffmpeg.exe` and `ffprobe.exe` already exist, download and verify the pinned release asset when either is missing, preserve dependency cleanup on uninstall and shortcut icon refresh.
- [ ] Add Rust tests for install path persistence, missing/empty archive failures, and hook/config guards.

### Task 4: Frontend Settings UI

**Files:**
- Modify: `frontend/src/state.ts`
- Modify: `frontend/src/api.ts`
- Modify: `frontend/src/render.ts`
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/api.test.ts`
- Modify: `frontend/src/render.test.ts`
- Modify: `frontend/src/state.test.ts`

- [ ] Add update status state.
- [ ] Add API wrappers for `check_app_update`, `install_app_update`, and `install_media_tools`.
- [ ] Render “应用更新” in Settings with current version, check button, install button, notes, and errors.
- [ ] Disable update install when unfinished active tasks exist.
- [ ] Add FFmpeg install button next to tool dependencies.
- [ ] Add Vitest coverage for API wrappers, Settings update states, dependency install refresh, and active-task blocking.

### Task 5: Verification And Publish

**Files:**
- Modify: `tasks/todo.md`

- [ ] Run targeted Rust and TS red/green tests.
- [ ] Run `cargo fmt --check`.
- [ ] Run `git diff --check`.
- [ ] Run `npm run test -- --pool=threads`.
- [ ] Run `npm run build`.
- [ ] Run `cargo check`.
- [ ] Run `cargo clippy -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Run `npm run tauri:build` with updater signing env vars.
- [ ] Commit and push to `origin/master`.
