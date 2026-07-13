import type { CSSProperties } from "react";

import {
  isDiamondPreset,
  type CrosshairPreset,
  type VisualSettings,
} from "../shared/settings";
import "./Crosshair.css";

const PRESET_LABELS = {
  "dot-ring": "圆环与中心点",
  "classic-cross": "缺口十字",
  "soft-target": "柔和同心标记",
  "fine-diamond": "细旗空心菱形",
  "inward-diamond": "内向旗空心菱形",
  "long-diamond": "长旗空心菱形",
  "solid-diamond": "长旗实心菱形",
} satisfies Record<CrosshairPreset, string>;

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
  const diamondPreset = isDiamondPreset(settings.preset);
  const diamondClass =
    settings.preset === "solid-diamond"
      ? "crosshair__diamond--solid"
      : "crosshair__diamond--outline";

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
      {diamondPreset && (
        <>
          <span
            aria-hidden="true"
            className="crosshair__flag crosshair__flag--top"
          />
          <span
            aria-hidden="true"
            className="crosshair__flag crosshair__flag--right"
          />
          <span
            aria-hidden="true"
            className="crosshair__flag crosshair__flag--bottom"
          />
          <span
            aria-hidden="true"
            className="crosshair__flag crosshair__flag--left"
          />
          <span
            aria-hidden="true"
            className={`crosshair__diamond ${diamondClass}`}
          />
        </>
      )}
      {!diamondPreset && <span className="crosshair__dot" />}
    </div>
  );
}
