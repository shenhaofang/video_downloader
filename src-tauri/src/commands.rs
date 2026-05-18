use crate::app_state::AppState;
use crate::auth::bilibili::{
    poll_login_qr, request_login_qr, verify_login_cookies, LoginPollOutcome, LoginPollResult,
    LoginQr,
};
use crate::errors::AppResult;
use crate::models::{AppConfig, DownloadEngine};
use crate::platform::bilibili::native::NativeBilibiliDownloader;
use crate::platform::bilibili::yt_dlp::{detect_ytdlp, YtDlpStatus};
use crate::platform::PlatformDownloader;
use crate::task::{create_group_from_probe, CreateTaskRequest, CreatedTaskGroup};
use serde::{Deserialize, Serialize};
use std::future::Future;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskCommand {
    pub url: String,
    pub output_dir: String,
    pub has_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTaskCommand {
    pub task_id: String,
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
pub async fn save_config(
    state: tauri::State<'_, AppState>,
    input: AppConfig,
) -> AppResult<AppConfig> {
    save_config_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn create_task(
    state: tauri::State<'_, AppState>,
    input: CreateTaskCommand,
) -> AppResult<CreatedTaskGroup> {
    create_task_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn list_task_groups(
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<CreatedTaskGroup>> {
    list_task_groups_from_state(state.inner()).await
}

#[tauri::command]
pub async fn run_task(
    state: tauri::State<'_, AppState>,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    run_task_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn list_platform_logins(
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<PlatformLoginRow>> {
    list_platform_logins_from_state(state.inner()).await
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

async fn save_config_from_state(state: &AppState, input: AppConfig) -> AppResult<AppConfig> {
    let config = crate::config::with_normalized_concurrency(input);
    state.storage.save_config(&config).await?;
    Ok(config)
}

async fn create_task_from_state(
    state: &AppState,
    input: CreateTaskCommand,
) -> AppResult<CreatedTaskGroup> {
    let config = state.storage.load_config().await?;
    match config.default_engine {
        DownloadEngine::Native => {
            let downloader = NativeBilibiliDownloader::default();
            create_task_with_downloader_from_state(state, input, config.default_engine, &downloader)
                .await
        }
        DownloadEngine::YtDlp => Err(crate::errors::AppError::structured(
            crate::errors::ErrorCode::EngineMissing,
            "yt-dlp task creation is not wired yet",
        )),
    }
}

async fn create_task_with_downloader_from_state(
    state: &AppState,
    input: CreateTaskCommand,
    engine: DownloadEngine,
    downloader: &dyn PlatformDownloader,
) -> AppResult<CreatedTaskGroup> {
    let has_login = input.has_login || state.bilibili_auth.load_cookie_string()?.is_some();
    let result = create_group_from_probe(
        downloader,
        CreateTaskRequest {
            url: input.url,
            output_dir: input.output_dir,
            engine,
            has_login,
        },
    )
    .await?;
    state.storage.insert_group(&result.group).await?;
    for task in &result.tasks {
        state.storage.insert_task(task).await?;
    }

    Ok(result)
}

async fn list_task_groups_from_state(state: &AppState) -> AppResult<Vec<CreatedTaskGroup>> {
    let groups = state.storage.load_task_groups().await?;
    let mut results = Vec::with_capacity(groups.len());
    for group in groups {
        let tasks = state.storage.load_tasks_for_group(group.id).await?;
        results.push(CreatedTaskGroup { group, tasks });
    }

    Ok(results)
}

async fn run_task_from_state(
    state: &AppState,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    let task = load_task_from_command(state, &input).await?;
    match task.engine {
        DownloadEngine::Native => {
            let config = state.storage.load_config().await?;
            let downloader = NativeBilibiliDownloader::with_ffmpeg_path(
                config.ffmpeg_path.map(std::path::PathBuf::from),
            );
            run_task_with_downloader_from_state(state, input, &downloader).await
        }
        DownloadEngine::YtDlp => Err(crate::errors::AppError::structured(
            crate::errors::ErrorCode::EngineMissing,
            "yt-dlp task execution is not wired yet",
        )),
    }
}

async fn run_task_with_downloader_from_state(
    state: &AppState,
    input: RunTaskCommand,
    downloader: &dyn PlatformDownloader,
) -> AppResult<crate::models::DownloadTask> {
    let task = load_task_from_command(state, &input).await?;
    crate::task::executor::run_task_once(&state.storage, task, downloader).await
}

async fn load_task_from_command(
    state: &AppState,
    input: &RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    let task_id = Uuid::parse_str(&input.task_id).map_err(|err| {
        crate::errors::AppError::structured(
            crate::errors::ErrorCode::FilesystemError,
            err.to_string(),
        )
    })?;

    state.storage.load_task(task_id).await
}

async fn list_platform_logins_from_state(state: &AppState) -> AppResult<Vec<PlatformLoginRow>> {
    let client = reqwest::Client::new();
    list_platform_logins_with_verifier_from_state(state, |cookies| {
        let client = client.clone();
        async move { verify_login_cookies(&client, &cookies).await }
    })
    .await
}

async fn list_platform_logins_with_verifier_from_state<F, Fut>(
    state: &AppState,
    verifier: F,
) -> AppResult<Vec<PlatformLoginRow>>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = AppResult<bool>>,
{
    let status = match state.bilibili_auth.load_cookie_string()? {
        Some(cookies) => match verifier(cookies).await {
            Ok(true) => "已登录",
            Ok(false) => {
                state.bilibili_auth.clear()?;
                "未登录"
            }
            Err(err) if err.code() == crate::errors::ErrorCode::NetworkError => "待验证",
            Err(err) => return Err(err),
        },
        None => "未登录",
    };
    Ok(vec![PlatformLoginRow {
        platform: "bilibili".into(),
        status: status.into(),
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
        ffmpeg: configured_tool_status(config.ffmpeg_path.as_deref()).into(),
        ffprobe: configured_tool_status(config.ffprobe_path.as_deref()).into(),
    }
}

fn configured_tool_status(path: Option<&str>) -> &'static str {
    path.filter(|value| std::path::Path::new(value).is_file())
        .map(|_| "available")
        .unwrap_or("missing")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::bilibili::BilibiliAuth;
    use crate::auth::session_store::SessionStore;
    use crate::errors::AppResult;
    use crate::models::DownloadEngine;
    use crate::platform::mock::MockDownloader;
    use crate::platform::{
        DownloadInput, DownloadItem, DownloadItemMetadata, DownloadOutput, EventSink,
        PlatformDownloader, ProbeInput, ProbeResult,
    };
    use std::fs;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn create_task_uses_mock_collection() {
        let state = command_test_state().await;
        let result = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: true,
            },
            DownloadEngine::Native,
            &MockDownloader,
        )
        .await
        .unwrap();

        assert_eq!(result.tasks.len(), 3);
        let first_file_name = std::path::Path::new(&result.tasks[0].output_file)
            .file_name()
            .unwrap()
            .to_string_lossy();
        assert_eq!(first_file_name, "01 - 安装 Tauri.mp4");
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn create_task_from_state_uses_config_login_and_persists_tasks() {
        let state = command_test_state().await;
        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();

        let result = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();

        assert_eq!(result.group.engine, DownloadEngine::Native);
        assert_eq!(result.tasks[0].bvid.as_deref(), Some("BV1xx411c7mD"));
        assert!(result.tasks[0].used_login);

        let persisted = state
            .storage
            .load_tasks_for_group(result.group.id)
            .await
            .unwrap();
        assert_eq!(persisted, result.tasks);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn create_task_from_state_reports_missing_ytdlp_creation_path() {
        let state = command_test_state().await;
        state
            .storage
            .save_config(&AppConfig {
                default_engine: DownloadEngine::YtDlp,
                ..AppConfig::default()
            })
            .await
            .unwrap();

        let err = create_task_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), crate::errors::ErrorCode::EngineMissing);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn run_task_with_downloader_from_state_loads_persisted_task_and_updates_storage() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();
        let downloader = CommandRunDownloader::default();

        let updated = run_task_with_downloader_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
            &downloader,
        )
        .await
        .unwrap();

        assert_eq!(updated.id, task.id);
        assert_eq!(updated.state, crate::models::TaskState::Completed);
        assert_eq!(updated.bytes_downloaded, 9);
        assert_eq!(updated.bytes_total, Some(9));
        let persisted = state.storage.load_task(task.id).await.unwrap();
        assert_eq!(persisted, updated);
        let input = downloader.input().unwrap();
        assert_eq!(input.output_path, task.output_file);
        assert_eq!(input.item.metadata.unwrap().bvid, "BV1xx411c7mD");
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn run_task_from_state_persists_missing_ffmpeg_failure_for_native_task() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();

        let err = run_task_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), crate::errors::ErrorCode::FfmpegError);
        let persisted = state.storage.load_task(task.id).await.unwrap();
        assert_eq!(persisted.state, crate::models::TaskState::Failed);
        assert_eq!(persisted.error_code.as_deref(), Some("ffmpeg_error"));
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn list_task_groups_from_state_returns_persisted_groups_with_tasks() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();

        let groups = list_task_groups_from_state(&state).await.unwrap();

        assert_eq!(groups, vec![created]);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn exposes_flat_platform_login_rows() {
        let state = command_test_state().await;
        let rows = list_platform_logins_from_state(&state).await.unwrap();

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
            ffmpeg_path: Some("C:\\tools\\ffmpeg.exe".into()),
            ffprobe_path: Some("C:\\tools\\ffprobe.exe".into()),
        };
        state.storage.save_config(&config).await.unwrap();

        let loaded = get_config_from_state(&state).await.unwrap();

        assert_eq!(loaded, config);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn save_config_from_state_persists_normalized_settings() {
        let state = command_test_state().await;
        let config = AppConfig {
            download_root: "E:\\Downloads".into(),
            concurrency: 99,
            default_engine: DownloadEngine::YtDlp,
            ytdlp_path: Some("C:\\tools\\yt-dlp.exe".into()),
            ffmpeg_path: Some("C:\\tools\\ffmpeg.exe".into()),
            ffprobe_path: Some("C:\\tools\\ffprobe.exe".into()),
        };

        let saved = save_config_from_state(&state, config).await.unwrap();

        assert_eq!(saved.download_root, "E:\\Downloads");
        assert_eq!(saved.concurrency, 8);
        assert_eq!(saved.default_engine, DownloadEngine::YtDlp);
        assert_eq!(state.storage.load_config().await.unwrap(), saved);
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
    async fn get_tool_status_reads_persisted_media_tool_paths_from_app_state() {
        let state = command_test_state().await;
        let tool_dir =
            std::env::temp_dir().join(format!("vd-media-tool-status-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tool_dir).unwrap();
        let ffmpeg = tool_dir.join("ffmpeg.exe");
        let ffprobe = tool_dir.join("ffprobe.exe");
        fs::write(&ffmpeg, b"test binary").unwrap();
        fs::write(&ffprobe, b"test binary").unwrap();
        let config = AppConfig {
            ffmpeg_path: Some(ffmpeg.to_string_lossy().to_string()),
            ffprobe_path: Some(ffprobe.to_string_lossy().to_string()),
            ..AppConfig::default()
        };
        state.storage.save_config(&config).await.unwrap();

        let status = get_tool_status_from_state(&state).await.unwrap();

        assert_eq!(status.ffmpeg, "available");
        assert_eq!(status.ffprobe, "available");
        cleanup_state(state).await;
        fs::remove_dir_all(tool_dir).unwrap();
    }

    #[tokio::test]
    async fn list_platform_logins_verifies_bilibili_auth_status_from_app_state() {
        let state = command_test_state().await;
        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();

        let rows = list_platform_logins_with_verifier_from_state(&state, |cookies| async move {
            Ok(cookies == "SESSDATA=auth-cookie")
        })
        .await
        .unwrap();

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
    async fn list_platform_logins_clears_invalid_bilibili_auth_status() {
        let state = command_test_state().await;
        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=expired-cookie".into())
            .unwrap();

        let rows =
            list_platform_logins_with_verifier_from_state(&state, |_cookies| async { Ok(false) })
                .await
                .unwrap();

        assert_eq!(
            rows,
            vec![PlatformLoginRow {
                platform: "bilibili".into(),
                status: "未登录".into(),
            }]
        );
        assert!(state.bilibili_auth.load_cookie_string().unwrap().is_none());
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn list_platform_logins_keeps_session_when_verification_network_fails() {
        let state = command_test_state().await;
        state
            .bilibili_auth
            .save_cookie_string("SESSDATA=auth-cookie".into())
            .unwrap();

        let rows = list_platform_logins_with_verifier_from_state(&state, |_cookies| async {
            Err(crate::errors::AppError::structured(
                crate::errors::ErrorCode::NetworkError,
                "offline",
            ))
        })
        .await
        .unwrap();

        assert_eq!(
            rows,
            vec![PlatformLoginRow {
                platform: "bilibili".into(),
                status: "待验证".into(),
            }]
        );
        assert_eq!(
            state.bilibili_auth.load_cookie_string().unwrap(),
            Some("SESSDATA=auth-cookie".into())
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

    #[test]
    fn tool_status_uses_configured_media_tool_paths() {
        let dir =
            std::env::temp_dir().join(format!("vd-media-tool-status-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let ffmpeg = dir.join("ffmpeg.exe");
        let ffprobe = dir.join("ffprobe.exe");
        fs::write(&ffmpeg, b"test binary").unwrap();
        fs::write(&ffprobe, b"test binary").unwrap();
        let config = AppConfig {
            ffmpeg_path: Some(ffmpeg.to_string_lossy().to_string()),
            ffprobe_path: Some(ffprobe.to_string_lossy().to_string()),
            ..AppConfig::default()
        };

        let status = tool_status_from_config(&config);

        assert_eq!(status.ffmpeg, "available");
        assert_eq!(status.ffprobe, "available");
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

    struct RecordingDownloader;

    impl PlatformDownloader for RecordingDownloader {
        fn probe<'a>(
            &'a self,
            input: ProbeInput,
        ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
            Box::pin(async move {
                Ok(ProbeResult {
                    group_title: "Rust 桌面应用入门".into(),
                    used_login: input.has_login,
                    items: vec![DownloadItem {
                        title: "安装 Tauri".into(),
                        output_file: "安装 Tauri.mp4".into(),
                        quality: Some("1080P".into()),
                        requires_login: input.has_login,
                        bytes_total: None,
                        metadata: Some(DownloadItemMetadata {
                            bvid: "BV1xx411c7mD".into(),
                            cid: 111,
                            page: 1,
                        }),
                    }],
                })
            })
        }

        fn download<'a>(
            &'a self,
            _input: DownloadInput,
            _sink: &'a dyn EventSink,
        ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
            Box::pin(async { unreachable!("command create tests do not download") })
        }
    }

    #[derive(Default)]
    struct CommandRunDownloader {
        input: Arc<Mutex<Option<DownloadInput>>>,
    }

    impl CommandRunDownloader {
        fn input(&self) -> Option<DownloadInput> {
            self.input.lock().unwrap().clone()
        }
    }

    impl PlatformDownloader for CommandRunDownloader {
        fn probe<'a>(
            &'a self,
            _input: ProbeInput,
        ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
            Box::pin(async { unreachable!("command run tests do not probe") })
        }

        fn download<'a>(
            &'a self,
            input: DownloadInput,
            sink: &'a dyn EventSink,
        ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
            Box::pin(async move {
                *self.input.lock().unwrap() = Some(input);
                sink.emit(crate::platform::DownloadEvent::Progress {
                    downloaded: 9,
                    total: Some(9),
                });
                Ok(DownloadOutput {
                    output_path: "D:\\Videos\\out.mp4".into(),
                    quality: Some("720P".into()),
                    used_login: false,
                    bytes_total: Some(9),
                })
            })
        }
    }
}
