import { describe, expect, test } from "vitest";
import { createInitialState, platformRowText, taskOutputDirectory } from "./state";

describe("state defaults", () => {
  test("uses native as the default engine", () => {
    const state = createInitialState();

    expect(state.activeTab).toBe("downloads");
    expect(state.settings.defaultEngine).toBe("native");
    expect(state.settings.concurrency).toBe(2);
    expect(state.settings.ffmpegPath).toBeNull();
    expect(state.settings.ffprobePath).toBeNull();
    expect(state.toolStatus).toEqual({
      ytdlp: "missing",
      ffmpeg: "missing",
      ffprobe: "missing",
    });
    expect(state.update).toEqual({
      phase: "idle",
      currentVersion: "0.1.4",
      latestVersion: null,
      notes: null,
      error: null,
    });
  });

  test("prefills task output directory from the default root", () => {
    expect(taskOutputDirectory("D:\\Videos")).toBe("D:\\Videos\\bilibili");
  });

  test("keeps platform rows flat", () => {
    expect(platformRowText({ platform: "bilibili", status: "未登录" })).toEqual([
      "bilibili",
      "未登录",
    ]);
  });
});
