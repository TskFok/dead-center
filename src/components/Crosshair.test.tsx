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

const DIAMOND_CASES = [
  ["fine-diamond", "crosshair__diamond--outline"],
  ["inward-diamond", "crosshair__diamond--outline"],
  ["long-diamond", "crosshair__diamond--outline"],
  ["solid-diamond", "crosshair__diamond--solid"],
] as const;

describe("Crosshair", () => {
  it.each([
    ["dot-ring", "圆环与中心点"],
    ["classic-cross", "缺口十字"],
    ["soft-target", "柔和同心标记"],
    ["fine-diamond", "细旗空心菱形"],
    ["inward-diamond", "内向旗空心菱形"],
    ["long-diamond", "长旗空心菱形"],
    ["solid-diamond", "长旗实心菱形"],
  ] as const)("渲染 %s 预设", (preset, label) => {
    renderPreset(preset);
    expect(screen.getByLabelText(label)).toHaveAttribute("data-preset", preset);
  });

  it.each(DIAMOND_CASES)(
    "%s 包含四向旗标和正确的菱形中心",
    (preset, diamondClass) => {
      const { container } = renderPreset(preset);
      const crosshair = container.querySelector(`[data-preset="${preset}"]`);

      expect(crosshair).not.toBeNull();
      expect(crosshair?.querySelectorAll(".crosshair__flag")).toHaveLength(4);
      for (const direction of ["top", "right", "bottom", "left"] as const) {
        expect(
          crosshair?.querySelector(`.crosshair__flag--${direction}`),
        ).toBeInTheDocument();
      }
      const diamond = crosshair?.querySelector(".crosshair__diamond");
      expect(diamond).not.toBeNull();
      expect(diamond).toHaveClass(diamondClass);
      expect(crosshair?.querySelector(".crosshair__dot")).toBeNull();
    },
  );

  it.each(["dot-ring", "classic-cross", "soft-target"] as const)(
    "%s 保留原有圆形中心点",
    (preset) => {
      const { container } = renderPreset(preset);
      expect(container.querySelector(".crosshair__dot")).toBeInTheDocument();
    },
  );

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
