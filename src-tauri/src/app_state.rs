use crate::auth::bilibili::BilibiliAuth;
use crate::auth::session_store::SessionStore;
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::storage::Storage;
use futures_util::future::AbortHandle;
use std::collections::{hash_map::Entry, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub bilibili_auth: BilibiliAuth,
    active_runs: Arc<Mutex<HashMap<Uuid, AbortHandle>>>,
    data_dir: PathBuf,
}

impl AppState {
    pub async fn new(data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        let database_url = sqlite_url_for(&data_dir.join("video_downloader.sqlite"));
        let storage = Storage::open(&database_url).await?;
        let bilibili_auth = BilibiliAuth::new(SessionStore::new(data_dir.join("sessions")));

        Ok(Self {
            storage,
            bilibili_auth,
            active_runs: Arc::new(Mutex::new(HashMap::new())),
            data_dir,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn register_task_abort(&self, task_id: Uuid, handle: AbortHandle) -> bool {
        match self.active_runs.lock().unwrap().entry(task_id) {
            Entry::Vacant(entry) => {
                entry.insert(handle);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn abort_task(&self, task_id: Uuid) -> bool {
        self.active_runs
            .lock()
            .unwrap()
            .remove(&task_id)
            .map(|handle| {
                handle.abort();
            })
            .is_some()
    }

    pub fn clear_task_abort(&self, task_id: Uuid) {
        self.active_runs.lock().unwrap().remove(&task_id);
    }

    pub async fn close(self) {
        self.storage.close().await;
    }
}

fn sqlite_url_for(path: &Path) -> String {
    format!(
        "sqlite://{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    )
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::models::{AppConfig, DownloadEngine};
    use std::fs;

    fn unique_data_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vd-app-state-{}", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn initializes_storage_database_and_bilibili_sessions_under_data_dir() {
        let data_dir = unique_data_dir();
        let state = AppState::new(data_dir.clone()).await.unwrap();

        assert!(data_dir.is_dir());
        assert!(data_dir.join("video_downloader.sqlite").is_file());

        let config = AppConfig {
            download_root: "E:\\Videos".into(),
            concurrency: 4,
            default_engine: DownloadEngine::YtDlp,
            ytdlp_path: Some("C:\\tools\\yt-dlp.exe".into()),
            ffmpeg_path: Some("C:\\tools\\ffmpeg.exe".into()),
            ffprobe_path: Some("C:\\tools\\ffprobe.exe".into()),
        };
        state.storage.save_config(&config).await.unwrap();
        assert_eq!(state.storage.load_config().await.unwrap(), config);

        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=durable-cookie".into())
            .unwrap();
        assert!(data_dir
            .join("sessions")
            .join("bilibili.session.enc")
            .is_file());
        assert_eq!(
            state.bilibili_auth.load_cookie_string().unwrap(),
            Some("SESSDATA=durable-cookie".into())
        );

        state.close().await;
        remove_dir_all_retry(&data_dir);
    }

    #[tokio::test]
    async fn closes_storage_before_removing_data_dir() {
        let data_dir = unique_data_dir();
        let state = AppState::new(data_dir.clone()).await.unwrap();

        state.close().await;
        remove_dir_all_retry(&data_dir);
    }

    fn remove_dir_all_retry(path: &std::path::Path) {
        for _ in 0..500 {
            match fs::remove_dir_all(path) {
                Ok(()) => return,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        panic!("failed to remove test app state dir {}", path.display());
    }
}
