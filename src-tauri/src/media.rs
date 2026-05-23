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
        if path.is_file() {
            return Ok(Some(path));
        }
    }

    let path = installer_managed_media_tool_path_from_exe(exe_path, tool)?;
    Ok(path.is_file().then_some(path))
}

pub fn installer_managed_media_tool_path(tool: &str) -> AppResult<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    installer_managed_media_tool_path_from_exe(&exe, tool)
}

pub fn installer_managed_media_tool_root() -> AppResult<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    installer_managed_media_tool_root_from_exe(&exe)
}

pub fn installer_managed_media_tool_root_from_exe(exe_path: &Path) -> AppResult<PathBuf> {
    Ok(app_install_dir_from_exe(exe_path)?
        .join("dependencies")
        .join("ffmpeg"))
}

pub fn installer_managed_media_tool_path_from_exe(
    exe_path: &Path,
    tool: &str,
) -> AppResult<PathBuf> {
    let name = media_tool_name(tool)?;

    Ok(installer_managed_media_tool_root_from_exe(exe_path)?
        .join("bin")
        .join(format!("{name}.exe")))
}

pub fn installer_managed_ytdlp_path() -> AppResult<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|err| AppError::structured(ErrorCode::FilesystemError, err.to_string()))?;
    installer_managed_ytdlp_path_from_exe(&exe)
}

pub fn installer_managed_ytdlp_path_from_exe(exe_path: &Path) -> AppResult<PathBuf> {
    Ok(app_install_dir_from_exe(exe_path)?
        .join("dependencies")
        .join("yt-dlp")
        .join("yt-dlp.exe"))
}

fn app_install_dir_from_exe(exe_path: &Path) -> AppResult<&Path> {
    exe_path.parent().ok_or_else(|| {
        AppError::structured(
            ErrorCode::FilesystemError,
            "runtime executable has no parent directory",
        )
    })
}

#[cfg_attr(not(windows), allow(dead_code))]
const NO_WINDOW_PROCESS_CREATION_FLAGS: u32 = 0x08000000;

fn external_command(path: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(path);
    apply_no_window_to_command(&mut command);
    command
}

pub(crate) fn async_external_command(path: &Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(path);
    apply_no_window_to_async_command(&mut command);
    command
}

#[cfg(windows)]
fn apply_no_window_to_command(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(NO_WINDOW_PROCESS_CREATION_FLAGS);
}

#[cfg(not(windows))]
fn apply_no_window_to_command(_command: &mut std::process::Command) {}

#[cfg(windows)]
fn apply_no_window_to_async_command(command: &mut tokio::process::Command) {
    command.creation_flags(NO_WINDOW_PROCESS_CREATION_FLAGS);
}

#[cfg(not(windows))]
fn apply_no_window_to_async_command(_command: &mut tokio::process::Command) {}

pub fn merge_with_ffmpeg(
    ffmpeg_path: &Path,
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> AppResult<()> {
    let output = external_command(ffmpeg_path)
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

pub async fn merge_with_ffmpeg_async(
    ffmpeg_path: &Path,
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> AppResult<()> {
    let output = async_external_command(ffmpeg_path)
        .arg("-nostdin")
        .arg("-y")
        .arg("-i")
        .arg(video_path)
        .arg("-i")
        .arg(audio_path)
        .arg("-c")
        .arg("copy")
        .arg(output_path)
        .kill_on_drop(true)
        .output()
        .await
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
    fn builds_installer_managed_media_tool_path_under_install_root_dependencies() {
        let root = installer_managed_media_tool_root_from_exe(Path::new(
            "C:\\Users\\me\\AppData\\Local\\Video Downloader\\video-downloader.exe",
        ))
        .unwrap();
        let path = installer_managed_media_tool_path_from_exe(
            Path::new("C:\\Users\\me\\AppData\\Local\\Video Downloader\\video-downloader.exe"),
            "ffmpeg",
        )
        .unwrap();

        assert_eq!(
            root,
            PathBuf::from("C:\\Users\\me\\AppData\\Local\\Video Downloader\\dependencies\\ffmpeg")
        );
        assert_eq!(
            path,
            PathBuf::from(
                "C:\\Users\\me\\AppData\\Local\\Video Downloader\\dependencies\\ffmpeg\\bin\\ffmpeg.exe"
            )
        );
    }

    #[test]
    fn builds_installer_managed_ytdlp_path_under_install_root_dependencies() {
        let path = installer_managed_ytdlp_path_from_exe(Path::new(
            "C:\\Users\\me\\AppData\\Local\\Video Downloader\\video-downloader.exe",
        ))
        .unwrap();

        assert_eq!(
            path,
            PathBuf::from(
                "C:\\Users\\me\\AppData\\Local\\Video Downloader\\dependencies\\yt-dlp\\yt-dlp.exe"
            )
        );
    }

    #[test]
    fn media_tool_path_uses_configured_path_before_installer_managed_path() {
        let dir = temp_test_dir();
        let configured = dir.join("ffmpeg.exe");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&configured, b"test binary").unwrap();
        let path = media_tool_path_from_exe(
            Some(configured.clone()),
            "ffmpeg",
            Path::new("C:\\app\\video-downloader.exe"),
        )
        .unwrap();

        assert_eq!(path, Some(configured));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn media_tool_path_falls_back_to_installer_managed_path_when_configured_path_is_missing() {
        let dir = temp_test_dir();
        let exe_dir = dir.join("app");
        let installed = exe_dir
            .join("dependencies")
            .join("ffmpeg")
            .join("bin")
            .join("ffmpeg.exe");
        fs::create_dir_all(installed.parent().unwrap()).unwrap();
        fs::write(&installed, b"test binary").unwrap();

        let path = media_tool_path_from_exe(
            Some(PathBuf::from("C:\\tools\\ffmpeg.exe")),
            "ffmpeg",
            &exe_dir.join("video-downloader.exe"),
        )
        .unwrap();

        assert_eq!(path, Some(installed));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn media_tool_path_uses_installer_managed_path_when_present() {
        let dir = temp_test_dir();
        let exe_dir = dir.join("app");
        let ffmpeg = exe_dir
            .join("dependencies")
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
    fn tauri_config_keeps_ffmpeg_archive_out_of_app_update_bundle() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let text = fs::read_to_string(config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let resources = json["bundle"]["resources"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        assert_eq!(json["bundle"]["externalBin"], serde_json::Value::Null);
        assert_eq!(json["bundle"]["targets"], serde_json::json!(["nsis"]));
        assert_eq!(
            json["bundle"]["createUpdaterArtifacts"],
            serde_json::json!(true)
        );
        assert_eq!(
            json["bundle"]["windows"]["nsis"]["installerHooks"],
            serde_json::json!("windows/hooks.nsh")
        );
        assert!(!resources.contains(&serde_json::json!(
            "resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip"
        )));
        assert!(resources.contains(&serde_json::json!("resources/install-media-tools.ps1")));
        assert_eq!(
            json["plugins"]["updater"]["endpoints"][0],
            serde_json::json!(
                "https://github.com/shenhaofang/video_downloader/releases/latest/download/latest.json"
            )
        );
        assert_eq!(
            json["plugins"]["updater"]["windows"]["installMode"],
            serde_json::json!("passive")
        );
        assert!(json["plugins"]["updater"]["pubkey"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
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
    fn app_icon_assets_are_generated_from_logo_source() {
        let icons_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
        let source = icons_dir.join("app-icon.svg");
        let ico = icons_dir.join("icon.ico");
        let png_32 = icons_dir.join("32x32.png");
        let png_128 = icons_dir.join("128x128.png");
        let png_256 = icons_dir.join("128x128@2x.png");

        assert!(source.is_file());
        assert!(fs::read_to_string(source)
            .unwrap()
            .contains("Video Downloader logo"));
        assert!(png_32.is_file());
        assert!(png_128.is_file());
        assert!(png_256.is_file());
        assert!(ico.is_file());
        assert!(fs::metadata(ico).unwrap().len() > 10_000);
    }

    #[test]
    fn tauri_config_embeds_generated_logo_icons() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let text = fs::read_to_string(config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let icons = json["bundle"]["icon"].as_array().unwrap();

        for expected in [
            "icons/32x32.png",
            "icons/128x128.png",
            "icons/128x128@2x.png",
            "icons/icon.icns",
            "icons/icon.ico",
        ] {
            assert!(icons.contains(&serde_json::json!(expected)));
        }
    }

    #[test]
    fn runtime_external_processes_use_no_window_helpers() {
        let media_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("media.rs");
        let media_text = fs::read_to_string(media_path).unwrap();
        let ytdlp_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("platform")
            .join("bilibili")
            .join("yt_dlp.rs");
        let ytdlp_text = fs::read_to_string(ytdlp_path).unwrap();
        let production_media_text = media_text.split("#[cfg(test)]").next().unwrap();

        assert!(production_media_text.contains("NO_WINDOW_PROCESS_CREATION_FLAGS"));
        assert!(production_media_text.contains("creation_flags(NO_WINDOW_PROCESS_CREATION_FLAGS)"));
        let std_command_new = ["std::process::Command", "::new"].concat();
        let tokio_command_new = ["tokio::process::Command", "::new"].concat();
        assert_eq!(production_media_text.matches(&std_command_new).count(), 1);
        assert_eq!(production_media_text.matches(&tokio_command_new).count(), 1);
        assert!(!ytdlp_text.contains(&tokio_command_new));
        assert!(ytdlp_text.contains("async_external_command(path)"));
    }

    #[test]
    fn nsis_hook_ensures_required_media_tools_only_when_missing() {
        let hook_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("windows")
            .join("hooks.nsh");
        let text = fs::read_to_string(hook_path).unwrap();

        assert!(text.contains("!macro NSIS_HOOK_POSTINSTALL"));
        assert!(!text.contains("NSISdl::download"));
        assert!(text.contains("-ArchiveUrl"));
        assert!(text.contains("SetDetailsView show"));
        assert!(text.contains("FFmpeg media tools already installed"));
        assert!(text.contains("Installing required FFmpeg media tools"));
        assert!(text.contains("powershell.exe -NoProfile -ExecutionPolicy Bypass"));
        assert!(text.contains("resources\\install-media-tools.ps1"));
        assert!(text.contains(
            "https://github.com/shenhaofang/video_downloader/releases/latest/download/ffmpeg-win64-lgpl.zip"
        ));
        assert!(!text.contains("resources\\vendor\\ffmpeg\\ffmpeg-win64-lgpl.zip"));
        assert!(text.contains("$INSTDIR\\dependencies\\ffmpeg"));
        assert!(text.contains("D3C0D41C26B64BB42ABBF9051A9494BC67185B6D9FA57798F20EFB0E0213CAF7"));
        assert!(text.contains("!macro NSIS_HOOK_PREUNINSTALL"));
        assert!(text.contains("RMDir /r \"$INSTDIR\\dependencies\""));
        assert!(text.contains("DeleteRegKey SHCTX \"${MANUPRODUCTKEY}\""));
    }

    #[test]
    fn installer_media_tools_script_downloads_only_when_required() {
        let script_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("install-media-tools.ps1");
        let text = fs::read_to_string(script_path).unwrap();

        assert!(text.contains("[string]$ArchiveUrl"));
        assert!(text.contains("Required FFmpeg media tools already installed"));
        assert!(text.contains("Invoke-WebRequest"));
        assert!(text.contains("Verifying FFmpeg archive"));
        assert!(text.contains("FFmpeg archive checksum mismatch"));
        assert!(text.contains("FFmpeg media tools installed"));
    }

    #[test]
    fn nsis_hook_refreshes_shortcut_icons_after_install() {
        let hook_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("windows")
            .join("hooks.nsh");
        let text = fs::read_to_string(hook_path).unwrap();

        assert!(text.contains("CreateShortcut"));
        assert!(text.contains("\"$INSTDIR\\${MAINBINARYNAME}.exe\" 0"));
        assert!(text.contains("$DESKTOP\\${PRODUCTNAME}.lnk"));
        assert!(text.contains("$SMPROGRAMS\\${PRODUCTNAME}.lnk"));
        assert!(text.contains("SHChangeNotify"));
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

    #[test]
    fn capability_allows_only_dialog_open_command() {
        let capability_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("capabilities")
            .join("default.json");
        let text = fs::read_to_string(capability_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let permissions = json["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|entry| entry.as_str())
            .collect::<Vec<_>>();

        assert!(permissions.contains(&"dialog:allow-open"));
        assert!(!permissions.contains(&"dialog:default"));
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
