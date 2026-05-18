# Video Downloader Tauri Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first working Rust + Tauri bilibili desktop downloader with the reviewed UI, durable task state, persisted encrypted login state, native default engine, optional `yt-dlp` fallback, and bundled `ffmpeg`/`ffprobe` detection.

**Architecture:** Use a Tauri 2 desktop shell with a vanilla TypeScript frontend. The Rust core owns domain models, SQLite persistence, task scheduling, session storage, downloader engines, media tooling, and frontend events. Start with a mock downloader path to make UI/task/storage behavior verifiable, then replace engine internals with real bilibili and tool integrations.

**Tech Stack:** Tauri 2, Rust, Tokio, SQLx SQLite, Serde, Thiserror, Reqwest, AES-GCM, Vite, TypeScript, Vitest, vanilla HTML/CSS.

---

## Source References

- Tauri official project setup: https://v2.tauri.app/start/create-project/
- Tauri official external binary sidecar docs: https://v2.tauri.app/develop/sidecar/
- Existing spec: `docs/superpowers/specs/2026-05-17-video-downloader-tauri-design.md`
- Reviewed prototype: `docs/superpowers/prototypes/video-downloader-ui/index.html`

## File Structure

Create the app at repository root:

- `package.json`: npm scripts for frontend, Tauri dev/build, tests.
- `index.html`: Tauri/Vite frontend entry.
- `frontend/src/main.ts`: frontend bootstrap and state wiring.
- `frontend/src/api.ts`: typed wrappers around Tauri commands and events.
- `frontend/src/state.ts`: client state reducer and selectors.
- `frontend/src/render.ts`: DOM rendering for Downloads/Login/Settings.
- `frontend/src/styles.css`: production CSS derived from reviewed prototype.
- `frontend/src/*.test.ts`: frontend unit tests.
- `src-tauri/Cargo.toml`: Rust crate configuration.
- `src-tauri/Cargo.lock`: Rust app dependency lockfile.
- `src-tauri/tauri.conf.json`: Tauri app config and bundled binaries.
- `src-tauri/icons/icon.ico`: minimal Windows app icon required by Tauri context generation.
- `src-tauri/capabilities/default.json`: Tauri shell/dialog permissions.
- `src-tauri/gen/`: ignored Tauri-generated schema output.
- `src-tauri/src/lib.rs`: Tauri app bootstrap.
- `src-tauri/src/main.rs`: desktop binary entrypoint.
- `src-tauri/src/models.rs`: shared domain models.
- `src-tauri/src/errors.rs`: structured error types.
- `src-tauri/src/config.rs`: app settings.
- `src-tauri/src/storage.rs`: SQLite repository and migrations.
- `src-tauri/src/task/mod.rs`: task manager public API.
- `src-tauri/src/task/queue.rs`: concurrency and retry loop.
- `src-tauri/src/task/events.rs`: event sink and Tauri event bridge.
- `src-tauri/src/media.rs`: output paths, filename sanitization, `ffmpeg`/`ffprobe`.
- `src-tauri/src/auth/bilibili.rs`: QR login and session validation.
- `src-tauri/src/auth/session_store.rs`: encrypted session file storage.
- `src-tauri/src/platform/mod.rs`: downloader trait and engine registry.
- `src-tauri/src/platform/mock.rs`: deterministic mock downloader for UI and task tests.
- `src-tauri/src/platform/bilibili/native.rs`: native bilibili implementation.
- `src-tauri/src/platform/bilibili/yt_dlp.rs`: optional `yt-dlp` adapter.
- `src-tauri/src/commands.rs`: Tauri command handlers.
- `src-tauri/tests/*.rs`: Rust integration tests.
- `src-tauri/binaries/README.md`: expected bundled binary names and license notes.

## Task 1: Scaffold Tauri App Skeleton

**Files:**
- Create: `package.json`
- Create: `index.html`
- Create: `frontend/src/main.ts`
- Create: `frontend/src/styles.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/Cargo.lock`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/icons/icon.ico`
- Create: `src-tauri/capabilities/default.json`
- Modify: `.gitignore`

**Support files:**
- Track `src-tauri/Cargo.lock` because this is an application crate and the lockfile makes Rust dependency resolution reproducible.
- Track `src-tauri/icons/icon.ico` so Windows Tauri context generation has a valid icon asset during `cargo check`.
- Ignore `src-tauri/gen/`; it is generated schema output recreated by Tauri/Cargo and is not part of the scaffold source.

- [ ] **Step 1: Create package metadata and scripts**

Create `package.json`:

```json
{
  "name": "video-downloader",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1",
    "build": "tsc && vite build",
    "test": "vitest run",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  },
  "dependencies": {
    "@tauri-apps/api": "latest"
  },
  "devDependencies": {
    "@tauri-apps/cli": "latest",
    "typescript": "latest",
    "vite": "latest",
    "vitest": "latest",
    "jsdom": "latest"
  }
}
```

- [ ] **Step 2: Create frontend entry files**

Create `index.html`:

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Video Downloader</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/frontend/src/main.ts"></script>
  </body>
</html>
```

Create `frontend/src/main.ts`:

```ts
import "./styles.css";

const root = document.querySelector<HTMLDivElement>("#app");
if (!root) {
  throw new Error("Missing #app root");
}

root.innerHTML = `<main class="boot-screen"><h1>Video Downloader</h1><p>应用初始化中</p></main>`;
```

Create `frontend/src/styles.css`:

```css
:root {
  font-family: Inter, "Segoe UI", Arial, sans-serif;
  color: #1f2937;
  background: #f5f7fb;
  letter-spacing: 0;
}

body {
  margin: 0;
}

.boot-screen {
  min-height: 100vh;
  display: grid;
  place-content: center;
  gap: 8px;
  text-align: center;
}
```

- [ ] **Step 3: Create Rust crate**

Create `src-tauri/Cargo.toml`:

```toml
[package]
name = "video-downloader"
version = "0.1.0"
description = "Local desktop video downloader"
authors = ["video-downloader"]
edition = "2021"

[lib]
name = "video_downloader"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "sync", "time"] }
thiserror = "2"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde", "clock"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls", "chrono", "uuid"] }
reqwest = { version = "0.12", features = ["json", "cookies", "rustls-tls"] }
aes-gcm = "0.10"
rand = "0.8"
base64 = "0.22"
directories = "5"
sanitize-filename = "0.6"
```

Create `src-tauri/src/main.rs`:

```rust
fn main() {
    video_downloader::run();
}
```

Create `src-tauri/src/lib.rs`:

```rust
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}
```

- [ ] **Step 4: Create Tauri config**

Create `src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Video Downloader",
  "version": "0.1.0",
  "identifier": "local.video-downloader.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://127.0.0.1:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Video Downloader",
        "width": 1100,
        "height": 760,
        "minWidth": 520,
        "minHeight": 640
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": "all"
  }
}
```

Create `src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default desktop capability",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```

- [ ] **Step 5: Install dependencies**

Run:

```powershell
npm install
```

Expected: npm installs dependencies and creates `package-lock.json`.

- [ ] **Step 6: Verify scaffold builds**

Run:

```powershell
npm run build
cd src-tauri
cargo check
```

Expected: frontend build succeeds and Rust crate type-checks.

- [ ] **Step 7: Commit scaffold**

```powershell
git add .gitignore package.json package-lock.json index.html frontend src-tauri
git commit -m "feat: scaffold tauri desktop app"
```

## Task 2: Define Domain Models And Errors

**Files:**
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/errors.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/models.rs`
- Test: `src-tauri/src/errors.rs`

- [ ] **Step 1: Add model tests first**

Create `src-tauri/src/models.rs` with tests and minimal model declarations:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadEngine {
    Native,
    YtDlp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Pending,
    Probing,
    Queued,
    Downloading,
    Merging,
    Completed,
    Failed,
    Interrupted,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskGroup {
    pub id: Uuid,
    pub source_url: String,
    pub platform: String,
    pub title: String,
    pub output_dir: String,
    pub engine: DownloadEngine,
    pub state: TaskState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadTask {
    pub id: Uuid,
    pub group_id: Uuid,
    pub title: String,
    pub output_file: String,
    pub state: TaskState,
    pub engine: DownloadEngine,
    pub quality: Option<String>,
    pub used_login: bool,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub retry_count: u8,
    pub max_retries: u8,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub download_root: String,
    pub concurrency: u8,
    pub default_engine: DownloadEngine,
    pub ytdlp_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_root: String::from("D:\\Videos"),
            concurrency: 2,
            default_engine: DownloadEngine::Native,
            ytdlp_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_defaults_match_spec() {
        let config = AppConfig::default();
        assert_eq!(config.concurrency, 2);
        assert_eq!(config.default_engine, DownloadEngine::Native);
        assert!(config.ytdlp_path.is_none());
    }

    #[test]
    fn task_state_serializes_as_snake_case() {
        let json = serde_json::to_string(&TaskState::Downloading).unwrap();
        assert_eq!(json, "\"downloading\"");
    }
}
```

- [ ] **Step 2: Add structured errors**

Create `src-tauri/src/errors.rs`:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NetworkError,
    LoginRequired,
    LoginExpired,
    PermissionDenied,
    UnsupportedContent,
    EngineMissing,
    FfmpegError,
    FilesystemError,
    PlatformChanged,
    UnknownError,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{code:?}: {message}")]
    Structured { code: ErrorCode, message: String },
}

impl AppError {
    pub fn structured(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Structured {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Structured { code, .. } => *code,
        }
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AppError::Structured { code, message } => {
                #[derive(Serialize)]
                struct WireError<'a> {
                    code: ErrorCode,
                    message: &'a str,
                }
                WireError { code: *code, message }.serialize(serializer)
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_error_code_for_frontend() {
        let err = AppError::structured(ErrorCode::UnsupportedContent, "native cannot expand this link");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("unsupported_content"));
        assert!(json.contains("native cannot expand this link"));
    }
}
```

- [ ] **Step 3: Export modules**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod errors;
pub mod models;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}
```

- [ ] **Step 4: Run tests**

Run:

```powershell
cd src-tauri
cargo test models
cargo test errors
```

Expected: all model and error tests pass.

- [ ] **Step 5: Commit models**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/models.rs src-tauri/src/errors.rs
git commit -m "feat: add downloader domain models"
```

## Task 3: Implement SQLite Storage And Settings

**Files:**
- Create: `src-tauri/src/storage.rs`
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/storage.rs`

- [ ] **Step 1: Write storage repository with migration tests**

Create `src-tauri/src/storage.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::models::{AppConfig, DownloadEngine, DownloadTask, TaskGroup, TaskState};
use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn open(database_url: &str) -> AppResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    async fn migrate(&self) -> AppResult<()> {
        let statements = [
            "CREATE TABLE IF NOT EXISTS app_config (id INTEGER PRIMARY KEY CHECK (id = 1), download_root TEXT NOT NULL, concurrency INTEGER NOT NULL, default_engine TEXT NOT NULL, ytdlp_path TEXT)",
            "CREATE TABLE IF NOT EXISTS task_groups (id TEXT PRIMARY KEY, source_url TEXT NOT NULL, platform TEXT NOT NULL, title TEXT NOT NULL, output_dir TEXT NOT NULL, engine TEXT NOT NULL, state TEXT NOT NULL, created_at TEXT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS download_tasks (id TEXT PRIMARY KEY, group_id TEXT NOT NULL, title TEXT NOT NULL, output_file TEXT NOT NULL, state TEXT NOT NULL, engine TEXT NOT NULL, quality TEXT, used_login INTEGER NOT NULL, bytes_downloaded INTEGER NOT NULL, bytes_total INTEGER, retry_count INTEGER NOT NULL, max_retries INTEGER NOT NULL, error_code TEXT, error_message TEXT)",
            "CREATE TABLE IF NOT EXISTS task_logs (id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, created_at TEXT NOT NULL, line TEXT NOT NULL)"
        ];
        for statement in statements {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        }
        Ok(())
    }

    pub async fn load_config(&self) -> AppResult<AppConfig> {
        let row = sqlx::query("SELECT download_root, concurrency, default_engine, ytdlp_path FROM app_config WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let Some(row) = row else {
            let config = AppConfig::default();
            self.save_config(&config).await?;
            return Ok(config);
        };
        Ok(AppConfig {
            download_root: row.get::<String, _>("download_root"),
            concurrency: row.get::<i64, _>("concurrency") as u8,
            default_engine: parse_engine(&row.get::<String, _>("default_engine")),
            ytdlp_path: row.get::<Option<String>, _>("ytdlp_path"),
        })
    }

    pub async fn save_config(&self, config: &AppConfig) -> AppResult<()> {
        sqlx::query("INSERT INTO app_config (id, download_root, concurrency, default_engine, ytdlp_path) VALUES (1, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET download_root = excluded.download_root, concurrency = excluded.concurrency, default_engine = excluded.default_engine, ytdlp_path = excluded.ytdlp_path")
            .bind(&config.download_root)
            .bind(config.concurrency as i64)
            .bind(engine_name(config.default_engine))
            .bind(&config.ytdlp_path)
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(())
    }

    pub async fn insert_group(&self, group: &TaskGroup) -> AppResult<()> {
        sqlx::query("INSERT INTO task_groups (id, source_url, platform, title, output_dir, engine, state, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(group.id.to_string())
            .bind(&group.source_url)
            .bind(&group.platform)
            .bind(&group.title)
            .bind(&group.output_dir)
            .bind(engine_name(group.engine))
            .bind(state_name(group.state))
            .bind(group.created_at.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(())
    }

    pub async fn insert_task(&self, task: &DownloadTask) -> AppResult<()> {
        sqlx::query("INSERT INTO download_tasks (id, group_id, title, output_file, state, engine, quality, used_login, bytes_downloaded, bytes_total, retry_count, max_retries, error_code, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(task.id.to_string())
            .bind(task.group_id.to_string())
            .bind(&task.title)
            .bind(&task.output_file)
            .bind(state_name(task.state))
            .bind(engine_name(task.engine))
            .bind(&task.quality)
            .bind(if task.used_login { 1_i64 } else { 0_i64 })
            .bind(task.bytes_downloaded as i64)
            .bind(task.bytes_total.map(|v| v as i64))
            .bind(task.retry_count as i64)
            .bind(task.max_retries as i64)
            .bind(&task.error_code)
            .bind(&task.error_message)
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(())
    }

    pub async fn append_log(&self, task_id: Uuid, line: &str) -> AppResult<()> {
        sqlx::query("INSERT INTO task_logs (task_id, created_at, line) VALUES (?, ?, ?)")
            .bind(task_id.to_string())
            .bind(Utc::now().to_rfc3339())
            .bind(line)
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(())
    }
}

fn engine_name(engine: DownloadEngine) -> &'static str {
    match engine {
        DownloadEngine::Native => "native",
        DownloadEngine::YtDlp => "yt_dlp",
    }
}

fn parse_engine(value: &str) -> DownloadEngine {
    match value {
        "yt_dlp" => DownloadEngine::YtDlp,
        _ => DownloadEngine::Native,
    }
}

fn state_name(state: TaskState) -> &'static str {
    match state {
        TaskState::Pending => "pending",
        TaskState::Probing => "probing",
        TaskState::Queued => "queued",
        TaskState::Downloading => "downloading",
        TaskState::Merging => "merging",
        TaskState::Completed => "completed",
        TaskState::Failed => "failed",
        TaskState::Interrupted => "interrupted",
        TaskState::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn creates_default_config_when_missing() {
        let db = TestDatabase::open().await;
        let storage = &db.storage;
        let config = storage.load_config().await.unwrap();
        assert_eq!(config.default_engine, DownloadEngine::Native);
        assert_eq!(config.concurrency, 2);
        db.close().await;
    }

    #[tokio::test]
    async fn stores_group_task_and_log() {
        let db = TestDatabase::open().await;
        let storage = &db.storage;
        let group = TaskGroup {
            id: Uuid::new_v4(),
            source_url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            platform: "bilibili".into(),
            title: "sample".into(),
            output_dir: "D:\\Videos\\bilibili".into(),
            engine: DownloadEngine::Native,
            state: TaskState::Queued,
            created_at: Utc::now(),
        };
        storage.insert_group(&group).await.unwrap();
        let task = DownloadTask {
            id: Uuid::new_v4(),
            group_id: group.id,
            title: "01 - sample".into(),
            output_file: "01 - sample.mp4".into(),
            state: TaskState::Queued,
            engine: DownloadEngine::Native,
            quality: None,
            used_login: false,
            bytes_downloaded: 0,
            bytes_total: None,
            retry_count: 0,
            max_retries: 3,
            error_code: None,
            error_message: None,
        };
        storage.insert_task(&task).await.unwrap();
        storage.append_log(task.id, "[task] queued").await.unwrap();
        db.close().await;
    }

    struct TestDatabase {
        storage: Storage,
        path: PathBuf,
    }

    impl TestDatabase {
        async fn open() -> Self {
            let path =
                std::env::temp_dir().join(format!("video-downloader-{}.sqlite", Uuid::new_v4()));
            let database_url = format!(
                "sqlite://{}?mode=rwc",
                path.to_string_lossy().replace('\\', "/")
            );
            let storage = Storage::open(&database_url).await.unwrap();

            Self { storage, path }
        }

        async fn close(self) {
            let path = self.path;
            self.storage.pool.close().await;
            drop(self.storage);
            let _ = std::fs::remove_file(path);
        }
    }
}
```

- [ ] **Step 2: Add module exports**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod config;
pub mod errors;
pub mod models;
pub mod storage;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}
```

Create `src-tauri/src/config.rs`:

```rust
use crate::models::AppConfig;

pub fn normalize_concurrency(value: u8) -> u8 {
    value.clamp(1, 8)
}

pub fn with_normalized_concurrency(mut config: AppConfig) -> AppConfig {
    config.concurrency = normalize_concurrency(config.concurrency);
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_concurrency() {
        assert_eq!(normalize_concurrency(0), 1);
        assert_eq!(normalize_concurrency(2), 2);
        assert_eq!(normalize_concurrency(99), 8);
    }
}
```

- [ ] **Step 3: Run storage tests**

Run:

```powershell
cd src-tauri
cargo test storage
cargo test config
```

Expected: storage and config tests pass.

- [ ] **Step 4: Commit storage**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/storage.rs src-tauri/src/config.rs
git commit -m "feat: add sqlite storage and app settings"
```

## Task 4: Implement File Output And Media Tool Detection

**Files:**
- Create: `src-tauri/src/media.rs`
- Create: `src-tauri/binaries/README.md`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/media.rs`

- [ ] **Step 1: Add path and filename tests**

Create `src-tauri/src/media.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use sanitize_filename::sanitize;
use std::path::{Path, PathBuf};

pub fn sanitize_title(input: &str) -> String {
    let cleaned = sanitize(input).trim().to_string();
    if cleaned.is_empty() {
        "untitled".to_string()
    } else if cleaned.chars().count() > 120 {
        cleaned.chars().take(120).collect()
    } else {
        cleaned
    }
}

pub fn output_path(root: &Path, platform: &str, collection: &str, index: Option<u32>, title: &str) -> PathBuf {
    let mut filename = match index {
        Some(value) => format!("{value:02} - {}", sanitize_title(title)),
        None => sanitize_title(title),
    };
    filename.push_str(".mp4");
    root.join(sanitize_title(platform))
        .join(sanitize_title(collection))
        .join(filename)
}

pub fn ensure_directory(path: &Path) -> AppResult<()> {
    std::fs::create_dir_all(path)
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))
}

pub fn first_available_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or("video");
    let ext = path.extension().and_then(|v| v.to_str()).unwrap_or("mp4");
    for index in 1..10_000 {
        let candidate = parent.join(format!("{stem} ({index}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem} (10000).{ext}"))
}

pub fn expected_sidecar_names() -> [&'static str; 2] {
    ["ffmpeg", "ffprobe"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_platform_collection_output_path() {
        let path = output_path(Path::new("D:\\Videos"), "bilibili", "合集:Rust?", Some(1), "安装/Tauri");
        let text = path.to_string_lossy();
        assert!(text.contains("bilibili"));
        assert!(text.contains("合集Rust"));
        assert!(text.ends_with("01 - 安装Tauri.mp4"));
    }

    #[test]
    fn empty_title_becomes_untitled() {
        assert_eq!(sanitize_title("::::"), "untitled");
    }
}
```

- [ ] **Step 2: Add binary documentation**

Create `src-tauri/binaries/README.md`:

```markdown
# Bundled Media Binaries

The app expects platform-specific Tauri sidecar binaries for:

- `ffmpeg`
- `ffprobe`

For each target, Tauri requires the binary filename to include the target triple suffix, for example:

- `ffmpeg-x86_64-pc-windows-msvc.exe`
- `ffprobe-x86_64-pc-windows-msvc.exe`

Run `rustc --print host-tuple` to discover the current host tuple. Public distribution must verify the bundled FFmpeg license profile before release.
```

- [ ] **Step 3: Export media module**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod storage;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}
```

- [ ] **Step 4: Run tests**

```powershell
cd src-tauri
cargo test media
```

Expected: output path tests pass.

- [ ] **Step 5: Commit media utilities**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/media.rs src-tauri/binaries/README.md
git commit -m "feat: add media output path utilities"
```

## Task 5: Add Downloader Interface And Mock Engine

**Files:**
- Create: `src-tauri/src/platform/mod.rs`
- Create: `src-tauri/src/platform/mock.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/platform/mock.rs`

- [ ] **Step 1: Define downloader trait and data**

Create `src-tauri/src/platform/mod.rs`:

```rust
pub mod mock;

use crate::errors::AppResult;
use crate::models::DownloadEngine;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeInput {
    pub url: String,
    pub engine: DownloadEngine,
    pub has_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadItem {
    pub title: String,
    pub output_file: String,
    pub quality: Option<String>,
    pub requires_login: bool,
    pub bytes_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeResult {
    pub group_title: String,
    pub items: Vec<DownloadItem>,
    pub used_login: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadInput {
    pub item: DownloadItem,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadOutput {
    pub output_path: String,
    pub quality: Option<String>,
    pub used_login: bool,
    pub bytes_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadEvent {
    Log(String),
    Progress { downloaded: u64, total: Option<u64> },
    State(String),
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: DownloadEvent);
}

pub trait PlatformDownloader: Send + Sync {
    fn probe<'a>(&'a self, input: ProbeInput) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>>;
    fn download<'a>(&'a self, input: DownloadInput, sink: &'a dyn EventSink) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>>;
}
```

- [ ] **Step 2: Add deterministic mock engine**

Create `src-tauri/src/platform/mock.rs`:

```rust
use super::{DownloadEvent, DownloadInput, DownloadItem, DownloadOutput, EventSink, PlatformDownloader, ProbeInput, ProbeResult};
use crate::errors::AppResult;
use std::future::Future;
use std::pin::Pin;

#[derive(Default)]
pub struct MockDownloader;

impl PlatformDownloader for MockDownloader {
    fn probe<'a>(&'a self, input: ProbeInput) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let is_collection = input.url.contains("collection") || input.url.contains("BV1xx411c7mD");
            let items = if is_collection {
                vec![
                    DownloadItem { title: "01 - 安装 Tauri".into(), output_file: "01 - 安装 Tauri.mp4".into(), quality: Some("1080P".into()), requires_login: true, bytes_total: Some(1_200_000_000) },
                    DownloadItem { title: "02 - Rust 命令与事件".into(), output_file: "02 - Rust 命令与事件.mp4".into(), quality: Some("1080P".into()), requires_login: true, bytes_total: Some(800_000_000) },
                    DownloadItem { title: "03 - 打包与发布".into(), output_file: "03 - 打包与发布.mp4".into(), quality: Some("720P".into()), requires_login: false, bytes_total: Some(384_000_000) },
                ]
            } else {
                vec![DownloadItem { title: "B站下载链路测试".into(), output_file: "B站下载链路测试.mp4".into(), quality: Some("720P".into()), requires_login: false, bytes_total: Some(384_000_000) }]
            };
            Ok(ProbeResult {
                group_title: if is_collection { "Rust 桌面应用入门".into() } else { items[0].title.clone() },
                used_login: input.has_login,
                items,
            })
        })
    }

    fn download<'a>(&'a self, input: DownloadInput, sink: &'a dyn EventSink) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
        Box::pin(async move {
            sink.emit(DownloadEvent::Log(format!("[mock] downloading {}", input.item.title)));
            sink.emit(DownloadEvent::Progress { downloaded: input.item.bytes_total.unwrap_or(1), total: input.item.bytes_total });
            sink.emit(DownloadEvent::State("completed".into()));
            Ok(DownloadOutput {
                output_path: input.output_path,
                quality: input.item.quality,
                used_login: input.item.requires_login,
                bytes_total: input.item.bytes_total,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadEngine;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct VecSink(Arc<Mutex<Vec<DownloadEvent>>>);

    impl EventSink for VecSink {
        fn emit(&self, event: DownloadEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[tokio::test]
    async fn expands_collection_sample() {
        let downloader = MockDownloader;
        let result = downloader.probe(ProbeInput {
            url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            engine: DownloadEngine::Native,
            has_login: true,
        }).await.unwrap();
        assert_eq!(result.items.len(), 3);
        assert_eq!(result.items[0].output_file, "01 - 安装 Tauri.mp4");
    }

    #[tokio::test]
    async fn emits_download_events() {
        let downloader = MockDownloader;
        let item = DownloadItem {
            title: "sample".into(),
            output_file: "sample.mp4".into(),
            quality: Some("720P".into()),
            requires_login: false,
            bytes_total: Some(10),
        };
        let sink = VecSink::default();
        let output = downloader.download(DownloadInput { item, output_path: "D:\\Videos\\sample.mp4".into() }, &sink).await.unwrap();
        assert_eq!(output.output_path, "D:\\Videos\\sample.mp4");
        assert!(!sink.0.lock().unwrap().is_empty());
    }
}
```

- [ ] **Step 3: Export platform module**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod platform;
pub mod storage;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}
```

- [ ] **Step 4: Run platform tests**

```powershell
cd src-tauri
cargo test platform
```

Expected: mock downloader tests pass.

- [ ] **Step 5: Commit downloader interface**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/platform
git commit -m "feat: add downloader interface and mock engine"
```

## Task 6: Add Task Manager With Queue, Retry, And Logs

**Files:**
- Create: `src-tauri/src/task/mod.rs`
- Create: `src-tauri/src/task/events.rs`
- Create: `src-tauri/src/task/queue.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/task/mod.rs`

- [ ] **Step 1: Add task event sink**

Create `src-tauri/src/task/events.rs`:

```rust
use crate::platform::{DownloadEvent, EventSink};
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct MemoryEventSink {
    events: Arc<Mutex<Vec<DownloadEvent>>>,
}

impl MemoryEventSink {
    pub fn events(&self) -> Vec<DownloadEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl EventSink for MemoryEventSink {
    fn emit(&self, event: DownloadEvent) {
        self.events.lock().unwrap().push(event);
    }
}
```

- [ ] **Step 2: Add task manager API**

Create `src-tauri/src/task/mod.rs`:

```rust
pub mod events;
pub mod queue;

use crate::errors::AppResult;
use crate::media::output_path;
use crate::models::{DownloadEngine, DownloadTask, TaskGroup, TaskState};
use crate::platform::{PlatformDownloader, ProbeInput};
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateTaskRequest {
    pub url: String,
    pub output_dir: String,
    pub engine: DownloadEngine,
    pub has_login: bool,
}

#[derive(Debug, Clone)]
pub struct CreatedTaskGroup {
    pub group: TaskGroup,
    pub tasks: Vec<DownloadTask>,
}

pub async fn create_group_from_probe(
    downloader: &dyn PlatformDownloader,
    request: CreateTaskRequest,
) -> AppResult<CreatedTaskGroup> {
    let probe = downloader.probe(ProbeInput {
        url: request.url.clone(),
        engine: request.engine,
        has_login: request.has_login,
    }).await?;
    let group_id = Uuid::new_v4();
    let group = TaskGroup {
        id: group_id,
        source_url: request.url,
        platform: "bilibili".into(),
        title: probe.group_title.clone(),
        output_dir: request.output_dir.clone(),
        engine: request.engine,
        state: TaskState::Queued,
        created_at: Utc::now(),
    };
    let is_collection = probe.items.len() > 1;
    let tasks = probe.items.into_iter().enumerate().map(|(idx, item)| {
        let output_title = if is_collection {
            strip_leading_numeric_prefix(&item.title)
        } else {
            item.title.as_str()
        };
        let path = output_path(
            Path::new(&request.output_dir),
            "bilibili",
            &group.title,
            is_collection.then_some((idx + 1) as u32),
            output_title,
        );
        DownloadTask {
            id: Uuid::new_v4(),
            group_id,
            title: item.title,
            output_file: path.to_string_lossy().to_string(),
            state: TaskState::Queued,
            engine: request.engine,
            quality: item.quality,
            used_login: item.requires_login,
            bytes_downloaded: 0,
            bytes_total: item.bytes_total,
            retry_count: 0,
            max_retries: 3,
            error_code: None,
            error_message: None,
        }
    }).collect();
    Ok(CreatedTaskGroup { group, tasks })
}

fn strip_leading_numeric_prefix(title: &str) -> &str {
    if let Some((prefix, rest)) = title.split_once(" - ") {
        if prefix.len() == 2 && prefix.chars().all(|value| value.is_ascii_digit()) {
            return rest;
        }
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::mock::MockDownloader;

    #[tokio::test]
    async fn creates_group_with_child_tasks() {
        let result = create_group_from_probe(&MockDownloader, CreateTaskRequest {
            url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            output_dir: "D:\\Videos".into(),
            engine: DownloadEngine::Native,
            has_login: true,
        }).await.unwrap();
        assert_eq!(result.group.platform, "bilibili");
        assert_eq!(result.tasks.len(), 3);
        assert!(result.tasks[0].output_file.contains("01 - 安装 Tauri.mp4"));
        assert!(!result.tasks[0].output_file.contains("01 - 01"));
    }
}
```

- [ ] **Step 3: Add queue policy**

Create `src-tauri/src/task/queue.rs`:

```rust
use crate::models::DownloadTask;

pub fn should_auto_retry(task: &DownloadTask) -> bool {
    matches!(task.error_code.as_deref(), Some("network_error")) && task.retry_count < task.max_retries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DownloadEngine, TaskState};
    use uuid::Uuid;

    fn failed_task(code: &str, retry_count: u8) -> DownloadTask {
        DownloadTask {
            id: Uuid::new_v4(),
            group_id: Uuid::new_v4(),
            title: "sample".into(),
            output_file: "sample.mp4".into(),
            state: TaskState::Failed,
            engine: DownloadEngine::Native,
            quality: None,
            used_login: false,
            bytes_downloaded: 0,
            bytes_total: None,
            retry_count,
            max_retries: 3,
            error_code: Some(code.into()),
            error_message: Some("failed".into()),
        }
    }

    #[test]
    fn retries_only_network_errors_under_limit() {
        assert!(should_auto_retry(&failed_task("network_error", 2)));
        assert!(!should_auto_retry(&failed_task("network_error", 3)));
        assert!(!should_auto_retry(&failed_task("login_required", 0)));
    }
}
```

- [ ] **Step 4: Export task module**

Modify `src-tauri/src/lib.rs` and add `pub mod task;`.

- [ ] **Step 5: Run task tests**

```powershell
cd src-tauri
cargo test task
```

Expected: task creation and retry policy tests pass.

- [ ] **Step 6: Commit task manager**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/task
git commit -m "feat: add task group creation and retry policy"
```

## Task 7: Add Tauri Commands Backed By Mock Engine

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/task/mod.rs`
- Test: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add command response types and handlers**

Create `src-tauri/src/commands.rs`:

```rust
use crate::errors::AppResult;
use crate::models::{AppConfig, DownloadEngine};
use crate::platform::mock::MockDownloader;
use crate::task::{create_group_from_probe, CreateTaskRequest, CreatedTaskGroup};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskCommand {
    pub url: String,
    pub output_dir: String,
    pub has_login: bool,
}

#[tauri::command]
pub fn get_config() -> AppResult<AppConfig> {
    Ok(AppConfig::default())
}

#[tauri::command]
pub async fn create_task(input: CreateTaskCommand) -> AppResult<CreatedTaskGroup> {
    create_group_from_probe(&MockDownloader, CreateTaskRequest {
        url: input.url,
        output_dir: input.output_dir,
        engine: DownloadEngine::Native,
        has_login: input.has_login,
    }).await
}

#[tauri::command]
pub fn list_platform_logins() -> AppResult<Vec<PlatformLoginRow>> {
    Ok(vec![PlatformLoginRow {
        platform: "bilibili".into(),
        status: "未登录".into(),
    }])
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformLoginRow {
    pub platform: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_task_uses_mock_collection() {
        let result = create_task(CreateTaskCommand {
            url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            output_dir: "D:\\Videos".into(),
            has_login: true,
        }).await.unwrap();
        assert_eq!(result.tasks.len(), 3);
        let first_file_name = std::path::Path::new(&result.tasks[0].output_file)
            .file_name()
            .unwrap()
            .to_string_lossy();
        assert_eq!(first_file_name, "01 - 安装 Tauri.mp4");
    }

    #[test]
    fn exposes_flat_platform_login_rows() {
        let rows = list_platform_logins().unwrap();
        assert_eq!(
            rows,
            vec![PlatformLoginRow {
                platform: "bilibili".into(),
                status: "未登录".into(),
            }]
        );
    }

    #[test]
    fn get_config_returns_native_default() {
        let config = get_config().unwrap();
        assert_eq!(config.default_engine, DownloadEngine::Native);
        assert_eq!(config.concurrency, 2);
    }
}
```

- [ ] **Step 2: Add command return serialization**

Modify `src-tauri/src/task/mod.rs` so the command return DTO can be serialized by Tauri:

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CreatedTaskGroup {
    pub group: TaskGroup,
    pub tasks: Vec<DownloadTask>,
}
```

- [ ] **Step 3: Register commands**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod commands;
pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod platform;
pub mod storage;
pub mod task;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::create_task,
            commands::list_platform_logins
        ])
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}
```

- [ ] **Step 4: Run command tests**

```powershell
cd src-tauri
cargo test commands
```

Expected: command tests pass.

- [ ] **Step 4: Commit command layer**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat: expose initial tauri commands"
```

## Task 8: Implement Reviewed Frontend UI Against Commands

**Files:**
- Create: `frontend/src/api.ts`
- Create: `frontend/src/state.ts`
- Create: `frontend/src/render.ts`
- Modify: `frontend/src/main.ts`
- Replace: `frontend/src/styles.css`
- Test: `frontend/src/state.test.ts`
- Test: `frontend/src/render.test.ts`

- [ ] **Step 1: Add state tests**

Create `frontend/src/state.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { createInitialState, platformRowText, taskOutputDirectory } from "./state";

describe("state helpers", () => {
  it("uses native as default engine", () => {
    expect(createInitialState().settings.defaultEngine).toBe("native");
  });

  it("prefills task output directory from default root", () => {
    expect(taskOutputDirectory("D:\\Videos")).toBe("D:\\Videos\\bilibili");
  });

  it("keeps platform login row flat", () => {
    expect(platformRowText({ platform: "bilibili", status: "未登录" })).toEqual(["bilibili", "未登录"]);
  });
});
```

- [ ] **Step 2: Implement frontend state**

Create `frontend/src/state.ts`:

```ts
export type Engine = "native" | "yt-dlp";

export interface SettingsState {
  downloadRoot: string;
  concurrency: number;
  defaultEngine: Engine;
}

export interface PlatformLoginRow {
  platform: string;
  status: "未登录" | "已登录";
}

export interface UiState {
  activeTab: "downloads" | "login" | "settings";
  settings: SettingsState;
  platformLogins: PlatformLoginRow[];
}

export function createInitialState(): UiState {
  return {
    activeTab: "downloads",
    settings: {
      downloadRoot: "D:\\Videos",
      concurrency: 2,
      defaultEngine: "native"
    },
    platformLogins: [{ platform: "bilibili", status: "未登录" }]
  };
}

export function taskOutputDirectory(downloadRoot: string): string {
  return `${downloadRoot.replace(/[\\\/]$/, "")}\\bilibili`;
}

export function platformRowText(row: PlatformLoginRow): [string, string] {
  return [row.platform, row.status];
}
```

- [ ] **Step 3: Add Tauri API wrapper**

Create `frontend/src/api.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";

export interface CreateTaskInput {
  url: string;
  output_dir: string;
  has_login: boolean;
}

export async function createTask(input: CreateTaskInput): Promise<unknown> {
  return invoke("create_task", { input });
}

export async function getConfig(): Promise<unknown> {
  return invoke("get_config");
}

export async function listPlatformLogins(): Promise<unknown> {
  return invoke("list_platform_logins");
}
```

- [ ] **Step 4: Implement render shell from prototype**

Create `frontend/src/render.ts` using the reviewed prototype structure:

```ts
import { createInitialState, taskOutputDirectory, UiState } from "./state";

export function renderApp(root: HTMLElement, state: UiState = createInitialState()): void {
  root.innerHTML = `
    <div class="shell">
      <aside class="sidebar">
        <div class="brand"><div class="brand-mark">VD</div><div>Video Downloader</div></div>
        <nav class="nav" aria-label="主导航">
          <button class="active" data-tab="downloads" type="button">下载任务</button>
          <button data-tab="login" type="button">登录状态</button>
          <button data-tab="settings" type="button">设置</button>
        </nav>
      </aside>
      <main class="main">
        <section id="downloads" class="view active">
          <div class="page-head"><div><h1>下载任务</h1><div class="subtle">输入 bilibili 链接后创建任务。内核选择统一在设置中管理。</div></div></div>
          <div class="toolbar">
            <div class="link-form">
              <div class="field"><label for="videoUrl">视频链接</label><input id="videoUrl" value="https://www.bilibili.com/video/BV1xx411c7mD"></div>
              <div class="field"><label for="outputDir">输出目录</label><div class="path-picker"><input id="outputDir" value="${taskOutputDirectory(state.settings.downloadRoot)}"><button id="selectOutputDir" type="button">选择</button></div></div>
              <button id="addTask" class="primary" type="button">添加下载</button>
            </div>
          </div>
          <div id="taskList" class="task-list"></div>
        </section>
        <section id="login" class="view">
          <div class="page-head"><div><h1>登录状态</h1><div class="subtle">列表展示各个平台的登录状态，点击平台行查看详情。</div></div></div>
          <div class="panel"><h2>平台登录状态</h2><div class="platform-list">${state.platformLogins.map((row) => `<article class="platform-card"><div class="platform-summary"><div class="platform-title"><strong>${row.platform}</strong></div><span class="pill amber">${row.status}</span></div><div class="platform-detail"><div class="login-layout"><div class="qr">QR</div><div class="settings-grid"><div class="detail-item"><span>保存方式</span><strong>本地加密文件</strong></div><button class="primary" type="button">扫码登录</button><button type="button">重新验证</button><button class="danger" type="button">清除登录态</button></div></div></div></article>`).join("")}</div></div>
        </section>
        <section id="settings" class="view">
          <div class="page-head"><div><h1>设置</h1><div class="subtle">下载根目录、并发数和默认内核在这里统一配置。</div></div></div>
          <div class="panel"><div class="settings-grid"><div class="setting-row"><label for="downloadRoot">默认下载根目录</label><input id="downloadRoot" value="${state.settings.downloadRoot}"></div><div class="setting-row"><label for="concurrency">并发数</label><input id="concurrency" type="number" min="1" max="8" value="${state.settings.concurrency}"></div><div class="setting-row"><label>默认内核</label><div class="segmented"><button class="active" type="button">native</button><button type="button">yt-dlp</button></div></div></div></div>
        </section>
      </main>
    </div>`;
}
```

- [ ] **Step 5: Wire frontend bootstrap**

Modify `frontend/src/main.ts`:

```ts
import "./styles.css";
import { renderApp } from "./render";
import { createInitialState } from "./state";

const root = document.querySelector<HTMLElement>("#app");
if (!root) {
  throw new Error("Missing #app root");
}

renderApp(root, createInitialState());

root.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const tabButton = target.closest<HTMLButtonElement>("[data-tab]");
  if (tabButton) {
    root.querySelectorAll("[data-tab]").forEach((button) => button.classList.remove("active"));
    root.querySelectorAll(".view").forEach((view) => view.classList.remove("active"));
    tabButton.classList.add("active");
    root.querySelector(`#${tabButton.dataset.tab}`)?.classList.add("active");
  }
  const platformSummary = target.closest(".platform-summary");
  if (platformSummary) {
    platformSummary.closest(".platform-card")?.classList.toggle("open");
  }
});
```

- [ ] **Step 6: Move reviewed CSS**

Replace `frontend/src/styles.css` with CSS ported from `docs/superpowers/prototypes/video-downloader-ui/index.html`. Preserve these verified rules:

```css
.nav button {
  min-width: 96px;
  white-space: nowrap;
}

@media (max-width: 980px) {
  .sidebar {
    flex-direction: row;
    flex-wrap: wrap;
  }

  .nav {
    display: flex;
    overflow: visible;
    flex-wrap: wrap;
  }

  .nav button {
    flex: 0 0 auto;
    justify-content: center;
    min-width: 112px;
    width: auto;
  }

  .platform-summary {
    grid-template-columns: minmax(0, 1fr) auto;
  }
}
```

- [ ] **Step 7: Run frontend tests and build**

```powershell
npm run test
npm run build
```

Expected: Vitest passes and Vite build succeeds.

- [ ] **Step 8: Commit frontend UI**

```powershell
git add frontend index.html package.json package-lock.json
git commit -m "feat: implement reviewed desktop ui"
```

## Task 9: Implement Encrypted Bilibili Session Store

**Files:**
- Create: `src-tauri/src/auth/mod.rs`
- Create: `src-tauri/src/auth/session_store.rs`
- Create: `src-tauri/src/auth/bilibili.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/auth/session_store.rs`
- Test: `src-tauri/src/auth/bilibili.rs`

Hardening tests for this task must cover corrupt session payloads, malformed `local.key` files without panic, encrypted payloads not containing plaintext cookies, missing sessions returning `None`, and safe platform-derived session filenames.

- [x] **Step 1: Add encrypted session store**

Create `src-tauri/src/auth/mod.rs`:

```rust
pub mod bilibili;
pub mod session_store;
```

Create `src-tauri/src/auth/session_store.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{fmt, fs, io};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSession {
    pub platform: String,
    pub cookies: String,
    pub expires_at: Option<String>,
    pub last_verified_at: Option<String>,
}

impl fmt::Debug for StoredSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoredSession")
            .field("platform", &self.platform)
            .field("cookies", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("last_verified_at", &self.last_verified_at)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn save(&self, session: &StoredSession) -> AppResult<()> {
        fs::create_dir_all(&self.dir).map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let mut nonce_bytes = [0_u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = serde_json::to_vec(session).map_err(|err| AppError::structured(ErrorCode::UnknownError, err.to_string()))?;
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|_| AppError::structured(ErrorCode::UnknownError, "failed to encrypt session"))?;
        let payload = format!("{}:{}", general_purpose::STANDARD.encode(nonce_bytes), general_purpose::STANDARD.encode(ciphertext));
        fs::write(self.session_path(&session.platform), payload).map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(())
    }

    pub fn load(&self, platform: &str) -> AppResult<Option<StoredSession>> {
        let path = self.session_path(platform);
        if !path.exists() {
            return Ok(None);
        }
        let payload = match fs::read_to_string(&path) {
            Ok(payload) => payload,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(AppError::structured(ErrorCode::FilesystemError, err.to_string())),
        };
        let (nonce_text, cipher_text) = payload.split_once(':').ok_or_else(|| AppError::structured(ErrorCode::LoginExpired, "invalid session file"))?;
        let nonce_bytes = general_purpose::STANDARD.decode(nonce_text).map_err(|_| AppError::structured(ErrorCode::LoginExpired, "invalid session nonce"))?;
        if nonce_bytes.len() != 12 {
            return Err(AppError::structured(ErrorCode::LoginExpired, "invalid session nonce"));
        }
        let ciphertext = general_purpose::STANDARD.decode(cipher_text).map_err(|_| AppError::structured(ErrorCode::LoginExpired, "invalid session payload"))?;
        let key = self.load_or_create_key()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let plaintext = cipher.decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref()).map_err(|_| AppError::structured(ErrorCode::LoginExpired, "failed to decrypt session"))?;
        let session: StoredSession = serde_json::from_slice(&plaintext).map_err(|_| AppError::structured(ErrorCode::LoginExpired, "invalid session json"))?;
        if session.platform != platform {
            return Err(AppError::structured(ErrorCode::LoginExpired, "session platform mismatch"));
        }
        Ok(Some(session))
    }

    pub fn clear(&self, platform: &str) -> AppResult<()> {
        let path = self.session_path(platform);
        if path.exists() {
            fs::remove_file(path).map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        }
        Ok(())
    }

    fn session_path(&self, platform: &str) -> PathBuf {
        self.dir.join(format!("{}.session.enc", safe_platform_file_stem(platform)))
    }

    fn key_path(&self) -> PathBuf {
        self.dir.join("local.key")
    }

    fn load_or_create_key(&self) -> AppResult<[u8; 32]> {
        fs::create_dir_all(&self.dir).map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let path = self.key_path();
        if path.exists() {
            let bytes = fs::read(path).map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
            if bytes.len() != 32 {
                return Err(AppError::structured(ErrorCode::LoginExpired, "invalid local session key"));
            }
            let mut key = [0_u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
        let mut key = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        fs::write(path, key).map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_encrypted_session() {
        let dir = std::env::temp_dir().join(format!("vd-session-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(dir);
        let session = StoredSession {
            platform: "bilibili".into(),
            cookies: "SESSDATA=abc".into(),
            expires_at: None,
            last_verified_at: Some("2026-05-17T00:00:00Z".into()),
        };
        store.save(&session).unwrap();
        let loaded = store.load("bilibili").unwrap().unwrap();
        assert_eq!(loaded.cookies, "SESSDATA=abc");
        store.clear("bilibili").unwrap();
        assert!(store.load("bilibili").unwrap().is_none());
    }
}

fn safe_platform_file_stem(platform: &str) -> String {
    let stem: String = platform
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        "session".to_string()
    } else {
        stem
    }
}
```

- [x] **Step 2: Add bilibili session facade**

Create `src-tauri/src/auth/bilibili.rs`:

```rust
use super::session_store::{SessionStore, StoredSession};
use crate::errors::AppResult;
use chrono::Utc;

#[derive(Clone)]
pub struct BilibiliAuth {
    store: SessionStore,
}

impl BilibiliAuth {
    pub fn new(store: SessionStore) -> Self {
        Self { store }
    }

    pub fn save_cookie_string(&self, cookies: String) -> AppResult<()> {
        self.store.save(&StoredSession {
            platform: "bilibili".into(),
            cookies,
            expires_at: None,
            last_verified_at: Some(Utc::now().to_rfc3339()),
        })
    }

    pub fn load_cookie_string(&self) -> AppResult<Option<String>> {
        Ok(self.store.load("bilibili")?.map(|session| session.cookies))
    }

    pub fn clear(&self) -> AppResult<()> {
        self.store.clear("bilibili")
    }
}
```

- [x] **Step 3: Export auth module**

Modify `src-tauri/src/lib.rs` and add `pub mod auth;`.

- [x] **Step 4: Run auth tests**

```powershell
cd src-tauri
cargo test auth
```

Expected: encrypted session round-trip test passes.

- [x] **Step 5: Commit session storage**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/auth
git commit -m "feat: add encrypted bilibili session storage"
```

## Task 10: Implement Native Bilibili P0 Engine

**Files:**
- Create: `src-tauri/src/platform/bilibili/mod.rs`
- Create: `src-tauri/src/platform/bilibili/native.rs`
- Modify: `src-tauri/src/platform/mod.rs`
- Test: `src-tauri/src/platform/bilibili/native.rs`

- [x] **Step 1: Add URL parser tests and implementation**

Create `src-tauri/src/platform/bilibili/mod.rs`:

```rust
pub mod native;
```

Create `src-tauri/src/platform/bilibili/native.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::platform::{DownloadInput, DownloadOutput, EventSink, PlatformDownloader, ProbeInput, ProbeResult};
use reqwest::Url;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilibiliVideoId {
    pub bvid: String,
}

pub fn parse_bvid(url: &str) -> AppResult<BilibiliVideoId> {
    let url = url.trim();
    let parsed = Url::parse(url)
        .or_else(|_| Url::parse(&format!("https://{url}")))
        .map_err(|_| AppError::structured(ErrorCode::UnsupportedContent, "invalid bilibili video url"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::structured(ErrorCode::UnsupportedContent, "unsupported bilibili video scheme"));
    }
    let host = parsed.host_str()
        .ok_or_else(|| AppError::structured(ErrorCode::UnsupportedContent, "missing bilibili video host"))?;
    if host != "bilibili.com" && !host.ends_with(".bilibili.com") {
        return Err(AppError::structured(ErrorCode::UnsupportedContent, "unsupported bilibili video host"));
    }
    let mut segments = parsed.path_segments()
        .ok_or_else(|| AppError::structured(ErrorCode::UnsupportedContent, "missing bilibili video marker"))?;
    if segments.next() != Some("video") {
        return Err(AppError::structured(ErrorCode::UnsupportedContent, "missing bilibili video marker"));
    }
    let bvid = segments.next().unwrap_or_default();
    if is_valid_bvid(bvid) {
        Ok(BilibiliVideoId { bvid: bvid.to_string() })
    } else {
        Err(AppError::structured(ErrorCode::UnsupportedContent, "missing BV id"))
    }
}

fn is_valid_bvid(value: &str) -> bool {
    value.len() == 12
        && value.starts_with("BV")
        && value.chars().all(|ch| ch.is_ascii_alphanumeric())
}

#[derive(Default)]
pub struct NativeBilibiliDownloader;

impl PlatformDownloader for NativeBilibiliDownloader {
    fn probe<'a>(&'a self, input: ProbeInput) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let id = parse_bvid(&input.url)?;
            Ok(ProbeResult {
                group_title: format!("bilibili {}", id.bvid),
                used_login: input.has_login,
                items: vec![crate::platform::DownloadItem {
                    title: format!("{} P1", id.bvid),
                    output_file: format!("{} P1.mp4", id.bvid),
                    quality: Some(if input.has_login { "1080P".into() } else { "720P".into() }),
                    requires_login: input.has_login,
                    bytes_total: None,
                }],
            })
        })
    }

    fn download<'a>(&'a self, _input: DownloadInput, _sink: &'a dyn EventSink) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
        Box::pin(async move {
            Err(AppError::structured(ErrorCode::PlatformChanged, "native media download requires the media API task"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bv_id_from_video_url() {
        let parsed = parse_bvid("https://www.bilibili.com/video/BV1xx411c7mD/?spm_id_from=333").unwrap();
        assert_eq!(parsed.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn parses_bv_id_before_path_suffix() {
        let parsed = parse_bvid("https://www.bilibili.com/video/BV1xx411c7mD/?p=2#reply").unwrap();
        assert_eq!(parsed.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn parses_video_url_without_scheme() {
        let parsed = parse_bvid("www.bilibili.com/video/BV1xx411c7mD/?next=https://example.com/path").unwrap();
        assert_eq!(parsed.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn parses_trimmed_root_and_mobile_hosts() {
        for url in [
            " https://bilibili.com/video/BV1xx411c7mD/\n",
            "\thttps://m.bilibili.com/video/BV1xx411c7mD/",
        ] {
            let parsed = parse_bvid(url).unwrap();
            assert_eq!(parsed.bvid, "BV1xx411c7mD");
        }
    }

    #[test]
    fn rejects_empty_url() {
        let err = parse_bvid("  ").unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_non_video_url() {
        let err = parse_bvid("https://space.bilibili.com/1").unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_lookalike_bilibili_host() {
        let err = parse_bvid("https://notbilibili.com/video/BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_bilibili_video_link_embedded_in_query() {
        let err = parse_bvid("https://example.com/watch?next=https://www.bilibili.com/video/BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_invalid_bv_id_shape() {
        for url in [
            "https://www.bilibili.com/video/BV12345678",
            "https://www.bilibili.com/video/BV!!!!!!!!!!",
            "https://www.bilibili.com/video/BV1234567890extra",
        ] {
            let err = parse_bvid(url).unwrap_err();
            assert_eq!(err.code(), ErrorCode::UnsupportedContent);
        }
    }

    #[test]
    fn rejects_non_http_video_url() {
        let err = parse_bvid("ftp://www.bilibili.com/video/BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }
}
```

- [x] **Step 2: Register bilibili modules**

Modify `src-tauri/src/platform/mod.rs` first line:

```rust
pub mod bilibili;
pub mod mock;
```

- [x] **Step 3: Run parser tests**

```powershell
cd src-tauri
cargo test bilibili
```

Expected: native URL parser tests pass.

Observed on 2026-05-18 for Task 10:
- RED: `cargo test bilibili` failed in `platform::bilibili::native` because parser/probe stubs returned `UnsupportedContent`.
- RED: parser hardening tests then failed because lookalike hosts, query-embedded links, schemeless links, loose BV ids, and non-http schemes were not handled correctly.
- GREEN: `cargo test bilibili` passed with 15 passed, 0 failed.

- [x] **Step 4: Commit native parser**

```powershell
git add src-tauri/src/platform
git commit -m "feat: add native bilibili parser"
```

## Task 11: Add `yt-dlp` Fallback Adapter And Tool Status

Execution plan for this pass:
- [x] RED: add `yt_dlp` adapter tests and `commands::tests` coverage, then run `cargo test yt_dlp commands::tests` to confirm the missing API failure.
- [x] GREEN: implement only the path detector, argument builder, tool status command, and Tauri handler registration required by the tests.
- [x] VERIFY: run the requested Rust test/check/fmt/clippy/diff commands and record the result before committing.

**Files:**
- Create: `src-tauri/src/platform/bilibili/yt_dlp.rs`
- Modify: `src-tauri/src/platform/bilibili/mod.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/platform/bilibili/yt_dlp.rs`

- [x] **Step 1: Add path-based status helper**

Create `src-tauri/src/platform/bilibili/yt_dlp.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YtDlpStatus {
    Missing,
    Available { path: PathBuf },
}

pub fn detect_ytdlp(configured_path: Option<&str>) -> YtDlpStatus {
    if let Some(path) = configured_path {
        let path = PathBuf::from(path);
        if path.is_file() {
            return YtDlpStatus::Available { path };
        }
    }
    YtDlpStatus::Missing
}

pub fn require_ytdlp(path: Option<&str>) -> AppResult<PathBuf> {
    match detect_ytdlp(path) {
        YtDlpStatus::Available { path } => Ok(path),
        YtDlpStatus::Missing => Err(AppError::structured(ErrorCode::EngineMissing, "yt-dlp is not installed")),
    }
}

pub fn ytdlp_json_args(url: &str, cookies_path: Option<&Path>) -> Vec<String> {
    let mut args = vec!["--dump-json".to_string(), "--no-warnings".to_string()];
    if let Some(path) = cookies_path {
        args.push("--cookies".to_string());
        args.push(path.to_string_lossy().to_string());
    }
    args.push(url.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_when_no_path_configured() {
        assert_eq!(detect_ytdlp(None), YtDlpStatus::Missing);
    }

    #[test]
    fn builds_json_args_with_cookie_file() {
        let args = ytdlp_json_args("https://www.bilibili.com/video/BV1xx411c7mD", Some(Path::new("cookies.txt")));
        assert!(args.contains(&"--dump-json".to_string()));
        assert!(args.contains(&"--cookies".to_string()));
        assert!(args.contains(&"cookies.txt".to_string()));
    }

    #[test]
    fn missing_when_configured_path_is_directory() {
        let dir = temp_test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(detect_ytdlp(Some(&dir.to_string_lossy())), YtDlpStatus::Missing);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
```

- [x] **Step 2: Expose tool status command**

Modify `src-tauri/src/commands.rs` and add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStatus {
    pub ytdlp: String,
    pub ffmpeg: String,
    pub ffprobe: String,
}

#[tauri::command]
pub async fn get_tool_status() -> AppResult<ToolStatus> {
    Ok(tool_status_from_config(&get_config()?))
}

fn tool_status_from_config(config: &AppConfig) -> ToolStatus {
    let ytdlp = match detect_ytdlp(config.ytdlp_path.as_deref()) {
        YtDlpStatus::Available { .. } => "available",
        YtDlpStatus::Missing => "missing",
    };
    ToolStatus {
        ytdlp: ytdlp.into(),
        ffmpeg: "missing".into(),
        ffprobe: "missing".into(),
    }
}
```

Register `commands::get_tool_status` in `tauri::generate_handler!`.

- [x] **Step 3: Run fallback tests**

```powershell
cd src-tauri
cargo test yt_dlp
cargo test commands::tests
```

Expected: `yt-dlp` status and command tests pass.

- [x] **Step 4: Commit fallback adapter shell**

```powershell
git add src-tauri/src/platform/bilibili/yt_dlp.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add yt-dlp fallback status"
```

**Task 11 Review:**
- RED: `cargo test yt_dlp commands::tests` was attempted first and failed because Cargo accepts only one test filter. Equivalent RED runs of `cargo test yt_dlp` and `cargo test commands::tests` then failed on missing `YtDlpStatus`, `detect_ytdlp`, `require_ytdlp`, `ytdlp_json_args`, `ToolStatus`, and `get_tool_status`.
- RED: directory-path hardening test failed while `detect_ytdlp` still used `exists()`, because directories were misreported as available.
- RED: `commands::tests` failed until tool status was derived from `AppConfig.ytdlp_path` and stopped reporting unbundled media tools as `bundled`.
- GREEN: `cargo test yt_dlp` passed 7 tests and `cargo test commands::tests` passed 5 tests after the minimal adapter and command implementation.
- Verification: `cargo test`, `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `git diff --check HEAD` all exited 0.
- Follow-up for Task 13: when commands are refactored to managed app state, `get_tool_status` must load the persisted config from storage and pass it to `tool_status_from_config`.
- Follow-up for Task 14: when ffmpeg/ffprobe sidecars are actually wired, `get_tool_status` must stop returning fixed `missing` and report the sidecar detection result.

## Task 12: Add Bilibili QR Login Command Flow

**Files:**
- Modify: `src-tauri/src/auth/bilibili.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/auth/bilibili.rs`

- [x] **Step 1: Add QR login state structs**

Modify `src-tauri/src/auth/bilibili.rs`:

```rust
use super::session_store::{SessionStore, StoredSession};
use crate::errors::AppResult;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginQr {
    pub qrcode_key: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginStatus {
    pub platform: String,
    pub status: String,
}

#[derive(Clone)]
pub struct BilibiliAuth {
    store: SessionStore,
}

impl BilibiliAuth {
    pub fn new(store: SessionStore) -> Self {
        Self { store }
    }

    pub fn create_mock_qr(&self) -> LoginQr {
        LoginQr {
            qrcode_key: "mock-qrcode-key".into(),
            url: "https://passport.bilibili.com/qrcode/mock".into(),
        }
    }

    pub fn save_cookie_string(&self, cookies: String) -> AppResult<()> {
        self.store.save(&StoredSession {
            platform: "bilibili".into(),
            cookies,
            expires_at: None,
            last_verified_at: Some(Utc::now().to_rfc3339()),
        })
    }

    pub fn load_cookie_string(&self) -> AppResult<Option<String>> {
        Ok(self.store.load("bilibili")?.map(|session| session.cookies))
    }

    pub fn status(&self) -> AppResult<LoginStatus> {
        let status = if self.load_cookie_string()?.is_some() { "已登录" } else { "未登录" };
        Ok(LoginStatus { platform: "bilibili".into(), status: status.into() })
    }

    pub fn clear(&self) -> AppResult<()> {
        self.store.clear("bilibili")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_qr_has_key_and_url() {
        let store = SessionStore::new(std::env::temp_dir().join(format!("vd-auth-{}", uuid::Uuid::new_v4())));
        let auth = BilibiliAuth::new(store);
        let qr = auth.create_mock_qr();
        assert_eq!(qr.qrcode_key, "mock-qrcode-key");
        assert!(qr.url.contains("bilibili"));
    }
}
```

- [x] **Step 2: Add login commands**

Modify `src-tauri/src/commands.rs` and add stateless shell commands before `AppState` replaces them in Task 13:

```rust
#[tauri::command]
pub async fn start_bilibili_login() -> AppResult<crate::auth::bilibili::LoginQr> {
    let dir = std::env::temp_dir().join(format!("video-downloader-login-shell-{}", uuid::Uuid::new_v4()));
    let auth = crate::auth::bilibili::BilibiliAuth::new(crate::auth::session_store::SessionStore::new(dir));
    Ok(auth.create_mock_qr())
}

#[tauri::command]
pub async fn clear_bilibili_login() -> AppResult<()> {
    Ok(())
}
```

Register both commands in `tauri::generate_handler!`.

- [x] **Step 3: Run auth command tests**

```powershell
cd src-tauri
cargo test auth
cargo test commands::tests
```

Expected: auth and command tests pass.

Task 12 review notes:
- RED: `cargo test auth` first failed because `LoginStatus`, `create_mock_qr`, and `status` were missing.
- RED: `cargo test commands::tests` first failed because `start_bilibili_login` and `clear_bilibili_login` were missing.
- RED: fixed temp session hardening test failed while `clear_bilibili_login` still cleared `std::env::temp_dir()/video-downloader-session`.
- RED: test-pollution review then found the hardening test still touched the same fixed temp path, so the test was changed to use a unique external session directory.
- GREEN: `cargo test auth` passed 10 tests and `cargo test commands::tests` passed 7 tests after the stateless command shell was implemented.
- Follow-up for Task 13: replace the stateless command shell with managed app-data state so `start_bilibili_login` and `clear_bilibili_login` use the durable encrypted session store.

- [x] **Step 4: Commit QR login shell**

```powershell
git add src-tauri/src/auth src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add bilibili login command shell"
```

## Task 13: Wire Durable App State Into Tauri Runtime

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/app_state.rs`
- Test: `src-tauri/src/app_state.rs`

- [x] **Step 1: Add app state initializer**

Create `src-tauri/src/app_state.rs`:

```rust
use crate::auth::bilibili::BilibiliAuth;
use crate::auth::session_store::SessionStore;
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::storage::Storage;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub bilibili_auth: BilibiliAuth,
}

impl AppState {
    pub async fn new(data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let db_url = format!("sqlite://{}", data_dir.join("video_downloader.sqlite").to_string_lossy());
        let storage = Storage::open(&db_url).await?;
        let bilibili_auth = BilibiliAuth::new(SessionStore::new(data_dir.join("sessions")));
        Ok(Self { storage, bilibili_auth })
    }
}
```

- [x] **Step 2: Register managed state**

Modify `src-tauri/src/lib.rs` to create state in setup:

```rust
pub mod app_state;
pub mod auth;
pub mod commands;
pub mod config;
pub mod errors;
pub mod media;
pub mod models;
pub mod platform;
pub mod storage;
pub mod task;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;

            let handle = app.handle().clone();
            let dir = handle.path().app_data_dir()?;
            let state = init_app_state(dir)?;
            handle.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::create_task,
            commands::list_platform_logins,
            commands::get_tool_status,
            commands::start_bilibili_login,
            commands::clear_bilibili_login
        ])
        .run(tauri::generate_context!())
        .expect("failed to run video downloader");
}

fn init_app_state(data_dir: std::path::PathBuf) -> Result<app_state::AppState, Box<dyn std::error::Error>> {
    Ok(tauri::async_runtime::block_on(app_state::AppState::new(data_dir))?)
}
```

- [x] **Step 3: Refactor commands to use state**

Modify command signatures:

```rust
#[tauri::command]
pub async fn get_config(state: tauri::State<'_, crate::app_state::AppState>) -> AppResult<AppConfig> {
    state.storage.load_config().await
}

#[tauri::command]
pub async fn list_platform_logins(state: tauri::State<'_, crate::app_state::AppState>) -> AppResult<Vec<PlatformLoginRow>> {
    let status = state.bilibili_auth.status()?.status;
    Ok(vec![PlatformLoginRow { platform: "bilibili".into(), status }])
}
```

- [x] **Step 4: Run Rust tests and check**

```powershell
cd src-tauri
cargo test
cargo check
```

Expected: all Rust tests pass and app type-checks.

Task 13 review notes:
- RED: `cargo test app_state` failed because `AppState` was not yet defined.
- RED: `cargo test commands::tests` failed because the state-based command helpers did not exist.
- RED: setup failure-path test failed until app-state initialization became a helper that returns setup errors instead of panicking with `expect`.
- RED: app-state cleanup test failed until `Storage::close` and `AppState::close` explicitly closed the SQLite pool before deleting test data directories.
- RED: full `cargo test` exposed Windows SQLite file-lock cleanup failures under parallel tests until the local SQLite pool was constrained to one connection.
- GREEN: `cargo test app_state` passed after `AppState::new(data_dir)` created the data directory, opened `video_downloader.sqlite`, and wired encrypted Bilibili sessions under `data_dir/sessions`.
- GREEN: `cargo test commands::tests` passed after config, login status, tool status, login start, and login clear read from managed `AppState`.
- Scope note: `tauri_plugin_shell` remains intentionally absent; shell and sidecar work belongs to Task 14.

- [x] **Step 5: Commit app state**

```powershell
git add src-tauri/src
git commit -m "feat: wire durable tauri app state"
```

## Task 14: Add Real Tool Execution Boundaries

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/media.rs`
- Modify: `src-tauri/capabilities/default.json`
- Test: `src-tauri/src/media.rs`

- [x] **Step 1: Add ffmpeg sidecar naming test**

Modify `src-tauri/src/media.rs` and add:

```rust
pub fn sidecar_base_name(tool: &str) -> AppResult<&'static str> {
    match tool {
        "ffmpeg" => Ok("ffmpeg"),
        "ffprobe" => Ok("ffprobe"),
        _ => Err(AppError::structured(ErrorCode::EngineMissing, "unknown bundled tool")),
    }
}

#[cfg(test)]
mod sidecar_tests {
    use super::*;

    #[test]
    fn accepts_only_bundled_media_tools() {
        assert_eq!(sidecar_base_name("ffmpeg").unwrap(), "ffmpeg");
        assert_eq!(sidecar_base_name("ffprobe").unwrap(), "ffprobe");
        assert_eq!(sidecar_base_name("yt-dlp").unwrap_err().code(), ErrorCode::EngineMissing);
    }
}
```

- [x] **Step 2: Lock shell permissions to known sidecars**

Ensure `src-tauri/capabilities/default.json` contains only `ffmpeg` and `ffprobe` sidecars for bundled media. Do not grant unrestricted shell execution.

- [x] **Step 3: Run tests**

```powershell
cd src-tauri
cargo test media
```

Expected: sidecar tests pass.

Task 14 review notes:
- RED: `cargo test media` failed because `sidecar_base_name` did not exist.
- RED: capability test failed because `default.json` had no constrained `shell:allow-execute` entry.
- RED: `cargo test media` then failed in Tauri's build script because `shell:allow-execute` was not registered until `tauri-plugin-shell` was added and initialized.
- RED: capability test required explicit `"args": false` so sidecar permissions cannot accept arbitrary frontend arguments.
- RED: review suggested guarding against extra shell permissions, so the capability test now asserts there is exactly one `shell:*` permission.
- GREEN: `cargo test media` passed with the sidecar whitelist, registered shell plugin, and constrained capability.
- Verification: `cargo test`, `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `git diff --check` all exited 0.
- Scope note: actual FFmpeg binaries and `bundle.externalBin` entries remain deferred until the binary distribution task; Task 14 only registers the execution boundary and frontend permission constraints.

- [x] **Step 4: Commit tool boundary**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/media.rs src-tauri/capabilities/default.json
git commit -m "feat: constrain bundled media tool execution"
```

## Task 15: Implement Native Bilibili API Parsing

**Files:**
- Create: `src-tauri/src/platform/bilibili/api.rs`
- Modify: `src-tauri/src/platform/bilibili/mod.rs`
- Modify: `src-tauri/src/platform/bilibili/native.rs`
- Test: `src-tauri/src/platform/bilibili/api.rs`

- [x] **Step 1: Add fixture-driven API parser**

Create `src-tauri/src/platform/bilibili/api.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoPage {
    pub cid: u64,
    pub page: u32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub title: String,
    pub pages: Vec<VideoPage>,
}

#[derive(Debug, Deserialize)]
struct ViewResponse {
    code: i32,
    message: String,
    data: Option<ViewData>,
}

#[derive(Debug, Deserialize)]
struct ViewData {
    title: String,
    pages: Vec<ViewPage>,
}

#[derive(Debug, Deserialize)]
struct ViewPage {
    cid: u64,
    page: u32,
    part: String,
}

pub fn parse_view_info(json: &str) -> AppResult<ViewInfo> {
    let parsed: ViewResponse = serde_json::from_str(json)
        .map_err(|err| AppError::structured(ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(AppError::structured(ErrorCode::PlatformChanged, parsed.message));
    }
    let data = parsed.data.ok_or_else(|| AppError::structured(ErrorCode::PlatformChanged, "missing view data"))?;
    Ok(ViewInfo {
        title: data.title,
        pages: data.pages.into_iter().map(|page| VideoPage {
            cid: page.cid,
            page: page.page,
            title: page.part,
        }).collect(),
    })
}

pub fn view_info_url(bvid: &str) -> String {
    format!("https://api.bilibili.com/x/web-interface/view?bvid={bvid}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_part_view_response() {
        let json = r#"{
          "code": 0,
          "message": "0",
          "data": {
            "title": "Rust 桌面应用入门",
            "pages": [
              {"cid": 111, "page": 1, "part": "安装 Tauri"},
              {"cid": 222, "page": 2, "part": "Rust 命令与事件"}
            ]
          }
        }"#;
        let info = parse_view_info(json).unwrap();
        assert_eq!(info.title, "Rust 桌面应用入门");
        assert_eq!(info.pages.len(), 2);
        assert_eq!(info.pages[0].title, "安装 Tauri");
    }
}
```

- [x] **Step 2: Wire native probe to view parser**

Modify `src-tauri/src/platform/bilibili/mod.rs`:

```rust
pub mod api;
pub mod native;
pub mod yt_dlp;
```

Modify `src-tauri/src/platform/bilibili/native.rs` probe body so `NativeBilibiliDownloader` uses `view_info_url` and `parse_view_info` for real metadata. Keep the parser unit-tested and make live HTTP tests ignored:

```rust
async fn fetch_view_info(client: &reqwest::Client, bvid: &str) -> AppResult<crate::platform::bilibili::api::ViewInfo> {
    let text = client
        .get(crate::platform::bilibili::api::view_info_url(bvid))
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .text()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;
    crate::platform::bilibili::api::parse_view_info(&text)
}
```

- [x] **Step 3: Run API tests**

```powershell
cd src-tauri
cargo test bilibili::api
```

Expected: fixture parser tests pass without network.

Task 15 review notes:
- RED: `cargo test bilibili::api` first failed because `VideoPage`, `parse_view_info`, and `view_info_url` did not exist.
- RED: native mapping tests failed until `ViewInfo` pages were converted into real `ProbeResult` items and single-video output names avoided a forced `01 -` prefix.
- RED: ignored live fetch test failed to compile until `fetch_view_info` was added and `NativeBilibiliDownloader::probe` used it.
- RED: a real Bilibili view API check showed single-page `part` can be empty, so parser coverage now falls back to the video title for that case.
- RED: quality review found empty `pages` could create an empty task group, so `parse_view_info` now rejects missing page data.
- RED: quality review found HTTP 4xx/5xx responses were parsed as platform changes, so `fetch_view_info` now maps non-2xx status to `NetworkError` with a local one-shot HTTP test.
- GREEN: `cargo test bilibili::api`, `cargo test bilibili`, and `cargo test live_fetch_view_info_returns_pages -- --ignored` passed.
- Verification: `cargo test`, `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `git diff --check` all exited 0.
- Scope note: media download/merge still returns the existing media-API placeholder error; Task 15 only replaces native probe metadata with real view metadata.

- [x] **Step 4: Commit API parser**

```powershell
git add src-tauri/src/platform/bilibili
git commit -m "feat: parse bilibili view metadata"
```

## Task 16: Implement Native Media Download And Merge

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/platform/bilibili/media.rs`
- Modify: `src-tauri/src/platform/bilibili/mod.rs`
- Modify: `src-tauri/src/platform/bilibili/native.rs`
- Modify: `src-tauri/src/media.rs`
- Test: `src-tauri/src/platform/bilibili/media.rs`

- [x] **Step 1: Add stream dependency**

Modify `src-tauri/Cargo.toml` and add if stream-by-stream downloads are implemented in this task:

```toml
futures-util = "0.3"
```

- [x] **Step 2: Add playurl parser**

Create `src-tauri/src/platform/bilibili/media.rs`:

```rust
use crate::errors::{AppError, AppResult, ErrorCode};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaStream {
    pub url: String,
    pub bandwidth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashSelection {
    pub quality: String,
    pub video: MediaStream,
    pub audio: MediaStream,
}

#[derive(Debug, Deserialize)]
struct PlayResponse {
    code: i32,
    message: String,
    data: Option<PlayData>,
}

#[derive(Debug, Deserialize)]
struct PlayData {
    quality: Option<u32>,
    dash: Option<DashData>,
}

#[derive(Debug, Deserialize)]
struct DashData {
    video: Vec<DashStream>,
    audio: Vec<DashStream>,
}

#[derive(Debug, Deserialize)]
struct DashStream {
    base_url: String,
    bandwidth: u64,
}

pub fn parse_dash_selection(json: &str) -> AppResult<DashSelection> {
    let parsed: PlayResponse = serde_json::from_str(json)
        .map_err(|err| AppError::structured(ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(AppError::structured(ErrorCode::PlatformChanged, parsed.message));
    }
    let data = parsed.data.ok_or_else(|| AppError::structured(ErrorCode::PlatformChanged, "missing play data"))?;
    let dash = data.dash.ok_or_else(|| AppError::structured(ErrorCode::UnsupportedContent, "missing dash streams"))?;
    let video = dash.video.into_iter().max_by_key(|stream| stream.bandwidth)
        .ok_or_else(|| AppError::structured(ErrorCode::UnsupportedContent, "missing video stream"))?;
    let audio = dash.audio.into_iter().max_by_key(|stream| stream.bandwidth)
        .ok_or_else(|| AppError::structured(ErrorCode::UnsupportedContent, "missing audio stream"))?;
    Ok(DashSelection {
        quality: quality_label(data.quality.unwrap_or(0)).to_string(),
        video: MediaStream { url: video.base_url, bandwidth: video.bandwidth },
        audio: MediaStream { url: audio.base_url, bandwidth: audio.bandwidth },
    })
}

pub fn quality_label(qn: u32) -> &'static str {
    match qn {
        120 => "4K",
        116 => "1080P60",
        80 => "1080P",
        64 => "720P",
        32 => "480P",
        16 => "360P",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_highest_bandwidth_dash_streams() {
        let json = r#"{
          "code": 0,
          "message": "0",
          "data": {
            "quality": 80,
            "dash": {
              "video": [
                {"base_url": "video-low.m4s", "bandwidth": 100},
                {"base_url": "video-high.m4s", "bandwidth": 200}
              ],
              "audio": [
                {"base_url": "audio-low.m4s", "bandwidth": 50},
                {"base_url": "audio-high.m4s", "bandwidth": 90}
              ]
            }
          }
        }"#;
        let selected = parse_dash_selection(json).unwrap();
        assert_eq!(selected.quality, "1080P");
        assert_eq!(selected.video.url, "video-high.m4s");
        assert_eq!(selected.audio.url, "audio-high.m4s");
    }
}
```

- [x] **Step 3: Wire media module**

Modify `src-tauri/src/platform/bilibili/mod.rs`:

```rust
pub mod api;
pub mod media;
pub mod native;
pub mod yt_dlp;
```

- [x] **Step 4: Implement native download boundaries**

Modify `NativeBilibiliDownloader::download` when task metadata carries `cid` / stream URLs and bundled ffmpeg binaries are configured. In this pass, add the tested foundations needed by that path:

1. Emit a log line for selected item.
2. Download video stream and audio stream to temporary `.m4s` files.
3. Invoke bundled `ffmpeg` through the media boundary to merge files.
4. Return `DownloadOutput` with final path, quality, login usage, and total bytes.

Use this internal helper signature:

```rust
async fn download_to_file(client: &reqwest::Client, url: &str, path: &std::path::Path, sink: &dyn EventSink) -> AppResult<u64>
```

The helper must map request failures to `network_error` and filesystem failures to `filesystem_error`.

- [x] **Step 5: Run media parser tests**

```powershell
cd src-tauri
cargo test bilibili::media
```

Expected: media parser tests pass without network.

Task 16 review notes:
- RED: `cargo test bilibili::media` first had zero tests; parser tests then failed because `parse_dash_selection` and `quality_label` did not exist.
- GREEN: `cargo test bilibili::media` passed after the playurl DASH parser selected highest-bandwidth video/audio streams and rejected missing DASH data.
- RED: download helper tests failed until `download_to_file` mapped HTTP failures to `NetworkError`, file-write failures to `FilesystemError`, wrote local files, and emitted progress.
- RED: ffmpeg boundary test failed until `merge_with_ffmpeg` mapped missing or failing ffmpeg execution to `FfmpegError`.
- RED: quality review found `download_to_file` buffered whole media responses in memory, so it now streams response chunks to disk and emits cumulative progress.
- RED: quality review found real Bilibili DASH streams may use `baseUrl`, so `DashStream.base_url` now accepts both `base_url` and `baseUrl`.
- GREEN: local one-shot HTTP tests cover stream downloads and HTTP error mapping without external network.
- Scope correction: full `NativeBilibiliDownloader::download` is not wired in this task because current `DownloadItem` does not carry `cid` or stream URLs, and no bundled ffmpeg sidecar binary / `externalBin` package entry exists yet. This task intentionally adds the parser and executable boundaries without pretending end-to-end media download is functional.
- Verification: `cargo test bilibili::media`, `cargo test`, `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `git diff --check` all exited 0.

- [x] **Step 6: Commit native media foundation**

```powershell
git add src-tauri/Cargo.toml src-tauri/src/platform/bilibili src-tauri/src/media.rs
git commit -m "feat: add native bilibili media selection"
```

## Task 17: Implement Real `yt-dlp` Adapter Execution

**Files:**
- Modify: `src-tauri/src/platform/bilibili/yt_dlp.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/platform/bilibili/yt_dlp.rs`

- [x] **Step 1: Add `yt-dlp` command builder tests**

Modify `src-tauri/src/platform/bilibili/yt_dlp.rs` and add:

```rust
pub fn ytdlp_download_args(url: &str, output_template: &str, cookies_path: Option<&Path>) -> Vec<String> {
    let mut args = vec![
        "--newline".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        "-o".to_string(),
        output_template.to_string(),
    ];
    if let Some(path) = cookies_path {
        args.push("--cookies".to_string());
        args.push(path.to_string_lossy().to_string());
    }
    args.push(url.to_string());
    args
}

#[cfg(test)]
mod download_arg_tests {
    use super::*;

    #[test]
    fn builds_download_args_with_mp4_merge() {
        let args = ytdlp_download_args("https://www.bilibili.com/video/BV1xx411c7mD", "D:\\Videos\\%(title)s.%(ext)s", None);
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"-o".to_string()));
    }
}
```

- [x] **Step 2: Add execution function boundary**

Add a function that accepts the resolved `yt-dlp` path and generated args:

```rust
pub async fn run_ytdlp(path: &Path, args: &[String]) -> AppResult<String> {
    let output = tokio::process::Command::new(path)
        .args(args)
        .output()
        .await
        .map_err(|err| AppError::structured(ErrorCode::EngineMissing, err.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AppError::structured(ErrorCode::UnknownError, stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

- [x] **Step 3: Run adapter tests**

```powershell
cd src-tauri
cargo test yt_dlp
```

Expected: argument builder tests pass. Execution function is covered in the manual verification pass because it requires a local `yt-dlp` binary.

- [x] **Step 4: Commit adapter execution**

```powershell
git add src-tauri/src/platform/bilibili/yt_dlp.rs
git commit -m "feat: add yt-dlp execution adapter"
```

Task 17 review notes:
- RED: `cargo test yt_dlp` failed first because `ytdlp_download_args` and `run_ytdlp` did not exist.
- GREEN: `ytdlp_download_args` now builds newline progress output, mp4 merge output, output template, optional cookie file, and URL ordering.
- GREEN: `run_ytdlp` executes a resolved local binary path, returns stdout on success, maps launch failures to `EngineMissing`, and maps non-zero process exits to `UnknownError`.
- Verification: `cargo test yt_dlp`, `cargo test`, `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `git diff --check`.

## Task 18: Implement Real Bilibili QR Login Polling

**Files:**
- Modify: `src-tauri/src/auth/bilibili.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/auth/bilibili.rs`

- [x] **Step 1: Add login response parsers**

Modify `src-tauri/src/auth/bilibili.rs` and add parser structs:

```rust
#[derive(Debug, serde::Deserialize)]
struct QrGenerateResponse {
    code: i32,
    data: Option<QrGenerateData>,
}

#[derive(Debug, serde::Deserialize)]
struct QrGenerateData {
    url: String,
    qrcode_key: String,
}

pub fn parse_qr_generate(json: &str) -> AppResult<LoginQr> {
    let parsed: QrGenerateResponse = serde_json::from_str(json)
        .map_err(|err| crate::errors::AppError::structured(crate::errors::ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(crate::errors::AppError::structured(crate::errors::ErrorCode::PlatformChanged, "failed to create QR login"));
    }
    let data = parsed.data.ok_or_else(|| crate::errors::AppError::structured(crate::errors::ErrorCode::PlatformChanged, "missing QR login data"))?;
    Ok(LoginQr { qrcode_key: data.qrcode_key, url: data.url })
}

#[cfg(test)]
mod qr_parser_tests {
    use super::*;

    #[test]
    fn parses_qr_generate_response() {
        let json = r#"{"code":0,"data":{"url":"https://passport.bilibili.com/qrcode","qrcode_key":"abc"}}"#;
        let qr = parse_qr_generate(json).unwrap();
        assert_eq!(qr.qrcode_key, "abc");
        assert!(qr.url.contains("passport.bilibili.com"));
    }
}
```

- [x] **Step 2: Implement QR generation and polling HTTP calls**

Add:

```rust
pub async fn request_login_qr(client: &reqwest::Client) -> AppResult<LoginQr> {
    let text = client
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .send()
        .await
        .map_err(|err| crate::errors::AppError::structured(crate::errors::ErrorCode::NetworkError, err.to_string()))?
        .text()
        .await
        .map_err(|err| crate::errors::AppError::structured(crate::errors::ErrorCode::NetworkError, err.to_string()))?;
    parse_qr_generate(&text)
}
```

- [x] **Step 3: Update commands to use real QR request and poll result persistence**

Modify `start_bilibili_login` to call `request_login_qr(&reqwest::Client::new()).await`, add `poll_bilibili_login`, and save confirmed login cookies through the encrypted bilibili session store.

- [x] **Step 4: Run parser tests**

```powershell
cd src-tauri
cargo test qr_parser_tests
```

Expected: parser and local HTTP tests pass without network. The live QR generation test is ignored by default and can be run manually with `cargo test live_request_login_qr_returns_key -- --ignored`.

- [x] **Step 5: Commit QR polling foundation**

```powershell
git add src-tauri/src/auth/bilibili.rs src-tauri/src/commands.rs
git commit -m "feat: add bilibili qr login request"
```

Task 18 review notes:
- RED: `cargo test qr` failed first because QR generation parsing, poll parsing, Set-Cookie extraction, and local HTTP request helpers did not exist.
- GREEN: QR generation now parses current Bilibili `qrcode_key` responses and `request_login_qr` calls the live generate endpoint.
- GREEN: QR polling now maps pending, scanned, expired, and confirmed statuses; unknown status codes fail as `PlatformChanged`.
- GREEN: Confirmed poll responses extract `Set-Cookie` pairs and `poll_bilibili_login` persists them via the encrypted bilibili session store.
- Verification: `cargo test qr`, `cargo test bilibili`, `cargo test commands::tests`, `cargo test live_request_login_qr_returns_key -- --ignored`, `cargo test`, `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `git diff --check`.

## Task 19: End-To-End Verification Pass

**Files:**
- Create: `docs/verification/first-release-checklist.md`
- Modify: `docs/superpowers/specs/2026-05-17-video-downloader-tauri-design.md` only if verification reveals spec drift.

- [ ] **Step 1: Create manual verification checklist**

Create `docs/verification/first-release-checklist.md`:

```markdown
# First Release Verification Checklist

## Desktop Shell

- [ ] `npm run build` succeeds.
- [ ] `cargo test` succeeds in `src-tauri`.
- [ ] `cargo check` succeeds in `src-tauri`.
- [ ] `npm run tauri:dev` opens the app window.

## UI

- [ ] Downloads/Login/Settings navigation works.
- [ ] Narrow navigation has no horizontal scrollbar.
- [ ] Navigation labels stay on one line.
- [ ] Download task creation exposes video link and output directory.
- [ ] Settings owns default engine selection.
- [ ] Downloads tab does not expose task-level engine selection.
- [ ] Login tab shows a flat platform list.
- [ ] Clicking bilibili expands login details.
- [ ] Collection task details show child video name, output file, progress, and retry count.

## Persistence

- [ ] Config persists after app restart.
- [ ] Task history persists after app restart.
- [ ] Encrypted bilibili session file persists after app restart.
- [ ] Clearing bilibili login removes the session file.

## Engines And Tools

- [ ] Native engine parses BV links.
- [ ] Native engine returns `unsupported_content` for unsupported links.
- [ ] Missing `yt-dlp` reports `engine_missing`.
- [ ] Bundled `ffmpeg` and `ffprobe` status is visible.
- [ ] FFmpeg license profile is recorded before public distribution.
```

- [ ] **Step 2: Run automated verification**

Run:

```powershell
npm run test
npm run build
cd src-tauri
cargo test
cargo check
```

Expected: every command exits with code 0.

- [ ] **Step 3: Run prototype parity check in browser**

Open:

```text
http://127.0.0.1:5173/
```

Verify against:

```text
docs/superpowers/prototypes/video-downloader-ui/index.html
```

Expected: reviewed UI behavior remains present in the real frontend.

- [ ] **Step 4: Commit verification docs**

```powershell
git add docs/verification/first-release-checklist.md
git commit -m "docs: add first release verification checklist"
```

## Self-Review Results

- Spec coverage: covered app shell, UI, settings-only engine selection, per-task output directory, flat login platform list, child task details, SQLite persistence, encrypted login state, native parser foundation, `yt-dlp` fallback status, bundled media tool boundaries, and verification.
- Intentional staging: real bilibili media downloading and QR polling are introduced behind tested boundaries after the app has working UI, persistence, and mock task flow. This keeps each task independently testable.
- Red-flag scan: no unresolved marker text is used in the plan.
- Type consistency: `DownloadEngine`, `TaskState`, `TaskGroup`, `DownloadTask`, `AppConfig`, `ErrorCode`, command names, and frontend `Engine` names are consistent across tasks.
