import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { DEFAULT_SETTINGS, type CrosshairPreset } from "../shared/settings";
import { Crosshair } from "./Crosshair";

const renderPreset = (preset: CrosshairPreset) =>
  render(
    <Crosshair
      settings={{
        ...DEFAULT_SETTINGS.visual,
        preset,
      }}
    />,
  );

describe("Crosshair", () => {
  it.each([
    ["dot-ring", "圆环与中心点"],
    ["classic-cross", "缺口十字"],
    ["soft-target", "柔和同心标记"],
  ] as const)("渲染 %s 预设", (preset, label) => {
    renderPreset(preset);
    expect(screen.getByLabelText(label)).toHaveAttribute("data-preset", preset);
  });

  it("把视觉参数映射为渲染变量", () => {
    render(<Crosshair settings={DEFAULT_SETTINGS.visual} />);
    const crosshair = screen.getByLabelText("缺口十字");

    expect(crosshair).toHaveStyle({
      opacity: "0.8",
      width: "32px",
      height: "32px",
    });
    expect(crosshair.style.getPropertyValue("--crosshair-primary")).toBe(
      "#4DFFB8",
    );
  });
});
