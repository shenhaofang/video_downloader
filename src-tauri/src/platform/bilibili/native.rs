use crate::errors::{AppError, AppResult, ErrorCode};
use crate::platform::{
    DownloadInput, DownloadItem, DownloadItemMetadata, DownloadOutput, EventSink,
    PlatformDownloader, ProbeInput, ProbeResult,
};
use reqwest::Url;
use std::future::Future;
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

#[derive(Default)]
pub struct NativeBilibiliDownloader;

impl PlatformDownloader for NativeBilibiliDownloader {
    fn probe<'a>(
        &'a self,
        input: ProbeInput,
    ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let id = parse_bvid(&input.url)?;
            let client = reqwest::Client::new();
            let info = fetch_view_info(&client, &id.bvid).await?;

            Ok(probe_result_from_view_info(info, &id.bvid, input.has_login))
        })
    }

    fn download<'a>(
        &'a self,
        _input: DownloadInput,
        _sink: &'a dyn EventSink,
    ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
        Box::pin(async move {
            Err(AppError::structured(
                ErrorCode::PlatformChanged,
                "native media download requires the media API task",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadEngine;
    use crate::platform::bilibili::api::{VideoPage, ViewInfo};
    use crate::platform::{DownloadItem, EventSink};
    use std::io::{Read, Write};
    use std::net::TcpListener;

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
        let downloader = NativeBilibiliDownloader;

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
    async fn download_returns_platform_changed_until_media_api_task() {
        let downloader = NativeBilibiliDownloader;
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
}
