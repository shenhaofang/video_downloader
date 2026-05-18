// @vitest-environment jsdom
import { beforeEach, describe, expect, test, vi } from "vitest";
import { renderApp } from "./render";
import { createInitialState, type CreatedTaskGroup } from "./state";

const createdCollection: CreatedTaskGroup = {
  group: {
    id: "group-1",
    title: "Rust 桌面应用入门",
    output_dir: "D:\\Videos\\bilibili",
    state: "queued",
  },
  tasks: [
    {
      id: "task-1",
      title: "01 - 安装 Tauri",
      output_file: "D:\\Videos\\bilibili\\Rust 桌面应用入门\\01 - 安装 Tauri.mp4",
      state: "queued",
      bytes_downloaded: 25,
      bytes_total: 100,
      retry_count: 1,
      max_retries: 3,
      quality: "1080p",
      used_login: false,
      engine: "native",
    },
    {
      id: "task-2",
      title: "02 - 命令桥接",
      output_file: "D:\\Videos\\bilibili\\Rust 桌面应用入门\\02 - 命令桥接.mp4",
      state: "downloading",
      bytes_downloaded: 50,
      bytes_total: 100,
      retry_count: 0,
      max_retries: 3,
      quality: "1080p",
      used_login: true,
      engine: "native",
    },
  ],
};

describe("renderApp", () => {
  let root: HTMLDivElement;

  beforeEach(() => {
    root = document.createElement("div");
    document.body.replaceChildren(root);
  });

  test("renders three nav tabs and no boot screen", () => {
    renderApp(root, createInitialState());

    const tabs = root.querySelectorAll(".nav button");
    expect([...tabs].map((button) => button.textContent?.trim())).toEqual([
      "下载任务",
      "登录状态",
      "设置",
    ]);
    expect(root.querySelector(".boot-screen")).toBeNull();
  });

  test("prefills the output directory input", () => {
    renderApp(root, createInitialState());

    const input = root.querySelector<HTMLInputElement>("[data-testid='output-directory']");
    expect(input).not.toBeNull();
    expect(input?.value).toBe("D:\\Videos\\bilibili");
  });

  test("renders login platform as a flat summary row", () => {
    renderApp(root, createInitialState());
    root.querySelector<HTMLButtonElement>("[data-tab='login']")?.click();

    const summary = root.querySelector<HTMLElement>(".platform-summary");
    expect(summary).not.toBeNull();
    expect(summary?.textContent).toContain("bilibili");
    expect(summary?.textContent).toContain("未登录");
    expect(summary?.querySelector("[data-testid='platform-name']")?.textContent).toBe("bilibili");
    expect(summary?.querySelector("[data-testid='platform-status']")?.textContent).toBe("未登录");
  });

  test("shows local encrypted file copy in expanded platform detail", () => {
    renderApp(root, createInitialState());
    root.querySelector<HTMLButtonElement>("[data-tab='login']")?.click();

    root.querySelector<HTMLButtonElement>(".platform-summary")?.click();

    const detail = root.querySelector<HTMLElement>(".platform-detail");
    expect(detail?.hidden).toBe(false);
    expect(detail?.textContent).toContain("本地加密文件");
  });

  test("renders created collection children with output, progress, and retries", async () => {
    const createTask = vi.fn().mockResolvedValue(createdCollection);
    const runResolvers = new Map<string, (task: CreatedTaskGroup["tasks"][number]) => void>();
    const runTask = vi.fn().mockImplementation(
      ({ task_id }: { task_id: string }) =>
        new Promise((resolve) => {
          runResolvers.set(task_id, resolve);
        }),
    );
    renderApp(root, createInitialState(), { createTask, runTask });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV1xx411c7mD";
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(root.textContent).toContain("01 - 安装 Tauri");
    });

    expect(root.textContent).toContain("01 - 安装 Tauri.mp4");
    expect(root.textContent).not.toContain("01 - 01");
    expect(root.textContent).toContain("25%");
    expect(root.textContent).toContain("重试 1/3");
    expect(root.textContent).toContain("02 - 命令桥接");
    expect(root.textContent).toContain("02 - 命令桥接.mp4");
    expect(runTask).toHaveBeenCalledWith({ task_id: "task-1" });

    runResolvers.get("task-1")!({
      ...createdCollection.tasks[0],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    await vi.waitFor(() => {
      expect(runTask).toHaveBeenCalledWith({ task_id: "task-2" });
    });
    runResolvers.get("task-2")!({
      ...createdCollection.tasks[1],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    await vi.waitFor(() => {
      expect(root.textContent).toContain("已完成");
    });
    expect(createTask).toHaveBeenCalledWith({
      url: "https://www.bilibili.com/video/BV1xx411c7mD",
      output_dir: "D:\\Videos\\bilibili",
      has_login: false,
    });
  });

  test("keeps default engine controls in settings only", () => {
    renderApp(root, createInitialState());

    const downloadPanel = root.querySelector<HTMLElement>("[data-panel='downloads']");
    const settingsPanel = root.querySelector<HTMLElement>("[data-panel='settings']");
    expect(settingsPanel?.querySelector("[data-testid='default-engine']")).not.toBeNull();
    expect(downloadPanel?.querySelector("[data-testid='default-engine']")).toBeNull();
    expect(downloadPanel?.textContent).not.toContain("默认内核");
  });

  test("saves settings and updates download output directory", async () => {
    const saveConfig = vi.fn().mockImplementation((settings) =>
      Promise.resolve({
        ...settings,
        concurrency: 4,
      }),
    );
    const state = createInitialState();
    renderApp(root, state, { saveConfig });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLInputElement>("[data-testid='download-root']")!.value = "E:\\Videos";
    root.querySelector<HTMLInputElement>("[data-testid='concurrency']")!.value = "4";
    root.querySelector<HTMLInputElement>("[data-testid='ffmpeg-path']")!.value =
      "C:\\tools\\ffmpeg.exe";
    root.querySelector<HTMLInputElement>("[data-testid='ffprobe-path']")!.value =
      "C:\\tools\\ffprobe.exe";
    root.querySelector<HTMLButtonElement>("[data-testid='engine-yt-dlp']")!.click();
    root.querySelector<HTMLButtonElement>("[data-testid='save-settings']")!.click();

    await vi.waitFor(() => {
      expect(saveConfig).toHaveBeenCalledWith({
        downloadRoot: "E:\\Videos",
        concurrency: 4,
        defaultEngine: "yt-dlp",
        ytdlpPath: null,
        ffmpegPath: "C:\\tools\\ffmpeg.exe",
        ffprobePath: "C:\\tools\\ffprobe.exe",
      });
    });
    expect(state.settings.downloadRoot).toBe("E:\\Videos");
    root.querySelector<HTMLButtonElement>("[data-tab='downloads']")?.click();
    expect(root.querySelector<HTMLInputElement>("[data-testid='output-directory']")?.value).toBe(
      "E:\\Videos\\bilibili",
    );
  });
});
