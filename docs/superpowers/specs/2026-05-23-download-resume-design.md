# 下载断点续传设计

## 目标

暂停后的下载必须能在两个内核里恢复：

- `native` Bilibili 下载在 CDN 支持字节范围请求时，应复用已经写入的音频/视频 partial 文件。
- `yt-dlp` 下载应使用 `yt-dlp` 自身的续传能力，并在暂停/开始之间保留它自己的 partial 文件。

用户可见控制保持不变：子任务行和任务组继续使用 `pause`、`start`、`retry` 和 `delete`。变化只发生在 `start` 和任务组 `continue` 背后的行为：从“总是从头下载”升级为“安全时续传，只有续传不安全或不支持时才重新下载”。

## 当前状态

`native` 当前每次运行都会创建新的 UUID 临时目录，用 `File::create` 下载视频和音频流，并在运行结束后删除临时目录。暂停会 abort 当前 active future 并持久化 `TaskState::Paused`，但再次开始时会执行一次全新的下载。

`yt-dlp` 当前已有检测、安装、参数构造和进程执行 helper。不过，持久化的 `YtDlp` 任务创建和任务执行还没有接到 `create_task` 与 `run_task`；这些路径目前仍返回 `engine_missing`。

## 需求

- 暂停不能把任务转换成 `failed`。
- 开始已暂停任务时，应优先恢复 partial 字节。
- 如果无法证明续传是安全的，应用必须对该任务或该流回退为干净的重新下载。
- 删除必须清理任务自有 partial 文件，不能删除无关用户文件。
- 从 `failed` 重试时，默认保留有用的可续传 partial，除非失败原因说明本地数据已损坏。
- 任务完成后应移除任务自有 resume metadata 和 partial 文件。
- UI 应继续展示现有进度、重试次数和失败详情。
- 创建表单或子任务行里不新增单任务内核选择器。

## native 续传设计

### 稳定的续传工作目录

每个任务在自己的输出目录下获得一个确定性的工作目录：

```text
<output-dir>/.video-downloader/<task-id>/
```

`native` 内核存储：

- `video.part`
- `audio.part`
- `resume.json`

`resume.json` 记录 task id、engine、bvid、cid、page、选定 quality、流 URL 或 URL fingerprint、当前流大小，以及可用时的预期总大小。

### Range 下载流程

对每条流分别处理：

1. 每次运行前重新获取 Bilibili playurl，刷新可能过期的 CDN URL。
2. 检查现有 `.part` 文件长度。
3. 如果文件为空或不存在，正常下载。
4. 如果文件已有字节，请求 `Range: bytes=<existing-size>-`。
5. 如果服务端返回 `206 Partial Content`，追加写入 `.part` 文件。
6. 如果服务端返回 `200 OK`，删除该流的 `.part`，从 byte 0 重新下载该流。
7. 如果服务端返回 `416 Range Not Satisfiable`，检查本地大小是否已经等于预期总大小；否则重新下载该流。

视频和音频相互独立。一个任务可以续传视频同时重下音频，也可以反过来。

### 合并流程

FFmpeg merge 不做断点续传。如果暂停发生在 merge 阶段，再次开始任务时应复用已经完整的 `.part` 媒体文件，并从头执行 merge。

成功 merge 后：

- 持久化 `completed`。
- 删除该任务工作目录。

### 安全规则

- 如果 bvid/cid/page/quality 不再匹配任务，绝不向 partial 文件追加。
- 如果现有文件大于预期总大小，绝不追加。
- 绝不删除 `<output-dir>/.video-downloader/<task-id>/` 之外的文件。
- 外部进程使用 `kill_on_drop`，确保 pause/delete 能停止 active work。

## yt-dlp 续传设计

### 接上缺失的任务路径

这个功能同时接上 `yt-dlp` 任务创建和执行：

- `yt-dlp --dump-json` 用于探测 metadata，并创建持久化 `DownloadTask` 行。
- 下载执行使用持久化 task URL/output path 和配置好的 `yt-dlp.exe`。
- 进程通过现有 no-window process helper 运行。
- active run 注册必须阻止同一个任务重复启动多个 `yt-dlp` 进程。

### 续传行为

应用应让 `yt-dlp` 管理它自己的 resume 文件：

- 传入 `--continue`。
- 不传入 `--no-continue`。
- 对每个持久化任务使用稳定 output template。
- 在暂停和开始之间保留 `yt-dlp` partial 文件。
- pause/delete 时 abort active `yt-dlp` 进程。pause 保留 partial 文件；delete 删除任务自有 partial 文件。

预期任务自有文件包括最终目标文件，以及从同一个 output template 派生的相邻文件，例如 `.part`、`.ytdl`、`.temp` 和 `.frag` 变体。清理必须保守：只删除从精确 task output path 派生的路径，或位于任务工作目录内的路径。

### 进度和日志

使用 `--newline` 输出，并解析足够的信息来更新：

- `bytes_downloaded`
- `bytes_total`
- task logs
- `downloading` / `merging` state

如果解析不完整，应保留原始输出行到 task logs，并保持粗粒度状态更新，而不是隐藏进程信息。

## 任务生命周期

- `run_task`：创建后的自动 runner。它应跳过 `paused` 任务，保留“暂停任务只能手动重新开始”的现有规则。
- `start_task`：手动启动 `queued`、`paused` 或 `interrupted` 任务，并启用续传行为。
- `retry_task`：重置失败字段并重新开始。默认不删除 partial。
- `pause_task`：abort active run 并持久化 `paused`。
- `delete_task`：abort active run，删除持久化任务，并清理任务自有 resume 文件。

## 错误处理

- native 续传中的网络中断仍是 `network_error`；只有 active run 返回错误时任务才变为 `failed`，暂停本身不会导致失败。
- 缺失或无效的 `yt-dlp` 仍是 `engine_missing`。
- `yt-dlp` 非零退出码映射为 `unknown_error`，除非能推断出更明确的现有错误码。
- 文件系统清理失败如果阻止任务完成或删除，应记录日志并暴露为 `filesystem_error`。
- native 平台响应变化仍映射为 `platform_changed`。

## 测试

native 测试：

- 已有 partial 文件加 `206 Partial Content` 时，只追加缺失字节。
- 已有 partial 文件加 `200 OK` 时，重新下载该流。
- `416` 且本地大小等于预期大小时，将该流视为已完成。
- pause 保留任务工作目录；completion 删除任务工作目录。
- delete 只删除任务工作目录。
- pause 后重跑 merge 会复用完整 `.part` 文件。

yt-dlp 测试：

- `ytdlp_download_args` 包含 `--continue`，且绝不包含 `--no-continue`。
- fake `yt-dlp` 进程能被 `pause_task` abort，留下 partial 文件并持久化 `paused`。
- 开始 paused `yt-dlp` 任务会调用同一个 output template。
- 删除 `yt-dlp` 任务只清理派生出的任务自有 partial 文件。
- 当配置好的 `yt-dlp.exe` 存在时，`create_task` 和 `run_task` 不再返回 `engine_missing`。

端到端门禁：

- `cargo test`
- `cargo check`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `npm run test -- --pool=threads`
- `npm run build`
- `git diff --check`
- `npm run tauri:build`

## 不在本次范围内

- 对正在进行中的 ffmpeg merge 做真正断点续传。
- 在 `native` 和 `yt-dlp` 之间复用同一套 resume 文件格式。
- 在创建表单或子任务行里增加单任务内核选择。
- 改动自动重试策略。
- partial 下载状态的远程/云端同步。

## 开放风险

- Bilibili CDN URL 可能过期或改变 Range 支持；native 实现必须重新获取 playurl，并对不支持续传的流安全重下。
- 某些 `yt-dlp` format 可能产生额外 sidecar 文件；清理必须保守，避免删除用户文件。
- append 过程中磁盘空间不足可能留下 partial；后续开始应能安全续传或重新下载。
- 已安装数据库里可能已有缺少 resume metadata 的 paused 任务；这些任务应通过干净重下继续运行。
