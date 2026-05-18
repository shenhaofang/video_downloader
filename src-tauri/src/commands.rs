use crate::errors::AppResult;
use crate::models::{AppConfig, DownloadEngine};
use crate::platform::bilibili::yt_dlp::{detect_ytdlp, YtDlpStatus};
use crate::platform::mock::MockDownloader;
use crate::task::{create_group_from_probe, CreateTaskRequest, CreatedTaskGroup};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskCommand {
    pub url: String,
    pub output_dir: String,
    pub has_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformLoginRow {
    pub platform: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStatus {
    pub ytdlp: String,
    pub ffmpeg: String,
    pub ffprobe: String,
}

#[tauri::command]
pub fn get_config() -> AppResult<AppConfig> {
    Ok(AppConfig::default())
}

#[tauri::command]
pub async fn create_task(input: CreateTaskCommand) -> AppResult<CreatedTaskGroup> {
    create_group_from_probe(
        &MockDownloader,
        CreateTaskRequest {
            url: input.url,
            output_dir: input.output_dir,
            engine: DownloadEngine::Native,
            has_login: input.has_login,
        },
    )
    .await
}

#[tauri::command]
pub fn list_platform_logins() -> AppResult<Vec<PlatformLoginRow>> {
    Ok(vec![PlatformLoginRow {
        platform: "bilibili".into(),
        status: "未登录".into(),
    }])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadEngine;
    use std::fs;

    #[tokio::test]
    async fn create_task_uses_mock_collection() {
        let result = create_task(CreateTaskCommand {
            url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            output_dir: "D:\\Videos".into(),
            has_login: true,
        })
        .await
        .unwrap();

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

    #[tokio::test]
    async fn get_tool_status_reports_default_missing_tools() {
        let status = get_tool_status().await.unwrap();

        assert_eq!(
            status,
            ToolStatus {
                ytdlp: "missing".into(),
                ffmpeg: "missing".into(),
                ffprobe: "missing".into(),
            }
        );
    }

    #[test]
    fn tool_status_uses_configured_ytdlp_path() {
        let dir = std::env::temp_dir().join(format!("vd-tool-status-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("yt-dlp.exe");
        fs::write(&binary, b"test binary").unwrap();
        let config = AppConfig {
            ytdlp_path: Some(binary.to_string_lossy().to_string()),
            ..AppConfig::default()
        };

        let status = tool_status_from_config(&config);

        assert_eq!(status.ytdlp, "available");
        assert_eq!(status.ffmpeg, "missing");
        assert_eq!(status.ffprobe, "missing");
        fs::remove_dir_all(dir).unwrap();
    }
}
