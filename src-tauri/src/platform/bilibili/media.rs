use crate::errors::{AppError, AppResult, ErrorCode};
use crate::platform::{DownloadEvent, EventSink};
use futures_util::StreamExt;
use serde::Deserialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

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

pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    sink: &dyn EventSink,
) -> AppResult<u64> {
    let response = client
        .get(url)
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

    fn temp_test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "video-downloader-bilibili-media-{}",
            uuid::Uuid::new_v4()
        ))
    }
}
