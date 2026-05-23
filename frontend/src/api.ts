import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  normalizeEngine,
  type AppSettings,
  type CreatedTaskGroup,
  type Engine,
  type PlatformLoginRow,
  type ProbePageItem,
} from "./state";

interface TauriConfig {
  download_root: string;
  concurrency: number;
  default_engine: string;
  ytdlp_path?: string | null;
  ffmpeg_path?: string | null;
  ffprobe_path?: string | null;
}

interface CreateTaskInput {
  url: string;
  output_dir: string;
  has_login: boolean;
  selected_pages?: number[] | null;
}

interface RunTaskInput {
  task_id: string;
}

interface ProbePagesInput {
  url: string;
  has_login: boolean;
}

interface TauriProbeResult {
  group_title: string;
  items: TauriProbeItem[];
  used_login: boolean;
}

interface TauriProbeItem {
  title: string;
  output_file: string;
  quality?: string | null;
  requires_login: boolean;
  metadata?: {
    page: number;
  } | null;
}

export interface ProbePagesResult {
  groupTitle: string;
  items: ProbePageItem[];
  usedLogin: boolean;
}

export interface LoginQr {
  qrcode_key: string;
  url: string;
}

export interface PollLoginInput {
  qrcode_key: string;
}

export interface LoginPollResult {
  status: string;
  message: string;
}

export interface ToolStatus {
  ytdlp: string;
  ffmpeg: string;
  ffprobe: string;
}

export interface AppUpdateStatus {
  available: boolean;
  currentVersion: string;
  latestVersion: string | null;
  notes: string | null;
  pubDate: string | null;
}

interface TauriAppUpdateStatus {
  available: boolean;
  current_version: string;
  latest_version?: string | null;
  notes?: string | null;
  pub_date?: string | null;
}

export async function getConfig(): Promise<AppSettings> {
  try {
    const config = await invoke<TauriConfig>("get_config");
    return normalizeConfig(config);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackConfig();
    }
    throw error;
  }
}

export async function saveConfig(input: AppSettings): Promise<AppSettings> {
  try {
    const result = await invoke<TauriConfig>("save_config", { input: configToTauri(input) });
    return normalizeConfig(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return input;
    }
    throw error;
  }
}

export async function installYtDlp(): Promise<AppSettings> {
  try {
    const result = await invoke<TauriConfig>("install_ytdlp");
    return normalizeConfig(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return {
        ...fallbackConfig(),
        ytdlpPath: "C:\\Program Files\\Video Downloader\\dependencies\\yt-dlp\\yt-dlp.exe",
      };
    }
    throw error;
  }
}

export async function installMediaTools(): Promise<AppSettings> {
  try {
    const result = await invoke<TauriConfig>("install_media_tools");
    return normalizeConfig(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return {
        ...fallbackConfig(),
        ffmpegPath: "C:\\Program Files\\Video Downloader\\dependencies\\ffmpeg\\bin\\ffmpeg.exe",
        ffprobePath: "C:\\Program Files\\Video Downloader\\dependencies\\ffmpeg\\bin\\ffprobe.exe",
      };
    }
    throw error;
  }
}

export async function createTask(input: CreateTaskInput): Promise<CreatedTaskGroup> {
  try {
    const result = await invoke<CreatedTaskGroup>("create_task", { input });
    return normalizeCreatedTaskGroup(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackTaskGroup(input.output_dir);
    }
    throw error;
  }
}

export async function selectOutputDirectory(defaultPath: string): Promise<string | null> {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath,
    });
    if (Array.isArray(selected)) {
      return selected[0] ?? null;
    }
    return selected ?? null;
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return null;
    }
    throw error;
  }
}

export async function probeBilibiliPages(input: ProbePagesInput): Promise<ProbePagesResult> {
  try {
    const result = await invoke<TauriProbeResult>("probe_bilibili_pages", { input });
    return normalizeProbePagesResult(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackProbePagesResult();
    }
    throw error;
  }
}

export async function listTaskGroups(): Promise<CreatedTaskGroup[]> {
  try {
    const result = await invoke<CreatedTaskGroup[]>("list_task_groups");
    return result.map(normalizeCreatedTaskGroup);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return [];
    }
    throw error;
  }
}

export async function runTask(input: RunTaskInput) {
  return taskCommand("run_task", input, "completed");
}

export async function startTask(input: RunTaskInput) {
  return taskCommand("start_task", input, "completed");
}

export async function retryTask(input: RunTaskInput) {
  return taskCommand("retry_task", input, "completed");
}

export async function pauseTask(input: RunTaskInput) {
  return taskCommand("pause_task", input, "paused");
}

export async function deleteTask(input: RunTaskInput): Promise<CreatedTaskGroup[]> {
  try {
    const result = await invoke<CreatedTaskGroup[]>("delete_task", { input });
    return result.map(normalizeCreatedTaskGroup);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return [];
    }
    throw error;
  }
}

async function taskCommand(
  command: string,
  input: RunTaskInput,
  fallbackState: CreatedTaskGroup["tasks"][number]["state"],
) {
  try {
    const result = await invoke<CreatedTaskGroup["tasks"][number]>(command, { input });
    return normalizeTask(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackRunTask(input.task_id, fallbackState);
    }
    throw error;
  }
}

export async function startBilibiliLogin(): Promise<LoginQr> {
  try {
    return await invoke<LoginQr>("start_bilibili_login");
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return {
        qrcode_key: "fallback-qrcode-key",
        url: "https://passport.bilibili.com/qrcode/fallback",
      };
    }
    throw error;
  }
}

export async function pollBilibiliLogin(input: PollLoginInput): Promise<LoginPollResult> {
  try {
    return await invoke<LoginPollResult>("poll_bilibili_login", { input });
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return { status: "pending", message: "等待扫码" };
    }
    throw error;
  }
}

export async function clearBilibiliLogin(): Promise<void> {
  try {
    await invoke("clear_bilibili_login");
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return;
    }
    throw error;
  }
}

export async function listPlatformLogins(): Promise<PlatformLoginRow[]> {
  try {
    return await invoke<PlatformLoginRow[]>("list_platform_logins");
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackPlatforms();
    }
    throw error;
  }
}

export async function getToolStatus(): Promise<ToolStatus> {
  try {
    return await invoke<ToolStatus>("get_tool_status");
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackToolStatus();
    }
    throw error;
  }
}

export async function checkAppUpdate(): Promise<AppUpdateStatus> {
  try {
    const result = await invoke<TauriAppUpdateStatus>("check_app_update");
    return normalizeAppUpdateStatus(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackAppUpdateStatus();
    }
    throw error;
  }
}

export async function installAppUpdate(): Promise<void> {
  try {
    await invoke("install_app_update");
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return;
    }
    throw error;
  }
}

function isTauriUnavailable(error: unknown): boolean {
  const hasRuntime = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  if (hasRuntime) {
    return false;
  }

  const message = error instanceof Error ? error.message : String(error);
  return (
    typeof window === "undefined" ||
    !("__TAURI_INTERNALS__" in window) ||
    message.includes("__TAURI_INTERNALS__")
  );
}

function fallbackConfig(): AppSettings {
  return {
    downloadRoot: "D:\\Videos",
    concurrency: 2,
    defaultEngine: "native",
    ytdlpPath: null,
    ffmpegPath: null,
    ffprobePath: null,
  };
}

function fallbackPlatforms(): PlatformLoginRow[] {
  return [{ platform: "bilibili", status: "未登录" }];
}

function fallbackToolStatus(): ToolStatus {
  return {
    ytdlp: "missing",
    ffmpeg: "missing",
    ffprobe: "missing",
  };
}

function fallbackAppUpdateStatus(): AppUpdateStatus {
  return {
    available: false,
    currentVersion: "0.1.0",
    latestVersion: null,
    notes: null,
    pubDate: null,
  };
}

function fallbackTaskGroup(outputDir: string): CreatedTaskGroup {
  return {
    group: {
      id: "fallback-group-1",
      title: "Rust 桌面应用入门",
      output_dir: outputDir,
      state: "queued",
    },
    tasks: [
      fallbackTask(
        "fallback-task-1",
        "01 - 安装 Tauri",
        `${outputDir}\\Rust 桌面应用入门\\01 - 安装 Tauri.mp4`,
        0,
      ),
      fallbackTask(
        "fallback-task-2",
        "02 - 命令桥接",
        `${outputDir}\\Rust 桌面应用入门\\02 - 命令桥接.mp4`,
        0,
      ),
      fallbackTask(
        "fallback-task-3",
        "03 - 打包发布",
        `${outputDir}\\Rust 桌面应用入门\\03 - 打包发布.mp4`,
        0,
      ),
    ],
  };
}

function fallbackProbePagesResult(): ProbePagesResult {
  return {
    groupTitle: "剑桥少儿英语PowerUp 2nd Edition",
    usedLogin: false,
    items: [58, 59, 60].map((page) => ({
      page,
      title: `分 P ${page}`,
      quality: "720P",
      requiresLogin: false,
    })),
  };
}

function fallbackTask(id: string, title: string, outputFile: string, progress: number) {
  return {
    id,
    title,
    output_file: outputFile,
    state: "queued" as const,
    bytes_downloaded: progress,
    bytes_total: 100,
    retry_count: 0,
    max_retries: 3,
    quality: "1080p",
    used_login: false,
    engine: "native" as Engine,
  };
}

function fallbackRunTask(
  taskId: string,
  state: CreatedTaskGroup["tasks"][number]["state"] = "completed",
) {
  return {
    ...fallbackTask(taskId, "本地预览任务", `D:\\Videos\\bilibili\\${taskId}.mp4`, 100),
    state,
    bytes_downloaded: state === "paused" ? 0 : 100,
    bytes_total: state === "paused" ? null : 100,
  };
}

function normalizeCreatedTaskGroup(result: CreatedTaskGroup): CreatedTaskGroup {
  return {
    group: result.group,
    tasks: result.tasks.map(normalizeTask),
  };
}

function normalizeProbePagesResult(result: TauriProbeResult): ProbePagesResult {
  return {
    groupTitle: result.group_title,
    usedLogin: result.used_login,
    items: result.items.map((item, index) => ({
      page: item.metadata?.page ?? index + 1,
      title: item.title,
      quality: item.quality ?? null,
      requiresLogin: item.requires_login,
    })),
  };
}

function normalizeTask(task: CreatedTaskGroup["tasks"][number]) {
  return {
    ...task,
    engine: normalizeEngine(task.engine),
  };
}

function normalizeConfig(config: TauriConfig): AppSettings {
  return {
    downloadRoot: config.download_root,
    concurrency: config.concurrency,
    defaultEngine: normalizeEngine(config.default_engine),
    ytdlpPath: config.ytdlp_path ?? null,
    ffmpegPath: config.ffmpeg_path ?? null,
    ffprobePath: config.ffprobe_path ?? null,
  };
}

function normalizeAppUpdateStatus(status: TauriAppUpdateStatus): AppUpdateStatus {
  return {
    available: status.available,
    currentVersion: status.current_version,
    latestVersion: status.latest_version ?? null,
    notes: status.notes ?? null,
    pubDate: status.pub_date ?? null,
  };
}

function configToTauri(settings: AppSettings): TauriConfig {
  return {
    download_root: settings.downloadRoot,
    concurrency: settings.concurrency,
    default_engine: settings.defaultEngine === "yt-dlp" ? "yt_dlp" : "native",
    ytdlp_path: emptyToNull(settings.ytdlpPath),
    ffmpeg_path: emptyToNull(settings.ffmpegPath),
    ffprobe_path: emptyToNull(settings.ffprobePath),
  };
}

function emptyToNull(value: string | null): string | null {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
}
