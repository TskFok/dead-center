# 四种菱形旗标准星实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标：** 在保留三种现有准星和默认设置的前提下，新增四种可调色、可缩放、可持久化的菱形旗标准星。

**架构：** 前端与 Rust 通过四个完全一致的 kebab-case 方案标识扩展现有 `VisualSettings` 协议；`Crosshair` 继续作为预览与覆盖层共用的唯一渲染组件。新造型由四个旗形臂和一个菱形中心组成，使用共享类型守卫、React 结构与 CSS 变量实现，不增加配置字段、版本或运行时依赖。

**技术栈：** TypeScript 5.8、React 19、CSS、Vitest 4、Testing Library、Rust 2021、Serde、Tauri 2。

## 全局约束

- 在当前 `main` 分支修改，不新建分支。
- 所有 Git 提交信息使用简体中文。
- 保留 `dot-ring`、`classic-cross`、`soft-target` 的外观、标识和行为；默认方案仍为 `classic-cross`。
- 新方案标识固定为 `fine-diamond`、`inward-diamond`、`long-diamond`、`solid-diamond`。
- 主色控制四向旗形臂，中心标记色控制菱形；不固定参考图中的红、绿、黄、青。
- 继续使用 `AppSettings.version = 1` 以及现有的颜色、透明度、尺寸、线宽和中心间距字段。
- 不增加第三方依赖，不把 `/Users/ushopal/Downloads/center.jpg` 打包进应用。
- 不在循环遍历中查询 SQL；本功能不新增或调用 SQL。
- 每次生产代码变更前必须先运行对应失败测试并确认失败原因正确。

---

### Task 1：同步前端与 Rust 的方案协议

**文件：**

- 修改：`src/shared/settings.ts:1-14`
- 测试：`src/shared/settings.test.ts:3-65`
- 修改：`src-tauri/src/config.rs:3-9`
- 测试：`src-tauri/src/config.rs:83-118`

**接口：**

- 产出：`DIAMOND_PRESETS: readonly ["fine-diamond", "inward-diamond", "long-diamond", "solid-diamond"]`
- 产出：`DiamondCrosshairPreset`
- 产出：`isDiamondPreset(preset: CrosshairPreset): preset is DiamondCrosshairPreset`
- 产出：Rust 枚举成员 `FineDiamond`、`InwardDiamond`、`LongDiamond`、`SolidDiamond`
- 后续依赖：任务 2 的 `Crosshair` 和任务 3 的 `SettingsApp` 使用 `isDiamondPreset`

- [ ] **步骤 1：先为前端方案集合和类型守卫写失败测试**

把 `src/shared/settings.test.ts` 的导入改为：

```ts
import {
  DEFAULT_SETTINGS,
  isDiamondPreset,
  isHexColor,
  normalizeVisualSettings,
} from "./settings";
```

在 `DEFAULT_SETTINGS` 测试块之后加入：

```ts
describe("isDiamondPreset", () => {
  it.each([
    "fine-diamond",
    "inward-diamond",
    "long-diamond",
    "solid-diamond",
  ] as const)("识别菱形旗标准星 %s", (preset) => {
    expect(isDiamondPreset(preset)).toBe(true);
  });

  it.each(["dot-ring", "classic-cross", "soft-target"] as const)(
    "不把旧方案 %s 识别为菱形旗标",
    (preset) => {
      expect(isDiamondPreset(preset)).toBe(false);
    },
  );
});
```

- [ ] **步骤 2：运行前端协议测试并确认 RED**

运行：

```bash
pnpm test -- src/shared/settings.test.ts
```

预期：失败，错误指出 `./settings` 尚未导出 `isDiamondPreset`；失败原因是新协议尚未实现，而不是测试语法或环境错误。

- [ ] **步骤 3：写入最小前端协议实现**

用以下代码替换 `src/shared/settings.ts` 顶部现有的 `CrosshairPreset` 定义，并紧接着加入共享集合和类型守卫：

```ts
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
```

不要修改 `VisualSettings`、`DEFAULT_SETTINGS`、归一化边界或配置版本。

- [ ] **步骤 4：运行前端协议测试并确认 GREEN**

运行：

```bash
pnpm test -- src/shared/settings.test.ts
```

预期：该文件全部通过；默认方案仍为 `classic-cross`，四个新标识返回 `true`，三个旧标识返回 `false`。

- [ ] **步骤 5：先为 Rust kebab-case 往返协议写失败测试**

在 `src-tauri/src/config.rs` 的 `tests` 模块中、`validation_clamps_visual_ranges` 之前加入：

```rust
    #[test]
    fn diamond_presets_use_kebab_case() {
        let cases = [
            (CrosshairPreset::FineDiamond, "\"fine-diamond\""),
            (CrosshairPreset::InwardDiamond, "\"inward-diamond\""),
            (CrosshairPreset::LongDiamond, "\"long-diamond\""),
            (CrosshairPreset::SolidDiamond, "\"solid-diamond\""),
        ];

        for (preset, serialized) in cases {
            assert_eq!(serde_json::to_string(&preset).unwrap(), serialized);
            assert_eq!(
                serde_json::from_str::<CrosshairPreset>(serialized).unwrap(),
                preset
            );
        }
    }
```

该循环只执行内存中的 Serde 往返断言，不包含 SQL 查询。

- [ ] **步骤 6：运行 Rust 协议测试并确认 RED**

运行：

```bash
cargo test --manifest-path src-tauri/Cargo.toml diamond_presets_use_kebab_case
```

预期：编译失败，错误分别指出 `CrosshairPreset` 尚无 `FineDiamond`、`InwardDiamond`、`LongDiamond`、`SolidDiamond` 成员。

- [ ] **步骤 7：写入最小 Rust 枚举实现**

把 `CrosshairPreset` 枚举改为：

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrosshairPreset {
    DotRing,
    ClassicCross,
    SoftTarget,
    FineDiamond,
    InwardDiamond,
    LongDiamond,
    SolidDiamond,
}
```

不要改变 `AppSettings::default()`，也不要提升版本号。

- [ ] **步骤 8：运行 Rust 协议测试并确认 GREEN**

运行：

```bash
cargo test --manifest-path src-tauri/Cargo.toml diamond_presets_use_kebab_case
```

预期：该测试通过，四个枚举值均能序列化为精确的 kebab-case 字符串并反序列化回来。

- [ ] **步骤 9：复跑本任务全部测试**

运行：

```bash
pnpm test -- src/shared/settings.test.ts
cargo test --manifest-path src-tauri/Cargo.toml config::tests
```

预期：两条命令退出码均为 0；前端默认值测试和 Rust 默认值、归一化测试没有回归。

- [ ] **步骤 10：提交协议改动**

```bash
git add src/shared/settings.ts src/shared/settings.test.ts src-tauri/src/config.rs
git commit -m "功能：同步新增菱形准星枚举"
```

---

### Task 2：渲染四种菱形旗标准星

**文件：**

- 测试：`src/components/Crosshair.test.tsx:1-40`
- 修改：`src/components/Crosshair.tsx:1-61`
- 修改：`src/components/Crosshair.css:1-81`

**接口：**

- 消费：任务 1 的 `CrosshairPreset` 和 `isDiamondPreset`
- 产出：每个新方案渲染四个 `.crosshair__flag` 和一个 `.crosshair__diamond`
- 产出：空心中心类 `.crosshair__diamond--outline` 与实心中心类 `.crosshair__diamond--solid`
- 产出：方向类 `--top`、`--right`、`--bottom`、`--left`
- 后续依赖：任务 3 的实时预览和 Tauri 覆盖层继续直接使用 `Crosshair`

- [ ] **步骤 1：先写七种标签、四臂、菱形中心和旧方案回归测试**

用以下完整内容替换 `src/components/Crosshair.test.tsx`：

```tsx
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
```

- [ ] **步骤 2：运行组件测试并确认 RED**

运行：

```bash
pnpm test -- src/components/Crosshair.test.tsx
```

预期：新标签查询和旗标／菱形结构断言失败；三个旧方案中心点和现有视觉变量测试仍通过。

- [ ] **步骤 3：写入最小 React 结构实现**

用以下完整内容替换 `src/components/Crosshair.tsx`：

```tsx
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
```

- [ ] **步骤 4：追加四个变体的 CSS 矢量规则**

把以下完整规则追加到 `src/components/Crosshair.css` 末尾：

```css
.crosshair--fine-diamond,
.crosshair--inward-diamond,
.crosshair--long-diamond,
.crosshair--solid-diamond {
  --flag-clearance: min(
    var(--flag-clearance-cap),
    max(calc(var(--crosshair-gap) / 2), var(--diamond-clearance))
  );
  --diamond-stroke: min(
    var(--crosshair-stroke),
    var(--diamond-stroke-cap)
  );
}

.crosshair--fine-diamond {
  --flag-inset: 22%;
  --flag-thickness: min(max(var(--crosshair-stroke), 1px), 12%);
  --flag-clearance-cap: 24%;
  --diamond-clearance: 12%;
  --diamond-size: clamp(4px, 18%, 14px);
  --diamond-stroke-cap: 1px;
}

.crosshair--inward-diamond {
  --flag-inset: 14%;
  --flag-thickness: min(
    max(calc(var(--crosshair-stroke) + 1px), 2px),
    16%
  );
  --flag-clearance-cap: 32%;
  --diamond-clearance: 13%;
  --diamond-size: clamp(5px, 20%, 16px);
  --diamond-stroke-cap: 1.5px;
}

.crosshair--long-diamond {
  --flag-inset: 0%;
  --flag-thickness: min(
    max(calc(var(--crosshair-stroke) + 2px), 3px),
    20%
  );
  --flag-clearance-cap: 40%;
  --diamond-clearance: 14%;
  --diamond-size: clamp(6px, 22%, 18px);
  --diamond-stroke-cap: 2px;
}

.crosshair--solid-diamond {
  --flag-inset: 0%;
  --flag-thickness: min(
    max(calc(var(--crosshair-stroke) + 3px), 4px),
    24%
  );
  --flag-clearance-cap: 40%;
  --diamond-clearance: 16%;
  --diamond-size: clamp(6px, 24%, 20px);
  --diamond-stroke-cap: 0px;
}

.crosshair__flag {
  position: absolute;
  display: block;
  background: var(--crosshair-primary);
  box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.75);
}

.crosshair__flag--left,
.crosshair__flag--right {
  top: calc(50% - var(--flag-thickness) / 2);
  width: max(
    0px,
    calc(50% - var(--flag-clearance) - var(--flag-inset))
  );
  height: var(--flag-thickness);
}

.crosshair__flag--left {
  left: var(--flag-inset);
}

.crosshair__flag--right {
  right: var(--flag-inset);
}

.crosshair__flag--top,
.crosshair__flag--bottom {
  left: calc(50% - var(--flag-thickness) / 2);
  width: var(--flag-thickness);
  height: max(
    0px,
    calc(50% - var(--flag-clearance) - var(--flag-inset))
  );
}

.crosshair__flag--top {
  top: var(--flag-inset);
}

.crosshair__flag--bottom {
  bottom: var(--flag-inset);
}

:is(.crosshair--fine-diamond, .crosshair--long-diamond)
  .crosshair__flag--left {
  clip-path: polygon(0 50%, 18% 0, 100% 0, 100% 100%, 18% 100%);
}

:is(.crosshair--fine-diamond, .crosshair--long-diamond)
  .crosshair__flag--right {
  clip-path: polygon(0 0, 82% 0, 100% 50%, 82% 100%, 0 100%);
}

:is(.crosshair--fine-diamond, .crosshair--long-diamond)
  .crosshair__flag--top {
  clip-path: polygon(50% 0, 100% 18%, 100% 100%, 0 100%, 0 18%);
}

:is(.crosshair--fine-diamond, .crosshair--long-diamond)
  .crosshair__flag--bottom {
  clip-path: polygon(0 0, 100% 0, 100% 82%, 50% 100%, 0 82%);
}

:is(.crosshair--inward-diamond, .crosshair--solid-diamond)
  .crosshair__flag--left {
  clip-path: polygon(0 0, 82% 0, 100% 50%, 82% 100%, 0 100%);
}

:is(.crosshair--inward-diamond, .crosshair--solid-diamond)
  .crosshair__flag--right {
  clip-path: polygon(18% 0, 100% 0, 100% 100%, 18% 100%, 0 50%);
}

:is(.crosshair--inward-diamond, .crosshair--solid-diamond)
  .crosshair__flag--top {
  clip-path: polygon(0 0, 100% 0, 100% 82%, 50% 100%, 0 82%);
}

:is(.crosshair--inward-diamond, .crosshair--solid-diamond)
  .crosshair__flag--bottom {
  clip-path: polygon(50% 0, 100% 18%, 100% 100%, 0 100%, 0 18%);
}

.crosshair__diamond {
  position: absolute;
  left: 50%;
  top: 50%;
  width: var(--diamond-size);
  height: var(--diamond-size);
  box-sizing: border-box;
  transform: translate(-50%, -50%) rotate(45deg);
}

.crosshair__diamond--outline {
  border: var(--diamond-stroke) solid var(--crosshair-accent);
  background: transparent;
  box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.72);
}

.crosshair__diamond--solid {
  background: var(--crosshair-accent);
  box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.72);
}
```

这些比例分别表达“最短最细、略长朝心、长臂空心、长臂实心”；`--flag-clearance-cap` 保证在 12px 尺寸与 24px 间距组合下仍保留可见旗标。

- [ ] **步骤 5：运行组件测试并确认 GREEN**

运行：

```bash
pnpm test -- src/components/Crosshair.test.tsx
```

预期：七种标签、四个新方案结构、空心／实心中心、旧方案圆点以及视觉变量测试全部通过。

- [ ] **步骤 6：运行前端构建检查类型与 CSS**

运行：

```bash
pnpm build
```

预期：TypeScript 编译和 Vite 构建退出码为 0；`PRESET_LABELS` 的穷举检查没有缺项，CSS 被正常打包。

- [ ] **步骤 7：提交渲染改动**

```bash
git add src/components/Crosshair.tsx src/components/Crosshair.css src/components/Crosshair.test.tsx
git commit -m "功能：渲染四种菱形旗标准星"
```

---

### Task 3：在设置页加入四种可选方案

**文件：**

- 测试：`src/SettingsApp.test.tsx:41-74`
- 修改：`src/SettingsApp.tsx:5-20`
- 修改：`src/SettingsApp.tsx:191-212`

**接口：**

- 消费：任务 1 的 `isDiamondPreset`
- 消费：任务 2 的七种 `Crosshair` 标签
- 产出：七项 `PRESETS` 选项、菱形方案可用的“中心缺口”控件、通用“中心标记颜色”文案
- 保持：现有 100 毫秒防抖保存、`aria-pressed` 选中语义和单列布局

- [ ] **步骤 1：先写设置页列表、文案与间距行为的失败测试**

在 `makeBridge` 后加入：

```ts
const PRESET_NAMES = [
  "圆环与中心点",
  "缺口十字",
  "柔和同心标记",
  "细旗空心菱形",
  "内向旗空心菱形",
  "长旗空心菱形",
  "长旗实心菱形",
] as const;
```

用以下完整内容替换现有 `describe("SettingsApp", ...)` 测试块：

```tsx
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
    expect(screen.getByText("准星已显示")).toBeVisible();
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
```

上述 `for` 只遍历内存中的方案名称并查询 DOM，不包含 SQL 查询。

- [ ] **步骤 2：运行设置页测试并确认 RED**

运行：

```bash
pnpm test -- src/SettingsApp.test.tsx
```

预期：失败原因包括四个新按钮和“中心标记颜色”尚不存在；菱形方案测试无法找到“细旗空心菱形”。原圆环禁用与防抖行为仍可通过。

- [ ] **步骤 3：导入类型守卫并追加四个方案**

用以下代码替换 `SettingsApp.tsx` 当前从 `./shared/settings` 的类型导入：

```ts
import {
  isDiamondPreset,
  type AppSnapshot,
  type CrosshairPreset,
  type VisualSettings,
} from "./shared/settings";
```

把 `PRESETS` 改为：

```ts
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
```

- [ ] **步骤 4：扩大中心间距适用范围并修改颜色文案**

把中心缺口控件改为：

```tsx
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
```

把第二个颜色控件改为：

```tsx
<ColorControl
  label="中心标记颜色"
  value={visual.accentColor}
  onChange={(accentColor) => setVisual({ ...visual, accentColor })}
/>
```

不要修改防抖保存、按钮结构、单列布局或其他控件。

- [ ] **步骤 5：运行设置页测试并确认 GREEN**

运行：

```bash
pnpm test -- src/SettingsApp.test.tsx
```

预期：七种按钮、通用颜色文案、圆环禁用缺口、菱形启用缺口以及两种防抖保存行为全部通过。

- [ ] **步骤 6：复跑全部前端测试**

运行：

```bash
pnpm test
```

预期：全部前端测试通过，输出不含未处理错误或警告。

- [ ] **步骤 7：提交设置页改动**

```bash
git add src/SettingsApp.tsx src/SettingsApp.test.tsx
git commit -m "功能：在设置页加入四种菱形准星"
```

---

### Task 4：更新说明并执行全量验证

**文件：**

- 修改：`README.md:7-8`
- 验证：全部前端与 Rust 代码

**接口：**

- 消费：前三个任务产出的七种可选方案与共享渲染组件
- 产出：与产品行为一致的功能说明和可复核的测试、构建、视觉检查结果

- [ ] **步骤 1：更新 README 的方案与参数说明**

用以下两行替换 `README.md` 当前功能列表的前两项：

```markdown
- 圆环与中心点、缺口十字、柔和同心标记，以及细旗空心菱形、内向旗空心菱形、长旗空心菱形、长旗实心菱形，共七种准星；默认使用缺口十字。
- 调节准星颜色、中心标记颜色、透明度、尺寸、线宽和中心缺口。
```

- [ ] **步骤 2：运行完整前端测试**

运行：

```bash
pnpm test
```

预期：退出码为 0，所有测试通过，无错误或警告。

- [ ] **步骤 3：运行生产构建**

运行：

```bash
pnpm build
```

预期：TypeScript 和 Vite 均成功，输出包含构建完成信息，退出码为 0。

- [ ] **步骤 4：运行完整 Rust 测试**

运行：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

预期：退出码为 0，全部 Rust 单元测试通过，包括四个新枚举的 Serde 往返测试。

- [ ] **步骤 5：启动 Tauri 开发应用并执行视觉检查**

运行：

```bash
pnpm tauri dev
```

逐项确认：

1. 设置页按现有单列布局显示七个按钮，默认“缺口十字”仍被选中。
2. 四个新方案分别呈现短细外向旗、略长内向旗、长粗外向旗、长粗内向旗；前三个中心空心，最后一个中心实心。
3. 修改“准星颜色”只改变四条臂，修改“中心标记颜色”只改变菱形。
4. 分别检查整体尺寸 12px 与 96px、线宽 1px 与 8px、中心缺口 0px 与 24px；四向臂和中心菱形保持可辨认且不超出根容器。
5. 实际屏幕中心覆盖层与设置页实时预览显示相同方案和参数。
6. 三个旧方案外观不变，两个圆环类方案的中心缺口控件仍禁用。

预期：六项全部满足；结束检查后正常退出开发进程。若发现视觉缺陷，先在相应测试文件添加能复现该行为的失败测试，再做最小修复并复跑本任务的全部验证命令。

- [ ] **步骤 6：检查格式和变更范围**

运行：

```bash
git diff --check
git status --short
```

预期：`git diff --check` 无输出；状态中没有意外修改，功能文件均已由前三个任务提交，本任务只剩 `README.md` 待提交以及会话过程记录文件。

- [ ] **步骤 7：提交文档改动**

```bash
git add README.md
git commit -m "文档：更新七种准星方案说明"
```

- [ ] **步骤 8：记录最终提交链并复核**

运行：

```bash
git log -5 --oneline --decorate
git status --short --branch
```

预期：最近提交依次包含设计、协议、渲染、设置页和 README 的中文提交；当前分支仍为 `main`，没有未提交的产品代码改动。
