use crate::config::{normalize_concurrency, normalize_persisted_concurrency};
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
            .map_err(storage_error)?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    async fn migrate(&self) -> AppResult<()> {
        let statements = [
            "CREATE TABLE IF NOT EXISTS app_config (id INTEGER PRIMARY KEY CHECK (id = 1), download_root TEXT NOT NULL, concurrency INTEGER NOT NULL, default_engine TEXT NOT NULL, ytdlp_path TEXT)",
            "CREATE TABLE IF NOT EXISTS task_groups (id TEXT PRIMARY KEY, source_url TEXT NOT NULL, platform TEXT NOT NULL, title TEXT NOT NULL, output_dir TEXT NOT NULL, engine TEXT NOT NULL, state TEXT NOT NULL, created_at TEXT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS download_tasks (id TEXT PRIMARY KEY, group_id TEXT NOT NULL, title TEXT NOT NULL, output_file TEXT NOT NULL, state TEXT NOT NULL, engine TEXT NOT NULL, quality TEXT, used_login INTEGER NOT NULL, bytes_downloaded INTEGER NOT NULL, bytes_total INTEGER, retry_count INTEGER NOT NULL, max_retries INTEGER NOT NULL, error_code TEXT, error_message TEXT)",
            "CREATE TABLE IF NOT EXISTS task_logs (id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, created_at TEXT NOT NULL, line TEXT NOT NULL)",
        ];

        for statement in statements {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(storage_error)?;
        }

        Ok(())
    }

    pub async fn load_config(&self) -> AppResult<AppConfig> {
        let row = sqlx::query(
            "SELECT download_root, concurrency, default_engine, ytdlp_path FROM app_config WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;

        let Some(row) = row else {
            let config = AppConfig::default();
            self.save_config(&config).await?;
            return Ok(config);
        };

        Ok(AppConfig {
            download_root: row.get("download_root"),
            concurrency: normalize_persisted_concurrency(row.get("concurrency")),
            default_engine: parse_engine(&row.get::<String, _>("default_engine")),
            ytdlp_path: row.get("ytdlp_path"),
        })
    }

    pub async fn save_config(&self, config: &AppConfig) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO app_config (id, download_root, concurrency, default_engine, ytdlp_path) VALUES (1, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET download_root = excluded.download_root, concurrency = excluded.concurrency, default_engine = excluded.default_engine, ytdlp_path = excluded.ytdlp_path",
        )
        .bind(&config.download_root)
        .bind(normalize_concurrency(config.concurrency) as i64)
        .bind(engine_name(config.default_engine))
        .bind(&config.ytdlp_path)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;

        Ok(())
    }

    pub async fn insert_group(&self, group: &TaskGroup) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO task_groups (id, source_url, platform, title, output_dir, engine, state, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
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
        .map_err(storage_error)?;

        Ok(())
    }

    pub async fn insert_task(&self, task: &DownloadTask) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO download_tasks (id, group_id, title, output_file, state, engine, quality, used_login, bytes_downloaded, bytes_total, retry_count, max_retries, error_code, error_message) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(task.id.to_string())
        .bind(task.group_id.to_string())
        .bind(&task.title)
        .bind(&task.output_file)
        .bind(state_name(task.state))
        .bind(engine_name(task.engine))
        .bind(&task.quality)
        .bind(if task.used_login { 1_i64 } else { 0_i64 })
        .bind(task.bytes_downloaded as i64)
        .bind(task.bytes_total.map(|value| value as i64))
        .bind(task.retry_count as i64)
        .bind(task.max_retries as i64)
        .bind(&task.error_code)
        .bind(&task.error_message)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;

        Ok(())
    }

    pub async fn append_log(&self, task_id: Uuid, line: &str) -> AppResult<()> {
        sqlx::query("INSERT INTO task_logs (task_id, created_at, line) VALUES (?, ?, ?)")
            .bind(task_id.to_string())
            .bind(Utc::now().to_rfc3339())
            .bind(line)
            .execute(&self.pool)
            .await
            .map_err(storage_error)?;

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

fn storage_error(err: sqlx::Error) -> AppError {
    AppError::structured(ErrorCode::FilesystemError, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::Storage;
    use crate::models::{AppConfig, DownloadEngine, DownloadTask, TaskGroup, TaskState};
    use chrono::Utc;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[tokio::test]
    async fn missing_config_creates_and_saves_default_config() {
        let db = TestDatabase::open().await;
        let storage = &db.storage;

        let config = storage.load_config().await.unwrap();
        let saved = storage.load_config().await.unwrap();

        assert_eq!(config.download_root, AppConfig::default().download_root);
        assert_eq!(config.concurrency, AppConfig::default().concurrency);
        assert_eq!(config.default_engine, AppConfig::default().default_engine);
        assert_eq!(saved.download_root, config.download_root);
        assert_eq!(saved.concurrency, config.concurrency);
        assert_eq!(saved.default_engine, config.default_engine);

        db.close().await;
    }

    #[tokio::test]
    async fn save_and_load_config_round_trips_settings() {
        let db = TestDatabase::open().await;
        let storage = &db.storage;
        let config = AppConfig {
            download_root: String::from("E:\\Downloads"),
            concurrency: 7,
            default_engine: DownloadEngine::YtDlp,
            ytdlp_path: Some(String::from("C:\\tools\\yt-dlp.exe")),
        };

        storage.save_config(&config).await.unwrap();
        let loaded = storage.load_config().await.unwrap();

        assert_eq!(loaded.download_root, config.download_root);
        assert_eq!(loaded.concurrency, config.concurrency);
        assert_eq!(loaded.default_engine, config.default_engine);
        assert_eq!(loaded.ytdlp_path, config.ytdlp_path);

        db.close().await;
    }

    #[tokio::test]
    async fn save_config_persists_normalized_concurrency() {
        let db = TestDatabase::open().await;
        let storage = &db.storage;
        let config = AppConfig {
            concurrency: 99,
            ..AppConfig::default()
        };

        storage.save_config(&config).await.unwrap();
        let loaded = storage.load_config().await.unwrap();
        let stored_concurrency: i64 =
            sqlx::query_scalar("SELECT concurrency FROM app_config WHERE id = 1")
                .fetch_one(&storage.pool)
                .await
                .unwrap();

        assert_eq!(loaded.concurrency, 8);
        assert_eq!(stored_concurrency, 8);

        db.close().await;
    }

    #[tokio::test]
    async fn load_config_clamps_untrusted_persisted_concurrency() {
        for (raw, expected) in [(-1_i64, 1_u8), (300_i64, 8_u8)] {
            let db = TestDatabase::open().await;
            let storage = &db.storage;

            sqlx::query(
                "INSERT INTO app_config (id, download_root, concurrency, default_engine, ytdlp_path) VALUES (1, ?, ?, ?, NULL)",
            )
            .bind("D:\\Videos")
            .bind(raw)
            .bind("native")
            .execute(&storage.pool)
            .await
            .unwrap();

            let loaded = storage.load_config().await.unwrap();

            assert_eq!(loaded.concurrency, expected);

            db.close().await;
        }
    }

    #[tokio::test]
    async fn inserts_group_task_and_log() {
        let db = TestDatabase::open().await;
        let storage = &db.storage;
        let group = TaskGroup {
            id: Uuid::new_v4(),
            source_url: String::from("https://www.bilibili.com/video/BV1xx411c7mD"),
            platform: String::from("bilibili"),
            title: String::from("Rust desktop app"),
            output_dir: String::from("D:\\Videos"),
            engine: DownloadEngine::Native,
            state: TaskState::Queued,
            created_at: Utc::now(),
        };
        let task = DownloadTask {
            id: Uuid::new_v4(),
            group_id: group.id,
            title: String::from("Part 1"),
            output_file: String::from("D:\\Videos\\part-1.mp4"),
            state: TaskState::Pending,
            engine: DownloadEngine::Native,
            quality: Some(String::from("1080P")),
            used_login: false,
            bytes_downloaded: 0,
            bytes_total: Some(1024),
            retry_count: 0,
            max_retries: 3,
            error_code: None,
            error_message: None,
        };

        storage.insert_group(&group).await.unwrap();
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
            remove_files(&path);
        }
    }

    fn remove_files(path: &std::path::Path) {
        let files = [
            path.to_path_buf(),
            path.with_extension("sqlite-shm"),
            path.with_extension("sqlite-wal"),
        ];

        for _ in 0..100 {
            let mut blocked = false;
            for file in &files {
                match std::fs::remove_file(file) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => blocked = true,
                }
            }

            if !blocked {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
