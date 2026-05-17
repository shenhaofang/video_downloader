import { beforeEach, describe, expect, test, vi } from "vitest";
import { createTask, getConfig } from "./api";

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
});
