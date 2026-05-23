# Video Downloader

本项目是一个本地桌面视频下载器，当前首发方向是 Windows 上的 Bilibili 下载体验。应用基于 Rust + Tauri 2 构建，前端使用 TypeScript + Vite，下载任务、设置和日志保存在本地 SQLite 中。

> 当前最新已发布版本：[`v0.1.5`](https://github.com/shenhaofang/video_downloader/releases/tag/v0.1.5)

## 功能

- Bilibili BV 链接解析、单视频下载和多 P 展开。
- 默认使用 Rust 原生 `native` 引擎，`yt-dlp` 作为可选 fallback。
- Bilibili 二维码登录，登录态保存在本地加密文件中。
- 下载任务按任务组和子任务展示，支持子任务进度、失败原因、重试、暂停、继续和删除等生命周期操作。
- 支持 DASH 音视频分离流合并，也支持 Bilibili `durl` 单文件 MP4 流下载。
- 任务历史、设置、输出路径、错误码和日志持久化到本地 SQLite。
- Settings 中集中维护默认下载目录、并发数、默认引擎和本地工具状态。
- Windows NSIS 安装包会处理 FFmpeg/FFprobe 依赖；应用更新使用 Tauri updater 和 GitHub Releases。

## 不做什么

- 不绕过 DRM、验证码、付费权益、区域限制或平台访问控制。
- 不保存账号密码。
- 不提供服务端托管下载、多人账号系统或订阅式定时抓取。
- 原生引擎优先覆盖核心 Bilibili 视频场景；更复杂内容可通过 `yt-dlp` fallback 处理。

## 安装

从 GitHub Releases 下载 Windows 安装包：

- `Video.Downloader_<version>_x64-full-setup.exe`：完整首次安装包，包含必需 FFmpeg 依赖资源。
- `Video.Downloader_<version>_x64-app-update.exe`：应用内更新使用的瘦安装包，通常不需要手动下载。

Release 页面：

```text
https://github.com/shenhaofang/video_downloader/releases
```

## 本地开发

### 环境要求

- Node.js + npm
- Rust stable toolchain
- Windows WebView2 Runtime
- Tauri 2 构建所需的 Windows 桌面开发环境

### 常用命令

安装前端依赖：

```powershell
npm install
```

启动开发窗口：

```powershell
npm run tauri:dev
```

运行前端测试和构建：

```powershell
npm run test -- --pool=threads
npm run build
```

运行 Rust 检查：

```powershell
cd src-tauri
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

构建普通 Tauri 安装包：

```powershell
npm run tauri:build
```

构建发布用 full/app-update 双安装包：

```powershell
npm run tauri:build:release
```

## 项目结构

```text
frontend/                 Tauri 前端 UI、状态和 API 封装
src-tauri/src/commands.rs Tauri 命令入口
src-tauri/src/platform/   平台下载引擎
src-tauri/src/task/       任务执行、队列、事件和生命周期
src-tauri/src/storage.rs  SQLite 持久化
src-tauri/src/auth/       Bilibili 登录态和本地会话存储
src-tauri/src/media.rs    文件输出、断点续传和媒体工具调用
scripts/                  发布安装包和 updater 元数据脚本
docs/                     设计、发布、依赖分发和验证文档
tasks/                    本地过程记录，不进入正式发布产物
```

## 发布与更新

应用更新依赖 GitHub Releases 中的 `latest.json` 和 Tauri updater 签名产物。发布约定是：

- full setup 用于首次安装，内置必需 FFmpeg 压缩包。
- app-update setup 用于应用内更新，避免每次更新重复携带大依赖。
- `ffmpeg-win64-lgpl.zip` 作为独立 release asset 保留，供依赖缺失或损坏时修复安装。

发布脚本会生成 full setup、app-update setup、签名文件和 updater metadata。

## 当前限制

- 当前重点平台是 Windows x64。
- 原生下载引擎仍以 Bilibili 核心视频链路为主，不承诺覆盖所有 Bilibili 内容类型。
- 任务进度已通过持久化轮询刷新；后端事件流和跨重启的后台调度仍是后续增强方向。
- 任何下载行为都应遵守平台规则、版权要求和用户自身账号权限。
