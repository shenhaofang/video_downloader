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
}

interface CreateTaskInput {
  url: string;
  output_dir: string;
  has_login: boolean;
}

export async function getConfig(): Promise<AppSettings> {
  try {
    const config = await invoke<TauriConfig>("get_config");
    return {
      downloadRoot: config.download_root,
      concurrency: config.concurrency,
      defaultEngine: normalizeEngine(config.default_engine),
    };
  } catch (error) {
    if (isTauriUnavailable(error)) {
      return fallbackConfig();
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
  };
}

function fallbackPlatforms(): PlatformLoginRow[] {
  return [{ platform: "bilibili", status: "未登录" }];
}

function fallbackTaskGroup(outputDir: string): CreatedTaskGroup {
  return {
    group: {
      title: "Rust 桌面应用入门",
      output_dir: outputDir,
      state: "queued",
    },
    tasks: [
      fallbackTask("01 - 安装 Tauri", `${outputDir}\\Rust 桌面应用入门\\01 - 安装 Tauri.mp4`, 0),
      fallbackTask("02 - 命令桥接", `${outputDir}\\Rust 桌面应用入门\\02 - 命令桥接.mp4`, 0),
      fallbackTask("03 - 打包发布", `${outputDir}\\Rust 桌面应用入门\\03 - 打包发布.mp4`, 0),
    ],
  };
}

function fallbackTask(title: string, outputFile: string, progress: number) {
  return {
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

function normalizeCreatedTaskGroup(result: CreatedTaskGroup): CreatedTaskGroup {
  return {
    group: result.group,
    tasks: result.tasks.map((task) => ({
      ...task,
      engine: normalizeEngine(task.engine),
    })),
  };
}
