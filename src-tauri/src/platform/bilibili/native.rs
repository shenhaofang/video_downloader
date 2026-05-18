use crate::errors::{AppError, AppResult, ErrorCode};
use crate::platform::{
    DownloadInput, DownloadItem, DownloadOutput, EventSink, PlatformDownloader, ProbeInput,
    ProbeResult,
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

#[derive(Default)]
pub struct NativeBilibiliDownloader;

impl PlatformDownloader for NativeBilibiliDownloader {
    fn probe<'a>(
        &'a self,
        input: ProbeInput,
    ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let id = parse_bvid(&input.url)?;
            let quality = if input.has_login { "1080P" } else { "720P" };

            Ok(ProbeResult {
                group_title: format!("bilibili {}", id.bvid),
                items: vec![DownloadItem {
                    title: format!("{} P1", id.bvid),
                    output_file: format!("{} P1.mp4", id.bvid),
                    quality: Some(quality.into()),
                    requires_login: input.has_login,
                    bytes_total: None,
                }],
                used_login: input.has_login,
            })
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
    use crate::platform::{DownloadItem, EventSink};

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
    async fn probe_returns_p0_placeholder_for_logged_in_video() {
        let downloader = NativeBilibiliDownloader;

        let result = downloader
            .probe(ProbeInput {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                engine: DownloadEngine::Native,
                has_login: true,
            })
            .await
            .unwrap();

        assert_eq!(result.group_title, "bilibili BV1xx411c7mD");
        assert!(result.used_login);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].title, "BV1xx411c7mD P1");
        assert_eq!(result.items[0].output_file, "BV1xx411c7mD P1.mp4");
        assert_eq!(result.items[0].quality, Some("1080P".into()));
        assert!(result.items[0].requires_login);
        assert_eq!(result.items[0].bytes_total, None);
    }

    #[tokio::test]
    async fn probe_returns_720p_placeholder_without_login() {
        let downloader = NativeBilibiliDownloader;

        let result = downloader
            .probe(ProbeInput {
                url: "https://www.bilibili.com/video/BV1xx411c7mD".into(),
                engine: DownloadEngine::Native,
                has_login: false,
            })
            .await
            .unwrap();

        assert_eq!(result.items[0].quality, Some("720P".into()));
        assert!(!result.items[0].requires_login);
        assert!(!result.used_login);
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
                    },
                    output_path: "D:\\Videos\\BV1xx411c7mD P1.mp4".into(),
                },
                &sink,
            )
            .await
            .unwrap_err();

        assert_eq!(err.code(), ErrorCode::PlatformChanged);
    }
}
