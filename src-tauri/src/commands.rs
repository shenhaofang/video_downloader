use crate::app_state::AppState;
use crate::auth::bilibili::{
    poll_login_qr, request_login_qr, LoginPollOutcome, LoginPollResult, LoginQr,
};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollLoginCommand {
    pub qrcode_key: String,
}

#[tauri::command]
pub async fn get_config(state: tauri::State<'_, AppState>) -> AppResult<AppConfig> {
    get_config_from_state(state.inner()).await
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
pub fn list_platform_logins(state: tauri::State<'_, AppState>) -> AppResult<Vec<PlatformLoginRow>> {
    list_platform_logins_from_state(state.inner())
}

#[tauri::command]
pub async fn start_bilibili_login(state: tauri::State<'_, AppState>) -> AppResult<LoginQr> {
    start_bilibili_login_from_state(state.inner()).await
}

#[tauri::command]
pub async fn poll_bilibili_login(
    state: tauri::State<'_, AppState>,
    input: PollLoginCommand,
) -> AppResult<LoginPollResult> {
    poll_bilibili_login_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn clear_bilibili_login(state: tauri::State<'_, AppState>) -> AppResult<()> {
    clear_bilibili_login_from_state(state.inner()).await
}

#[tauri::command]
pub async fn get_tool_status(state: tauri::State<'_, AppState>) -> AppResult<ToolStatus> {
    get_tool_status_from_state(state.inner()).await
}

async fn get_config_from_state(state: &AppState) -> AppResult<AppConfig> {
    state.storage.load_config().await
}

fn list_platform_logins_from_state(state: &AppState) -> AppResult<Vec<PlatformLoginRow>> {
    let status = state.bilibili_auth.status()?;
    Ok(vec![PlatformLoginRow {
        platform: status.platform,
        status: status.status,
    }])
}

async fn start_bilibili_login_from_state(_state: &AppState) -> AppResult<LoginQr> {
    request_login_qr(&reqwest::Client::new()).await
}

async fn poll_bilibili_login_from_state(
    state: &AppState,
    input: PollLoginCommand,
) -> AppResult<LoginPollResult> {
    let outcome = poll_login_qr(&reqwest::Client::new(), &input.qrcode_key).await?;
    persist_bilibili_poll_outcome(state, outcome)
}

fn persist_bilibili_poll_outcome(
    state: &AppState,
    outcome: LoginPollOutcome,
) -> AppResult<LoginPollResult> {
    if outcome.result.status == "confirmed" {
        let cookies = outcome.cookies.ok_or_else(|| {
            crate::errors::AppError::structured(
                crate::errors::ErrorCode::PlatformChanged,
                "missing login cookies",
            )
        })?;
        state.bilibili_auth.save_cookie_string(cookies)?;
    }

    Ok(outcome.result)
}

async fn clear_bilibili_login_from_state(state: &AppState) -> AppResult<()> {
    state.bilibili_auth.clear()
}

async fn get_tool_status_from_state(state: &AppState) -> AppResult<ToolStatus> {
    Ok(tool_status_from_config(&state.storage.load_config().await?))
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
    use crate::auth::bilibili::BilibiliAuth;
    use crate::auth::session_store::SessionStore;
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

    #[tokio::test]
    async fn exposes_flat_platform_login_rows() {
        let state = command_test_state().await;
        let rows = list_platform_logins_from_state(&state).unwrap();

        assert_eq!(
            rows,
            vec![PlatformLoginRow {
                platform: "bilibili".into(),
                status: "未登录".into(),
            }]
        );
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn clear_bilibili_login_does_not_touch_external_session_before_app_state() {
        let dir = std::env::temp_dir().join(format!(
            "video-downloader-external-session-{}",
            uuid::Uuid::new_v4()
        ));
        let auth = BilibiliAuth::new(SessionStore::new(dir.clone()));
        auth.save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();
        assert_eq!(auth.status().unwrap().status, "已登录");

        let state = command_test_state().await;
        clear_bilibili_login_from_state(&state).await.unwrap();

        assert_eq!(auth.status().unwrap().status, "已登录");
        cleanup_state(state).await;
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn get_config_returns_native_default() {
        let state = command_test_state().await;
        let config = get_config_from_state(&state).await.unwrap();

        assert_eq!(config.default_engine, DownloadEngine::Native);
        assert_eq!(config.concurrency, 2);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn get_config_reads_persisted_config_from_app_state() {
        let state = command_test_state().await;
        let config = AppConfig {
            download_root: "E:\\Videos".into(),
            concurrency: 5,
            default_engine: DownloadEngine::YtDlp,
            ytdlp_path: Some("C:\\tools\\yt-dlp.exe".into()),
        };
        state.storage.save_config(&config).await.unwrap();

        let loaded = get_config_from_state(&state).await.unwrap();

        assert_eq!(loaded, config);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn get_tool_status_reports_default_missing_tools() {
        let state = command_test_state().await;
        let status = get_tool_status_from_state(&state).await.unwrap();

        assert_eq!(
            status,
            ToolStatus {
                ytdlp: "missing".into(),
                ffmpeg: "missing".into(),
                ffprobe: "missing".into(),
            }
        );
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn get_tool_status_reads_persisted_ytdlp_path_from_app_state() {
        let state = command_test_state().await;
        let tool_dir =
            std::env::temp_dir().join(format!("vd-tool-status-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tool_dir).unwrap();
        let binary = tool_dir.join("yt-dlp.exe");
        fs::write(&binary, b"test binary").unwrap();
        let config = AppConfig {
            ytdlp_path: Some(binary.to_string_lossy().to_string()),
            ..AppConfig::default()
        };
        state.storage.save_config(&config).await.unwrap();

        let status = get_tool_status_from_state(&state).await.unwrap();

        assert_eq!(status.ytdlp, "available");
        assert_eq!(status.ffmpeg, "missing");
        assert_eq!(status.ffprobe, "missing");
        cleanup_state(state).await;
        fs::remove_dir_all(tool_dir).unwrap();
    }

    #[tokio::test]
    async fn list_platform_logins_reads_bilibili_auth_status_from_app_state() {
        let state = command_test_state().await;
        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();

        let rows = list_platform_logins_from_state(&state).unwrap();

        assert_eq!(
            rows,
            vec![PlatformLoginRow {
                platform: "bilibili".into(),
                status: "已登录".into(),
            }]
        );
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn clear_bilibili_login_clears_managed_session() {
        let state = command_test_state().await;
        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();

        clear_bilibili_login_from_state(&state).await.unwrap();

        assert_eq!(state.bilibili_auth.status().unwrap().status, "未登录");
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn confirmed_poll_outcome_saves_managed_session() {
        let state = command_test_state().await;

        let result = persist_bilibili_poll_outcome(
            &state,
            LoginPollOutcome {
                result: LoginPollResult {
                    status: "confirmed".into(),
                    message: "登录成功".into(),
                },
                cookies: Some("SESSDATA=auth-cookie".into()),
            },
        )
        .unwrap();

        assert_eq!(result.status, "confirmed");
        assert_eq!(
            state.bilibili_auth.load_cookie_string().unwrap(),
            Some("SESSDATA=auth-cookie".into())
        );
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn pending_poll_outcome_does_not_save_session() {
        let state = command_test_state().await;

        let result = persist_bilibili_poll_outcome(
            &state,
            LoginPollOutcome {
                result: LoginPollResult {
                    status: "pending".into(),
                    message: "未扫码".into(),
                },
                cookies: None,
            },
        )
        .unwrap();

        assert_eq!(result.status, "pending");
        assert!(state.bilibili_auth.load_cookie_string().unwrap().is_none());
        cleanup_state(state).await;
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

    async fn command_test_state() -> crate::app_state::AppState {
        crate::app_state::AppState::new(command_test_dir())
            .await
            .unwrap()
    }

    fn command_test_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vd-command-state-{}", uuid::Uuid::new_v4()))
    }

    async fn cleanup_state(state: crate::app_state::AppState) {
        let dir = state.data_dir().to_path_buf();
        state.close().await;
        remove_dir_all_retry(&dir);
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
