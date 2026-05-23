# GitHub Release 更新与依赖拆包设计

## 目标

应用可以在“设置”页检测 GitHub Releases 是否有新版本，并在有新版本时直接安装更新。更新包只包含应用本体，不再重复携带 FFmpeg 这类大依赖；FFmpeg 和 yt-dlp 作为独立依赖按需下载安装。

## 范围

- GitHub 仓库使用公开 release 作为更新源。
- 应用更新使用 Tauri 官方 updater，启用签名校验和 Windows NSIS updater artifacts。
- FFmpeg 依赖从 Tauri `bundle.resources` 中移除，避免进入每次应用更新包。
- Settings 增加“应用更新”和“依赖工具”操作：检查更新、立即更新、下载 yt-dlp、下载 FFmpeg。
- 第一版只支持 Windows x64 的 NSIS 更新与 FFmpeg zip 依赖。

## 不做

- 不做二进制差分 patch。
- 不做 macOS/Linux 更新包。
- 不隐藏安装更新流程；Windows updater 使用 `passive` 安装模式。

## 发布约定

GitHub Release 需要包含：

- Tauri updater 需要的 `latest.json`。
- Windows updater artifact 和 `.sig`。
- 普通 NSIS 安装包。
- 独立依赖资产 `ffmpeg-win64-lgpl.zip`。

`latest.json` 使用 Tauri updater 官方静态 JSON 格式。依赖资产 URL 固定指向：

`https://github.com/shenhaofang/video_downloader/releases/latest/download/ffmpeg-win64-lgpl.zip`

## 后端设计

- 新增 `src-tauri/src/updater.rs`，封装 Tauri updater 的 `check` 与 `download_and_install`。
- 新增命令：
  - `check_app_update` 返回当前版本、是否有更新、最新版本、说明。
  - `install_app_update` 下载并安装已发布的应用更新，然后重启应用。
  - `install_media_tools` 下载 FFmpeg zip，校验 SHA256，解压到安装目录 `dependencies\ffmpeg`，并持久化 `ffmpeg_path` / `ffprobe_path`。
- 新增 `ErrorCode::UpdateError` 表示更新链路错误。
- 下载依赖时使用 release asset，不依赖安装器内置资源。

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

- Rust 单测覆盖 updater 状态映射、Tauri 配置守卫、FFmpeg zip 安装、命令注册。
- TS 单测覆盖 API invoke/fallback、Settings 更新区、依赖安装按钮、活跃任务禁用更新。
- 完整验证：`npm run test -- --pool=threads`、`npm run build`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test`、`npm run tauri:build`。
