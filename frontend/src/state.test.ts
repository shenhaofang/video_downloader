import { describe, expect, test } from "vitest";
import { createInitialState, platformRowText, taskOutputDirectory } from "./state";

describe("state defaults", () => {
  test("uses native as the default engine", () => {
    const state = createInitialState();

    expect(state.activeTab).toBe("downloads");
    expect(state.settings.defaultEngine).toBe("native");
    expect(state.settings.concurrency).toBe(2);
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
