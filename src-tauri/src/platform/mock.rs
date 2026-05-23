use super::{
    DownloadEvent, DownloadInput, DownloadItem, DownloadOutput, EventSink, PlatformDownloader,
    ProbeInput, ProbeResult,
};
use crate::errors::AppResult;
use std::future::Future;
use std::pin::Pin;

#[derive(Default)]
pub struct MockDownloader;

impl PlatformDownloader for MockDownloader {
    fn probe<'a>(
        &'a self,
        input: ProbeInput,
    ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let is_collection =
                input.url.contains("collection") || input.url.contains("BV1xx411c7mD");
            let items = if is_collection {
                vec![
                    DownloadItem {
                        title: "01 - 安装 Tauri".into(),
                        output_file: "01 - 安装 Tauri.mp4".into(),
                        quality: Some("1080P".into()),
                        requires_login: true,
                        bytes_total: Some(1_200_000_000),
                        metadata: None,
                    },
                    DownloadItem {
                        title: "02 - Rust 命令与事件".into(),
                        output_file: "02 - Rust 命令与事件.mp4".into(),
                        quality: Some("1080P".into()),
                        requires_login: true,
                        bytes_total: Some(800_000_000),
                        metadata: None,
                    },
                    DownloadItem {
                        title: "03 - 打包与发布".into(),
                        output_file: "03 - 打包与发布.mp4".into(),
                        quality: Some("720P".into()),
                        requires_login: false,
                        bytes_total: Some(384_000_000),
                        metadata: None,
                    },
                ]
            } else {
                vec![DownloadItem {
                    title: "B站下载链路测试".into(),
                    output_file: "B站下载链路测试.mp4".into(),
                    quality: Some("720P".into()),
                    requires_login: false,
                    bytes_total: Some(384_000_000),
                    metadata: None,
                }]
            };

            Ok(ProbeResult {
                group_title: if is_collection {
                    "Rust 桌面应用入门".into()
                } else {
                    items[0].title.clone()
                },
                items,
                used_login: input.has_login,
            })
        })
    }

    fn download<'a>(
        &'a self,
        input: DownloadInput,
        sink: &'a dyn EventSink,
    ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
        Box::pin(async move {
            sink.emit(DownloadEvent::Log(format!(
                "[mock] downloading {}",
                input.item.title
            )));
            sink.emit(DownloadEvent::Progress {
                downloaded: input.item.bytes_total.unwrap_or(1),
                total: input.item.bytes_total,
            });
            sink.emit(DownloadEvent::State("completed".into()));

            Ok(DownloadOutput {
                output_path: input.output_path,
                quality: input.item.quality,
                used_login: input.item.requires_login,
                bytes_total: input.item.bytes_total,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadEngine;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct VecSink(Arc<Mutex<Vec<DownloadEvent>>>);

    impl EventSink for VecSink {
        fn emit(&self, event: DownloadEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[tokio::test]
    async fn expands_collection_sample() {
        let downloader = MockDownloader;

        let result = downloader
            .probe(ProbeInput {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                engine: DownloadEngine::Native,
                has_login: true,
            })
            .await
            .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.items[0].output_file, "01 - 安装 Tauri.mp4");
        assert_eq!(result.group_title, "Rust 桌面应用入门");
        assert!(result.used_login);
    }

    #[tokio::test]
    async fn probes_single_video_sample() {
        let downloader = MockDownloader;

        let result = downloader
            .probe(ProbeInput {
                url: "https://www.bilibili.com/video/BV1single".into(),
                engine: DownloadEngine::Native,
                has_login: false,
            })
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.group_title, "B站下载链路测试");
        assert_eq!(result.items[0].quality, Some("720P".into()));
        assert!(!result.used_login);
    }

    #[tokio::test]
    async fn emits_download_events() {
        let downloader = MockDownloader;
        let sink = VecSink::default();
        let item = DownloadItem {
            title: "sample".into(),
            output_file: "sample.mp4".into(),
            quality: Some("720P".into()),
            requires_login: true,
            bytes_total: Some(10),
            metadata: None,
        };

        let output = downloader
            .download(
                DownloadInput {
                    task_id: "task-1".into(),
                    source_url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                    item,
                    output_path: "D:\\Videos\\sample.mp4".into(),
                },
                &sink,
            )
            .await
            .unwrap();
        let events = sink.0.lock().unwrap();

        assert_eq!(output.output_path, "D:\\Videos\\sample.mp4");
        assert!(output.used_login);
        assert!(events
            .iter()
            .any(|event| matches!(event, DownloadEvent::Log(_))));
        assert!(events
            .iter()
            .any(|event| matches!(event, DownloadEvent::Progress { .. })));
        assert!(events
            .iter()
            .any(|event| matches!(event, DownloadEvent::State(state) if state == "completed")));
    }
}
