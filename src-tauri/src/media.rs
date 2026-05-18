use crate::errors::{AppError, AppResult, ErrorCode};
use sanitize_filename::sanitize;
use std::path::{Path, PathBuf};

pub fn sanitize_title(input: &str) -> String {
    let cleaned = sanitize(input).trim().to_string();
    if cleaned.is_empty() {
        "untitled".to_string()
    } else if cleaned.chars().count() > 120 {
        cleaned.chars().take(120).collect()
    } else {
        cleaned
    }
}

pub fn output_path(
    root: &Path,
    platform: &str,
    collection: &str,
    index: Option<u32>,
    title: &str,
) -> PathBuf {
    let mut filename = match index {
        Some(value) => format!("{value:02} - {}", sanitize_title(title)),
        None => sanitize_title(title),
    };
    filename.push_str(".mp4");

    root.join(sanitize_title(platform))
        .join(sanitize_title(collection))
        .join(filename)
}

pub fn ensure_directory(path: &Path) -> AppResult<()> {
    std::fs::create_dir_all(path)
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))
}

pub fn first_available_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("video");
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("mp4");

    for index in 1..10_000 {
        let candidate = parent.join(format!("{stem} ({index}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{stem} (10000).{ext}"))
}

pub fn expected_sidecar_names() -> [&'static str; 2] {
    ["ffmpeg", "ffprobe"]
}

pub fn sidecar_base_name(tool: &str) -> AppResult<&'static str> {
    match tool {
        "ffmpeg" => Ok("ffmpeg"),
        "ffprobe" => Ok("ffprobe"),
        _ => Err(AppError::structured(
            ErrorCode::EngineMissing,
            "unknown bundled tool",
        )),
    }
}

pub fn merge_with_ffmpeg(
    ffmpeg_path: &Path,
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> AppResult<()> {
    let output = std::process::Command::new(ffmpeg_path)
        .arg("-nostdin")
        .arg("-y")
        .arg("-i")
        .arg(video_path)
        .arg("-i")
        .arg(audio_path)
        .arg("-c")
        .arg("copy")
        .arg(output_path)
        .output()
        .map_err(|err| AppError::structured(ErrorCode::FfmpegError, err.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(AppError::structured(
            ErrorCode::FfmpegError,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_platform_collection_output_path() {
        let path = output_path(
            Path::new("D:\\Videos"),
            "bilibili",
            "合集:Rust?",
            Some(1),
            "安装/Tauri",
        );
        let text = path.to_string_lossy();

        assert!(text.contains("bilibili"));
        assert!(text.contains("合集Rust"));
        assert!(text.ends_with("01 - 安装Tauri.mp4"));
    }

    #[test]
    fn empty_title_becomes_untitled() {
        assert_eq!(sanitize_title("::::"), "untitled");
    }

    #[test]
    fn first_available_path_uses_numbered_sibling_for_collisions() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let base = dir.join("video.mp4");
        let first_collision = dir.join("video (1).mp4");
        fs::write(&base, b"base").unwrap();
        fs::write(&first_collision, b"collision").unwrap();

        let available = first_available_path(&base);

        assert_eq!(available.file_name().unwrap(), "video (2).mp4");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_directory_creates_nested_directories() {
        let dir = temp_test_dir();
        let nested = dir.join("one").join("two");

        ensure_directory(&nested).unwrap();

        assert!(nested.is_dir());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_directory_maps_create_failures_to_filesystem_error() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("not-a-directory");
        fs::write(&file_path, b"file").unwrap();

        let err = ensure_directory(&file_path).unwrap_err();

        assert_eq!(err.code(), ErrorCode::FilesystemError);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn expected_sidecar_names_include_ffmpeg_and_ffprobe() {
        assert_eq!(expected_sidecar_names(), ["ffmpeg", "ffprobe"]);
    }

    #[test]
    fn accepts_only_bundled_media_tools() {
        assert_eq!(sidecar_base_name("ffmpeg").unwrap(), "ffmpeg");
        assert_eq!(sidecar_base_name("ffprobe").unwrap(), "ffprobe");
        assert_eq!(
            sidecar_base_name("yt-dlp").unwrap_err().code(),
            ErrorCode::EngineMissing
        );
    }

    #[test]
    fn merge_requires_configured_ffmpeg_binary() {
        let err = merge_with_ffmpeg(
            Path::new("missing-ffmpeg.exe"),
            Path::new("video.m4s"),
            Path::new("audio.m4s"),
            Path::new("output.mp4"),
        )
        .unwrap_err();

        assert_eq!(err.code(), ErrorCode::FfmpegError);
    }

    #[test]
    fn merge_maps_ffmpeg_failure_to_ffmpeg_error() {
        let dir = temp_test_dir();
        fs::create_dir_all(&dir).unwrap();
        let ffmpeg = dir.join("fake-ffmpeg.bat");
        fs::write(
            &ffmpeg,
            "@echo off\r\necho merge failed 1>&2\r\nexit /b 7\r\n",
        )
        .unwrap();

        let err = merge_with_ffmpeg(
            &ffmpeg,
            Path::new("video.m4s"),
            Path::new("audio.m4s"),
            Path::new("output.mp4"),
        )
        .unwrap_err();

        assert_eq!(err.code(), ErrorCode::FfmpegError);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn capability_allows_only_bundled_media_sidecars() {
        let capability_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("capabilities")
            .join("default.json");
        let text = fs::read_to_string(capability_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let permissions = json["permissions"].as_array().unwrap();
        let shell_permissions = permissions
            .iter()
            .filter(|entry| {
                entry
                    .get("identifier")
                    .and_then(|value| value.as_str())
                    .is_some_and(|identifier| identifier.starts_with("shell:"))
                    || entry
                        .as_str()
                        .is_some_and(|identifier| identifier.starts_with("shell:"))
            })
            .collect::<Vec<_>>();
        assert_eq!(shell_permissions.len(), 1);
        let shell_permission = permissions
            .iter()
            .find(|entry| entry["identifier"] == "shell:allow-execute")
            .expect("missing constrained shell execute permission");
        let allowed = shell_permission["allow"].as_array().unwrap();
        let names = allowed
            .iter()
            .map(|entry| {
                assert_eq!(entry["sidecar"], true);
                assert!(entry.get("cmd").is_none());
                assert_eq!(entry["args"], false);
                entry["name"].as_str().unwrap()
            })
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["binaries/ffmpeg", "binaries/ffprobe"]);
    }

    fn temp_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "video-downloader-media-{}-{nanos}",
            std::process::id()
        ))
    }
}
