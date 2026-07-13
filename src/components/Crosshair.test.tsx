// @ts-expect-error -- Vitest 在 Node 中运行，但应用 tsconfig 故意不包含 Node 类型
import { readFileSync } from "node:fs";

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { DEFAULT_SETTINGS, type CrosshairPreset } from "../shared/settings";
import { Crosshair } from "./Crosshair";

const crosshairCss = readFileSync("src/components/Crosshair.css", "utf8");

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

type LongFlagPreset = "long-diamond" | "solid-diamond";

interface FlagGeometry {
  insetPercent: number;
  clearanceCapPercent: number;
  minimumClearancePercent: number;
}

interface DiamondSize {
  minimumPx: number;
  preferredPercent: number;
  maximumPx: number;
}

const readPresetBlock = (preset: LongFlagPreset) => {
  const matchingBlocks = [
    ...crosshairCss.matchAll(
      new RegExp(`\\.crosshair--${preset}\\s*\\{([\\s\\S]*?)\\}`, "g"),
    ),
  ];
  const block = matchingBlocks[matchingBlocks.length - 1]?.[1];
  if (!block) {
    throw new Error(`未找到 ${preset} 的 CSS 规则`);
  }
  return block;
};

const readFlagGeometry = (preset: LongFlagPreset): FlagGeometry => {
  const block = readPresetBlock(preset);

  const readPercent = (variable: string) => {
    const value = block.match(
      new RegExp(`${variable}:\\s*([\\d.]+)%`),
    )?.[1];
    if (value === undefined) {
      throw new Error(`未找到 ${preset} 的 ${variable}`);
    }
    return Number(value);
  };

  return {
    insetPercent: readPercent("--flag-inset"),
    clearanceCapPercent: readPercent("--flag-clearance-cap"),
    minimumClearancePercent: readPercent("--diamond-clearance"),
  };
};

const readDiamondSize = (preset: LongFlagPreset): DiamondSize => {
  const size = readPresetBlock(preset).match(
    /--diamond-size:\s*clamp\(\s*([\d.]+)px,\s*([\d.]+)%,\s*([\d.]+)px\s*\)/,
  );
  if (!size) {
    throw new Error(`未找到 ${preset} 的 --diamond-size`);
  }

  return {
    minimumPx: Number(size[1]),
    preferredPercent: Number(size[2]),
    maximumPx: Number(size[3]),
  };
};

const flagClearancePx = (
  geometry: FlagGeometry,
  sizePx: number,
  gapPx: number,
) =>
  Math.min(
    (sizePx * geometry.clearanceCapPercent) / 100,
    Math.max(
      gapPx / 2,
      (sizePx * geometry.minimumClearancePercent) / 100,
    ),
  );

const flagArmLengthPx = (
  geometry: FlagGeometry,
  sizePx: number,
  gapPx: number,
) =>
  sizePx / 2 -
  flagClearancePx(geometry, sizePx, gapPx) -
  (sizePx * geometry.insetPercent) / 100;

const rotatedDiamondHalfExtentPx = (
  diamondSize: DiamondSize,
  crosshairSizePx: number,
) => {
  const sidePx = Math.min(
    diamondSize.maximumPx,
    Math.max(
      diamondSize.minimumPx,
      (crosshairSizePx * diamondSize.preferredPercent) / 100,
    ),
  );
  return sidePx * Math.SQRT1_2;
};

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

  it("solid-diamond 在常规与封顶间距下严格保留更长旗臂", () => {
    const long = readFlagGeometry("long-diamond");
    const solid = readFlagGeometry("solid-diamond");
    const regular = { sizePx: 32, gapPx: 8 };
    const capped = { sizePx: 12, gapPx: 24 };

    expect({
      minimumClearanceIsSmaller:
        solid.minimumClearancePercent < long.minimumClearancePercent,
      clearanceCapIsSmaller:
        solid.clearanceCapPercent < long.clearanceCapPercent,
      regularArmIsLonger:
        flagArmLengthPx(solid, regular.sizePx, regular.gapPx) >
        flagArmLengthPx(long, regular.sizePx, regular.gapPx),
      cappedArmIsLonger:
        flagArmLengthPx(solid, capped.sizePx, capped.gapPx) >
        flagArmLengthPx(long, capped.sizePx, capped.gapPx),
    }).toEqual({
      minimumClearanceIsSmaller: true,
      clearanceCapIsSmaller: true,
      regularArmIsLonger: true,
      cappedArmIsLonger: true,
    });
  });

  it("solid-diamond 的中心不碰旗臂且极端尺寸下旗臂可见", () => {
    const solid = readFlagGeometry("solid-diamond");
    const diamondSize = readDiamondSize("solid-diamond");
    const scenarios = [
      { sizePx: 32, gapPx: 8 },
      { sizePx: 12, gapPx: 24 },
    ] as const;

    for (const { sizePx, gapPx } of scenarios) {
      expect(flagClearancePx(solid, sizePx, gapPx)).toBeGreaterThan(
        rotatedDiamondHalfExtentPx(diamondSize, sizePx),
      );
    }
    expect(flagArmLengthPx(solid, 12, 24)).toBeGreaterThan(0);
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
