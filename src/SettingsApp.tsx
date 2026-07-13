import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";

import { Crosshair } from "./components/Crosshair";
import type { AppBridge } from "./shared/bridge";
import {
  CROSSHAIR_SIZE_PERCENT_MAX,
  CROSSHAIR_SIZE_PERCENT_MIN,
  isDiamondPreset,
  type AppSnapshot,
  type CrosshairPreset,
  type VisualSettings,
} from "./shared/settings";
import "./SettingsApp.css";

const PRESETS: Array<{
  id: CrosshairPreset;
  name: string;
  hint: string;
}> = [
  { id: "dot-ring", name: "圆环与中心点", hint: "稳定视线锚点" },
  { id: "classic-cross", name: "缺口十字", hint: "复杂画面清晰" },
  { id: "soft-target", name: "柔和同心标记", hint: "长时间低干扰" },
  { id: "fine-diamond", name: "细旗空心菱形", hint: "轻量低干扰" },
  { id: "inward-diamond", name: "内向旗空心菱形", hint: "朝心方向提示" },
  { id: "long-diamond", name: "长旗空心菱形", hint: "远距保持清晰" },
  { id: "solid-diamond", name: "长旗实心菱形", hint: "强化中心锚点" },
];

interface SettingsAppProps {
  bridge: AppBridge;
}

type PreviewViewportStyle = CSSProperties & {
  "--preview-width-by-height": string;
  "--preview-height-by-width": string;
};

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function SettingsApp({ bridge }: SettingsAppProps) {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [visual, setVisual] = useState<VisualSettings | null>(null);
  const [shortcutDraft, setShortcutDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const visualReady = useRef(false);

  useEffect(() => {
    bridge
      .getSnapshot()
      .then((next) => {
        setSnapshot(next);
        setVisual(next.settings.visual);
        setShortcutDraft(next.settings.toggleShortcut);
      })
      .catch((reason) => setError(errorMessage(reason)));
  }, [bridge]);

  useEffect(() => {
    if (!snapshot || !visual) return;
    if (!visualReady.current) {
      visualReady.current = true;
      return;
    }
    const timeout = window.setTimeout(async () => {
      setSaving(true);
      try {
        const next = await bridge.updateVisual(visual);
        setSnapshot(next);
        setError(null);
      } catch (reason) {
        setError(errorMessage(reason));
      } finally {
        setSaving(false);
      }
    }, 100);
    return () => window.clearTimeout(timeout);
  }, [bridge, visual]);

  const run = async (operation: () => Promise<AppSnapshot>) => {
    try {
      const next = await operation();
      setSnapshot(next);
      setVisual(next.settings.visual);
      setShortcutDraft(next.settings.toggleShortcut);
      setError(null);
    } catch (reason) {
      setError(errorMessage(reason));
    }
  };

  if (!snapshot || !visual) {
    return (
      <main className="loading-screen">
        <div className="brand-mark" aria-hidden="true" />
        <p>{error ?? "正在连接准星服务…"}</p>
      </main>
    );
  }

  const statusError = error ?? snapshot.status.error;
  const validPreviewMonitors = snapshot.monitors.filter(
    (monitor) =>
      Number.isFinite(monitor.width) &&
      Number.isFinite(monitor.height) &&
      monitor.width > 0 &&
      monitor.height > 0,
  );
  const previewMonitor =
    validPreviewMonitors.find(
      (monitor) => monitor.id === snapshot.status.resolvedMonitorId,
    ) ??
    validPreviewMonitors.find((monitor) => monitor.isPrimary) ??
    validPreviewMonitors[0];
  const previewWidth = previewMonitor?.width ?? 16;
  const previewHeight = previewMonitor?.height ?? 9;
  const previewRatio = previewWidth / previewHeight;
  const previewStyle: PreviewViewportStyle = {
    aspectRatio: `${previewWidth} / ${previewHeight}`,
    "--preview-width-by-height": `${previewRatio * 100}cqh`,
    "--preview-height-by-width": `${(1 / previewRatio) * 100}cqw`,
  };

  return (
    <main className="settings-shell">
      <header className="app-header">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true" />
          <div>
            <strong>Dead Center</strong>
            <span>防晕 3D 视线锚点</span>
          </div>
        </div>
        <button
          className={`visibility-button ${snapshot.status.visible ? "is-on" : ""}`}
          onClick={() => run(() => bridge.setVisibility(!snapshot.status.visible))}
          type="button"
        >
          <span aria-hidden="true" />
          {snapshot.status.visible ? "隐藏准星" : "显示准星"}
        </button>
      </header>

      {(snapshot.status.warning || statusError) && (
        <section className={`notice ${statusError ? "notice--error" : ""}`}>
          <div>
            <strong>{statusError ? "准星服务需要处理" : "兼容性提示"}</strong>
            <p>{statusError ?? snapshot.status.warning}</p>
          </div>
          {statusError && (
            <button type="button" onClick={() => run(bridge.retryOverlay)}>
              重试
            </button>
          )}
        </section>
      )}

      <div className="settings-grid">
        <section className="preview-panel" aria-label="准星预览">
          <div className="section-heading">
            <div>
              <span className="eyebrow">实时预览</span>
              <h1>保持视线稳定</h1>
            </div>
            <span className="live-badge">即时生效</span>
          </div>
          <div className="preview-stage">
            <div
              aria-label="目标屏幕预览画布"
              className="preview-viewport"
              data-monitor-id={previewMonitor?.id ?? "unknown"}
              style={previewStyle}
            >
              <div className="preview-grid" />
              <Crosshair settings={visual} />
            </div>
          </div>
          <div className="preset-list">
            {PRESETS.map((preset) => (
              <button
                aria-label={preset.name}
                aria-pressed={visual.preset === preset.id}
                className="preset-button"
                key={preset.id}
                onClick={() => setVisual({ ...visual, preset: preset.id })}
                type="button"
              >
                <span>{preset.name}</span>
                <small>{preset.hint}</small>
              </button>
            ))}
          </div>
        </section>

        <section className="controls-panel">
          <div className="control-group">
            <div className="group-title">
              <div>
                <span className="eyebrow">外观</span>
                <h2>准星参数</h2>
              </div>
              <span className="save-state">{saving ? "保存中…" : "已自动保存"}</span>
            </div>

            <RangeControl
              label="透明度"
              max={100}
              min={10}
              suffix="%"
              value={Math.round(visual.opacity * 100)}
              onChange={(value) => setVisual({ ...visual, opacity: value / 100 })}
            />
            <RangeControl
              label="整体尺寸"
              max={CROSSHAIR_SIZE_PERCENT_MAX}
              min={CROSSHAIR_SIZE_PERCENT_MIN}
              suffix="%"
              value={visual.sizePercent}
              onChange={(value) => setVisual({ ...visual, sizePercent: value })}
            />
            <RangeControl
              label="线条粗细"
              max={8}
              min={1}
              suffix="px"
              value={visual.strokePx}
              onChange={(value) => setVisual({ ...visual, strokePx: value })}
            />
            <RangeControl
              disabled={
                visual.preset !== "classic-cross" && !isDiamondPreset(visual.preset)
              }
              label="中心缺口"
              max={24}
              min={0}
              suffix="px"
              value={visual.gapPx}
              onChange={(value) => setVisual({ ...visual, gapPx: value })}
            />

            <div className="color-row">
              <ColorControl
                label="准星颜色"
                value={visual.primaryColor}
                onChange={(primaryColor) => setVisual({ ...visual, primaryColor })}
              />
              <ColorControl
                label="中心标记颜色"
                value={visual.accentColor}
                onChange={(accentColor) => setVisual({ ...visual, accentColor })}
              />
            </div>
          </div>

          <div className="control-group control-group--system">
            <div className="group-title">
              <div>
                <span className="eyebrow">系统</span>
                <h2>显示与控制</h2>
              </div>
            </div>
            <label className="field">
              <span>目标屏幕</span>
              <select
                value={snapshot.settings.targetMonitorId ?? ""}
                onChange={(event) =>
                  run(() => bridge.selectMonitor(event.target.value || null))
                }
              >
                <option value="">主显示器（默认）</option>
                {snapshot.monitors.map((monitor) => (
                  <option key={monitor.id} value={monitor.id}>
                    {monitor.name} · {monitor.width}×{monitor.height}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>显示／隐藏快捷键</span>
              <input
                value={shortcutDraft}
                onChange={(event) => setShortcutDraft(event.target.value)}
                onBlur={() => run(() => bridge.setToggleShortcut(shortcutDraft.trim()))}
                onKeyDown={(event) => {
                  if (event.key === "Enter") event.currentTarget.blur();
                }}
              />
              <small>格式示例：Alt+Shift+X</small>
            </label>
            <ToggleControl
              checked={snapshot.settings.launchAtLogin}
              label="登录系统时启动"
              onChange={(enabled) => run(() => bridge.setLaunchAtLogin(enabled))}
            />
            <ToggleControl
              checked={snapshot.settings.showOnLaunch}
              label="启动后立即显示准星"
              onChange={(enabled) => run(() => bridge.setShowOnLaunch(enabled))}
            />
          </div>
        </section>
      </div>

      <footer className="app-footer">
        <div className="runtime-status">
          <span className={snapshot.status.visible ? "status-dot" : "status-dot is-off"} />
          <strong>{snapshot.status.visible ? "准星已显示" : "准星已隐藏"}</strong>
          <span>·</span>
          <span>
            {snapshot.status.usingFallbackMonitor
              ? "目标屏幕离线，已回退主屏"
              : snapshot.monitors.find(
                  (monitor) => monitor.id === snapshot.status.resolvedMonitorId,
                )?.name ?? "主显示器"}
          </span>
        </div>
        <span>关闭窗口后仍在系统托盘运行</span>
      </footer>
    </main>
  );
}

interface RangeControlProps {
  label: string;
  min: number;
  max: number;
  value: number;
  suffix: string;
  disabled?: boolean;
  onChange(value: number): void;
}

function RangeControl({
  label,
  min,
  max,
  value,
  suffix,
  disabled,
  onChange,
}: RangeControlProps) {
  return (
    <label className={`range-control ${disabled ? "is-disabled" : ""}`}>
      <span>{label}</span>
      <input
        aria-label={label}
        disabled={disabled}
        max={max}
        min={min}
        onChange={(event) => onChange(Number(event.target.value))}
        type="range"
        value={value}
      />
      <output>
        {value}
        {suffix}
      </output>
    </label>
  );
}

function ColorControl({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange(value: string): void;
}) {
  return (
    <label className="color-control">
      <span>{label}</span>
      <div>
        <input
          aria-label={label}
          onChange={(event) => onChange(event.target.value.toUpperCase())}
          type="color"
          value={value}
        />
        <code>{value}</code>
      </div>
    </label>
  );
}

function ToggleControl({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange(value: boolean): void;
}) {
  return (
    <label className="toggle-control">
      <span>{label}</span>
      <input
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        type="checkbox"
      />
      <span aria-hidden="true" className="toggle-track" />
    </label>
  );
}
