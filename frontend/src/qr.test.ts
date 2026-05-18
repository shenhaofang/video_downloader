import { describe, expect, test } from "vitest";
import { createQrDataUrl } from "./qr";

describe("createQrDataUrl", () => {
  test("renders text as an SVG data URL", async () => {
    const dataUrl = await createQrDataUrl("https://passport.bilibili.com/qrcode/qr-key-1");

    expect(dataUrl).toMatch(/^data:image\/svg\+xml;charset=UTF-8,/);
    expect(decodeURIComponent(dataUrl.split(",")[1])).toContain("<svg");
  });
});
