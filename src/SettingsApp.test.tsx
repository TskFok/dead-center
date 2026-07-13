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

const PRESET_NAMES = [
  "圆环与中心点",
  "缺口十字",
  "柔和同心标记",
  "细旗空心菱形",
  "内向旗空心菱形",
  "长旗空心菱形",
  "长旗实心菱形",
] as const;

describe("SettingsApp", () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  it("加载快照后显示七种预设、通用颜色文案和当前状态", async () => {
    render(<SettingsApp bridge={makeBridge()} />);

    expect(await screen.findByRole("button", { name: "缺口十字" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    for (const name of PRESET_NAMES) {
      expect(screen.getByRole("button", { name })).toBeVisible();
    }
    expect(screen.getByLabelText("中心标记颜色")).toBeVisible();
    expect(screen.getByLabelText("整体尺寸")).toHaveAttribute("min", "0");
    expect(screen.getByLabelText("整体尺寸")).toHaveAttribute("max", "100");
    expect(screen.getByText("3%")).toBeVisible();
    expect(screen.getByText("准星已显示")).toBeVisible();
    const viewport = screen.getByLabelText("目标屏幕预览画布");
    expect(viewport).toHaveAttribute("data-monitor-id", "primary");
    expect(viewport).toHaveStyle({ aspectRatio: "2560 / 1440" });
  });

  it("预览使用运行时实际解析出的回退屏幕比例", async () => {
    const fallbackSnapshot: AppSnapshot = {
      ...snapshot,
      status: {
        ...snapshot.status,
        resolvedMonitorId: "fallback",
        usingFallbackMonitor: true,
      },
      monitors: [
        ...snapshot.monitors,
        {
          id: "fallback",
          name: "回退屏幕",
          isPrimary: false,
          width: 1920,
          height: 1200,
          scaleFactor: 1,
        },
      ],
    };
    const bridge = makeBridge();
    vi.mocked(bridge.getSnapshot).mockResolvedValue(fallbackSnapshot);
    render(<SettingsApp bridge={bridge} />);

    const viewport = await screen.findByLabelText("目标屏幕预览画布");
    expect(viewport).toHaveAttribute("data-monitor-id", "fallback");
    expect(viewport).toHaveStyle({ aspectRatio: "1920 / 1200" });
  });

  it("运行时变化后刷新完整屏幕快照且保留防抖中的草稿", async () => {
    vi.useFakeTimers();
    const fallbackSnapshot: AppSnapshot = {
      ...snapshot,
      settings: {
        ...snapshot.settings,
        visual: {
          ...snapshot.settings.visual,
          sizePercent: 73,
        },
        targetMonitorId: "fallback",
        toggleShortcut: "Alt+Shift+R",
      },
      status: {
        ...snapshot.status,
        resolvedMonitorId: "fallback",
        usingFallbackMonitor: true,
      },
      monitors: [
        ...snapshot.monitors,
        {
          id: "fallback",
          name: "运行时回退屏幕",
          isPrimary: false,
          width: 1920,
          height: 1200,
          scaleFactor: 1,
        },
      ],
    };
    let runtimeHandler: Parameters<AppBridge["onRuntimeChanged"]>[0] | undefined;
    const bridge = makeBridge();
    vi.mocked(bridge.getSnapshot)
      .mockResolvedValueOnce(snapshot)
      .mockResolvedValueOnce(fallbackSnapshot);
    vi.mocked(bridge.onRuntimeChanged).mockImplementation(async (handler) => {
      runtimeHandler = handler;
      return () => undefined;
    });
    render(<SettingsApp bridge={bridge} />);
    await act(async () => undefined);

    fireEvent.change(screen.getByLabelText("整体尺寸"), {
      target: { value: "25" },
    });
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Alt+Shift+D" },
    });
    expect(runtimeHandler).toBeTypeOf("function");

    await act(async () => runtimeHandler?.(fallbackSnapshot.status));

    expect(bridge.getSnapshot).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("option", { name: "运行时回退屏幕 · 1920×1200" })).toBeVisible();
    expect(screen.getByText("目标屏幕离线，已回退主屏")).toBeVisible();
    const viewport = screen.getByLabelText("目标屏幕预览画布");
    expect(viewport).toHaveAttribute("data-monitor-id", "fallback");
    expect(viewport).toHaveStyle({ aspectRatio: "1920 / 1200" });
    expect(screen.getByLabelText("整体尺寸")).toHaveValue("25");
    expect(screen.getByRole("textbox")).toHaveValue("Alt+Shift+D");
    expect(bridge.updateVisual).not.toHaveBeenCalled();
  });

  it("处理运行时订阅和快照刷新失败", async () => {
    const subscriptionBridge = makeBridge();
    vi.mocked(subscriptionBridge.onRuntimeChanged).mockRejectedValue(
      new Error("运行时订阅失败"),
    );
    const first = render(<SettingsApp bridge={subscriptionBridge} />);
    expect(await screen.findByText("运行时订阅失败")).toBeVisible();
    first.unmount();

    let runtimeHandler: Parameters<AppBridge["onRuntimeChanged"]>[0] | undefined;
    const refreshBridge = makeBridge();
    vi.mocked(refreshBridge.getSnapshot)
      .mockResolvedValueOnce(snapshot)
      .mockRejectedValueOnce(new Error("运行时快照失败"));
    vi.mocked(refreshBridge.onRuntimeChanged).mockImplementation(async (handler) => {
      runtimeHandler = handler;
      return () => undefined;
    });
    render(<SettingsApp bridge={refreshBridge} />);
    await act(async () => undefined);
    expect(runtimeHandler).toBeTypeOf("function");

    await act(async () => runtimeHandler?.(snapshot.status));

    expect(await screen.findByText("运行时快照失败")).toBeVisible();
  });

  it("卸载时可靠解除尚在注册中的运行时监听", async () => {
    let resolveUnlisten: ((unlisten: () => void) => void) | undefined;
    const unlisten = vi.fn();
    const bridge = makeBridge();
    vi.mocked(bridge.onRuntimeChanged).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveUnlisten = resolve;
        }),
    );
    const view = render(<SettingsApp bridge={bridge} />);
    await act(async () => undefined);
    expect(bridge.onRuntimeChanged).toHaveBeenCalledOnce();

    view.unmount();
    await act(async () => resolveUnlisten?.(unlisten));

    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("解析屏幕尺寸无效时回退到有效主屏比例", async () => {
    const invalidResolvedSnapshot: AppSnapshot = {
      ...snapshot,
      status: {
        ...snapshot.status,
        resolvedMonitorId: "invalid-resolved",
      },
      monitors: [
        {
          id: "invalid-resolved",
          name: "零宽屏幕",
          isPrimary: false,
          width: 0,
          height: 1080,
          scaleFactor: 1,
        },
        ...snapshot.monitors,
      ],
    };
    const bridge = makeBridge();
    vi.mocked(bridge.getSnapshot).mockResolvedValue(invalidResolvedSnapshot);
    render(<SettingsApp bridge={bridge} />);

    const viewport = await screen.findByLabelText("目标屏幕预览画布");
    expect(viewport).toHaveAttribute("data-monitor-id", "primary");
    expect(viewport).toHaveStyle({ aspectRatio: "2560 / 1440" });
  });

  it("所有屏幕尺寸无效时回退安全的 16:9 预览", async () => {
    const invalidSnapshot: AppSnapshot = {
      ...snapshot,
      status: {
        ...snapshot.status,
        resolvedMonitorId: "zero-height",
      },
      monitors: [
        {
          id: "zero-height",
          name: "零高屏幕",
          isPrimary: true,
          width: 1920,
          height: 0,
          scaleFactor: 1,
        },
        {
          id: "infinite-width",
          name: "无限宽屏幕",
          isPrimary: false,
          width: Number.POSITIVE_INFINITY,
          height: 1080,
          scaleFactor: 1,
        },
      ],
    };
    const bridge = makeBridge();
    vi.mocked(bridge.getSnapshot).mockResolvedValue(invalidSnapshot);
    render(<SettingsApp bridge={bridge} />);

    const viewport = await screen.findByLabelText("目标屏幕预览画布");
    expect(viewport).toHaveAttribute("data-monitor-id", "unknown");
    expect(viewport).toHaveStyle({ aspectRatio: "16 / 9" });
    expect(viewport.getAttribute("style")).not.toMatch(/NaN|Infinity/);
  });

  it("没有屏幕时回退安全的 16:9 预览", async () => {
    const noMonitorSnapshot: AppSnapshot = {
      ...snapshot,
      status: {
        ...snapshot.status,
        resolvedMonitorId: "missing",
      },
      monitors: [],
    };
    const bridge = makeBridge();
    vi.mocked(bridge.getSnapshot).mockResolvedValue(noMonitorSnapshot);
    render(<SettingsApp bridge={bridge} />);

    const viewport = await screen.findByLabelText("目标屏幕预览画布");
    expect(viewport).toHaveAttribute("data-monitor-id", "unknown");
    expect(viewport).toHaveStyle({ aspectRatio: "16 / 9" });
    expect(viewport.getAttribute("style")).not.toMatch(/NaN|Infinity/);
  });

  it("按百分比调整整体尺寸并防抖保存", async () => {
    vi.useFakeTimers();
    const bridge = makeBridge();
    render(<SettingsApp bridge={bridge} />);
    await act(async () => undefined);

    fireEvent.change(screen.getByLabelText("整体尺寸"), {
      target: { value: "25" },
    });

    expect(bridge.updateVisual).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(100));
    expect(bridge.updateVisual).toHaveBeenCalledWith(
      expect.objectContaining({ sizePercent: 25 }),
    );
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

  it("切换到菱形方案后启用缺口并防抖保存", async () => {
    vi.useFakeTimers();
    const bridge = makeBridge();
    render(<SettingsApp bridge={bridge} />);
    await act(async () => undefined);

    fireEvent.click(screen.getByRole("button", { name: "细旗空心菱形" }));

    expect(screen.getByLabelText("中心缺口")).toBeEnabled();
    expect(bridge.updateVisual).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(100));
    expect(bridge.updateVisual).toHaveBeenCalledWith(
      expect.objectContaining({ preset: "fine-diamond" }),
    );
  });
});
