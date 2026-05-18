use crate::errors::{AppError, AppResult, ErrorCode};
use crate::models::{DownloadTask, TaskState};
use crate::platform::{
    DownloadEvent, DownloadInput, DownloadItem, DownloadItemMetadata, PlatformDownloader,
};
use crate::storage::Storage;

pub async fn run_task_once(
    storage: &Storage,
    task: DownloadTask,
    downloader: &dyn PlatformDownloader,
) -> AppResult<DownloadTask> {
    let mut current = task;
    current.state = TaskState::Downloading;
    current.error_code = None;
    current.error_message = None;
    storage.update_task(&current).await?;

    let sink = crate::task::events::MemoryEventSink::default();
    let result = downloader
        .download(
            DownloadInput {
                item: download_item_from_task(&current),
                output_path: current.output_file.clone(),
            },
            &sink,
        )
        .await;

    apply_events(storage, &mut current, sink.events()).await?;

    match result {
        Ok(output) => {
            current.state = TaskState::Completed;
            current.quality = output.quality.or(current.quality);
            current.bytes_total = output.bytes_total.or(current.bytes_total);
            if let Some(total) = current.bytes_total {
                current.bytes_downloaded = total;
            }
            current.error_code = None;
            current.error_message = None;
            storage.update_task(&current).await?;
            Ok(current)
        }
        Err(err) => {
            let (code, message) = error_parts(&err);
            current.state = TaskState::Failed;
            current.retry_count = current.retry_count.saturating_add(1);
            current.error_code = Some(error_code_name(code).into());
            current.error_message = Some(message);
            storage.update_task(&current).await?;
            Err(err)
        }
    }
}

async fn apply_events(
    storage: &Storage,
    task: &mut DownloadTask,
    events: Vec<DownloadEvent>,
) -> AppResult<()> {
    for event in events {
        match event {
            DownloadEvent::Log(line) => {
                storage.append_log(task.id, &line).await?;
            }
            DownloadEvent::Progress { downloaded, total } => {
                task.bytes_downloaded = downloaded;
                task.bytes_total = total.or(task.bytes_total);
                storage.update_task(task).await?;
            }
            DownloadEvent::State(state) => {
                if state.starts_with("downloading") {
                    task.state = TaskState::Downloading;
                    storage.update_task(task).await?;
                } else if state == "merging" {
                    task.state = TaskState::Merging;
                    storage.update_task(task).await?;
                }
            }
        }
    }

    Ok(())
}

fn download_item_from_task(task: &DownloadTask) -> DownloadItem {
    DownloadItem {
        title: task.title.clone(),
        output_file: task.output_file.clone(),
        quality: task.quality.clone(),
        requires_login: task.used_login,
        bytes_total: task.bytes_total,
        metadata: match (&task.bvid, task.cid, task.page) {
            (Some(bvid), Some(cid), Some(page)) => Some(DownloadItemMetadata {
                bvid: bvid.clone(),
                cid,
                page,
            }),
            _ => None,
        },
    }
}

fn error_parts(err: &AppError) -> (ErrorCode, String) {
    match err {
        AppError::Structured { code, message } => (*code, message.clone()),
    }
}

fn error_code_name(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::NetworkError => "network_error",
        ErrorCode::LoginRequired => "login_required",
        ErrorCode::LoginExpired => "login_expired",
        ErrorCode::PermissionDenied => "permission_denied",
        ErrorCode::UnsupportedContent => "unsupported_content",
        ErrorCode::EngineMissing => "engine_missing",
        ErrorCode::FfmpegError => "ffmpeg_error",
        ErrorCode::FilesystemError => "filesystem_error",
        ErrorCode::PlatformChanged => "platform_changed",
        ErrorCode::UnknownError => "unknown_error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{AppError, AppResult, ErrorCode};
    use crate::models::{DownloadEngine, DownloadTask, TaskGroup, TaskState};
    use crate::platform::{
        DownloadInput, DownloadOutput, EventSink, PlatformDownloader, ProbeInput, ProbeResult,
    };
    use crate::storage::Storage;
    use chrono::Utc;
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[tokio::test]
    async fn run_task_once_persists_success_state_progress_and_logs() {
        let db = TestDatabase::open().await;
        let task = persist_task(&db.storage, queued_task()).await;
        let downloader = RecordingDownloader::success();

        run_task_once(&db.storage, task.clone(), &downloader)
            .await
            .unwrap();

        let loaded = db
            .storage
            .load_tasks_for_group(task.group_id)
            .await
            .unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].state, TaskState::Completed);
        assert_eq!(loaded[0].bytes_downloaded, 11);
        assert_eq!(loaded[0].bytes_total, Some(11));
        assert_eq!(loaded[0].quality, Some("480P".into()));
        assert_eq!(loaded[0].error_code, None);
        assert_eq!(loaded[0].error_message, None);

        let logs = db.storage.load_logs_for_task(task.id).await.unwrap();
        assert_eq!(logs, vec!["[fake] started".to_string()]);
        let input = downloader.input().unwrap();
        assert_eq!(input.output_path, task.output_file);
        assert_eq!(input.item.metadata.unwrap().cid, 62131);

        db.close().await;
    }

    #[tokio::test]
    async fn run_task_once_persists_failure_and_increments_retry_count() {
        let db = TestDatabase::open().await;
        let mut task = queued_task();
        task.retry_count = 1;
        let task = persist_task(&db.storage, task).await;
        let downloader = RecordingDownloader::failure();

        let err = run_task_once(&db.storage, task.clone(), &downloader)
            .await
            .unwrap_err();

        assert_eq!(err.code(), ErrorCode::NetworkError);
        let loaded = db
            .storage
            .load_tasks_for_group(task.group_id)
            .await
            .unwrap();
        assert_eq!(loaded[0].state, TaskState::Failed);
        assert_eq!(loaded[0].retry_count, 2);
        assert_eq!(loaded[0].error_code.as_deref(), Some("network_error"));
        assert_eq!(loaded[0].error_message.as_deref(), Some("download failed"));

        db.close().await;
    }

    struct RecordingDownloader {
        mode: RecordingMode,
        input: Arc<Mutex<Option<DownloadInput>>>,
    }

    #[derive(Clone, Copy)]
    enum RecordingMode {
        Success,
        Failure,
    }

    impl RecordingDownloader {
        fn success() -> Self {
            Self {
                mode: RecordingMode::Success,
                input: Arc::new(Mutex::new(None)),
            }
        }

        fn failure() -> Self {
            Self {
                mode: RecordingMode::Failure,
                input: Arc::new(Mutex::new(None)),
            }
        }

        fn input(&self) -> Option<DownloadInput> {
            self.input.lock().unwrap().clone()
        }
    }

    impl PlatformDownloader for RecordingDownloader {
        fn probe<'a>(
            &'a self,
            _input: ProbeInput,
        ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
            Box::pin(async { unreachable!("executor tests do not probe") })
        }

        fn download<'a>(
            &'a self,
            input: DownloadInput,
            sink: &'a dyn EventSink,
        ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
            Box::pin(async move {
                *self.input.lock().unwrap() = Some(input);
                sink.emit(crate::platform::DownloadEvent::Log("[fake] started".into()));
                sink.emit(crate::platform::DownloadEvent::State(
                    "downloading video".into(),
                ));
                sink.emit(crate::platform::DownloadEvent::Progress {
                    downloaded: 5,
                    total: Some(11),
                });
                sink.emit(crate::platform::DownloadEvent::State("merging".into()));
                sink.emit(crate::platform::DownloadEvent::Progress {
                    downloaded: 11,
                    total: Some(11),
                });
                match self.mode {
                    RecordingMode::Success => Ok(DownloadOutput {
                        output_path: "D:\\Videos\\bilibili\\out.mp4".into(),
                        quality: Some("480P".into()),
                        used_login: false,
                        bytes_total: Some(11),
                    }),
                    RecordingMode::Failure => Err(AppError::structured(
                        ErrorCode::NetworkError,
                        "download failed",
                    )),
                }
            })
        }
    }

    async fn persist_task(storage: &Storage, task: DownloadTask) -> DownloadTask {
        let group = TaskGroup {
            id: task.group_id,
            source_url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
            platform: "bilibili".into(),
            title: "B站下载链路测试".into(),
            output_dir: "D:\\Videos".into(),
            engine: task.engine,
            state: TaskState::Queued,
            created_at: Utc::now(),
        };
        storage.insert_group(&group).await.unwrap();
        storage.insert_task(&task).await.unwrap();
        task
    }

    fn queued_task() -> DownloadTask {
        DownloadTask {
            id: Uuid::new_v4(),
            group_id: Uuid::new_v4(),
            title: "B站下载链路测试".into(),
            output_file: "D:\\Videos\\bilibili\\out.mp4".into(),
            state: TaskState::Queued,
            engine: DownloadEngine::Native,
            quality: Some("720P".into()),
            used_login: false,
            bytes_downloaded: 0,
            bytes_total: None,
            retry_count: 0,
            max_retries: 3,
            error_code: None,
            error_message: None,
            bvid: Some("BV1xx411c7mD".into()),
            cid: Some(62131),
            page: Some(1),
        }
    }

    struct TestDatabase {
        storage: Storage,
        path: PathBuf,
    }

    impl TestDatabase {
        async fn open() -> Self {
            let path = std::env::temp_dir().join(format!(
                "video-downloader-executor-{}.sqlite",
                Uuid::new_v4()
            ));
            let database_url = format!(
                "sqlite://{}?mode=rwc",
                path.to_string_lossy().replace('\\', "/")
            );
            let storage = Storage::open(&database_url).await.unwrap();

            Self { storage, path }
        }

        async fn close(self) {
            let path = self.path;
            self.storage.close().await;
            remove_files(&path);
        }
    }

    fn remove_files(path: &std::path::Path) {
        for file in [
            path.to_path_buf(),
            path.with_extension("sqlite-shm"),
            path.with_extension("sqlite-wal"),
        ] {
            let _ = std::fs::remove_file(file);
        }
    }
}
