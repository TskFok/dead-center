export type CrosshairPreset =
  | "dot-ring"
  | "classic-cross"
  | "soft-target"
  | "fine-diamond"
  | "inward-diamond"
  | "long-diamond"
  | "solid-diamond";

export const DIAMOND_PRESETS = [
  "fine-diamond",
  "inward-diamond",
  "long-diamond",
  "solid-diamond",
] as const satisfies readonly CrosshairPreset[];

export type DiamondCrosshairPreset = (typeof DIAMOND_PRESETS)[number];

export function isDiamondPreset(
  preset: CrosshairPreset,
): preset is DiamondCrosshairPreset {
  return DIAMOND_PRESETS.some((candidate) => candidate === preset);
}

export interface VisualSettings {
  preset: CrosshairPreset;
  primaryColor: string;
  accentColor: string;
  opacity: number;
  sizePx: number;
  strokePx: number;
  gapPx: number;
}

export interface AppSettings {
  version: 1;
  visual: VisualSettings;
  targetMonitorId: string | null;
  toggleShortcut: string;
  launchAtLogin: boolean;
  showOnLaunch: boolean;
}

export type SessionType = "windows" | "macos" | "x11" | "wayland";

export interface RuntimeStatus {
  visible: boolean;
  resolvedMonitorId: string;
  usingFallbackMonitor: boolean;
  sessionType: SessionType;
  warning?: string;
  error?: string;
}

export interface MonitorInfo {
  id: string;
  name: string;
  isPrimary: boolean;
  width: number;
  height: number;
  scaleFactor: number;
}

export interface AppSnapshot {
  settings: AppSettings;
  status: RuntimeStatus;
  monitors: MonitorInfo[];
}

export const DEFAULT_SETTINGS: AppSettings = {
  version: 1,
  visual: {
    preset: "classic-cross",
    primaryColor: "#4DFFB8",
    accentColor: "#F4FF4D",
    opacity: 0.8,
    sizePx: 32,
    strokePx: 3,
    gapPx: 8,
  },
  targetMonitorId: null,
  toggleShortcut: "Alt+Shift+X",
  launchAtLogin: false,
  showOnLaunch: true,
};

const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

export function isHexColor(value: string): boolean {
  return /^#[0-9a-f]{6}$/i.test(value);
}

export function normalizeVisualSettings(
  value: VisualSettings,
): VisualSettings {
  return {
    ...value,
    primaryColor: isHexColor(value.primaryColor)
      ? value.primaryColor.toUpperCase()
      : DEFAULT_SETTINGS.visual.primaryColor,
    accentColor: isHexColor(value.accentColor)
      ? value.accentColor.toUpperCase()
      : DEFAULT_SETTINGS.visual.accentColor,
    opacity: clamp(value.opacity, 0.1, 1),
    sizePx: clamp(value.sizePx, 12, 96),
    strokePx: clamp(value.strokePx, 1, 8),
    gapPx: clamp(value.gapPx, 0, 24),
  };
}
