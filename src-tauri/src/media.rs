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

pub fn expected_media_tool_names() -> [&'static str; 2] {
    ["ffmpeg", "ffprobe"]
}

pub fn media_tool_name(tool: &str) -> AppResult<&'static str> {
    match tool {
        "ffmpeg" => Ok("ffmpeg"),
        "ffprobe" => Ok("ffprobe"),
        _ => Err(AppError::structured(
            ErrorCode::EngineMissing,
            "unknown media tool",
        )),
    }
}

pub fn media_tool_path(configured_path: Option<PathBuf>, tool: &str) -> AppResult<Option<PathBuf>> {
    let exe = std::env::current_exe()
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    media_tool_path_from_exe(configured_path, tool, &exe)
}

pub fn media_tool_path_from_exe(
    configured_path: Option<PathBuf>,
    tool: &str,
    exe_path: &Path,
) -> AppResult<Option<PathBuf>> {
    if let Some(path) = configured_path {
        return Ok(Some(path));
    }

    let path = installer_managed_media_tool_path_from_exe(exe_path, tool)?;
    Ok(path.is_file().then_some(path))
}

pub fn installer_managed_media_tool_path_from_exe(
    exe_path: &Path,
    tool: &str,
) -> AppResult<PathBuf> {
    let name = media_tool_name(tool)?;
    let exe_dir = exe_path.parent().ok_or_else(|| {
        AppError::structured(
            ErrorCode::FilesystemError,
            "runtime executable has no parent directory",
        )
    })?;

    Ok(exe_dir
        .join("resources")
        .join("media-tools")
        .join("ffmpeg")
        .join("bin")
        .join(format!("{name}.exe")))
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
    fn expected_media_tool_names_include_ffmpeg_and_ffprobe() {
        assert_eq!(expected_media_tool_names(), ["ffmpeg", "ffprobe"]);
    }

    #[test]
    fn accepts_only_installer_managed_media_tools() {
        assert_eq!(media_tool_name("ffmpeg").unwrap(), "ffmpeg");
        assert_eq!(media_tool_name("ffprobe").unwrap(), "ffprobe");
        assert_eq!(
            media_tool_name("yt-dlp").unwrap_err().code(),
            ErrorCode::EngineMissing
        );
    }

    #[test]
    fn builds_installer_managed_media_tool_path_next_to_app_resources() {
        let path = installer_managed_media_tool_path_from_exe(
            Path::new("C:\\Users\\me\\AppData\\Local\\Video Downloader\\video-downloader.exe"),
            "ffmpeg",
        )
        .unwrap();

        assert_eq!(
            path,
            PathBuf::from(
                "C:\\Users\\me\\AppData\\Local\\Video Downloader\\resources\\media-tools\\ffmpeg\\bin\\ffmpeg.exe"
            )
        );
    }

    #[test]
    fn media_tool_path_uses_configured_path_before_installer_managed_path() {
        let configured = PathBuf::from("D:\\tools\\ffmpeg.exe");
        let path = media_tool_path_from_exe(
            Some(configured.clone()),
            "ffmpeg",
            Path::new("C:\\app\\video-downloader.exe"),
        )
        .unwrap();

        assert_eq!(path, Some(configured));
    }

    #[test]
    fn media_tool_path_uses_installer_managed_path_when_present() {
        let dir = temp_test_dir();
        let exe_dir = dir.join("app");
        let ffmpeg = exe_dir
            .join("resources")
            .join("media-tools")
            .join("ffmpeg")
            .join("bin")
            .join("ffmpeg.exe");
        fs::create_dir_all(ffmpeg.parent().unwrap()).unwrap();
        fs::write(&ffmpeg, b"test binary").unwrap();

        let path = media_tool_path_from_exe(None, "ffmpeg", &exe_dir.join("video-downloader.exe"))
            .unwrap();

        assert_eq!(path, Some(ffmpeg));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn media_tool_path_is_none_when_installer_managed_tool_is_missing() {
        let path =
            media_tool_path_from_exe(None, "ffprobe", Path::new("C:\\app\\video-downloader.exe"))
                .unwrap();

        assert_eq!(path, None);
    }

    #[test]
    fn tauri_config_bundles_media_tools_for_nsis_without_external_bins() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let text = fs::read_to_string(config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let resources = json["bundle"]["resources"].as_array().unwrap();

        assert_eq!(json["bundle"]["externalBin"], serde_json::Value::Null);
        assert_eq!(json["bundle"]["targets"], serde_json::json!(["nsis"]));
        assert_eq!(
            json["bundle"]["windows"]["nsis"]["installerHooks"],
            serde_json::json!("windows/hooks.nsh")
        );
        assert!(resources.contains(&serde_json::json!("resources/install-media-tools.ps1")));
        assert!(resources.contains(&serde_json::json!(
            "resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip"
        )));
    }

    #[test]
    fn tauri_config_allows_nsis_install_directory_selection() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let text = fs::read_to_string(config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(
            json["bundle"]["windows"]["nsis"]["installMode"],
            serde_json::json!("both")
        );
    }

    #[test]
    fn tauri_identifier_does_not_end_with_app_extension() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let text = fs::read_to_string(config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let identifier = json["identifier"].as_str().unwrap();

        assert!(!identifier.ends_with(".app"));
    }

    #[test]
    fn cargo_lib_name_does_not_collide_with_bin_target_pdb_name() {
        let cargo_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let text = fs::read_to_string(cargo_path).unwrap();
        let package_name = extract_toml_string(&text, "package", "name").unwrap();
        let lib_name = extract_toml_string(&text, "lib", "name").unwrap();
        let normalized_package_name = package_name.replace('-', "_");

        assert_ne!(lib_name, normalized_package_name);
    }

    #[test]
    fn windows_release_entrypoint_uses_gui_subsystem() {
        let main_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("main.rs");
        let text = fs::read_to_string(main_path).unwrap();

        assert!(text.contains("cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")"));
    }

    #[test]
    fn nsis_hook_installs_bundled_media_tools_and_cleans_them_on_uninstall() {
        let hook_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("windows")
            .join("hooks.nsh");
        let text = fs::read_to_string(hook_path).unwrap();

        assert!(text.contains("!macro NSIS_HOOK_POSTINSTALL"));
        assert!(!text.contains("NSISdl::download"));
        assert!(!text.contains("-ArchiveUrl"));
        assert!(text.contains("SetDetailsView show"));
        assert!(text.contains("DetailPrint \"Installing bundled FFmpeg media tools"));
        assert!(text.contains("resources\\vendor\\ffmpeg\\ffmpeg-win64-lgpl.zip"));
        assert!(text.contains("install-media-tools.ps1"));
        assert!(text.contains("!macro NSIS_HOOK_PREUNINSTALL"));
        assert!(text.contains("RMDir /r \"$INSTDIR\\resources\\media-tools\""));
        assert!(text.contains("DeleteRegKey SHCTX \"${MANUPRODUCTKEY}\""));
    }

    #[test]
    fn dependency_install_script_verifies_bundled_archive_before_extracting() {
        let script_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("install-media-tools.ps1");
        let text = fs::read_to_string(script_path).unwrap();

        assert!(!text.contains("ArchiveUrl"));
        assert!(!text.contains("HttpClient"));
        assert!(!text.contains("curl.exe"));
        assert!(text.contains("ExpectedSha256"));
        assert!(text.contains("Get-FileHash"));
        assert!(text.contains("Expand-Archive"));
        assert!(text.contains("ffmpeg.exe"));
        assert!(text.contains("ffprobe.exe"));
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
    fn capability_does_not_allow_shell_sidecar_execution() {
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

        assert!(shell_permissions.is_empty());
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

    fn extract_toml_string(text: &str, section: &str, key: &str) -> Option<String> {
        let mut in_section = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_section = trimmed == format!("[{section}]");
                continue;
            }
            if !in_section {
                continue;
            }
            let Some((name, value)) = trimmed.split_once('=') else {
                continue;
            };
            if name.trim() == key {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
        None
    }
}
