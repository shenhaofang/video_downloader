// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { renderApp } from "./render";
import { createInitialState, type CreatedTaskGroup } from "./state";

const testDir = dirname(fileURLToPath(import.meta.url));

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

  test("keeps the desktop sidebar fixed outside the workspace scroll", () => {
    const styles = readFileSync(join(testDir, "styles.css"), "utf8");

    expect(styles).toMatch(/\.sidebar\s*{[^}]*position:\s*sticky/s);
    expect(styles).toMatch(/\.sidebar\s*{[^}]*top:\s*0/s);
    expect(styles).toMatch(/\.sidebar\s*{[^}]*height:\s*100vh/s);
    expect(styles).toMatch(/\.sidebar\s*{[^}]*overflow:\s*hidden/s);
  });

  test("prefills the output directory input", () => {
    renderApp(root, createInitialState());

    const input = root.querySelector<HTMLInputElement>("[data-testid='output-directory']");
    expect(input).not.toBeNull();
    expect(input?.value).toBe("D:\\Videos\\bilibili");
  });

  test("chooses an output directory for the next download", async () => {
    const selectOutputDirectory = vi.fn().mockResolvedValue("E:\\Videos\\bilibili");
    const createTask = vi.fn().mockResolvedValue({
      ...createdCollectionFixture(),
      group: {
        ...createdCollectionFixture().group,
        output_dir: "E:\\Videos\\bilibili",
      },
      tasks: [createdCollectionFixture().tasks[0]],
    });
    const runTask = vi.fn().mockResolvedValue({
      ...createdCollectionFixture().tasks[0],
      state: "completed",
    });
    renderApp(root, createInitialState(), { selectOutputDirectory, createTask, runTask });

    root.querySelector<HTMLButtonElement>("[data-testid='select-output-directory']")?.click();

    await vi.waitFor(() => {
      expect(selectOutputDirectory).toHaveBeenCalledWith("D:\\Videos\\bilibili");
      expect(root.querySelector<HTMLInputElement>("[data-testid='output-directory']")?.value).toBe(
        "E:\\Videos\\bilibili",
      );
    });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV1xx411c7mD";
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(createTask).toHaveBeenCalledWith({
        url: "https://www.bilibili.com/video/BV1xx411c7mD",
        output_dir: "E:\\Videos\\bilibili",
        has_login: false,
      });
    });
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

  test("keeps multi-part preview scroll position when checking a page", async () => {
    const probeBilibiliPages = vi.fn().mockResolvedValue({
      groupTitle: "剑桥少儿英语PowerUp 2nd Edition",
      usedLogin: false,
      items: Array.from({ length: 80 }, (_, index) => {
        const page = index + 1;
        return {
          page,
          title: `分 P ${page}`,
          quality: "720P",
          requiresLogin: false,
        };
      }),
    });
    renderApp(root, createInitialState(), { probeBilibiliPages });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV17KxizLE17?p=58";
    root.querySelector<HTMLButtonElement>("[data-testid='probe-pages']")!.click();

    await vi.waitFor(() => {
      expect(root.querySelector<HTMLElement>(".page-preview-list")).not.toBeNull();
    });
    const list = root.querySelector<HTMLElement>(".page-preview-list")!;
    list.scrollTop = 320;
    root.querySelector<HTMLInputElement>("[data-testid='page-checkbox-40']")!.click();

    expect(root.querySelector<HTMLElement>(".page-preview-list")?.scrollTop).toBe(320);
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
      expect(root.querySelector<HTMLElement>(".state-pill")?.textContent).toBe("下载中");
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

  test("does not auto-start the next child after pausing an automatic run", async () => {
    const baseCollection = createdCollectionFixture();
    const createdCollection = {
      ...baseCollection,
      tasks: baseCollection.tasks.map((task) => ({
        ...task,
        state: "queued" as const,
      })),
    };
    const createTask = vi.fn().mockResolvedValue(createdCollection);
    let resolveFirstRun: (task: CreatedTaskGroup["tasks"][number]) => void = () => {};
    const runTask = vi.fn().mockImplementation(({ task_id }: { task_id: string }) => {
      if (task_id === "task-1") {
        return new Promise((resolve) => {
          resolveFirstRun = resolve;
        });
      }
      return Promise.resolve({
        ...createdCollection.tasks[1],
        state: "completed",
        bytes_downloaded: 100,
        bytes_total: 100,
      });
    });
    const pauseTask = vi.fn().mockResolvedValue({
      ...createdCollection.tasks[0],
      state: "paused",
    });
    const state = createInitialState();
    state.settings.concurrency = 1;
    renderApp(root, state, {
      createTask,
      runTask,
      pauseTask,
      progressPollMs: 10_000,
    });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV1xx411c7mD";
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(runTask).toHaveBeenCalledWith({ task_id: "task-1" });
    });
    root.querySelector<HTMLButtonElement>("[data-testid='pause-task-task-1']")!.click();

    await vi.waitFor(() => {
      expect(pauseTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(root.querySelector<HTMLElement>(".child-state")?.textContent).toBe("已暂停");
    });
    resolveFirstRun({
      ...createdCollection.tasks[0],
      state: "paused",
    });

    await vi.waitFor(() => {
      expect(runTask).toHaveBeenCalledTimes(1);
    });
  });

  test("refreshes persisted task state when running a task fails quickly", async () => {
    const baseCollection = createdCollectionFixture();
    const createdCollection = {
      ...baseCollection,
      tasks: [baseCollection.tasks[0]],
    };
    const failedCollection = {
      ...createdCollection,
      tasks: [
        {
          ...createdCollection.tasks[0],
          state: "failed" as const,
          retry_count: 1,
        },
      ],
    };
    const createTask = vi.fn().mockResolvedValue(createdCollection);
    const runTask = vi.fn().mockRejectedValue(new Error("ffmpeg missing"));
    const listTaskGroups = vi.fn().mockResolvedValue([failedCollection]);
    renderApp(root, createInitialState(), {
      createTask,
      runTask,
      listTaskGroups,
      progressPollMs: 10_000,
    });

    root.querySelector<HTMLInputElement>("[data-testid='video-url']")!.value =
      "https://www.bilibili.com/video/BV1xx411c7mD";
    root.querySelector<HTMLButtonElement>("[data-testid='add-task']")!.click();

    await vi.waitFor(() => {
      expect(runTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(listTaskGroups).toHaveBeenCalled();
      expect(root.querySelector<HTMLElement>(".state-pill")?.textContent).toBe("失败");
      expect(root.textContent).toContain("重试 1/3");
    });
  });

  test("shows task failure details in child rows", () => {
    const failedCollection = createdCollectionFixture();
    failedCollection.tasks[0] = {
      ...failedCollection.tasks[0],
      state: "failed",
      error_code: "platform_changed",
      error_message: "duplicate field `base_url`",
    };
    renderApp(root, {
      ...createInitialState(),
      taskGroups: [failedCollection],
    });

    expect(root.textContent).toContain("失败原因");
    expect(root.textContent).toContain("duplicate field `base_url`");
  });

  test("adds semantic color classes to group and child task states", () => {
    const failedCollection = createdCollectionFixture();
    failedCollection.tasks[0] = {
      ...failedCollection.tasks[0],
      state: "failed",
      error_message: "network failed",
    };
    failedCollection.tasks[1] = {
      ...failedCollection.tasks[1],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    };
    failedCollection.tasks.push(
      {
        ...failedCollection.tasks[0],
        id: "task-3",
        title: "03 - 打包发布",
        state: "queued",
      },
      {
        ...failedCollection.tasks[1],
        id: "task-4",
        title: "04 - 下载中",
        state: "downloading",
      },
      {
        ...failedCollection.tasks[1],
        id: "task-5",
        title: "05 - 已暂停",
        state: "paused",
      },
    );

    renderApp(root, {
      ...createInitialState(),
      taskGroups: [failedCollection],
    });

    const groupState = root.querySelector<HTMLElement>(".state-pill");
    const childStates = root.querySelectorAll<HTMLElement>(".child-state");

    expect(groupState?.textContent).toBe("下载中");
    expect(groupState?.classList.contains("state-downloading")).toBe(true);
    expect(childStates[0].classList.contains("state-failed")).toBe(true);
    expect(childStates[1].classList.contains("state-completed")).toBe(true);
    expect(childStates[2].classList.contains("state-queued")).toBe(true);
    expect(childStates[3].classList.contains("state-downloading")).toBe(true);
    expect(childStates[4].classList.contains("state-paused")).toBe(true);
  });

  test("keeps a collapsed task group collapsed while a task is downloading", () => {
    const collection = createdCollectionFixture();
    collection.group.state = "downloading";
    collection.tasks = collection.tasks.map((task) => ({
      ...task,
      state: task.id === "task-2" ? "downloading" : "completed",
      bytes_downloaded: task.id === "task-2" ? 50 : 100,
      bytes_total: 100,
    }));
    const state = {
      ...createInitialState(),
      taskGroups: [collection],
    };
    renderApp(root, state);

    const details = root.querySelector<HTMLDetailsElement>(".task-card details")!;
    expect(details.open).toBe(true);

    details.open = false;
    details.dispatchEvent(new Event("toggle"));
    renderApp(root, state);

    expect(root.querySelector<HTMLDetailsElement>(".task-card details")?.open).toBe(false);
  });

  test("shows only applicable child task actions for every state", () => {
    const collection = createdCollectionFixture();
    const matrix: Array<[CreatedTaskGroup["tasks"][number]["state"], string[]]> = [
      ["pending", ["pause", "delete"]],
      ["probing", ["pause", "delete"]],
      ["queued", ["start", "pause", "delete"]],
      ["downloading", ["pause", "delete"]],
      ["merging", ["pause", "delete"]],
      ["completed", ["delete"]],
      ["failed", ["retry", "delete"]],
      ["paused", ["start", "delete"]],
      ["interrupted", ["start", "pause", "delete"]],
      ["cancelled", ["delete"]],
    ];
    collection.tasks = matrix.map(([stateName], index) => ({
      ...collection.tasks[0],
      id: `${stateName}-task`,
      title: `${index + 1} - ${stateName}`,
      state: stateName,
      bytes_downloaded: stateName === "completed" ? 100 : 0,
      bytes_total: 100,
      error_message: stateName === "failed" ? "network failed" : null,
    }));

    renderApp(root, {
      ...createInitialState(),
      taskGroups: [collection],
    });

    for (const [stateName, expectedActions] of matrix) {
      for (const action of ["start", "pause", "retry", "delete"]) {
        const button = root.querySelector(`[data-testid='${action}-task-${stateName}-task']`);
        if (expectedActions.includes(action)) {
          expect(button, `${stateName} should show ${action}`).not.toBeNull();
        } else {
          expect(button, `${stateName} should hide ${action}`).toBeNull();
        }
      }
    }
  });

  test("shows only applicable group task actions for every state", () => {
    const matrix: Array<[CreatedTaskGroup["tasks"][number]["state"], string[]]> = [
      ["pending", ["pause", "delete"]],
      ["probing", ["pause", "delete"]],
      ["queued", ["continue", "pause", "delete"]],
      ["downloading", ["pause", "delete"]],
      ["merging", ["pause", "delete"]],
      ["completed", ["delete"]],
      ["failed", ["continue", "delete"]],
      ["paused", ["continue", "delete"]],
      ["interrupted", ["continue", "pause", "delete"]],
      ["cancelled", ["delete"]],
    ];

    for (const [stateName, expectedActions] of matrix) {
      const collection = createdCollectionFixture();
      collection.group.id = `${stateName}-group`;
      collection.group.state = stateName;
      collection.tasks = [
        {
          ...collection.tasks[0],
          id: `${stateName}-task`,
          state: stateName,
          bytes_downloaded: stateName === "completed" ? 100 : 0,
          bytes_total: 100,
          error_message: stateName === "failed" ? "network failed" : null,
        },
      ];
      renderApp(root, {
        ...createInitialState(),
        taskGroups: [collection],
      });

      for (const action of ["continue", "pause", "delete"]) {
        const button = root.querySelector(`[data-testid='${action}-group-${stateName}-group']`);
        if (expectedActions.includes(action)) {
          expect(button, `${stateName} group should show ${action}`).not.toBeNull();
        } else {
          expect(button, `${stateName} group should hide ${action}`).toBeNull();
        }
      }
    }
  });

  test("controls individual child tasks", async () => {
    const createdCollection = createdCollectionFixture();
    const pauseTask = vi.fn().mockResolvedValue({
      ...createdCollection.tasks[1],
      state: "paused",
    });
    const startTask = vi.fn().mockResolvedValue({
      ...createdCollection.tasks[1],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    const failedCollection = {
      ...createdCollection,
      tasks: [
        {
          ...createdCollection.tasks[0],
          state: "failed" as const,
          error_message: "network failed",
        },
        createdCollection.tasks[1],
      ],
    };
    const retryTask = vi.fn().mockResolvedValue({
      ...failedCollection.tasks[0],
      state: "completed",
      error_message: null,
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    const deleteTask = vi.fn().mockResolvedValue([
      {
        ...createdCollection,
        tasks: [createdCollection.tasks[1]],
      },
    ]);
    const state = {
      ...createInitialState(),
      taskGroups: [failedCollection],
    };
    renderApp(root, state, {
      pauseTask,
      startTask,
      retryTask,
      deleteTask,
      progressPollMs: 10_000,
    });

    root.querySelector<HTMLButtonElement>("[data-testid='pause-task-task-2']")!.click();
    await vi.waitFor(() => {
      expect(pauseTask).toHaveBeenCalledWith({ task_id: "task-2" });
      expect(root.textContent).toContain("已暂停");
    });

    root.querySelector<HTMLButtonElement>("[data-testid='retry-task-task-1']")!.click();
    await vi.waitFor(() => {
      expect(retryTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(root.textContent).toContain("100%");
    });

    root.querySelector<HTMLButtonElement>("[data-testid='start-task-task-2']")!.click();
    await vi.waitFor(() => {
      expect(startTask).toHaveBeenCalledWith({ task_id: "task-2" });
    });

    root.querySelector<HTMLButtonElement>("[data-testid='delete-task-task-1']")!.click();
    await vi.waitFor(() => {
      expect(deleteTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(root.textContent).not.toContain("01 - 安装 Tauri");
    });
  });

  test("refreshes persisted task state when manual retry fails", async () => {
    const createdCollection = createdCollectionFixture();
    const failedCollection = {
      ...createdCollection,
      tasks: [
        {
          ...createdCollection.tasks[0],
          state: "failed" as const,
          error_message: "ffmpeg failed",
          retry_count: 2,
        },
      ],
    };
    const retryTask = vi.fn().mockRejectedValue(new Error("ffmpeg failed"));
    const listTaskGroups = vi.fn().mockResolvedValue([failedCollection]);
    const state = {
      ...createInitialState(),
      taskGroups: [
        {
          ...createdCollection,
          tasks: [
            {
              ...createdCollection.tasks[0],
              state: "failed" as const,
              error_message: "network failed",
            },
          ],
        },
      ],
    };
    renderApp(root, state, {
      retryTask,
      listTaskGroups,
      progressPollMs: 10_000,
    });

    root.querySelector<HTMLButtonElement>("[data-testid='retry-task-task-1']")!.click();

    await vi.waitFor(() => {
      expect(retryTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(listTaskGroups).toHaveBeenCalled();
      expect(root.textContent).toContain("ffmpeg failed");
      expect(root.textContent).toContain("重试 2/3");
    });
  });

  test("continues unfinished child tasks from the group action", async () => {
    const createdCollection = createdCollectionFixture();
    const partialCollection = {
      ...createdCollection,
      tasks: [
        {
          ...createdCollection.tasks[0],
          state: "completed" as const,
          bytes_downloaded: 100,
          bytes_total: 100,
        },
        {
          ...createdCollection.tasks[1],
          state: "queued" as const,
          bytes_downloaded: 0,
          bytes_total: 100,
        },
        {
          ...createdCollection.tasks[0],
          id: "task-3",
          title: "03 - 打包发布",
          state: "failed" as const,
          error_message: "network failed",
        },
        {
          ...createdCollection.tasks[0],
          id: "task-4",
          title: "04 - 已取消",
          state: "cancelled" as const,
        },
      ],
    };
    const startTask = vi.fn().mockResolvedValue({
      ...partialCollection.tasks[1],
      state: "completed",
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    const retryTask = vi.fn().mockResolvedValue({
      ...partialCollection.tasks[2],
      state: "completed",
      error_message: null,
      bytes_downloaded: 100,
      bytes_total: 100,
    });
    const runTask = vi.fn();
    renderApp(
      root,
      {
        ...createInitialState(),
        taskGroups: [partialCollection],
      },
      { startTask, retryTask, runTask, progressPollMs: 10_000 },
    );

    root.querySelector<HTMLButtonElement>("[data-testid='continue-group-group-1']")!.click();

    await vi.waitFor(() => {
      expect(startTask).toHaveBeenCalledWith({ task_id: "task-2" });
      expect(retryTask).toHaveBeenCalledWith({ task_id: "task-3" });
      expect(startTask).not.toHaveBeenCalledWith({ task_id: "task-1" });
      expect(startTask).not.toHaveBeenCalledWith({ task_id: "task-4" });
      expect(retryTask).not.toHaveBeenCalledWith({ task_id: "task-4" });
      expect(runTask).not.toHaveBeenCalled();
      expect(root.querySelector<HTMLElement>(".state-pill")?.textContent).toBe("已取消");
    });
  });

  test("pauses active child tasks from the group action", async () => {
    const createdCollection = createdCollectionFixture();
    const activeCollection = {
      ...createdCollection,
      tasks: [
        {
          ...createdCollection.tasks[0],
          state: "completed" as const,
          bytes_downloaded: 100,
          bytes_total: 100,
        },
        {
          ...createdCollection.tasks[1],
          state: "queued" as const,
          bytes_downloaded: 0,
          bytes_total: 100,
        },
        {
          ...createdCollection.tasks[0],
          id: "task-3",
          title: "03 - 打包发布",
          state: "downloading" as const,
          bytes_downloaded: 40,
          bytes_total: 100,
        },
        {
          ...createdCollection.tasks[0],
          id: "task-4",
          title: "04 - 已暂停",
          state: "paused" as const,
        },
        {
          ...createdCollection.tasks[0],
          id: "task-5",
          title: "05 - 失败",
          state: "failed" as const,
        },
        {
          ...createdCollection.tasks[0],
          id: "task-6",
          title: "06 - 已取消",
          state: "cancelled" as const,
        },
      ],
    };
    const pauseTask = vi.fn().mockImplementation(({ task_id }: { task_id: string }) => {
      const task = activeCollection.tasks.find((item) => item.id === task_id)!;
      return Promise.resolve({
        ...task,
        state: "paused",
      });
    });
    renderApp(
      root,
      {
        ...createInitialState(),
        taskGroups: [activeCollection],
      },
      { pauseTask },
    );

    root.querySelector<HTMLButtonElement>("[data-testid='pause-group-group-1']")!.click();

    await vi.waitFor(() => {
      expect(pauseTask).toHaveBeenCalledTimes(2);
      expect(pauseTask).toHaveBeenCalledWith({ task_id: "task-2" });
      expect(pauseTask).toHaveBeenCalledWith({ task_id: "task-3" });
      expect(pauseTask).not.toHaveBeenCalledWith({ task_id: "task-1" });
      expect(pauseTask).not.toHaveBeenCalledWith({ task_id: "task-4" });
      expect(pauseTask).not.toHaveBeenCalledWith({ task_id: "task-5" });
      expect(pauseTask).not.toHaveBeenCalledWith({ task_id: "task-6" });
      expect(root.querySelector<HTMLElement>(".state-pill")?.textContent).toBe("失败");
    });
  });

  test("deletes all child tasks from the group action", async () => {
    const createdCollection = createdCollectionFixture();
    const deleteTask = vi.fn().mockResolvedValue([]);
    renderApp(
      root,
      {
        ...createInitialState(),
        taskGroups: [createdCollection],
      },
      { deleteTask },
    );

    root.querySelector<HTMLButtonElement>("[data-testid='delete-group-group-1']")!.click();

    await vi.waitFor(() => {
      expect(deleteTask).toHaveBeenCalledWith({ task_id: "task-1" });
      expect(deleteTask).toHaveBeenCalledWith({ task_id: "task-2" });
      expect(root.textContent).not.toContain("Rust 桌面应用入门");
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

  test("installs yt-dlp from settings and refreshes status", async () => {
    const installYtDlp = vi.fn().mockResolvedValue({
      downloadRoot: "D:\\Videos",
      concurrency: 2,
      defaultEngine: "native",
      ytdlpPath: "C:\\Program Files\\Video Downloader\\dependencies\\yt-dlp\\yt-dlp.exe",
      ffmpegPath: null,
      ffprobePath: null,
    });
    const getToolStatus = vi.fn().mockResolvedValue({
      ytdlp: "available",
      ffmpeg: "missing",
      ffprobe: "missing",
    });
    const state = createInitialState();
    renderApp(root, state, { installYtDlp, getToolStatus });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLButtonElement>("[data-testid='install-ytdlp']")?.click();

    await vi.waitFor(() => {
      expect(installYtDlp).toHaveBeenCalledOnce();
      expect(root.querySelector<HTMLInputElement>("[data-testid='ytdlp-path']")?.value).toBe(
        "C:\\Program Files\\Video Downloader\\dependencies\\yt-dlp\\yt-dlp.exe",
      );
      expect(root.textContent).toContain("yt-dlp可用");
    });
  });

  test("installs FFmpeg media tools from settings and refreshes status", async () => {
    const installMediaTools = vi.fn().mockResolvedValue({
      downloadRoot: "D:\\Videos",
      concurrency: 2,
      defaultEngine: "native",
      ytdlpPath: null,
      ffmpegPath: "C:\\Program Files\\Video Downloader\\dependencies\\ffmpeg\\bin\\ffmpeg.exe",
      ffprobePath: "C:\\Program Files\\Video Downloader\\dependencies\\ffmpeg\\bin\\ffprobe.exe",
    });
    const getToolStatus = vi.fn().mockResolvedValue({
      ytdlp: "missing",
      ffmpeg: "available",
      ffprobe: "available",
    });
    const state = createInitialState();
    renderApp(root, state, { installMediaTools, getToolStatus });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLButtonElement>("[data-testid='install-media-tools']")?.click();

    await vi.waitFor(() => {
      expect(installMediaTools).toHaveBeenCalledOnce();
      expect(root.querySelector<HTMLInputElement>("[data-testid='ffmpeg-path']")?.value).toBe(
        "C:\\Program Files\\Video Downloader\\dependencies\\ffmpeg\\bin\\ffmpeg.exe",
      );
      expect(root.textContent).toContain("ffmpeg可用");
      expect(root.textContent).toContain("ffprobe可用");
    });
  });

  test("checks for app updates and starts install when no tasks are active", async () => {
    const checkAppUpdate = vi.fn().mockResolvedValue({
      available: true,
      currentVersion: "0.1.0",
      latestVersion: "0.1.1",
      notes: "修复下载恢复",
      pubDate: "2026-05-23T00:00:00Z",
    });
    const installAppUpdate = vi.fn().mockResolvedValue(undefined);
    const state = createInitialState();
    renderApp(root, state, { checkAppUpdate, installAppUpdate });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLButtonElement>("[data-testid='check-update']")?.click();

    await vi.waitFor(() => {
      expect(checkAppUpdate).toHaveBeenCalledOnce();
      expect(root.textContent).toContain("发现 0.1.1");
      expect(root.textContent).toContain("修复下载恢复");
    });

    root.querySelector<HTMLButtonElement>("[data-testid='install-update']")?.click();

    await vi.waitFor(() => {
      expect(installAppUpdate).toHaveBeenCalledOnce();
      expect(root.textContent).toContain("正在下载并安装");
    });
  });

  test("shows app update progress from update progress events", async () => {
    const checkAppUpdate = vi.fn().mockResolvedValue({
      available: true,
      currentVersion: "0.1.0",
      latestVersion: "0.1.1",
      notes: null,
      pubDate: null,
    });
    let progressHandler: (progress: {
      downloaded: number;
      total?: number | null;
      percent?: number | null;
    }) => void = () => {
      throw new Error("progress listener was not registered");
    };
    const stopListening = vi.fn();
    const listenAppUpdateProgress = vi.fn().mockImplementation((handler) => {
      progressHandler = handler;
      return Promise.resolve(stopListening);
    });
    const installAppUpdate = vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          window.setTimeout(resolve, 10);
        }),
    );
    const state = createInitialState();
    renderApp(root, state, { checkAppUpdate, installAppUpdate, listenAppUpdateProgress });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLButtonElement>("[data-testid='check-update']")?.click();
    await vi.waitFor(() => {
      expect(root.textContent).toContain("发现 0.1.1");
    });

    root.querySelector<HTMLButtonElement>("[data-testid='install-update']")?.click();
    await vi.waitFor(() => {
      expect(listenAppUpdateProgress).toHaveBeenCalledOnce();
    });
    progressHandler({ downloaded: 512, total: 1024, percent: 50 });

    await vi.waitFor(() => {
      expect(root.textContent).toContain("更新进度 50%");
    });
    await vi.waitFor(() => {
      expect(stopListening).toHaveBeenCalledOnce();
    });
  });

  test("shows structured update check errors as readable text", async () => {
    const checkAppUpdate = vi.fn().mockRejectedValue({
      code: "update_error",
      message: "无法下载更新元数据",
    });
    const state = createInitialState();
    renderApp(root, state, { checkAppUpdate });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLButtonElement>("[data-testid='check-update']")?.click();

    await vi.waitFor(() => {
      expect(root.textContent).toContain("无法下载更新元数据");
      expect(root.textContent).not.toContain("[object Object]");
    });
  });

  test("disables update install while queued or running tasks exist", async () => {
    const checkAppUpdate = vi.fn().mockResolvedValue({
      available: true,
      currentVersion: "0.1.0",
      latestVersion: "0.1.1",
      notes: null,
      pubDate: null,
    });
    const state = createInitialState();
    state.taskGroups = [createdCollectionFixture()];
    renderApp(root, state, { checkAppUpdate });

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();
    root.querySelector<HTMLButtonElement>("[data-testid='check-update']")?.click();

    await vi.waitFor(() => {
      expect(root.querySelector<HTMLButtonElement>("[data-testid='install-update']")?.disabled).toBe(
        true,
      );
      expect(root.textContent).toContain("请先暂停或完成下载任务再更新");
    });
  });

  test("uses install-root dependency placeholders instead of legacy C tools paths", () => {
    const state = createInitialState();
    renderApp(root, state);

    root.querySelector<HTMLButtonElement>("[data-tab='settings']")?.click();

    expect(
      root.querySelector<HTMLInputElement>("[data-testid='ytdlp-path']")?.placeholder,
    ).toBe("dependencies\\yt-dlp\\yt-dlp.exe");
    expect(
      root.querySelector<HTMLInputElement>("[data-testid='ffmpeg-path']")?.placeholder,
    ).toBe("dependencies\\ffmpeg\\bin\\ffmpeg.exe");
    expect(
      root.querySelector<HTMLInputElement>("[data-testid='ffprobe-path']")?.placeholder,
    ).toBe("dependencies\\ffmpeg\\bin\\ffprobe.exe");
  });
});
