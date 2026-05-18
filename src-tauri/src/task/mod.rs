pub mod events;
pub mod executor;
pub mod queue;

use crate::errors::AppResult;
use crate::media::output_path;
use crate::models::{DownloadEngine, DownloadTask, TaskGroup, TaskState};
use crate::platform::{PlatformDownloader, ProbeInput};
use chrono::Utc;
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateTaskRequest {
    pub url: String,
    pub output_dir: String,
    pub engine: DownloadEngine,
    pub has_login: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CreatedTaskGroup {
    pub group: TaskGroup,
    pub tasks: Vec<DownloadTask>,
}

pub async fn create_group_from_probe(
    downloader: &dyn PlatformDownloader,
    request: CreateTaskRequest,
) -> AppResult<CreatedTaskGroup> {
    let probe = downloader
        .probe(ProbeInput {
            url: request.url.clone(),
            engine: request.engine,
            has_login: request.has_login,
        })
        .await?;
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
    let tasks = probe
        .items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
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
                bvid: item.metadata.as_ref().map(|metadata| metadata.bvid.clone()),
                cid: item.metadata.as_ref().map(|metadata| metadata.cid),
                page: item.metadata.as_ref().map(|metadata| metadata.page),
            }
        })
        .collect();
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
    use crate::errors::AppResult;
    use crate::models::DownloadEngine;
    use crate::platform::mock::MockDownloader;
    use crate::platform::{
        DownloadInput, DownloadItem, DownloadItemMetadata, DownloadOutput, EventSink, ProbeResult,
    };
    use std::future::Future;
    use std::pin::Pin;

    #[tokio::test]
    async fn creates_group_with_child_tasks() {
        let result = create_group_from_probe(
            &MockDownloader,
            CreateTaskRequest {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                engine: DownloadEngine::Native,
                has_login: true,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.group.platform, "bilibili");
        assert_eq!(result.group.title, "Rust 桌面应用入门");
        assert_eq!(result.tasks.len(), 3);

        let first_path = std::path::Path::new(&result.tasks[0].output_file);
        assert_eq!(
            first_path.file_name().unwrap().to_string_lossy(),
            "01 - 安装 Tauri.mp4"
        );
        assert!(!result.tasks[0].output_file.contains("01 - 01"));
        assert!(result.tasks[0].output_file.contains("bilibili"));
        assert!(result.tasks[0].output_file.contains("Rust 桌面应用入门"));
        assert!(result
            .tasks
            .iter()
            .all(|task| task.group_id == result.group.id));
    }

    #[tokio::test]
    async fn creates_single_video_without_forced_numeric_prefix() {
        let result = create_group_from_probe(
            &MockDownloader,
            CreateTaskRequest {
                url: "https://www.bilibili.com/video/BV1single".into(),
                output_dir: "D:\\Videos".into(),
                engine: DownloadEngine::Native,
                has_login: false,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.tasks.len(), 1);
        let filename = std::path::Path::new(&result.tasks[0].output_file)
            .file_name()
            .unwrap()
            .to_string_lossy();
        assert!(!filename.starts_with("01 -"));
    }

    #[tokio::test]
    async fn copies_probe_metadata_to_download_tasks() {
        let result = create_group_from_probe(
            &MetadataDownloader,
            CreateTaskRequest {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                output_dir: "D:\\Videos".into(),
                engine: DownloadEngine::Native,
                has_login: false,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.tasks[0].bvid.as_deref(), Some("BV1xx411c7mD"));
        assert_eq!(result.tasks[0].cid, Some(111));
        assert_eq!(result.tasks[0].page, Some(1));
    }

    struct MetadataDownloader;

    impl PlatformDownloader for MetadataDownloader {
        fn probe<'a>(
            &'a self,
            _input: ProbeInput,
        ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
            Box::pin(async {
                Ok(ProbeResult {
                    group_title: "Rust 桌面应用入门".into(),
                    used_login: false,
                    items: vec![DownloadItem {
                        title: "安装 Tauri".into(),
                        output_file: "安装 Tauri.mp4".into(),
                        quality: Some("1080P".into()),
                        requires_login: false,
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
            Box::pin(async { unreachable!("task creation tests do not download") })
        }
    }
}
