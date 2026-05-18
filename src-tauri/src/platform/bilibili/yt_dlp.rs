use crate::errors::{AppError, AppResult, ErrorCode};
use std::path::{Path, PathBuf};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorCode;
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
