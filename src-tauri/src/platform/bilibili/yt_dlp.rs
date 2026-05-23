use crate::errors::{AppError, AppResult, ErrorCode};
use crate::media::async_external_command;
use crate::platform::{
    DownloadEvent, DownloadInput, DownloadItem, DownloadOutput, EventSink, PlatformDownloader,
    ProbeInput, ProbeResult,
};
use serde::Deserialize;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YtDlpStatus {
    Missing,
    Available { path: PathBuf },
}

pub fn detect_ytdlp(configured_path: Option<&str>) -> YtDlpStatus {
    if let Some(path) = configured_path {
        let path = PathBuf::from(path);
        if path.is_file() {
            return YtDlpStatus::Available { path };
        }
    }

    YtDlpStatus::Missing
}

pub fn require_ytdlp(configured_path: Option<&str>) -> AppResult<PathBuf> {
    match detect_ytdlp(configured_path) {
        YtDlpStatus::Available { path } => Ok(path),
        YtDlpStatus::Missing => Err(AppError::structured(
            ErrorCode::EngineMissing,
            "yt-dlp is not installed",
        )),
    }
}

pub fn ytdlp_json_args(url: &str, cookies_path: Option<&Path>) -> Vec<String> {
    let mut args = vec!["--dump-json".to_string(), "--no-warnings".to_string()];
    if let Some(path) = cookies_path {
        args.push("--cookies".to_string());
        args.push(path.to_string_lossy().to_string());
    }
    args.push(url.to_string());
    args
}

pub fn ytdlp_download_args(
    url: &str,
    output_template: &str,
    cookies_path: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![
        "--newline".to_string(),
        "--continue".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        "-o".to_string(),
        output_template.to_string(),
    ];
    if let Some(path) = cookies_path {
        args.push("--cookies".to_string());
        args.push(path.to_string_lossy().to_string());
    }
    args.push(url.to_string());
    args
}

pub async fn run_ytdlp(path: &Path, args: &[String]) -> AppResult<String> {
    let output = async_external_command(path)
        .args(args)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|err| AppError::structured(ErrorCode::EngineMissing, err.to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("yt-dlp exited with status {}", output.status)
        } else {
            stderr
        };
        Err(AppError::structured(ErrorCode::UnknownError, message))
    }
}

pub async fn run_ytdlp_download(
    path: &Path,
    args: &[String],
    sink: &dyn EventSink,
) -> AppResult<()> {
    let mut child = async_external_command(path)
        .args(args)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| AppError::structured(ErrorCode::EngineMissing, err.to_string()))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_task = async {
        if let Some(stdout) = stdout {
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines
                .next_line()
                .await
                .map_err(|err| AppError::structured(ErrorCode::UnknownError, err.to_string()))?
            {
                emit_ytdlp_line(&line, sink);
            }
        }
        Ok::<(), AppError>(())
    };

    let stderr_task = async {
        let mut stderr_lines = Vec::new();
        if let Some(stderr) = stderr {
            let mut lines = BufReader::new(stderr).lines();
            while let Some(line) = lines
                .next_line()
                .await
                .map_err(|err| AppError::structured(ErrorCode::UnknownError, err.to_string()))?
            {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    sink.emit(DownloadEvent::Log(format!("[yt-dlp] {trimmed}")));
                    stderr_lines.push(trimmed.to_string());
                }
            }
        }
        Ok::<Vec<String>, AppError>(stderr_lines)
    };

    let wait_task = async {
        child
            .wait()
            .await
            .map_err(|err| AppError::structured(ErrorCode::UnknownError, err.to_string()))
    };

    let (stdout_result, stderr_result, status_result) =
        tokio::join!(stdout_task, stderr_task, wait_task);
    stdout_result?;
    let stderr_lines = stderr_result?;
    let status = status_result?;

    if status.success() {
        Ok(())
    } else {
        let message = if stderr_lines.is_empty() {
            format!("yt-dlp exited with status {status}")
        } else {
            stderr_lines.join("\n")
        };
        Err(AppError::structured(ErrorCode::UnknownError, message))
    }
}

pub struct YtDlpDownloader {
    path: PathBuf,
}

impl YtDlpDownloader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl PlatformDownloader for YtDlpDownloader {
    fn probe<'a>(
        &'a self,
        input: ProbeInput,
    ) -> Pin<Box<dyn Future<Output = AppResult<ProbeResult>> + Send + 'a>> {
        Box::pin(async move {
            let stdout = run_ytdlp(&self.path, &ytdlp_json_args(&input.url, None)).await?;
            probe_result_from_json(&stdout, input.has_login)
        })
    }

    fn download<'a>(
        &'a self,
        input: DownloadInput,
        sink: &'a dyn EventSink,
    ) -> Pin<Box<dyn Future<Output = AppResult<DownloadOutput>> + Send + 'a>> {
        Box::pin(async move {
            run_ytdlp_download(
                &self.path,
                &ytdlp_download_args(&input.source_url, &input.output_path, None),
                sink,
            )
            .await?;
            let bytes_total = std::fs::metadata(&input.output_path)
                .ok()
                .filter(|metadata| metadata.is_file())
                .map(|metadata| metadata.len());
            if let Some(total) = bytes_total {
                sink.emit(DownloadEvent::Progress {
                    downloaded: total,
                    total: Some(total),
                });
            }
            Ok(DownloadOutput {
                output_path: input.output_path,
                quality: input.item.quality,
                used_login: input.item.requires_login,
                bytes_total,
            })
        })
    }
}

fn emit_ytdlp_line(line: &str, sink: &dyn EventSink) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    sink.emit(DownloadEvent::Log(format!("[yt-dlp] {trimmed}")));
    if let Some((downloaded, total)) = parse_ytdlp_progress(trimmed) {
        sink.emit(DownloadEvent::Progress {
            downloaded,
            total: Some(total),
        });
    }
}

fn parse_ytdlp_progress(line: &str) -> Option<(u64, u64)> {
    let percent_end = line.find('%')?;
    let before_percent = &line[..percent_end];
    let percent_start = before_percent
        .rfind(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .map(|index| index + 1)
        .unwrap_or(0);
    let percent: f64 = before_percent[percent_start..].trim().parse().ok()?;

    let after_percent = &line[percent_end + 1..];
    let size_start = after_percent.find(" of ")? + " of ".len();
    let after_of = after_percent[size_start..].trim_start();
    let size_end = [" at ", " ETA ", " in "]
        .iter()
        .filter_map(|delimiter| after_of.find(delimiter))
        .min()
        .unwrap_or(after_of.len());
    let total = parse_ytdlp_size(&after_of[..size_end])?;
    let downloaded = ((total as f64) * (percent / 100.0)).round() as u64;

    Some((downloaded.min(total), total))
}

fn parse_ytdlp_size(text: &str) -> Option<u64> {
    let cleaned = text.trim().trim_start_matches('~').trim().replace(',', "");
    if cleaned.is_empty() || cleaned.eq_ignore_ascii_case("unknown") {
        return None;
    }

    let number_end = cleaned
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .unwrap_or(cleaned.len());
    let value: f64 = cleaned[..number_end].trim().parse().ok()?;
    let unit = cleaned[number_end..].trim().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "" | "b" | "byte" | "bytes" => 1_f64,
        "kb" => 1_000_f64,
        "mb" => 1_000_000_f64,
        "gb" => 1_000_000_000_f64,
        "tb" => 1_000_000_000_000_f64,
        "kib" => 1_024_f64,
        "mib" => 1_048_576_f64,
        "gib" => 1_073_741_824_f64,
        "tib" => 1_099_511_627_776_f64,
        _ => return None,
    };

    Some((value * multiplier).round() as u64)
}

#[derive(Deserialize)]
struct YtDlpJson {
    title: Option<String>,
    fulltitle: Option<String>,
    filesize: Option<u64>,
    filesize_approx: Option<u64>,
}

fn probe_result_from_json(text: &str, has_login: bool) -> AppResult<ProbeResult> {
    let parsed: YtDlpJson = serde_json::from_str(text).map_err(|err| {
        AppError::structured(
            ErrorCode::PlatformChanged,
            format!("yt-dlp JSON parse failed: {err}"),
        )
    })?;
    let title = parsed
        .title
        .or(parsed.fulltitle)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "yt-dlp video".into());

    Ok(ProbeResult {
        group_title: title.clone(),
        used_login: has_login,
        items: vec![DownloadItem {
            title: title.clone(),
            output_file: format!("{title}.mp4"),
            quality: Some("auto".into()),
            requires_login: has_login,
            bytes_total: parsed.filesize.or(parsed.filesize_approx),
            metadata: None,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;
    use crate::task::events::MemoryEventSink;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_missing_when_no_path_configured() {
        assert_eq!(detect_ytdlp(None), YtDlpStatus::Missing);
    }

    #[test]
    fn detects_missing_when_configured_path_does_not_exist() {
        let missing = temp_test_dir().join("missing-yt-dlp.exe");

        assert_eq!(
            detect_ytdlp(Some(&missing.to_string_lossy())),
            YtDlpStatus::Missing
        );
    }

    #[test]
    fn detects_available_when_configured_file_exists() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("yt-dlp.exe");
        fs::write(&binary, b"test binary").unwrap();

        assert_eq!(
            detect_ytdlp(Some(&binary.to_string_lossy())),
            YtDlpStatus::Available {
                path: binary.clone()
            }
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn detects_missing_when_configured_path_is_directory() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();

        assert_eq!(
            detect_ytdlp(Some(&dir.to_string_lossy())),
            YtDlpStatus::Missing
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn require_ytdlp_maps_missing_to_engine_missing() {
        let err = require_ytdlp(None).unwrap_err();

        assert_eq!(err.code(), ErrorCode::EngineMissing);
    }

    #[test]
    fn builds_json_args_with_optional_cookie_file() {
        let args = ytdlp_json_args(
            "https://www.bilibili.com/video/BV1xx411c7mD",
            Some(Path::new("cookies.txt")),
        );

        assert!(args.contains(&"--dump-json".to_string()));
        assert!(args.contains(&"--no-warnings".to_string()));
        assert!(args.contains(&"--cookies".to_string()));
        assert!(args.contains(&"cookies.txt".to_string()));
        assert_eq!(
            args.last().unwrap(),
            "https://www.bilibili.com/video/BV1xx411c7mD"
        );
    }

    #[test]
    fn builds_json_args_without_cookies_when_not_configured() {
        let args = ytdlp_json_args("https://www.bilibili.com/video/BV1xx411c7mD", None);

        assert!(args.contains(&"--dump-json".to_string()));
        assert!(args.contains(&"--no-warnings".to_string()));
        assert!(!args.contains(&"--cookies".to_string()));
    }

    #[test]
    fn builds_download_args_with_mp4_merge_and_output_template() {
        let url = "https://www.bilibili.com/video/BV1xx411c7mD";
        let output_template = "D:\\Videos\\%(title)s.%(ext)s";

        let args = ytdlp_download_args(url, output_template, None);

        assert_eq!(
            args,
            vec![
                "--newline",
                "--continue",
                "--merge-output-format",
                "mp4",
                "-o",
                output_template,
                url
            ]
        );
        assert!(!args.contains(&"--no-continue".to_string()));
    }

    #[test]
    fn builds_download_args_with_cookie_file() {
        let url = "https://www.bilibili.com/video/BV1xx411c7mD";
        let cookies_path = Path::new("D:\\App\\cookies.txt");

        let args = ytdlp_download_args(url, "D:\\Videos\\%(title)s.%(ext)s", Some(cookies_path));

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--cookies", "D:\\App\\cookies.txt"]));
        assert_eq!(args.last().unwrap(), url);
    }

    #[tokio::test]
    async fn run_ytdlp_returns_stdout_for_successful_process() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("fake-ytdlp.bat");
        fs::write(&binary, "@echo off\r\necho downloaded\r\n").unwrap();

        let stdout = run_ytdlp(&binary, &["--version".to_string()])
            .await
            .unwrap();

        assert!(stdout.contains("downloaded"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn run_ytdlp_maps_missing_process_to_engine_missing() {
        let missing = temp_test_dir().join("missing-ytdlp.exe");

        let err = run_ytdlp(&missing, &[]).await.unwrap_err();

        assert_eq!(err.code(), ErrorCode::EngineMissing);
    }

    #[tokio::test]
    async fn run_ytdlp_maps_failed_process_to_unknown_error() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("fake-ytdlp.bat");
        fs::write(
            &binary,
            "@echo off\r\necho download failed 1>&2\r\nexit /b 7\r\n",
        )
        .unwrap();

        let err = run_ytdlp(&binary, &[]).await.unwrap_err();

        assert_eq!(err.code(), ErrorCode::UnknownError);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn parses_download_progress_line_with_binary_units() {
        assert_eq!(
            parse_ytdlp_progress("[download]  50.0% of 10.00MiB at 1.00MiB/s ETA 00:01"),
            Some((5_242_880, 10_485_760))
        );
    }

    #[test]
    fn ignores_download_progress_without_total_size() {
        assert_eq!(
            parse_ytdlp_progress("[download]  50.0% of Unknown B at 1.00MiB/s ETA 00:01"),
            None
        );
    }

    #[tokio::test]
    async fn run_ytdlp_download_emits_stdout_logs_and_progress() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("fake-ytdlp.bat");
        fs::write(
            &binary,
            "@echo off\r\necho [download]  50.0%% of 10.00MiB at 1.00MiB/s ETA 00:01\r\n",
        )
        .unwrap();
        let sink = MemoryEventSink::default();

        run_ytdlp_download(&binary, &["download".to_string()], &sink)
            .await
            .unwrap();

        let events = sink.events();
        assert!(events
            .iter()
            .any(|event| { matches!(event, DownloadEvent::Log(line) if line.contains("50.0%")) }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                DownloadEvent::Progress {
                    downloaded: 5_242_880,
                    total: Some(10_485_760),
                }
            )
        }));
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "video-downloader-ytdlp-{}-{nanos}",
            std::process::id()
        ))
    }
}
