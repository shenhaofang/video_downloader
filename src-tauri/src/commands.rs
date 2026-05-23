use crate::app_state::AppState;
use crate::auth::bilibili::{
    poll_login_qr, request_login_qr, verify_login_cookies, LoginPollOutcome, LoginPollResult,
    LoginQr,
};
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::models::{AppConfig, DownloadEngine, DownloadTask, TaskState};
use crate::platform::bilibili::native::NativeBilibiliDownloader;
use crate::platform::bilibili::yt_dlp::{detect_ytdlp, YtDlpStatus};
use crate::platform::bilibili::yt_dlp::{require_ytdlp, YtDlpDownloader};
use crate::platform::{PlatformDownloader, ProbeInput, ProbeResult};
use crate::task::{create_group_from_probe, CreateTaskRequest, CreatedTaskGroup};
use crate::updater::AppUpdateStatus;
use futures_util::future::{AbortHandle, Abortable};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const YTDLP_DOWNLOAD_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const FFMPEG_DOWNLOAD_URL: &str =
    "https://github.com/shenhaofang/video_downloader/releases/latest/download/ffmpeg-win64-lgpl.zip";
const FFMPEG_ARCHIVE_SHA256: &str =
    "d3c0d41c26b64bb42abbf9051a9494bc67185b6d9fa57798f20efb0e0213caf7";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskCommand {
    pub url: String,
    pub output_dir: String,
    pub has_login: bool,
    pub selected_pages: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeTaskCommand {
    pub url: String,
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
pub async fn probe_bilibili_pages(
    state: tauri::State<'_, AppState>,
    input: ProbeTaskCommand,
) -> AppResult<ProbeResult> {
    probe_bilibili_pages_from_state(state.inner(), input).await
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
pub async fn start_task(
    state: tauri::State<'_, AppState>,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    start_task_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn retry_task(
    state: tauri::State<'_, AppState>,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    retry_task_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn pause_task(
    state: tauri::State<'_, AppState>,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    pause_task_from_state(state.inner(), input).await
}

#[tauri::command]
pub async fn delete_task(
    state: tauri::State<'_, AppState>,
    input: RunTaskCommand,
) -> AppResult<Vec<CreatedTaskGroup>> {
    delete_task_from_state(state.inner(), input).await
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

#[tauri::command]
pub async fn install_ytdlp(state: tauri::State<'_, AppState>) -> AppResult<AppConfig> {
    install_ytdlp_from_state(state.inner()).await
}

#[tauri::command]
pub async fn install_media_tools(state: tauri::State<'_, AppState>) -> AppResult<AppConfig> {
    install_media_tools_from_state(state.inner()).await
}

#[tauri::command]
pub async fn check_app_update(app: tauri::AppHandle) -> AppResult<AppUpdateStatus> {
    crate::updater::check_app_update(&app).await
}

#[tauri::command]
pub async fn install_app_update(app: tauri::AppHandle) -> AppResult<()> {
    crate::updater::install_app_update(&app).await
}

async fn get_config_from_state(state: &AppState) -> AppResult<AppConfig> {
    let config = state.storage.load_config().await?;
    config_with_installer_managed_tool_defaults(config)
}

async fn save_config_from_state(state: &AppState, input: AppConfig) -> AppResult<AppConfig> {
    let config = crate::config::with_normalized_concurrency(input);
    let config = config_with_installer_managed_tool_defaults(config)?;
    state.storage.save_config(&config).await?;
    Ok(config)
}

fn config_with_installer_managed_tool_defaults(mut config: AppConfig) -> AppResult<AppConfig> {
    config.ytdlp_path = Some(existing_or_default_tool_path(
        config.ytdlp_path,
        crate::media::installer_managed_ytdlp_path()?,
    ));
    config.ffmpeg_path = Some(existing_or_default_tool_path(
        config.ffmpeg_path,
        crate::media::installer_managed_media_tool_path("ffmpeg")?,
    ));
    config.ffprobe_path = Some(existing_or_default_tool_path(
        config.ffprobe_path,
        crate::media::installer_managed_media_tool_path("ffprobe")?,
    ));
    Ok(config)
}

fn existing_or_default_tool_path(path: Option<String>, default_path: PathBuf) -> String {
    path.filter(|value| Path::new(value).is_file())
        .unwrap_or_else(|| default_path.to_string_lossy().to_string())
}

async fn install_ytdlp_from_state(state: &AppState) -> AppResult<AppConfig> {
    let bytes = download_url_bytes(YTDLP_DOWNLOAD_URL, "yt-dlp").await?;
    install_ytdlp_bytes_from_state(state, bytes).await
}

async fn install_media_tools_from_state(state: &AppState) -> AppResult<AppConfig> {
    let bytes = download_url_bytes(FFMPEG_DOWNLOAD_URL, "FFmpeg").await?;
    install_media_tools_bytes_from_state(state, bytes).await
}

async fn download_url_bytes(url: &str, label: &str) -> AppResult<Vec<u8>> {
    let response = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "video-downloader")
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;

    if !response.status().is_success() {
        return Err(AppError::structured(
            ErrorCode::NetworkError,
            format!("{label} download failed with status {}", response.status()),
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;
    if bytes.is_empty() {
        return Err(AppError::structured(
            ErrorCode::NetworkError,
            format!("{label} download returned an empty file"),
        ));
    }

    Ok(bytes.to_vec())
}

async fn install_ytdlp_bytes_from_state(state: &AppState, bytes: Vec<u8>) -> AppResult<AppConfig> {
    install_ytdlp_bytes_to_path_from_state(
        state,
        bytes,
        crate::media::installer_managed_ytdlp_path()?,
    )
    .await
}

async fn install_ytdlp_bytes_to_path_from_state(
    state: &AppState,
    bytes: Vec<u8>,
    path: PathBuf,
) -> AppResult<AppConfig> {
    if bytes.is_empty() {
        return Err(AppError::structured(
            ErrorCode::NetworkError,
            "yt-dlp download returned an empty file",
        ));
    }

    let install_dir = path.parent().ok_or_else(|| {
        AppError::structured(
            ErrorCode::FilesystemError,
            "yt-dlp install path has no parent directory",
        )
    })?;
    std::fs::create_dir_all(install_dir)
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    std::fs::write(&path, bytes)
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;

    let mut config = state.storage.load_config().await?;
    config.ytdlp_path = Some(path.to_string_lossy().to_string());
    let config = crate::config::with_normalized_concurrency(config);
    let config = config_with_installer_managed_tool_defaults(config)?;
    state.storage.save_config(&config).await?;
    Ok(config)
}

async fn install_media_tools_bytes_from_state(
    state: &AppState,
    bytes: Vec<u8>,
) -> AppResult<AppConfig> {
    let install_root = crate::media::installer_managed_media_tool_root()?;
    install_media_tools_bytes_to_path_from_state(state, bytes, install_root).await
}

async fn install_media_tools_bytes_to_path_from_state(
    state: &AppState,
    bytes: Vec<u8>,
    install_root: PathBuf,
) -> AppResult<AppConfig> {
    install_media_tools_archive_to_path(&bytes, FFMPEG_ARCHIVE_SHA256, &install_root)?;

    let ffmpeg_path = install_root.join("bin").join("ffmpeg.exe");
    let ffprobe_path = install_root.join("bin").join("ffprobe.exe");
    let mut config = state.storage.load_config().await?;
    config.ffmpeg_path = Some(ffmpeg_path.to_string_lossy().to_string());
    config.ffprobe_path = Some(ffprobe_path.to_string_lossy().to_string());
    let config = crate::config::with_normalized_concurrency(config);
    let config = config_with_installer_managed_tool_defaults(config)?;
    state.storage.save_config(&config).await?;
    Ok(config)
}

fn install_media_tools_archive_to_path(
    bytes: &[u8],
    expected_sha256: &str,
    install_root: &Path,
) -> AppResult<()> {
    verify_archive_sha256(bytes, expected_sha256)?;

    let parent = install_root.parent().ok_or_else(|| {
        AppError::structured(
            ErrorCode::FilesystemError,
            "FFmpeg install path has no parent directory",
        )
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    let temp_root = parent.join(format!(".ffmpeg-install-{}", Uuid::new_v4()));
    if temp_root.exists() {
        std::fs::remove_dir_all(&temp_root)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    }
    std::fs::create_dir_all(&temp_root)
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;

    let result = extract_ffmpeg_archive(bytes, &temp_root).and_then(|_| {
        ensure_media_tools_exist(&temp_root)?;
        if install_root.exists() {
            std::fs::remove_dir_all(install_root)
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        }
        std::fs::rename(&temp_root, install_root)
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))
    });

    if result.is_err() && temp_root.exists() {
        let _ = std::fs::remove_dir_all(&temp_root);
    }
    result
}

fn verify_archive_sha256(bytes: &[u8], expected_sha256: &str) -> AppResult<()> {
    if bytes.is_empty() {
        return Err(AppError::structured(
            ErrorCode::NetworkError,
            "FFmpeg download returned an empty file",
        ));
    }

    use sha2::{Digest, Sha256};
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected_sha256.to_ascii_lowercase() {
        return Err(AppError::structured(
            ErrorCode::UpdateError,
            format!("FFmpeg archive checksum mismatch. Expected {expected_sha256}, got {actual}"),
        ));
    }
    Ok(())
}

fn extract_ffmpeg_archive(bytes: &[u8], target_root: &Path) -> AppResult<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|err| AppError::structured(ErrorCode::UpdateError, err.to_string()))?;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|err| AppError::structured(ErrorCode::UpdateError, err.to_string()))?;
        let enclosed = file.enclosed_name().ok_or_else(|| {
            AppError::structured(
                ErrorCode::UpdateError,
                "FFmpeg archive contains unsafe path",
            )
        })?;
        let relative = strip_archive_root(&enclosed);
        if relative.as_os_str().is_empty() {
            continue;
        }
        let output = target_root.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(&output)
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        } else {
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    AppError::structured(ErrorCode::FilesystemError, err.to_string())
                })?;
            }
            let mut output_file = std::fs::File::create(&output)
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
            std::io::copy(&mut file, &mut output_file)
                .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        }
    }
    Ok(())
}

fn strip_archive_root(path: &Path) -> PathBuf {
    let mut components = path.components();
    if components
        .next()
        .is_some_and(|component| component.as_os_str() == "bin")
    {
        path.to_path_buf()
    } else {
        path.components().skip(1).collect()
    }
}

fn ensure_media_tools_exist(root: &Path) -> AppResult<()> {
    for tool in ["ffmpeg.exe", "ffprobe.exe"] {
        let path = root.join("bin").join(tool);
        if !path.is_file() {
            return Err(AppError::structured(
                ErrorCode::UpdateError,
                format!("{tool} missing from FFmpeg archive"),
            ));
        }
    }
    Ok(())
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
        DownloadEngine::YtDlp => {
            let ytdlp_path = require_ytdlp(config.ytdlp_path.as_deref())?;
            let downloader = YtDlpDownloader::new(ytdlp_path);
            create_task_with_downloader_from_state(state, input, config.default_engine, &downloader)
                .await
        }
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
            selected_pages: input.selected_pages,
        },
    )
    .await?;
    state.storage.insert_group(&result.group).await?;
    for task in &result.tasks {
        state.storage.insert_task(task).await?;
    }

    Ok(result)
}

async fn probe_bilibili_pages_from_state(
    state: &AppState,
    input: ProbeTaskCommand,
) -> AppResult<ProbeResult> {
    let downloader = NativeBilibiliDownloader::default();
    probe_bilibili_pages_with_downloader_from_state(state, input, &downloader).await
}

async fn probe_bilibili_pages_with_downloader_from_state(
    state: &AppState,
    input: ProbeTaskCommand,
    downloader: &dyn PlatformDownloader,
) -> AppResult<ProbeResult> {
    let has_login = input.has_login || state.bilibili_auth.load_cookie_string()?.is_some();
    downloader
        .probe(ProbeInput {
            url: input.url,
            engine: DownloadEngine::Native,
            has_login,
        })
        .await
}

async fn list_task_groups_from_state(state: &AppState) -> AppResult<Vec<CreatedTaskGroup>> {
    let groups = state.storage.load_task_groups().await?;
    let mut results = Vec::with_capacity(groups.len());
    for group in groups {
        let tasks = state.storage.load_tasks_for_group(group.id).await?;
        let tasks = recover_stale_active_tasks(state, tasks).await?;
        results.push(CreatedTaskGroup { group, tasks });
    }

    Ok(results)
}

async fn recover_stale_active_tasks(
    state: &AppState,
    tasks: Vec<DownloadTask>,
) -> AppResult<Vec<DownloadTask>> {
    let mut recovered = Vec::with_capacity(tasks.len());
    for mut task in tasks {
        if is_runtime_active_state(task.state) && !state.is_task_active(task.id) {
            task.state = TaskState::Interrupted;
            state.storage.update_task(&task).await?;
        }
        recovered.push(task);
    }
    Ok(recovered)
}

fn is_runtime_active_state(state: TaskState) -> bool {
    matches!(
        state,
        TaskState::Pending | TaskState::Probing | TaskState::Downloading | TaskState::Merging
    )
}

async fn run_task_from_state(
    state: &AppState,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    run_task_from_state_mode(state, input, false).await
}

async fn start_task_from_state(
    state: &AppState,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    run_task_from_state_mode(state, input, true).await
}

async fn retry_task_from_state(
    state: &AppState,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    let mut task = load_task_from_command(state, &input).await?;
    task.state = TaskState::Queued;
    task.bytes_downloaded = 0;
    task.bytes_total = None;
    task.error_code = None;
    task.error_message = None;
    state.storage.update_task(&task).await?;
    run_task_from_state_mode(state, input, true).await
}

async fn pause_task_from_state(
    state: &AppState,
    input: RunTaskCommand,
) -> AppResult<crate::models::DownloadTask> {
    let mut task = load_task_from_command(state, &input).await?;
    state.abort_task(task.id);
    if matches!(
        task.state,
        TaskState::Pending
            | TaskState::Probing
            | TaskState::Queued
            | TaskState::Downloading
            | TaskState::Merging
            | TaskState::Interrupted
    ) {
        task.state = TaskState::Paused;
        state.storage.update_task(&task).await?;
    }
    Ok(task)
}

async fn delete_task_from_state(
    state: &AppState,
    input: RunTaskCommand,
) -> AppResult<Vec<CreatedTaskGroup>> {
    let task = load_task_from_command(state, &input).await?;
    let was_active = state.abort_task(task.id);
    let cleanup_result = cleanup_task_resume_files(&task);
    if !was_active {
        cleanup_result?;
    }
    state.storage.delete_task(task.id).await?;
    list_task_groups_from_state(state).await
}

fn cleanup_task_resume_files(task: &DownloadTask) -> AppResult<()> {
    match task.engine {
        DownloadEngine::Native => {
            let output_path = Path::new(&task.output_file);
            let workspace = output_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".video-downloader")
                .join(task.id.to_string());
            remove_dir_if_exists(&workspace)?;
        }
        DownloadEngine::YtDlp => {
            for path in ytdlp_partial_paths(Path::new(&task.output_file)) {
                remove_file_if_exists(&path)?;
            }
        }
    }
    Ok(())
}

fn ytdlp_partial_paths(output_path: &Path) -> Vec<PathBuf> {
    let output = output_path.to_string_lossy();
    let mut paths: Vec<PathBuf> = [".part", ".ytdl", ".temp", ".frag"]
        .into_iter()
        .map(|suffix| PathBuf::from(format!("{output}{suffix}")))
        .collect();
    let Some(parent) = output_path.parent() else {
        return paths;
    };
    let Some(stem) = output_path.file_stem().and_then(|value| value.to_str()) else {
        return paths;
    };
    let Some(file_name) = output_path.file_name().and_then(|value| value.to_str()) else {
        return paths;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return paths;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || paths.contains(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if is_ytdlp_partial_name(name, stem, file_name) {
            paths.push(path);
        }
    }
    paths
}

fn is_ytdlp_partial_name(name: &str, output_stem: &str, output_file_name: &str) -> bool {
    if name.starts_with(output_file_name) {
        return name.ends_with(".part")
            || name.ends_with(".ytdl")
            || name.ends_with(".temp")
            || name.ends_with(".frag")
            || name.contains(".part-");
    }

    let Some(rest) = name.strip_prefix(output_stem) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(".f") else {
        return false;
    };
    let format_id_len = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .count();
    if format_id_len == 0 {
        return false;
    }
    let suffix = &rest[format_id_len..];
    suffix.ends_with(".part") || suffix.contains(".part-")
}

fn remove_file_if_exists(path: &Path) -> AppResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AppError::structured(
            ErrorCode::FilesystemError,
            err.to_string(),
        )),
    }
}

fn remove_dir_if_exists(path: &Path) -> AppResult<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AppError::structured(
            ErrorCode::FilesystemError,
            err.to_string(),
        )),
    }
}

async fn run_task_from_state_mode(
    state: &AppState,
    input: RunTaskCommand,
    allow_paused: bool,
) -> AppResult<crate::models::DownloadTask> {
    let task = load_task_from_command(state, &input).await?;
    if !should_run_task(&task, allow_paused) {
        return Ok(task);
    }
    match task.engine {
        DownloadEngine::Native => {
            let config = state.storage.load_config().await?;
            let ffmpeg_path = crate::media::media_tool_path(
                config.ffmpeg_path.map(std::path::PathBuf::from),
                "ffmpeg",
            )?;
            let downloader = NativeBilibiliDownloader::with_ffmpeg_path(ffmpeg_path);
            run_task_with_downloader_from_state_mode(state, input, &downloader, allow_paused).await
        }
        DownloadEngine::YtDlp => {
            let config = state.storage.load_config().await?;
            let ytdlp_path = require_ytdlp(config.ytdlp_path.as_deref())?;
            let downloader = YtDlpDownloader::new(ytdlp_path);
            run_task_with_downloader_from_state_mode(state, input, &downloader, allow_paused).await
        }
    }
}

#[cfg(test)]
async fn run_task_with_downloader_from_state(
    state: &AppState,
    input: RunTaskCommand,
    downloader: &dyn PlatformDownloader,
) -> AppResult<crate::models::DownloadTask> {
    run_task_with_downloader_from_state_mode(state, input, downloader, false).await
}

#[cfg(test)]
async fn start_task_with_downloader_from_state(
    state: &AppState,
    input: RunTaskCommand,
    downloader: &dyn PlatformDownloader,
) -> AppResult<crate::models::DownloadTask> {
    run_task_with_downloader_from_state_mode(state, input, downloader, true).await
}

#[cfg(test)]
async fn retry_task_with_downloader_from_state(
    state: &AppState,
    input: RunTaskCommand,
    downloader: &dyn PlatformDownloader,
) -> AppResult<crate::models::DownloadTask> {
    let mut task = load_task_from_command(state, &input).await?;
    task.state = TaskState::Queued;
    task.bytes_downloaded = 0;
    task.bytes_total = None;
    task.error_code = None;
    task.error_message = None;
    state.storage.update_task(&task).await?;
    run_task_with_downloader_from_state_mode(state, input, downloader, false).await
}

async fn run_task_with_downloader_from_state_mode(
    state: &AppState,
    input: RunTaskCommand,
    downloader: &dyn PlatformDownloader,
    allow_paused: bool,
) -> AppResult<crate::models::DownloadTask> {
    let task = load_task_from_command(state, &input).await?;
    if !should_run_task(&task, allow_paused) {
        return Ok(task);
    }

    let task_id = task.id;
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    if !state.register_task_abort(task_id, abort_handle) {
        return state.storage.load_task(task_id).await;
    }
    let result = Abortable::new(
        crate::task::executor::run_task_once(&state.storage, task, downloader),
        abort_registration,
    )
    .await;
    state.clear_task_abort(task_id);
    match result {
        Ok(result) => result,
        Err(_) => {
            let mut paused = state.storage.load_task(task_id).await?;
            paused.state = TaskState::Paused;
            state.storage.update_task(&paused).await?;
            Ok(paused)
        }
    }
}

fn should_run_task(task: &DownloadTask, allow_paused: bool) -> bool {
    matches!(task.state, TaskState::Queued | TaskState::Interrupted)
        || (allow_paused && matches!(task.state, TaskState::Paused))
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
    tool_status_from_config_with_installed_media(config, |tool| {
        crate::media::media_tool_path(None, tool).ok().flatten()
    })
}

fn tool_status_from_config_with_installed_media<F>(
    config: &AppConfig,
    installed_path: F,
) -> ToolStatus
where
    F: Fn(&str) -> Option<std::path::PathBuf>,
{
    let ytdlp = match detect_ytdlp(config.ytdlp_path.as_deref()) {
        YtDlpStatus::Available { .. } => "available",
        YtDlpStatus::Missing => "missing",
    };

    ToolStatus {
        ytdlp: ytdlp.into(),
        ffmpeg: media_tool_status(config.ffmpeg_path.as_deref(), installed_path("ffmpeg")).into(),
        ffprobe: media_tool_status(config.ffprobe_path.as_deref(), installed_path("ffprobe"))
            .into(),
    }
}

fn configured_tool_status(path: Option<&str>) -> &'static str {
    path.filter(|value| std::path::Path::new(value).is_file())
        .map(|_| "available")
        .unwrap_or("missing")
}

fn media_tool_status(
    configured_path: Option<&str>,
    installed_path: Option<std::path::PathBuf>,
) -> &'static str {
    if configured_tool_status(configured_path) == "available" {
        return "available";
    }
    installed_path
        .filter(|value| value.is_file())
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
                selected_pages: None,
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
                selected_pages: None,
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
                selected_pages: None,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), crate::errors::ErrorCode::EngineMissing);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn create_and_run_task_from_state_uses_configured_ytdlp() {
        let state = command_test_state().await;
        let dir = command_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let ytdlp = dir.join("fake-ytdlp.bat");
        fs::write(
            &ytdlp,
            "@echo off\r\necho %* | findstr /C:\"--dump-json\" >nul\r\nif %errorlevel%==0 (\r\n  echo {\"title\":\"YTDLP Sample\",\"filesize\":123}\r\n  exit /b 0\r\n)\r\necho downloaded with continue\r\nexit /b 0\r\n",
        )
        .unwrap();
        state
            .storage
            .save_config(&AppConfig {
                download_root: dir.to_string_lossy().to_string(),
                default_engine: DownloadEngine::YtDlp,
                ytdlp_path: Some(ytdlp.to_string_lossy().to_string()),
                ..AppConfig::default()
            })
            .await
            .unwrap();

        let created = create_task_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: dir.to_string_lossy().to_string(),
                has_login: false,
                selected_pages: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(created.group.engine, DownloadEngine::YtDlp);
        assert_eq!(created.tasks.len(), 1);
        assert_eq!(created.tasks[0].engine, DownloadEngine::YtDlp);
        assert_eq!(created.tasks[0].title, "YTDLP Sample");
        assert_eq!(created.tasks[0].bvid, None);

        let updated = run_task_from_state(
            &state,
            RunTaskCommand {
                task_id: created.tasks[0].id.to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.state, TaskState::Completed);
        let logs = state
            .storage
            .load_logs_for_task(created.tasks[0].id)
            .await
            .unwrap();
        assert!(logs
            .iter()
            .any(|line| line.contains("downloaded with continue")));
        cleanup_state(state).await;
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn probe_bilibili_pages_from_state_returns_items_without_persisting_tasks() {
        let state = command_test_state().await;

        let result = probe_bilibili_pages_with_downloader_from_state(
            &state,
            ProbeTaskCommand {
                url: "https://www.bilibili.com/video/BV17KxizLE17?p=58".into(),
                has_login: false,
            },
            &MultiPageProbeDownloader,
        )
        .await
        .unwrap();

        assert_eq!(result.group_title, "剑桥少儿英语PowerUp 2nd Edition");
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].metadata.as_ref().unwrap().page, 58);
        assert_eq!(result.items[1].metadata.as_ref().unwrap().page, 59);
        assert!(state.storage.load_task_groups().await.unwrap().is_empty());
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
                selected_pages: None,
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
    async fn automatic_run_skips_paused_task_until_manually_started() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let mut task = created.tasks[0].clone();
        task.state = crate::models::TaskState::Paused;
        state.storage.update_task(&task).await.unwrap();
        let downloader = CommandRunDownloader::default();

        let automatic = run_task_with_downloader_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
            &downloader,
        )
        .await
        .unwrap();

        assert_eq!(automatic.state, crate::models::TaskState::Paused);
        assert!(downloader.input().is_none());

        let manual = start_task_with_downloader_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
            &downloader,
        )
        .await
        .unwrap();

        assert_eq!(manual.state, crate::models::TaskState::Completed);
        assert!(downloader.input().is_some());
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn start_task_does_not_duplicate_an_active_run() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();
        let (abort_handle, _abort_registration) = AbortHandle::new_pair();
        assert!(state.register_task_abort(task.id, abort_handle));
        let downloader = CommandRunDownloader::default();

        let started = start_task_with_downloader_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
            &downloader,
        )
        .await
        .unwrap();

        assert_eq!(started.id, task.id);
        assert!(downloader.input().is_none());
        state.clear_task_abort(task.id);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn pause_task_from_state_marks_task_paused() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();

        let paused = pause_task_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(paused.state, crate::models::TaskState::Paused);
        assert_eq!(
            state.storage.load_task(task.id).await.unwrap().state,
            crate::models::TaskState::Paused
        );
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn retry_task_clears_error_and_runs_failed_task() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let mut task = created.tasks[0].clone();
        task.state = crate::models::TaskState::Failed;
        task.error_code = Some("network_error".into());
        task.error_message = Some("download failed".into());
        state.storage.update_task(&task).await.unwrap();
        let downloader = CommandRunDownloader::default();

        let retried = retry_task_with_downloader_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
            &downloader,
        )
        .await
        .unwrap();

        assert_eq!(retried.state, crate::models::TaskState::Completed);
        assert_eq!(retried.error_code, None);
        assert_eq!(retried.error_message, None);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn delete_task_from_state_removes_single_child_and_empty_group() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();

        let groups = delete_task_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
        )
        .await
        .unwrap();

        assert!(groups.is_empty());
        assert!(state.storage.load_task_groups().await.unwrap().is_empty());
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn delete_task_from_state_removes_native_resume_workspace() {
        let state = command_test_state().await;
        let dir = command_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: dir.to_string_lossy().to_string(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();
        let workspace = std::path::Path::new(&task.output_file)
            .parent()
            .unwrap()
            .join(".video-downloader")
            .join(task.id.to_string());
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("video.part"), b"partial").unwrap();

        delete_task_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
        )
        .await
        .unwrap();

        assert!(!workspace.exists());
        cleanup_state(state).await;
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn delete_active_task_tolerates_resume_cleanup_failure() {
        let state = command_test_state().await;
        let dir = command_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: dir.to_string_lossy().to_string(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();
        let workspace = std::path::Path::new(&task.output_file)
            .parent()
            .unwrap()
            .join(".video-downloader")
            .join(task.id.to_string());
        fs::create_dir_all(workspace.parent().unwrap()).unwrap();
        fs::write(&workspace, b"not a directory").unwrap();
        let (abort_handle, _abort_registration) = AbortHandle::new_pair();
        assert!(state.register_task_abort(task.id, abort_handle));

        let groups = delete_task_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
        )
        .await
        .unwrap();

        assert!(groups.is_empty());
        assert!(state.storage.load_task_groups().await.unwrap().is_empty());
        cleanup_state(state).await;
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn delete_task_from_state_removes_ytdlp_partial_files() {
        let state = command_test_state().await;
        let dir = command_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: dir.to_string_lossy().to_string(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::YtDlp,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let task = created.tasks[0].clone();
        fs::create_dir_all(std::path::Path::new(&task.output_file).parent().unwrap()).unwrap();
        let partial = format!("{}.part", task.output_file);
        let ytdl = format!("{}.ytdl", task.output_file);
        let user_note = std::path::Path::new(&task.output_file).with_file_name(format!(
            "{}.notes.part",
            std::path::Path::new(&task.output_file)
                .file_stem()
                .unwrap()
                .to_string_lossy()
        ));
        let format_video = std::path::Path::new(&task.output_file).with_file_name(format!(
            "{}.f137.mp4.part",
            std::path::Path::new(&task.output_file)
                .file_stem()
                .unwrap()
                .to_string_lossy()
        ));
        let fragment = std::path::Path::new(&task.output_file).with_file_name(format!(
            "{}.mp4.part-Frag10.part",
            std::path::Path::new(&task.output_file)
                .file_stem()
                .unwrap()
                .to_string_lossy()
        ));
        fs::write(&partial, b"partial").unwrap();
        fs::write(&ytdl, b"state").unwrap();
        fs::write(&user_note, b"user note").unwrap();
        fs::write(&format_video, b"partial video").unwrap();
        fs::write(&fragment, b"partial fragment").unwrap();

        delete_task_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
        )
        .await
        .unwrap();

        assert!(!std::path::Path::new(&partial).exists());
        assert!(!std::path::Path::new(&ytdl).exists());
        assert!(!format_video.exists());
        assert!(!fragment.exists());
        assert!(user_note.exists());
        cleanup_state(state).await;
        fs::remove_dir_all(dir).unwrap();
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
                selected_pages: None,
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
                selected_pages: None,
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
    async fn list_task_groups_from_state_marks_stale_active_tasks_interrupted() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &MockDownloader,
        )
        .await
        .unwrap();
        let mut downloading = created.tasks[0].clone();
        downloading.state = TaskState::Downloading;
        downloading.bytes_downloaded = 25;
        downloading.bytes_total = Some(100);
        state.storage.update_task(&downloading).await.unwrap();
        let mut merging = created.tasks[1].clone();
        merging.state = TaskState::Merging;
        merging.bytes_downloaded = 100;
        merging.bytes_total = Some(100);
        state.storage.update_task(&merging).await.unwrap();

        let groups = list_task_groups_from_state(&state).await.unwrap();
        let tasks = &groups[0].tasks;

        assert_eq!(tasks[0].state, TaskState::Interrupted);
        assert_eq!(tasks[1].state, TaskState::Interrupted);
        assert_eq!(tasks[2].state, TaskState::Queued);
        assert_eq!(
            state.storage.load_task(downloading.id).await.unwrap().state,
            TaskState::Interrupted
        );
        assert_eq!(
            state.storage.load_task(merging.id).await.unwrap().state,
            TaskState::Interrupted
        );
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn list_task_groups_from_state_keeps_live_active_tasks_running() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let mut task = created.tasks[0].clone();
        task.state = TaskState::Downloading;
        state.storage.update_task(&task).await.unwrap();
        let (abort_handle, _abort_registration) = AbortHandle::new_pair();
        assert!(state.register_task_abort(task.id, abort_handle));

        let groups = list_task_groups_from_state(&state).await.unwrap();

        assert_eq!(groups[0].tasks[0].state, TaskState::Downloading);
        assert_eq!(
            state.storage.load_task(task.id).await.unwrap().state,
            TaskState::Downloading
        );
        state.clear_task_abort(task.id);
        cleanup_state(state).await;
    }

    #[tokio::test]
    async fn start_task_runs_after_stale_active_task_is_recovered() {
        let state = command_test_state().await;
        let created = create_task_with_downloader_from_state(
            &state,
            CreateTaskCommand {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                has_login: false,
                selected_pages: None,
            },
            DownloadEngine::Native,
            &RecordingDownloader,
        )
        .await
        .unwrap();
        let mut task = created.tasks[0].clone();
        task.state = TaskState::Downloading;
        state.storage.update_task(&task).await.unwrap();

        let listed = list_task_groups_from_state(&state).await.unwrap();
        assert_eq!(listed[0].tasks[0].state, TaskState::Interrupted);

        let downloader = CommandRunDownloader::default();
        let started = start_task_with_downloader_from_state(
            &state,
            RunTaskCommand {
                task_id: task.id.to_string(),
            },
            &downloader,
        )
        .await
        .unwrap();

        assert_eq!(started.state, TaskState::Completed);
        assert!(downloader.input().is_some());
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
        let tool_dir =
            std::env::temp_dir().join(format!("vd-config-tools-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tool_dir).unwrap();
        let ytdlp = tool_dir.join("yt-dlp.exe");
        let ffmpeg = tool_dir.join("ffmpeg.exe");
        let ffprobe = tool_dir.join("ffprobe.exe");
        fs::write(&ytdlp, b"test binary").unwrap();
        fs::write(&ffmpeg, b"test binary").unwrap();
        fs::write(&ffprobe, b"test binary").unwrap();
        let config = AppConfig {
            download_root: "E:\\Videos".into(),
            concurrency: 5,
            default_engine: DownloadEngine::YtDlp,
            ytdlp_path: Some(ytdlp.to_string_lossy().to_string()),
            ffmpeg_path: Some(ffmpeg.to_string_lossy().to_string()),
            ffprobe_path: Some(ffprobe.to_string_lossy().to_string()),
        };
        state.storage.save_config(&config).await.unwrap();

        let loaded = get_config_from_state(&state).await.unwrap();

        assert_eq!(loaded, config);
        cleanup_state(state).await;
        fs::remove_dir_all(tool_dir).unwrap();
    }

    #[tokio::test]
    async fn get_config_replaces_missing_tool_paths_with_installer_managed_defaults() {
        let state = command_test_state().await;
        let config = AppConfig {
            ytdlp_path: Some("C:\\tools\\yt-dlp.exe".into()),
            ffmpeg_path: Some("C:\\tools\\ffmpeg.exe".into()),
            ffprobe_path: Some("C:\\tools\\ffprobe.exe".into()),
            ..AppConfig::default()
        };
        state.storage.save_config(&config).await.unwrap();

        let loaded = get_config_from_state(&state).await.unwrap();

        assert_ne!(loaded.ytdlp_path.as_deref(), Some("C:\\tools\\yt-dlp.exe"));
        assert_ne!(loaded.ffmpeg_path.as_deref(), Some("C:\\tools\\ffmpeg.exe"));
        assert_ne!(
            loaded.ffprobe_path.as_deref(),
            Some("C:\\tools\\ffprobe.exe")
        );
        assert!(loaded
            .ytdlp_path
            .as_deref()
            .unwrap()
            .ends_with("dependencies\\yt-dlp\\yt-dlp.exe"));
        assert!(loaded
            .ffmpeg_path
            .as_deref()
            .unwrap()
            .ends_with("dependencies\\ffmpeg\\bin\\ffmpeg.exe"));
        assert!(loaded
            .ffprobe_path
            .as_deref()
            .unwrap()
            .ends_with("dependencies\\ffmpeg\\bin\\ffprobe.exe"));
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
    async fn install_ytdlp_bytes_from_state_writes_binary_and_persists_path() {
        let state = command_test_state().await;
        let install_root =
            std::env::temp_dir().join(format!("vd-ytdlp-install-{}", uuid::Uuid::new_v4()));
        let path = install_root
            .join("dependencies")
            .join("yt-dlp")
            .join("yt-dlp.exe");

        let saved =
            install_ytdlp_bytes_to_path_from_state(&state, b"yt-dlp binary".to_vec(), path.clone())
                .await
                .unwrap();

        assert_eq!(saved.ytdlp_path, Some(path.to_string_lossy().to_string()));
        assert_eq!(fs::read(&path).unwrap(), b"yt-dlp binary");
        let status = get_tool_status_from_state(&state).await.unwrap();
        assert_eq!(status.ytdlp, "available");
        cleanup_state(state).await;
        fs::remove_dir_all(install_root).unwrap();
    }

    #[tokio::test]
    async fn install_media_tools_bytes_from_state_extracts_archive_and_persists_paths() {
        let state = command_test_state().await;
        let install_root =
            std::env::temp_dir().join(format!("vd-ffmpeg-install-{}", uuid::Uuid::new_v4()));
        let archive = fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("vendor")
                .join("ffmpeg")
                .join("ffmpeg-win64-lgpl.zip"),
        )
        .unwrap();

        let saved = install_media_tools_bytes_to_path_from_state(
            &state,
            archive,
            install_root.join("dependencies").join("ffmpeg"),
        )
        .await
        .unwrap();

        assert!(saved.ffmpeg_path.as_deref().is_some_and(|path| path
            .ends_with("dependencies\\ffmpeg\\bin\\ffmpeg.exe")
            || path.ends_with("dependencies/ffmpeg/bin/ffmpeg.exe")));
        assert!(saved.ffprobe_path.as_deref().is_some_and(|path| path
            .ends_with("dependencies\\ffmpeg\\bin\\ffprobe.exe")
            || path.ends_with("dependencies/ffmpeg/bin/ffprobe.exe")));
        assert!(install_root
            .join("dependencies")
            .join("ffmpeg")
            .join("bin")
            .join("ffmpeg.exe")
            .is_file());
        assert!(install_root
            .join("dependencies")
            .join("ffmpeg")
            .join("bin")
            .join("ffprobe.exe")
            .is_file());
        cleanup_state(state).await;
        fs::remove_dir_all(install_root).unwrap();
    }

    #[test]
    fn install_media_tools_archive_to_path_rejects_checksum_mismatch() {
        let install_root =
            std::env::temp_dir().join(format!("vd-ffmpeg-bad-{}", uuid::Uuid::new_v4()));

        let err =
            install_media_tools_archive_to_path(b"not a zip", FFMPEG_ARCHIVE_SHA256, &install_root)
                .unwrap_err();

        assert_eq!(err.code(), crate::errors::ErrorCode::UpdateError);
        assert!(!install_root.exists());
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

    #[test]
    fn tool_status_uses_installer_managed_media_tools_when_config_paths_are_empty() {
        let dir = std::env::temp_dir().join(format!(
            "vd-installed-media-tool-status-{}",
            uuid::Uuid::new_v4()
        ));
        let ffmpeg = dir.join("ffmpeg.exe");
        let ffprobe = dir.join("ffprobe.exe");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&ffmpeg, b"test binary").unwrap();
        fs::write(&ffprobe, b"test binary").unwrap();

        let status =
            tool_status_from_config_with_installed_media(
                &AppConfig::default(),
                |tool| match tool {
                    "ffmpeg" => Some(ffmpeg.clone()),
                    "ffprobe" => Some(ffprobe.clone()),
                    _ => None,
                },
            );

        assert_eq!(status.ytdlp, "missing");
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

    struct MultiPageProbeDownloader;

    impl PlatformDownloader for MultiPageProbeDownloader {
        fn probe<'a>(
            &'a self,
            input: ProbeInput,
        ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
            Box::pin(async move {
                Ok(ProbeResult {
                    group_title: "剑桥少儿英语PowerUp 2nd Edition".into(),
                    used_login: input.has_login,
                    items: [58, 59]
                        .into_iter()
                        .map(|page| DownloadItem {
                            title: format!("字幕版_PU2E_L0_Chant {page}"),
                            output_file: format!("{page}.mp4"),
                            quality: Some("720P".into()),
                            requires_login: input.has_login,
                            bytes_total: None,
                            metadata: Some(DownloadItemMetadata {
                                bvid: "BV17KxizLE17".into(),
                                cid: page as u64,
                                page,
                            }),
                        })
                        .collect(),
                })
            })
        }

        fn download<'a>(
            &'a self,
            _input: DownloadInput,
            _sink: &'a dyn EventSink,
        ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
            Box::pin(async { unreachable!("probe tests do not download") })
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
