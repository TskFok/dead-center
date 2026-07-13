import { describe, expect, it } from "vitest";

import {
  CROSSHAIR_SIZE_PERCENT_MAX,
  CROSSHAIR_SIZE_PERCENT_MIN,
  DEFAULT_SETTINGS,
  isDiamondPreset,
  isHexColor,
  normalizeVisualSettings,
} from "./settings";

describe("DEFAULT_SETTINGS", () => {
  it("默认使用版本 2 和 3% 尺寸", () => {
    expect(DEFAULT_SETTINGS.version).toBe(2);
    expect(DEFAULT_SETTINGS.visual).toMatchObject({
      preset: "classic-cross",
      primaryColor: "#4DFFB8",
      accentColor: "#F4FF4D",
      opacity: 0.8,
      sizePercent: 3,
      strokePx: 3,
      gapPx: 8,
    });
    expect(DEFAULT_SETTINGS.visual).not.toHaveProperty("sizePx");
    expect(DEFAULT_SETTINGS.toggleShortcut).toBe("Alt+Shift+X");
    expect(DEFAULT_SETTINGS.showOnLaunch).toBe(true);
    expect(DEFAULT_SETTINGS.launchAtLogin).toBe(false);
  });
});

describe("isDiamondPreset", () => {
  it.each([
    "fine-diamond",
    "inward-diamond",
    "long-diamond",
    "solid-diamond",
  ] as const)("识别菱形旗标准星 %s", (preset) => {
    expect(isDiamondPreset(preset)).toBe(true);
  });

  it.each(["dot-ring", "classic-cross", "soft-target"] as const)(
    "不把旧方案 %s 识别为菱形旗标",
    (preset) => {
      expect(isDiamondPreset(preset)).toBe(false);
    },
  );
});

describe("normalizeVisualSettings", () => {
  it("把百分比尺寸限制到 0 至 100", () => {
    expect(CROSSHAIR_SIZE_PERCENT_MIN).toBe(0);
    expect(CROSSHAIR_SIZE_PERCENT_MAX).toBe(100);
    expect(
      normalizeVisualSettings({
        ...DEFAULT_SETTINGS.visual,
        sizePercent: 120,
      }).sizePercent,
    ).toBe(100);
    expect(
      normalizeVisualSettings({
        ...DEFAULT_SETTINGS.visual,
        sizePercent: -1,
      }).sizePercent,
    ).toBe(0);
  });

  it("把透明度、线宽和缺口限制到产品边界", () => {
    expect(
      normalizeVisualSettings({
        ...DEFAULT_SETTINGS.visual,
        opacity: 0,
        strokePx: 0,
        gapPx: 30,
      }),
    ).toMatchObject({ opacity: 0.1, strokePx: 1, gapPx: 24 });
  });

  it("拒绝无效颜色并恢复默认颜色", () => {
    expect(
      normalizeVisualSettings({
        ...DEFAULT_SETTINGS.visual,
        primaryColor: "green",
        accentColor: "#12345",
      }),
    ).toMatchObject({
      primaryColor: "#4DFFB8",
      accentColor: "#F4FF4D",
    });
  });
});

describe("isHexColor", () => {
  it("只接受六位十六进制颜色", () => {
    expect(isHexColor("#abcdef")).toBe(true);
    expect(isHexColor("#ABC123")).toBe(true);
    expect(isHexColor("#fff")).toBe(false);
    expect(isHexColor("red")).toBe(false);
  });
});
