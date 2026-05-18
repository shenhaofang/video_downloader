import { beforeEach, describe, expect, test, vi } from "vitest";
import { createTask, getConfig, runTask } from "./api";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

describe("api fallback detection", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.unstubAllGlobals();
  });

  test("falls back deterministically when Tauri runtime is absent", async () => {
    invokeMock.mockRejectedValue(new Error("__TAURI_INTERNALS__ is not defined"));

    await expect(getConfig()).resolves.toEqual({
      downloadRoot: "D:\\Videos",
      concurrency: 2,
      defaultEngine: "native",
    });
    expect(invokeMock).toHaveBeenCalledWith("get_config");
  });

  test("rethrows command errors when Tauri runtime exists", async () => {
    const error = new Error("tool not available");
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockRejectedValue(error);

    await expect(
      createTask({
        url: "https://www.bilibili.com/video/BV1xx411c7mD",
        output_dir: "D:\\Videos\\bilibili",
        has_login: false,
      }),
    ).rejects.toBe(error);
  });

  test("runs task through Tauri command and normalizes engine", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue({
      id: "task-1",
      title: "安装 Tauri",
      output_file: "D:\\Videos\\out.mp4",
      state: "completed",
      bytes_downloaded: 9,
      bytes_total: 9,
      retry_count: 0,
      max_retries: 3,
      quality: "720P",
      used_login: false,
      engine: "yt_dlp",
    });

    await expect(runTask({ task_id: "task-1" })).resolves.toMatchObject({
      id: "task-1",
      state: "completed",
      engine: "yt-dlp",
    });
    expect(invokeMock).toHaveBeenCalledWith("run_task", { input: { task_id: "task-1" } });
  });
});
