import { describe, expect, it } from "vitest";

import {
  DEFAULT_SETTINGS,
  isHexColor,
  normalizeVisualSettings,
} from "./settings";

describe("DEFAULT_SETTINGS", () => {
  it("使用缺口十字和已确认的默认视觉参数", () => {
    expect(DEFAULT_SETTINGS.visual).toEqual({
      preset: "classic-cross",
      primaryColor: "#4DFFB8",
      accentColor: "#F4FF4D",
      opacity: 0.8,
      sizePx: 32,
      strokePx: 3,
      gapPx: 8,
    });
    expect(DEFAULT_SETTINGS.toggleShortcut).toBe("Alt+Shift+X");
    expect(DEFAULT_SETTINGS.showOnLaunch).toBe(true);
    expect(DEFAULT_SETTINGS.launchAtLogin).toBe(false);
  });
});

describe("normalizeVisualSettings", () => {
  it("把超出范围的数值限制到产品边界", () => {
    expect(
      normalizeVisualSettings({
        ...DEFAULT_SETTINGS.visual,
        opacity: 0,
        sizePx: 120,
        strokePx: 0,
        gapPx: 30,
      }),
    ).toMatchObject({
      opacity: 0.1,
      sizePx: 96,
      strokePx: 1,
      gapPx: 24,
    });
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
