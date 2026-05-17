// @vitest-environment jsdom
import { beforeEach, describe, expect, test, vi } from "vitest";
import { renderApp } from "./render";
import { createInitialState, type CreatedTaskGroup } from "./state";

const createdCollection: CreatedTaskGroup = {
  group: {
    title: "Rust 桌面应用入门",
    output_dir: "D:\\Videos\\bilibili",
    state: "queued",
  },
  tasks: [
    {
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
      "Downloads",
      "Login",
      "Settings",
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

  test("renders created collection children with output, progress, and retries", async () => {
    const createTask = vi.fn().mockResolvedValue(createdCollection);
    renderApp(root, createInitialState(), { createTask });

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
});
