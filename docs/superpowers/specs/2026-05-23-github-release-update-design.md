# GitHub Release 更新与依赖拆包设计

## 目标

应用可以在“设置”页检测 GitHub Releases 是否有新版本，并在有新版本时直接安装更新。发布产物分成完整安装包和瘦更新包：完整安装包内置 FFmpeg 必需依赖，首次安装不需要下载；瘦更新包只包含应用本体和小安装脚本，应用内更新不会重复携带 FFmpeg。

## 范围

- GitHub 仓库使用公开 release 作为更新源。
- 应用更新使用 Tauri 官方 updater，启用签名校验和 Windows NSIS updater artifacts。
- 默认 full NSIS 安装包在 Tauri `bundle.resources` 中包含 `resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip` 和安装脚本，首次安装直接使用内置资源完成 FFmpeg 安装。
- 发布流程额外构建 slim app-update 安装包，只包含应用本体和 `install-media-tools.ps1`，并让 `latest.json` 指向这个瘦包。
- 发布脚本 `scripts/build-release-installers.ps1` 负责先构建 full setup，再临时移除 FFmpeg zip 资源构建 slim app-update，最后恢复默认配置。
- NSIS 安装或升级完成后检查安装目录 `dependencies\ffmpeg`：已有完整 `ffmpeg.exe` / `ffprobe.exe` 时跳过；full 安装包从内置 zip 安装；slim 包只在依赖缺失或损坏时才下载、校验并安装 FFmpeg，不重复下载已存在的依赖。
- Settings 增加“应用更新”和“依赖工具”操作：检查更新、立即更新、下载 yt-dlp、下载 FFmpeg，用于用户手动修复或重装依赖。
- 第一版只支持 Windows x64 的 NSIS 更新与 FFmpeg zip 依赖。

## 不做

- 不做二进制差分 patch。
- 不做 macOS/Linux 更新包。
- 不隐藏安装更新流程；Windows updater 使用 `passive` 安装模式。

## 发布约定

GitHub Release 需要包含：

- Tauri updater 需要的 `latest.json`。
- Windows slim app-update artifact 和 `.sig`，由 `latest.json` 指向。
- 内置 FFmpeg 的普通 full NSIS 安装包，供用户首次安装。
- 独立依赖资产 `ffmpeg-win64-lgpl.zip`。

`latest.json` 使用 Tauri updater 官方静态 JSON 格式。依赖资产 URL 固定指向：

`https://github.com/shenhaofang/video_downloader/releases/latest/download/ffmpeg-win64-lgpl.zip`

## 后端设计

- 新增 `src-tauri/src/updater.rs`，封装 Tauri updater 的 `check` 与 `download_and_install`。
- 新增命令：
  - `check_app_update` 返回当前版本、是否有更新、最新版本、说明。
  - `install_app_update` 下载并安装已发布的应用更新，然后重启应用。
  - `install_media_tools` 下载 FFmpeg zip，校验 SHA256，解压到安装目录 `dependencies\ffmpeg`，并持久化 `ffmpeg_path` / `ffprobe_path`；full 安装包复用同一脚本从本地内置 zip 安装，slim 升级包在依赖存在时不重复下载。
- 新增 `ErrorCode::UpdateError` 表示更新链路错误。
- 安装器 hook 复用同一安装脚本和 SHA256：full 安装包优先使用内置 zip，slim 更新包缺失依赖时下载 release asset；已有依赖不下载。

## 前端设计

Settings 保持单页：

- “工具状态”继续显示 yt-dlp、ffmpeg、ffprobe。
- yt-dlp 路径旁保留“下载 yt-dlp”。
- FFmpeg 路径区增加“下载 FFmpeg”。
- 新增“应用更新”区域：
  - 初始显示当前版本。
  - 点击“检查更新”后显示“已是最新版本”或“发现 vX.Y.Z”。
  - 有更新时显示“立即更新”，点击后显示“正在下载并安装，应用将自动重启”。
  - 有未完成任务处于 `pending/probing/queued/downloading/merging` 时禁用立即更新，提示先暂停或完成任务。

## 验证

- Rust 单测覆盖 updater 状态映射、Tauri 配置守卫、安装器按需确保 FFmpeg、FFmpeg zip 安装、命令注册。
- TS 单测覆盖 API invoke/fallback、Settings 更新区、依赖安装按钮、活跃任务禁用更新。
- 完整验证：`npm run test -- --pool=threads`、`npm run build`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test`、`npm run tauri:build:release`。
