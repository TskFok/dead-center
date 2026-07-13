import type { CSSProperties } from "react";

import type { VisualSettings } from "../shared/settings";
import "./Crosshair.css";

const PRESET_LABELS = {
  "dot-ring": "圆环与中心点",
  "classic-cross": "缺口十字",
  "soft-target": "柔和同心标记",
} as const;

type CrosshairStyle = CSSProperties & {
  "--crosshair-primary": string;
  "--crosshair-accent": string;
  "--crosshair-stroke": string;
  "--crosshair-gap": string;
};

interface CrosshairProps {
  settings: VisualSettings;
}

export function Crosshair({ settings }: CrosshairProps) {
  const style: CrosshairStyle = {
    width: `${settings.sizePx}px`,
    height: `${settings.sizePx}px`,
    opacity: settings.opacity,
    "--crosshair-primary": settings.primaryColor,
    "--crosshair-accent": settings.accentColor,
    "--crosshair-stroke": `${settings.strokePx}px`,
    "--crosshair-gap": `${settings.gapPx}px`,
  };

  return (
    <div
      aria-label={PRESET_LABELS[settings.preset]}
      className={`crosshair crosshair--${settings.preset}`}
      data-preset={settings.preset}
      style={style}
    >
      {settings.preset === "classic-cross" && (
        <>
          <span className="crosshair__line crosshair__line--left" />
          <span className="crosshair__line crosshair__line--right" />
          <span className="crosshair__line crosshair__line--top" />
          <span className="crosshair__line crosshair__line--bottom" />
        </>
      )}
      {settings.preset === "dot-ring" && (
        <span className="crosshair__ring crosshair__ring--single" />
      )}
      {settings.preset === "soft-target" && (
        <>
          <span className="crosshair__ring crosshair__ring--inner" />
          <span className="crosshair__ring crosshair__ring--outer" />
        </>
      )}
      <span className="crosshair__dot" />
    </div>
  );
}
