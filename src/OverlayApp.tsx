import { useEffect, useState } from "react";

import { Crosshair } from "./components/Crosshair";
import type { AppBridge } from "./shared/bridge";
import { DEFAULT_SETTINGS, type VisualSettings } from "./shared/settings";

export function OverlayApp({ bridge }: { bridge: AppBridge }) {
  const [visual, setVisual] = useState<VisualSettings>(DEFAULT_SETTINGS.visual);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    bridge.getSnapshot().then((snapshot) => {
      if (active) setVisual(snapshot.settings.visual);
    });
    bridge.onVisualChanged((next) => setVisual(next)).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [bridge]);

  return (
    <main className="overlay-root">
      <Crosshair settings={visual} />
    </main>
  );
}
