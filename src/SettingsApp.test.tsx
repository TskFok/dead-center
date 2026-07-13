import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AppBridge } from "./shared/bridge";
import { DEFAULT_SETTINGS, type AppSnapshot } from "./shared/settings";
import { SettingsApp } from "./SettingsApp";

const snapshot: AppSnapshot = {
  settings: DEFAULT_SETTINGS,
  status: {
    visible: true,
    resolvedMonitorId: "primary",
    usingFallbackMonitor: false,
    sessionType: "macos",
  },
  monitors: [
    {
      id: "primary",
      name: "主显示器",
      isPrimary: true,
      width: 2560,
      height: 1440,
      scaleFactor: 2,
    },
  ],
};

const makeBridge = (): AppBridge => ({
  getSnapshot: vi.fn().mockResolvedValue(snapshot),
  updateVisual: vi.fn().mockResolvedValue(snapshot),
  selectMonitor: vi.fn().mockResolvedValue(snapshot),
  setToggleShortcut: vi.fn().mockResolvedValue(snapshot),
  setLaunchAtLogin: vi.fn().mockResolvedValue(snapshot),
  setShowOnLaunch: vi.fn().mockResolvedValue(snapshot),
  setVisibility: vi.fn().mockResolvedValue(snapshot),
  retryOverlay: vi.fn().mockResolvedValue(snapshot),
  onVisualChanged: vi.fn().mockResolvedValue(() => undefined),
  onRuntimeChanged: vi.fn().mockResolvedValue(() => undefined),
});

describe("SettingsApp", () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  it("加载快照后显示三种预设和当前状态", async () => {
    render(<SettingsApp bridge={makeBridge()} />);

    expect(await screen.findByRole("button", { name: "缺口十字" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "圆环与中心点" })).toBeVisible();
    expect(screen.getByRole("button", { name: "柔和同心标记" })).toBeVisible();
    expect(screen.getByText("准星已显示")).toBeVisible();
  });

  it("切换到圆环预设后禁用缺口并防抖保存", async () => {
    vi.useFakeTimers();
    const bridge = makeBridge();
    render(<SettingsApp bridge={bridge} />);
    await act(async () => undefined);
    expect(screen.getByRole("button", { name: "缺口十字" })).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "圆环与中心点" }));

    expect(screen.getByLabelText("中心缺口")).toBeDisabled();
    expect(bridge.updateVisual).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(100));
    expect(bridge.updateVisual).toHaveBeenCalledWith(
      expect.objectContaining({ preset: "dot-ring" }),
    );
  });
});
