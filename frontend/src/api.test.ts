import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  clearBilibiliLogin,
  createTask,
  getConfig,
  getToolStatus,
  listTaskGroups,
  pollBilibiliLogin,
  probeBilibiliPages,
  runTask,
  saveConfig,
  startBilibiliLogin,
} from "./api";

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
      ytdlpPath: null,
      ffmpegPath: null,
      ffprobePath: null,
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

  test("creates task with selected pages when provided", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue({
      group: {
        id: "group-1",
        title: "剑桥少儿英语PowerUp 2nd Edition",
        output_dir: "D:\\Videos\\bilibili",
        state: "queued",
      },
      tasks: [],
    });

    await createTask({
      url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
      output_dir: "D:\\Videos\\bilibili",
      has_login: false,
      selected_pages: [58, 59],
    });

    expect(invokeMock).toHaveBeenCalledWith("create_task", {
      input: {
        url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
        output_dir: "D:\\Videos\\bilibili",
        has_login: false,
        selected_pages: [58, 59],
      },
    });
  });

  test("probes bilibili pages and maps metadata page numbers", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue({
      group_title: "剑桥少儿英语PowerUp 2nd Edition",
      used_login: false,
      items: [
        {
          title: "字幕版_PU2E_L0_Chant 1 Page 5_video",
          output_file: "58.mp4",
          quality: "720P",
          requires_login: false,
          metadata: {
            bvid: "BV17KxizLE17",
            cid: 58,
            page: 58,
          },
        },
      ],
    });

    await expect(
      probeBilibiliPages({
        url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
        has_login: false,
      }),
    ).resolves.toEqual({
      groupTitle: "剑桥少儿英语PowerUp 2nd Edition",
      usedLogin: false,
      items: [
        {
          title: "字幕版_PU2E_L0_Chant 1 Page 5_video",
          page: 58,
          quality: "720P",
          requiresLogin: false,
        },
      ],
    });
    expect(invokeMock).toHaveBeenCalledWith("probe_bilibili_pages", {
      input: {
        url: "https://www.bilibili.com/video/BV17KxizLE17?p=58",
        has_login: false,
      },
    });
  });

  test("saves config through Tauri command using snake case payload", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue({
      download_root: "E:\\Videos",
      concurrency: 4,
      default_engine: "yt_dlp",
      ytdlp_path: "C:\\tools\\yt-dlp.exe",
      ffmpeg_path: "C:\\tools\\ffmpeg.exe",
      ffprobe_path: "C:\\tools\\ffprobe.exe",
    });

    await expect(
      saveConfig({
        downloadRoot: "E:\\Videos",
        concurrency: 4,
        defaultEngine: "yt-dlp",
        ytdlpPath: "C:\\tools\\yt-dlp.exe",
        ffmpegPath: "C:\\tools\\ffmpeg.exe",
        ffprobePath: "C:\\tools\\ffprobe.exe",
      }),
    ).resolves.toEqual({
      downloadRoot: "E:\\Videos",
      concurrency: 4,
      defaultEngine: "yt-dlp",
      ytdlpPath: "C:\\tools\\yt-dlp.exe",
      ffmpegPath: "C:\\tools\\ffmpeg.exe",
      ffprobePath: "C:\\tools\\ffprobe.exe",
    });
    expect(invokeMock).toHaveBeenCalledWith("save_config", {
      input: {
        download_root: "E:\\Videos",
        concurrency: 4,
        default_engine: "yt_dlp",
        ytdlp_path: "C:\\tools\\yt-dlp.exe",
        ffmpeg_path: "C:\\tools\\ffmpeg.exe",
        ffprobe_path: "C:\\tools\\ffprobe.exe",
      },
    });
  });

  test("loads persisted task groups and normalizes task engines", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue([
      {
        group: {
          id: "group-1",
          title: "Rust 桌面应用入门",
          output_dir: "D:\\Videos\\bilibili",
          state: "completed",
        },
        tasks: [
          {
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
          },
        ],
      },
    ]);

    await expect(listTaskGroups()).resolves.toEqual([
      expect.objectContaining({
        group: expect.objectContaining({ id: "group-1" }),
        tasks: [expect.objectContaining({ id: "task-1", engine: "yt-dlp" })],
      }),
    ]);
    expect(invokeMock).toHaveBeenCalledWith("list_task_groups");
  });

  test("starts and polls bilibili QR login through Tauri commands", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock
      .mockResolvedValueOnce({
        qrcode_key: "qr-key-1",
        url: "https://passport.bilibili.com/qrcode/qr-key-1",
      })
      .mockResolvedValueOnce({
        status: "confirmed",
        message: "登录成功",
      });

    await expect(startBilibiliLogin()).resolves.toEqual({
      qrcode_key: "qr-key-1",
      url: "https://passport.bilibili.com/qrcode/qr-key-1",
    });
    await expect(pollBilibiliLogin({ qrcode_key: "qr-key-1" })).resolves.toEqual({
      status: "confirmed",
      message: "登录成功",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(1, "start_bilibili_login");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "poll_bilibili_login", {
      input: { qrcode_key: "qr-key-1" },
    });
  });

  test("clears bilibili login through Tauri command", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue(undefined);

    await expect(clearBilibiliLogin()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("clear_bilibili_login");
  });

  test("loads local media tool status through Tauri command", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    invokeMock.mockResolvedValue({
      ytdlp: "missing",
      ffmpeg: "available",
      ffprobe: "missing",
    });

    await expect(getToolStatus()).resolves.toEqual({
      ytdlp: "missing",
      ffmpeg: "available",
      ffprobe: "missing",
    });
    expect(invokeMock).toHaveBeenCalledWith("get_tool_status");
  });
});
