use crate::errors::{AppError, AppResult, ErrorCode};
use crate::platform::{DownloadEvent, EventSink};
use futures_util::StreamExt;
use serde::Deserialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

const BILIBILI_REFERER: &str = "https://www.bilibili.com";
const BILIBILI_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaStream {
    pub url: String,
    pub bandwidth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashSelection {
    pub quality: String,
    pub video: MediaStream,
    pub audio: MediaStream,
}

#[derive(Debug, Deserialize)]
struct PlayResponse {
    code: i32,
    message: String,
    data: Option<PlayData>,
}

#[derive(Debug, Deserialize)]
struct PlayData {
    quality: Option<u32>,
    dash: Option<DashData>,
}

#[derive(Debug, Deserialize)]
struct DashData {
    video: Vec<DashStream>,
    audio: Vec<DashStream>,
}

#[derive(Debug, Deserialize)]
struct DashStream {
    #[serde(alias = "baseUrl")]
    base_url: String,
    bandwidth: u64,
}

pub fn parse_dash_selection(json: &str) -> AppResult<DashSelection> {
    let parsed: PlayResponse = serde_json::from_str(json)
        .map_err(|err| AppError::structured(ErrorCode::PlatformChanged, err.to_string()))?;
    if parsed.code != 0 {
        return Err(AppError::structured(
            ErrorCode::PlatformChanged,
            parsed.message,
        ));
    }
    let data = parsed
        .data
        .ok_or_else(|| AppError::structured(ErrorCode::PlatformChanged, "missing play data"))?;
    let dash = data.dash.ok_or_else(|| {
        AppError::structured(ErrorCode::UnsupportedContent, "missing dash streams")
    })?;
    let video = dash
        .video
        .into_iter()
        .max_by_key(|stream| stream.bandwidth)
        .ok_or_else(|| {
            AppError::structured(ErrorCode::UnsupportedContent, "missing video stream")
        })?;
    let audio = dash
        .audio
        .into_iter()
        .max_by_key(|stream| stream.bandwidth)
        .ok_or_else(|| {
            AppError::structured(ErrorCode::UnsupportedContent, "missing audio stream")
        })?;

    Ok(DashSelection {
        quality: quality_label(data.quality.unwrap_or(0)).to_string(),
        video: MediaStream {
            url: video.base_url,
            bandwidth: video.bandwidth,
        },
        audio: MediaStream {
            url: audio.base_url,
            bandwidth: audio.bandwidth,
        },
    })
}

pub fn quality_label(qn: u32) -> &'static str {
    match qn {
        120 => "4K",
        116 => "1080P60",
        80 => "1080P",
        64 => "720P",
        32 => "480P",
        16 => "360P",
        _ => "unknown",
    }
}

pub fn playurl_url(bvid: &str, cid: u64) -> String {
    format!("https://api.bilibili.com/x/player/playurl?bvid={bvid}&cid={cid}&fnval=4048&fourk=1")
}

pub async fn fetch_playurl_selection(
    client: &reqwest::Client,
    bvid: &str,
    cid: u64,
) -> AppResult<DashSelection> {
    fetch_playurl_selection_from_url(client, &playurl_url(bvid, cid)).await
}

pub(crate) async fn fetch_playurl_selection_from_url(
    client: &reqwest::Client,
    url: &str,
) -> AppResult<DashSelection> {
    let text = bilibili_get(client, url)
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .error_for_status()
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .text()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;

    parse_dash_selection(&text)
}

pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    sink: &dyn EventSink,
) -> AppResult<u64> {
    let response = bilibili_get(client, url)
        .send()
        .await
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?
        .error_for_status()
        .map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    let mut downloaded = 0_u64;

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|err| AppError::structured(ErrorCode::NetworkError, err.to_string()))?;
        file.write_all(&chunk)
            .await
            .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
        downloaded += chunk.len() as u64;
        sink.emit(DownloadEvent::Progress { downloaded, total });
    }
    file.flush()
        .await
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;

    Ok(downloaded)
}

fn bilibili_get(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header(reqwest::header::USER_AGENT, BILIBILI_USER_AGENT)
        .header(reqwest::header::REFERER, BILIBILI_REFERER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;
    use crate::platform::{DownloadEvent, EventSink};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[test]
    fn selects_highest_bandwidth_dash_streams() {
        let json = r#"{
          "code": 0,
          "message": "0",
          "data": {
            "quality": 80,
            "dash": {
              "video": [
                {"base_url": "video-low.m4s", "bandwidth": 100},
                {"base_url": "video-high.m4s", "bandwidth": 200}
              ],
              "audio": [
                {"base_url": "audio-low.m4s", "bandwidth": 50},
                {"base_url": "audio-high.m4s", "bandwidth": 90}
              ]
            }
          }
        }"#;

        let selected = parse_dash_selection(json).unwrap();

        assert_eq!(selected.quality, "1080P");
        assert_eq!(selected.video.url, "video-high.m4s");
        assert_eq!(selected.video.bandwidth, 200);
        assert_eq!(selected.audio.url, "audio-high.m4s");
        assert_eq!(selected.audio.bandwidth, 90);
    }

    #[test]
    fn parses_realistic_camel_case_dash_urls() {
        let json = r#"{
          "code": 0,
          "message": "0",
          "data": {
            "quality": 80,
            "dash": {
              "video": [
                {"baseUrl": "https://video.example.com/high.m4s", "bandwidth": 200}
              ],
              "audio": [
                {"baseUrl": "https://audio.example.com/high.m4s", "bandwidth": 90}
              ]
            }
          }
        }"#;

        let selected = parse_dash_selection(json).unwrap();

        assert_eq!(selected.video.url, "https://video.example.com/high.m4s");
        assert_eq!(selected.audio.url, "https://audio.example.com/high.m4s");
    }

    #[test]
    fn rejects_missing_dash_streams() {
        let err =
            parse_dash_selection(r#"{"code":0,"message":"0","data":{"quality":80}}"#).unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnsupportedContent);
    }

    #[test]
    fn labels_known_quality_numbers() {
        assert_eq!(quality_label(120), "4K");
        assert_eq!(quality_label(116), "1080P60");
        assert_eq!(quality_label(80), "1080P");
        assert_eq!(quality_label(64), "720P");
        assert_eq!(quality_label(32), "480P");
        assert_eq!(quality_label(16), "360P");
        assert_eq!(quality_label(0), "unknown");
    }

    #[test]
    fn builds_playurl_url_for_dash_request() {
        let url = playurl_url("BV1xx411c7mD", 62131);

        assert!(url.starts_with("https://api.bilibili.com/x/player/playurl?"));
        assert!(url.contains("bvid=BV1xx411c7mD"));
        assert!(url.contains("cid=62131"));
        assert!(url.contains("fnval=4048"));
        assert!(url.contains("fourk=1"));
    }

    #[tokio::test]
    async fn fetches_playurl_selection_from_url() {
        let url = one_shot_http_url(
            "200 OK",
            r#"{"code":0,"message":"OK","data":{"quality":32,"dash":{"video":[{"baseUrl":"video.m4s","bandwidth":10}],"audio":[{"baseUrl":"audio.m4s","bandwidth":5}]}}}"#
                .as_bytes()
                .to_vec(),
        );

        let selection = fetch_playurl_selection_from_url(&reqwest::Client::new(), &url)
            .await
            .unwrap();

        assert_eq!(selection.quality, "480P");
        assert_eq!(selection.video.url, "video.m4s");
        assert_eq!(selection.audio.url, "audio.m4s");
    }

    #[tokio::test]
    async fn maps_playurl_http_error_to_network_error() {
        let url = one_shot_http_url("500 Internal Server Error", b"failed".to_vec());

        let err = fetch_playurl_selection_from_url(&reqwest::Client::new(), &url)
            .await
            .unwrap_err();

        assert_eq!(err.code(), ErrorCode::NetworkError);
    }

    #[tokio::test]
    async fn fetch_playurl_sends_bilibili_browser_headers() {
        let url = one_shot_bilibili_header_guard_url(
            r#"{"code":0,"message":"OK","data":{"quality":32,"dash":{"video":[{"baseUrl":"video.m4s","bandwidth":10}],"audio":[{"baseUrl":"audio.m4s","bandwidth":5}]}}}"#
                .as_bytes()
                .to_vec(),
        );

        let selection = fetch_playurl_selection_from_url(&reqwest::Client::new(), &url)
            .await
            .unwrap();

        assert_eq!(selection.quality, "480P");
    }

    #[tokio::test]
    async fn downloads_stream_to_file_and_emits_progress() {
        let chunks = vec![b"media-".to_vec(), b"bytes".to_vec()];
        let url = one_shot_chunked_http_url(chunks.clone());
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("video.m4s");
        let sink = RecordingSink::default();

        let bytes = download_to_file(&reqwest::Client::new(), &url, &target, &sink)
            .await
            .unwrap();

        let expected = chunks.concat();
        assert_eq!(bytes, expected.len() as u64);
        assert_eq!(fs::read(&target).unwrap(), expected);
        let progress = sink
            .events()
            .into_iter()
            .filter_map(|event| match event {
                DownloadEvent::Progress { downloaded, total } => Some((downloaded, total)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(progress.len(), 2);
        assert_eq!(progress[0], (6, None));
        assert_eq!(progress[1], (11, None));

        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn maps_download_http_error_to_network_error() {
        let url = one_shot_http_url("500 Internal Server Error", b"failed".to_vec());
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();

        let err = download_to_file(
            &reqwest::Client::new(),
            &url,
            &dir.join("video.m4s"),
            &RecordingSink::default(),
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), ErrorCode::NetworkError);
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn maps_download_file_create_error_to_filesystem_error() {
        let url = one_shot_http_url("200 OK", b"media-bytes".to_vec());
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();

        let err = download_to_file(
            &reqwest::Client::new(),
            &url,
            &dir,
            &RecordingSink::default(),
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), ErrorCode::FilesystemError);
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn download_to_file_sends_bilibili_browser_headers() {
        let url = one_shot_bilibili_header_guard_url(b"media-bytes".to_vec());
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("video.m4s");

        let bytes = download_to_file(
            &reqwest::Client::new(),
            &url,
            &target,
            &RecordingSink::default(),
        )
        .await
        .unwrap();

        assert_eq!(bytes, 11);
        assert_eq!(fs::read(&target).unwrap(), b"media-bytes");
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

    fn one_shot_http_url(status: &str, body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_string();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        format!("http://{address}/stream")
    }

    fn one_shot_chunked_http_url(chunks: Vec<Vec<u8>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            for chunk in chunks {
                write!(stream, "{:x}\r\n", chunk.len()).unwrap();
                stream.write_all(&chunk).unwrap();
                write!(stream, "\r\n").unwrap();
                stream.flush().unwrap();
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            write!(stream, "0\r\n\r\n").unwrap();
        });
        format!("http://{address}/stream")
    }

    fn one_shot_bilibili_header_guard_url(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            let has_user_agent = request
                .lines()
                .any(|line| line.starts_with("user-agent: Mozilla/"));
            let has_referer = request
                .lines()
                .any(|line| line == "referer: https://www.bilibili.com");
            if has_user_agent && has_referer {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            } else {
                write!(
                    stream,
                    "HTTP/1.1 403 Forbidden\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing"
                )
                .unwrap();
            }
        });
        format!("http://{address}/guarded")
    }

    fn temp_test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "video-downloader-bilibili-media-{}",
            uuid::Uuid::new_v4()
        ))
    }
}
