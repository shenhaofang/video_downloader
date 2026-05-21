use crate::errors::{AppError, AppResult, ErrorCode};
use crate::platform::{
    DownloadInput, DownloadItem, DownloadItemMetadata, DownloadOutput, EventSink,
    PlatformDownloader, ProbeInput, ProbeResult,
};
use reqwest::Url;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilibiliVideoId {
    pub bvid: String,
}

pub fn parse_bvid(url: &str) -> AppResult<BilibiliVideoId> {
    let url = url.trim();
    let parsed = Url::parse(url)
        .or_else(|_| Url::parse(&format!("https://{url}")))
        .map_err(|_| {
            AppError::structured(ErrorCode::UnsupportedContent, "invalid bilibili video url")
        })?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::structured(
            ErrorCode::UnsupportedContent,
            "unsupported bilibili video scheme",
        ));
    }
    let host = parsed.host_str().ok_or_else(|| {
        AppError::structured(ErrorCode::UnsupportedContent, "missing bilibili video host")
    })?;
    if host != "bilibili.com" && !host.ends_with(".bilibili.com") {
        return Err(AppError::structured(
            ErrorCode::UnsupportedContent,
            "unsupported bilibili video host",
        ));
    }

    let mut segments = parsed.path_segments().ok_or_else(|| {
        AppError::structured(
            ErrorCode::UnsupportedContent,
            "missing bilibili video marker",
        )
    })?;
    if segments.next() != Some("video") {
        return Err(AppError::structured(
            ErrorCode::UnsupportedContent,
            "missing bilibili video marker",
        ));
    }
    let bvid = segments.next().unwrap_or_default();

    if is_valid_bvid(bvid) {
        Ok(BilibiliVideoId {
            bvid: bvid.to_string(),
        })
    } else {
        Err(AppError::structured(
            ErrorCode::UnsupportedContent,
            "missing BV id",
        ))
    }
}

fn is_valid_bvid(value: &str) -> bool {
    value.len() == 12
        && value.starts_with("BV")
        && value.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn probe_result_from_view_info(
    info: crate::platform::bilibili::api::ViewInfo,
    bvid: &str,
    has_login: bool,
) -> ProbeResult {
    let quality = if has_login { "1080P" } else { "720P" };
    let is_collection = info.pages.len() > 1;
    let items = info
        .pages
        .into_iter()
        .map(|page| DownloadItem {
            title: page.title.clone(),
            output_file: if is_collection {
                format!("{:02} - {}.mp4", page.page, page.title)
            } else {
                format!("{}.mp4", page.title)
            },
            quality: Some(quality.into()),
            requires_login: has_login,
            bytes_total: None,
            metadata: Some(DownloadItemMetadata {
                bvid: bvid.to_string(),
                cid: page.cid,
                page: page.page,
            }),
        })
        .collect();

    ProbeResult {
        group_title: info.title,
        items,
        used_login: has_login,
    }
}

async fn fetch_view_info(
    client: &reqwest::Client,
    bvid: &str,
) -> AppResult<crate::platform::bilibili::api::ViewInfo> {
    fetch_view_info_from_url(client, &crate::platform::bilibili::api::view_info_url(bvid)).await
}

async fn fetch_view_info_from_url(
    client: &reqwest::Client,
    url: &str,
) -> AppResult<crate::platform::bilibili::api::ViewInfo> {
    let text = client
        .get(url)
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .error_for_status()
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .text()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;

    crate::platform::bilibili::api::parse_view_info(&text)
}

pub struct NativeBilibiliDownloader {
    client: reqwest::Client,
    ffmpeg_path: Option<PathBuf>,
    playurl_url_override: Option<String>,
}

impl Default for NativeBilibiliDownloader {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            ffmpeg_path: None,
            playurl_url_override: None,
        }
    }
}

impl NativeBilibiliDownloader {
    pub fn with_ffmpeg_path(ffmpeg_path: Option<PathBuf>) -> Self {
        Self {
            client: reqwest::Client::new(),
            ffmpeg_path,
            playurl_url_override: None,
        }
    }

    #[cfg(test)]
    fn with_media_dependencies(
        client: reqwest::Client,
        ffmpeg_path: Option<PathBuf>,
        playurl_url: String,
    ) -> Self {
        Self {
            client,
            ffmpeg_path,
            playurl_url_override: Some(playurl_url),
        }
    }
}

impl PlatformDownloader for NativeBilibiliDownloader {
    fn probe<'a>(
        &'a self,
        input: ProbeInput,
    ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let id = parse_bvid(&input.url)?;
            let info = fetch_view_info(&self.client, &id.bvid).await?;

            Ok(probe_result_from_view_info(info, &id.bvid, input.has_login))
        })
    }

    fn download<'a>(
        &'a self,
        input: DownloadInput,
        sink: &'a dyn EventSink,
    ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
        Box::pin(async move {
            let metadata = input.item.metadata.clone().ok_or_else(|| {
                AppError::structured(ErrorCode::PlatformChanged, "missing bilibili task metadata")
            })?;
            let ffmpeg_path = self.ffmpeg_path.as_deref().ok_or_else(|| {
                AppError::structured(ErrorCode::FfmpegError, "ffmpeg path is not configured")
            })?;
            let output_path = PathBuf::from(&input.output_path);

            sink.emit(crate::platform::DownloadEvent::State("probing".into()));
            let selection = if let Some(url) = &self.playurl_url_override {
                crate::platform::bilibili::media::fetch_playurl_selection_from_url(
                    &self.client,
                    url,
                )
                .await?
            } else {
                crate::platform::bilibili::media::fetch_playurl_selection(
                    &self.client,
                    &metadata.bvid,
                    metadata.cid,
                )
                .await?
            };

            if let Some(parent) = output_path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
            {
                crate::media::ensure_directory(parent)?;
            }
            let temp_dir = native_download_temp_dir(&output_path);
            crate::media::ensure_directory(&temp_dir)?;
            let result = async {
                let video_path = temp_dir.join(format!("p{}-video.m4s", metadata.page));
                let audio_path = temp_dir.join(format!("p{}-audio.m4s", metadata.page));

                sink.emit(crate::platform::DownloadEvent::State(
                    "downloading video".into(),
                ));
                let video_bytes = crate::platform::bilibili::media::download_to_file(
                    &self.client,
                    &selection.video.url,
                    &video_path,
                    sink,
                )
                .await?;

                sink.emit(crate::platform::DownloadEvent::State(
                    "downloading audio".into(),
                ));
                let audio_bytes = crate::platform::bilibili::media::download_to_file(
                    &self.client,
                    &selection.audio.url,
                    &audio_path,
                    sink,
                )
                .await?;

                sink.emit(crate::platform::DownloadEvent::State("merging".into()));
                crate::media::merge_with_ffmpeg_async(
                    ffmpeg_path,
                    &video_path,
                    &audio_path,
                    &output_path,
                )
                .await?;

                Ok::<_, AppError>(DownloadOutput {
                    output_path: input.output_path,
                    quality: Some(selection.quality),
                    used_login: input.item.requires_login,
                    bytes_total: Some(video_bytes + audio_bytes),
                })
            }
            .await;
            let _ = std::fs::remove_dir_all(&temp_dir);

            result
        })
    }
}

fn native_download_temp_dir(output_path: &Path) -> PathBuf {
    output_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".video-downloader-{}", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadEngine;
    use crate::platform::bilibili::api::{VideoPage, ViewInfo};
    use crate::platform::{DownloadEvent, DownloadItem, DownloadItemMetadata, EventSink};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    struct NoopSink;

    impl EventSink for NoopSink {
        fn emit(&self, _event: crate::platform::DownloadEvent) {}
    }

    #[test]
    fn parses_bv_id_from_video_url() {
        let parsed =
            parse_bvid("https://www.bilibili.com/video/BV1xx411c7mD/?spm_id_from=333").unwrap();

        assert_eq!(parsed.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn parses_bv_id_before_path_suffix() {
        let parsed = parse_bvid("https://www.bilibili.com/video/BV1xx411c7mD/?p=2#reply").unwrap();

        assert_eq!(parsed.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn parses_video_url_without_scheme() {
        let parsed =
            parse_bvid("www.bilibili.com/video/BV1xx411c7mD/?next=https://example.com/path")
                .unwrap();

        assert_eq!(parsed.bvid, "BV1xx411c7mD");
    }

    #[test]
    fn parses_trimmed_root_and_mobile_hosts() {
        for url in [
            " https://bilibili.com/video/BV1xx411c7mD/\n",
            "\thttps://m.bilibili.com/video/BV1xx411c7mD/",
        ] {
            let parsed = parse_bvid(url).unwrap();
            assert_eq!(parsed.bvid, "BV1xx411c7mD");
        }
    }

    #[test]
    fn rejects_empty_url() {
        let err = parse_bvid("  ").unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_non_video_url() {
        let err = parse_bvid("https://space.bilibili.com/1").unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_lookalike_bilibili_host() {
        let err = parse_bvid("https://notbilibili.com/video/BV1xx411c7mD").unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_bilibili_video_link_embedded_in_query() {
        let err = parse_bvid(
            "https://example.com/watch?next=https://www.bilibili.com/video/BV1xx411c7mD",
        )
        .unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn rejects_invalid_bv_id_shape() {
        for url in [
            "https://www.bilibili.com/video/BV12345678",
            "https://www.bilibili.com/video/BV!!!!!!!!!!",
            "https://www.bilibili.com/video/BV1234567890extra",
        ] {
            let err = parse_bvid(url).unwrap_err();
            assert_eq!(err.code(), ErrorCode::UnsupportedContent);
        }
    }

    #[test]
    fn rejects_non_http_video_url() {
        let err = parse_bvid("ftp://www.bilibili.com/video/BV1xx411c7mD").unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[tokio::test]
    async fn maps_view_info_to_probe_items_for_logged_in_collection() {
        let result = probe_result_from_view_info(
            ViewInfo {
                title: "Rust 桌面应用入门".into(),
                pages: vec![
                    VideoPage {
                        cid: 111,
                        page: 1,
                        title: "安装 Tauri".into(),
                    },
                    VideoPage {
                        cid: 222,
                        page: 2,
                        title: "Rust 命令与事件".into(),
                    },
                ],
            },
            "BV1xx411c7mD",
            true,
        );

        assert_eq!(result.group_title, "Rust 桌面应用入门");
        assert!(result.used_login);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].title, "安装 Tauri");
        assert_eq!(result.items[0].output_file, "01 - 安装 Tauri.mp4");
        assert_eq!(result.items[0].quality, Some("1080P".into()));
        assert!(result.items[0].requires_login);
        assert_eq!(result.items[0].bytes_total, None);
        assert_eq!(
            result.items[0].metadata,
            Some(crate::platform::DownloadItemMetadata {
                bvid: "BV1xx411c7mD".into(),
                cid: 111,
                page: 1,
            })
        );
    }

    #[test]
    fn maps_view_info_to_720p_without_login() {
        let result = probe_result_from_view_info(
            ViewInfo {
                title: "B站下载链路测试".into(),
                pages: vec![VideoPage {
                    cid: 111,
                    page: 1,
                    title: "B站下载链路测试".into(),
                }],
            },
            "BV1xx411c7mD",
            false,
        );

        assert_eq!(result.items[0].quality, Some("720P".into()));
        assert_eq!(result.items[0].output_file, "B站下载链路测试.mp4");
        assert!(!result.items[0].requires_login);
        assert!(!result.used_login);
    }

    #[tokio::test]
    #[ignore]
    async fn live_fetch_view_info_returns_pages() {
        let client = reqwest::Client::new();

        let info = fetch_view_info(&client, "BV1xx411c7mD").await.unwrap();

        assert!(!info.title.is_empty());
        assert!(!info.pages.is_empty());
    }

    #[tokio::test]
    async fn fetch_view_info_maps_http_error_status_to_network_error() {
        let client = reqwest::Client::new();
        let url = one_shot_http_url("500 Internal Server Error", r#"{"code":0}"#);

        let err = fetch_view_info_from_url(&client, &url).await.unwrap_err();

        assert_eq!(err.code(), ErrorCode::NetworkError);
    }

    #[tokio::test]
    async fn probe_rejects_non_video_url() {
        let downloader = NativeBilibiliDownloader::default();

        let err = downloader
            .probe(ProbeInput {
                url: "https://space.bilibili.com/1".into(),
                engine: DownloadEngine::Native,
                has_login: true,
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[tokio::test]
    async fn download_requires_persisted_bilibili_metadata() {
        let downloader = NativeBilibiliDownloader::default();
        let sink = NoopSink;

        let err = downloader
            .download(
                DownloadInput {
                    item: DownloadItem {
                        title: "BV1xx411c7mD P1".into(),
                        output_file: "BV1xx411c7mD P1.mp4".into(),
                        quality: Some("720P".into()),
                        requires_login: false,
                        bytes_total: None,
                        metadata: None,
                    },
                    output_path: "D:\\Videos\\BV1xx411c7mD P1.mp4".into(),
                },
                &sink,
            )
            .await
            .unwrap_err();

        assert_eq!(err.code(), ErrorCode::PlatformChanged);
    }

    #[tokio::test]
    async fn download_requires_configured_ffmpeg_path() {
        let downloader = NativeBilibiliDownloader::default();
        let sink = NoopSink;

        let err = downloader
            .download(
                DownloadInput {
                    item: bilibili_download_item(),
                    output_path: "D:\\Videos\\BV1xx411c7mD P1.mp4".into(),
                },
                &sink,
            )
            .await
            .unwrap_err();

        assert_eq!(err.code(), ErrorCode::FfmpegError);
    }

    #[tokio::test]
    async fn download_fetches_streams_and_merges_output() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let video_url = one_shot_bytes_url(b"video".to_vec());
        let audio_url = one_shot_bytes_url(b"audio!".to_vec());
        let playurl = one_shot_http_url(
            "200 OK",
            &format!(
                r#"{{"code":0,"message":"OK","data":{{"quality":32,"dash":{{"video":[{{"baseUrl":"{video_url}","bandwidth":10}}],"audio":[{{"baseUrl":"{audio_url}","bandwidth":5}}]}}}}}}"#
            ),
        );
        let ffmpeg = dir.join("fake-ffmpeg.bat");
        fs::write(&ffmpeg, "@echo off\r\necho merged>\"%~9\"\r\n").unwrap();
        let output_path = dir.join("out.mp4");
        let downloader = NativeBilibiliDownloader::with_media_dependencies(
            reqwest::Client::new(),
            Some(ffmpeg),
            playurl,
        );
        let sink = RecordingSink::default();

        let output = downloader
            .download(
                DownloadInput {
                    item: bilibili_download_item(),
                    output_path: output_path.to_string_lossy().to_string(),
                },
                &sink,
            )
            .await
            .unwrap();

        assert_eq!(
            output.output_path,
            output_path.to_string_lossy().to_string()
        );
        assert_eq!(output.quality, Some("480P".into()));
        assert_eq!(output.bytes_total, Some(11));
        assert_eq!(fs::read_to_string(&output_path).unwrap().trim(), "merged");
        assert!(sink
            .events()
            .iter()
            .any(|event| matches!(event, DownloadEvent::State(state) if state == "merging")));

        fs::remove_dir_all(dir).unwrap();
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Arc<Mutex<Vec<DownloadEvent>>>,
    }

    impl RecordingSink {
        fn events(&self) -> Vec<DownloadEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    impl EventSink for RecordingSink {
        fn emit(&self, event: DownloadEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn bilibili_download_item() -> DownloadItem {
        DownloadItem {
            title: "BV1xx411c7mD P1".into(),
            output_file: "BV1xx411c7mD P1.mp4".into(),
            quality: Some("480P".into()),
            requires_login: false,
            bytes_total: None,
            metadata: Some(DownloadItemMetadata {
                bvid: "BV1xx411c7mD".into(),
                cid: 62131,
                page: 1,
            }),
        }
    }

    fn one_shot_http_url(status: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_string();
        let body = body.to_string();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        format!("http://{address}/view")
    }

    fn one_shot_bytes_url(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        format!("http://{address}/media")
    }

    fn temp_test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "video-downloader-native-download-{}",
            uuid::Uuid::new_v4()
        ))
    }
}
