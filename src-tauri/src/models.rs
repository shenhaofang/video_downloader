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
        assert_eq!(config.download_root, "D:\\Videos");
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
