// @vitest-environment jsdom
import { beforeEach, describe, expect, test, vi } from "vitest";
import { renderApp } from "./render";
import { createInitialState, type CreatedTaskGroup } from "./state";

function createdCollectionFixture(): CreatedTaskGroup {
  return {
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
}

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

  test("starts, polls, and clears bilibili QR login from expanded platform detail", async () => {
    const startBilibiliLogin = vi.fn().mockResolvedValue({
      qrcode_key: "qr-key-1",
      url: "https://passport.bilibili.com/qrcode/qr-key-1",
    });
    const pollBilibiliLogin = vi.fn().mockResolvedValue({
      status: "confirmed",
      message: "登录成功",
    });
    const clearBilibiliLogin = vi.fn().mockResolvedValue(undefined);
    const state = createInitialState();
    renderApp(root, state, { startBilibiliLogin, pollBilibiliLogin, clearBilibiliLogin });
    root.querySelector<HTMLButtonElement>("[data-tab='login']")?.click();
    root.querySelector<HTMLButtonElement>(".platform-summary")?.click();

    root.querySelector<HTMLButtonElement>("[data-testid='start-bilibili-login']")?.click();

    await vi.waitFor(() => {
      expect(root.textContent).toContain("https://passport.bilibili.com/qrcode/qr-key-1");
    });
    expect(startBilibiliLogin).toHaveBeenCalledOnce();

    root.querySelector<HTMLButtonElement>("[data-testid='poll-bilibili-login']")?.click();
    await vi.waitFor(() => {
      expect(pollBilibiliLogin).toHaveBeenCalledWith({ qrcode_key: "qr-key-1" });
      expect(root.querySelector("[data-testid='platform-status']")?.textContent).toBe("已登录");
    });

    root.querySelector<HTMLButtonElement>("[data-testid='clear-bilibili-login']")?.click();
    await vi.waitFor(() => {
      expect(clearBilibiliLogin).toHaveBeenCalledOnce();
      expect(root.querySelector("[data-testid='platform-status']")?.textContent).toBe("未登录");
    });
  });

  test("automatically polls bilibili QR login after starting", async () => {
    const startBilibiliLogin = vi.fn().mockResolvedValue({
      qrcode_key: "qr-key-1",
      url: "https://passport.bilibili.com/qrcode/qr-key-1",
    });
    const pollBilibiliLogin = vi.fn().mockResolvedValue({
      status: "confirmed",
      message: "登录成功",
    });
    renderApp(root, createInitialState(), {
      startBilibiliLogin,
      pollBilibiliLogin,
      qrPollMs: 20,
    });
    root.querySelector<HTMLButtonElement>("[data-tab='login']")?.click();
    root.querySelector<HTMLButtonElement>(".platform-summary")?.click();

    root.querySelector<HTMLButtonElement>("[data-testid='start-bilibili-login']")?.click();

    await vi.waitFor(() => {
      expect(pollBilibiliLogin).toHaveBeenCalledWith({ qrcode_key: "qr-key-1" });
      expect(root.querySelector("[data-testid='platform-status']")?.textContent).toBe("已登录");
    });
  });

  test("renders a QR image after starting bilibili login", async () => {
    const startBilibiliLogin = vi.fn().mockResolvedValue({
      qrcode_key: "qr-key-1",
      url: "https://passport.bilibili.com/qrcode/qr-key-1",
    });
    const pollBilibiliLogin = vi.fn().mockResolvedValue({
      status: "expired",
      message: "二维码已失效",
    });
    const createQrDataUrl = vi.fn().mockResolvedValue("data:image/svg+xml,%3Csvg%3E%3C/svg%3E");
    renderApp(root, createInitialState(), {
      startBilibiliLogin,
      pollBilibiliLogin,
      createQrDataUrl,
      qrPollMs: 20,
    });
    root.querySelector<HTMLButtonElement>("[data-tab='login']")?.click();
    root.querySelector<HTMLButtonElement>(".platform-summary")?.click();

    root.querySelector<HTMLButtonElement>("[data-testid='start-bilibili-login']")?.click();

    await vi.waitFor(() => {
      const image = root.querySelector<HTMLImageElement>("[data-testid='bilibili-qr-image']");
      expect(createQrDataUrl).toHaveBeenCalledWith(
        "https://passport.bilibili.com/qrcode/qr-key-1",
      );
      expect(image?.alt).toBe("bilibili 登录二维码");
      expect(image?.src).toContain("data:image/svg+xml");
      expect(root.textContent).toContain("https://passport.bilibili.com/qrcode/qr-key-1");
    });
  });

  test("renders created collection children with output, progress, and retries", async () => {
    const createdCollection = createdCollectionFixture();
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
    await vi.waitFor(() => {
      expect(runTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(runTask).toHaveBeenCalledWith({ task_id: "task-2" });
    });

    runResolvers.get("task-1")!({
      ...createdCollection.tasks[0],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
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

  test("probes multi-part pages and creates selected range", async () => {
    const createdCollection = createdCollectionFixture();
    const createTask = vi.fn().mockResolvedValue({
      ...createdCollection,
      tasks: [createdCollection.tasks[0]],
    });
    const probeBilibiliPages = vi.fn().mockResolvedValue({
      groupTitle: "剑桥少儿英语PowerUp 2nd Edition",
      usedLogin: false,
      items: [
        {
          page: 58,
          title: "字幕版_PU2E_L0_Chant 1 Page 5_video",
          quality: "720P",
          requiresLogin: false,
        },
        {
          page: 59,
          title: "字幕版_PU2E_L0_Chant 2 Page 6_video",
          quality: "720P",
          requiresLogin: false,
        },
        {
          page: 60,
          title: "字幕版_PU2E_L0_Chant 3 Page 7_video",
          quality: "720P",
          requiresLogin: false,
        },
      ],
    });
    const runTask = vi.fn().mockResolvedValue({
      ...createdCollection.tasks[0],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    renderApp(root, createInitialState(), { createTask, probeBilibiliPages, runTask });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV17KxizLE17?p=58";
    root.querySelector<HTMLButtonElement>("[data-testid='probe-pages']")!.click();

    await vi.waitFor(() => {
      expect(probeBilibiliPages).toHaveBeenCalledWith({
        url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
        has_login: false,
      });
      expect(root.textContent).toContain("剑桥少儿英语PowerUp 2nd Edition");
      expect(root.textContent).toContain("58");
      expect(root.textContent).toContain("60");
    });

    root.querySelector<HTMLInputElement>("[data-testid='page-range-start']")!.value = "58";
    root.querySelector<HTMLInputElement>("[data-testid='page-range-end']")!.value = "59";
    root.querySelector<HTMLButtonElement>("[data-testid='apply-page-range']")!.click();
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(createTask).toHaveBeenCalledWith({
        url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
        output_dir: "D:\\Videos\\bilibili",
        has_login: false,
        selected_pages: [58, 59],
      });
    });
  });

  test("creates checked multi-part pages from preview", async () => {
    const createdCollection = createdCollectionFixture();
    const createTask = vi.fn().mockResolvedValue({
      ...createdCollection,
      tasks: [createdCollection.tasks[0]],
    });
    const probeBilibiliPages = vi.fn().mockResolvedValue({
      groupTitle: "剑桥少儿英语PowerUp 2nd Edition",
      usedLogin: false,
      items: [58, 59, 60].map((page) => ({
        page,
        title: `分 P ${page}`,
        quality: "720P",
        requiresLogin: false,
      })),
    });
    const runTask = vi.fn().mockResolvedValue({
      ...createdCollection.tasks[0],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    renderApp(root, createInitialState(), { createTask, probeBilibiliPages, runTask });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV17KxizLE17?p=58";
    root.querySelector<HTMLButtonElement>("[data-testid='probe-pages']")!.click();

    await vi.waitFor(() => {
      expect(root.querySelector<HTMLInputElement>("[data-testid='page-checkbox-59']")).not.toBeNull();
    });
    root.querySelector<HTMLInputElement>("[data-testid='page-checkbox-59']")!.click();
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(createTask).toHaveBeenCalledWith({
        url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
        output_dir: "D:\\Videos\\bilibili",
        has_login: false,
        selected_pages: [58, 60],
      });
    });
  });

  test("polls persisted task groups while a task is running", async () => {
    const baseCollection = createdCollectionFixture();
    const createdCollection = {
      ...baseCollection,
      tasks: [baseCollection.tasks[0]],
    };
    const createTask = vi.fn().mockResolvedValue(createdCollection);
    let resolveRunTask: (task: CreatedTaskGroup["tasks"][number]) => void = () => {};
    const runTask = vi.fn().mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveRunTask = resolve;
        }),
    );
    const listTaskGroups = vi.fn().mockResolvedValue([
      {
        ...createdCollection,
        tasks: [
          {
            ...createdCollection.tasks[0],
            state: "downloading",
            bytes_downloaded: 75,
            bytes_total: 100,
          },
        ],
      },
    ]);
    renderApp(root, createInitialState(), {
      createTask,
      runTask,
      listTaskGroups,
      progressPollMs: 20,
    });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV1xx411c7mD";
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(root.textContent).toContain("01 - 安装 Tauri");
    });
    await vi.waitFor(() => {
      expect(listTaskGroups).toHaveBeenCalled();
      expect(root.textContent).toContain("75%");
    });

    resolveRunTask({
      ...createdCollection.tasks[0],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    await vi.waitFor(() => {
      expect(root.textContent).toContain("100%");
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
    const getToolStatus = vi.fn().mockResolvedValue({
      ytdlp: "missing",
      ffmpeg: "available",
      ffprobe: "available",
    });
    const state = createInitialState();
    renderApp(root, state, { saveConfig, getToolStatus });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    expect(root.textContent).toContain("yt-dlp缺失");
    expect(root.textContent).toContain("ffmpeg缺失");
    expect(root.textContent).toContain("ffprobe缺失");
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
    await vi.waitFor(() => {
      expect(getToolStatus).toHaveBeenCalledOnce();
      expect(root.textContent).toContain("ffmpeg可用");
      expect(root.textContent).toContain("ffprobe可用");
    });
    root.querySelector<HTMLButtonElement>("[data-tab='downloads']")?.click();
    expect(root.querySelector<HTMLInputElement>("[data-testid='output-directory']")?.value).toBe(
      "E:\\Videos\\bilibili",
    );
  });
});
