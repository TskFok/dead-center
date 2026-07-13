import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppSnapshot,
  RuntimeStatus,
  VisualSettings,
} from "./settings";

export interface AppBridge {
  getSnapshot(): Promise<AppSnapshot>;
  updateVisual(visual: VisualSettings): Promise<AppSnapshot>;
  selectMonitor(monitorId: string | null): Promise<AppSnapshot>;
  setToggleShortcut(shortcut: string): Promise<AppSnapshot>;
  setLaunchAtLogin(enabled: boolean): Promise<AppSnapshot>;
  setShowOnLaunch(enabled: boolean): Promise<AppSnapshot>;
  setVisibility(visible: boolean): Promise<AppSnapshot>;
  retryOverlay(): Promise<AppSnapshot>;
  onVisualChanged(handler: (visual: VisualSettings) => void): Promise<UnlistenFn>;
  onRuntimeChanged(handler: (status: RuntimeStatus) => void): Promise<UnlistenFn>;
}

export const tauriBridge: AppBridge = {
  getSnapshot: () => invoke("get_snapshot"),
  updateVisual: (visual) => invoke("update_visual", { visual }),
  selectMonitor: (monitorId) => invoke("select_monitor", { monitorId }),
  setToggleShortcut: (shortcut) => invoke("set_toggle_shortcut", { shortcut }),
  setLaunchAtLogin: (enabled) => invoke("set_launch_at_login", { enabled }),
  setShowOnLaunch: (enabled) => invoke("set_show_on_launch", { enabled }),
  setVisibility: (visible) => invoke("set_visibility", { visible }),
  retryOverlay: () => invoke("retry_overlay"),
  onVisualChanged: (handler) =>
    listen<VisualSettings>("crosshair://visual-changed", (event) =>
      handler(event.payload),
    ),
  onRuntimeChanged: (handler) =>
    listen<RuntimeStatus>("crosshair://runtime-changed", (event) =>
      handler(event.payload),
    ),
};
