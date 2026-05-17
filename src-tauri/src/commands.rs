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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformLoginRow {
    pub platform: String,
    pub status: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadEngine;

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
}
