import { invoke } from "@tauri-apps/api/core";
import {
  normalizeEngine,
  type AppSettings,
  type CreatedTaskGroup,
  type Engine,
  type PlatformLoginRow,
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
}

interface RunTaskInput {
  task_id: string;
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
  try {
    const result = await invoke<CreatedTaskGroup["tasks"][number]>("run_task", { input });
    return normalizeTask(result);
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackRunTask(input.task_id);
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

function fallbackRunTask(taskId: string) {
  return {
    ...fallbackTask(taskId, "本地预览任务", `D:\\Videos\\bilibili\\${taskId}.mp4`, 100),
    state: "completed" as const,
  };
}

function normalizeCreatedTaskGroup(result: CreatedTaskGroup): CreatedTaskGroup {
  return {
    group: result.group,
    tasks: result.tasks.map(normalizeTask),
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
