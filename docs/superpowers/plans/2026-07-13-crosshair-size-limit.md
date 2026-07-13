# 准星大小上限扩展实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将准星整体尺寸上限从 96px 提高到 192px，并用 256px 覆盖层窗口保证完整显示。

**Architecture:** 前端导出统一的尺寸边界常量，设置页滑杆与 TypeScript 归一化共用该常量；Rust 配置层使用同样的 192px 上限。Tauri 覆盖层保持固定尺寸，但扩大到 256px，并继续通过现有显示器缩放与居中函数定位。

**Tech Stack:** React 19、TypeScript、Vitest、Tauri 2、Rust

## Global Constraints

- 准星尺寸范围固定为 12–192px，默认值仍为 32px。
- 覆盖层窗口固定为 256×256px，不实现动态窗口缩放。
- 不调整准星图形比例、线宽上限或中心缺口上限。
- 默认在当前分支修改，不新建分支。
- 提交消息使用简体中文。
- 禁止在循环遍历中查询 SQL；本变更不涉及 SQL。

---

### Task 1: 统一前端尺寸上限

**Files:**
- Modify: `src/shared/settings.ts:44-86`
- Test: `src/shared/settings.test.ts:37-56`
- Modify: `src/SettingsApp.tsx:178-187`
- Test: `src/SettingsApp.test.tsx:50-70`

**Interfaces:**
- Produces: `CROSSHAIR_SIZE_MIN: 12` 与 `CROSSHAIR_SIZE_MAX: 192`
- Consumes: `RangeControl` 的现有 `min`、`max` 数值属性

- [ ] **Step 1: 写入失败测试**

在 `src/shared/settings.test.ts` 的导入中加入两个尺寸常量，并把范围测试写为：

```ts
import {
  CROSSHAIR_SIZE_MAX,
  CROSSHAIR_SIZE_MIN,
  DEFAULT_SETTINGS,
  isDiamondPreset,
  isHexColor,
  normalizeVisualSettings,
} from "./settings";

it("把超出范围的数值限制到产品边界", () => {
  expect(CROSSHAIR_SIZE_MIN).toBe(12);
  expect(CROSSHAIR_SIZE_MAX).toBe(192);
  expect(
    normalizeVisualSettings({
      ...DEFAULT_SETTINGS.visual,
      opacity: 0,
      sizePx: 240,
      strokePx: 0,
      gapPx: 30,
    }),
  ).toMatchObject({
    opacity: 0.1,
    sizePx: 192,
    strokePx: 1,
    gapPx: 24,
  });
});
```

在 `src/SettingsApp.test.tsx` 的首个测试末尾加入：

```ts
expect(screen.getByLabelText("整体尺寸")).toHaveAttribute("min", "12");
expect(screen.getByLabelText("整体尺寸")).toHaveAttribute("max", "192");
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `pnpm test -- src/shared/settings.test.ts src/SettingsApp.test.tsx`

Expected: FAIL；`settings.ts` 尚未导出尺寸常量，且滑杆最大值仍为 `96`。

- [ ] **Step 3: 最小化实现前端边界**

在 `src/shared/settings.ts` 的默认设置前加入：

```ts
export const CROSSHAIR_SIZE_MIN = 12;
export const CROSSHAIR_SIZE_MAX = 192;
```

把 `normalizeVisualSettings` 中的尺寸归一化改为：

```ts
sizePx: clamp(value.sizePx, CROSSHAIR_SIZE_MIN, CROSSHAIR_SIZE_MAX),
```

在 `src/SettingsApp.tsx` 从 `./shared/settings` 的现有导入中加入 `CROSSHAIR_SIZE_MIN` 和 `CROSSHAIR_SIZE_MAX`，并把整体尺寸控件改为：

```tsx
<RangeControl
  label="整体尺寸"
  max={CROSSHAIR_SIZE_MAX}
  min={CROSSHAIR_SIZE_MIN}
  suffix="px"
  value={visual.sizePx}
  onChange={(value) => setVisual({ ...visual, sizePx: value })}
/>
```

- [ ] **Step 4: 运行测试并确认 GREEN**

Run: `pnpm test -- src/shared/settings.test.ts src/SettingsApp.test.tsx`

Expected: 两个测试文件全部 PASS。

- [ ] **Step 5: 提交前端变更**

```bash
git add src/shared/settings.ts src/shared/settings.test.ts src/SettingsApp.tsx src/SettingsApp.test.tsx
git commit -m "功能：提高前端准星大小上限"
```

### Task 2: 同步 Rust 配置归一化上限

**Files:**
- Modify: `src-tauri/src/config.rs:16-33`
- Test: `src-tauri/src/config.rs:122-140`

**Interfaces:**
- Produces: Rust 后端接受 12–192px 的 `VisualSettings::size_px`
- Consumes: `VisualSettings::normalize(&mut self)` 现有配置保存链路

- [ ] **Step 1: 写入失败测试**

把 `validation_clamps_visual_ranges` 测试中的输入和尺寸断言改为：

```rust
let mut visual = VisualSettings {
    opacity: 0.0,
    size_px: 240.0,
    stroke_px: 0.0,
    gap_px: 30.0,
    ..AppSettings::default().visual
};

visual.normalize();

assert_eq!(visual.opacity, 0.1);
assert_eq!(visual.size_px, 192.0);
assert_eq!(visual.stroke_px, 1.0);
assert_eq!(visual.gap_px, 24.0);
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests::validation_clamps_visual_ranges -- --exact`

Expected: FAIL，实际 `size_px` 为 `96.0`，预期为 `192.0`。

- [ ] **Step 3: 最小化实现 Rust 边界**

把 `VisualSettings::normalize` 中的尺寸边界改为：

```rust
self.size_px = self.size_px.clamp(12.0, 192.0);
```

- [ ] **Step 4: 运行测试并确认 GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests::validation_clamps_visual_ranges -- --exact`

Expected: PASS。

- [ ] **Step 5: 提交 Rust 配置变更**

```bash
git add src-tauri/src/config.rs
git commit -m "功能：同步后端准星大小上限"
```

### Task 3: 放大覆盖层并验证居中定位

**Files:**
- Modify: `src-tauri/src/overlay.rs:12-13`
- Test: `src-tauri/src/monitor.rs:148-158`

**Interfaces:**
- Produces: `pub(crate) const OVERLAY_SIZE: f64 = 256.0`
- Consumes: `centered_overlay_position(&MonitorGeometry, f64) -> PhysicalPoint`

- [ ] **Step 1: 写入失败测试**

在 `src-tauri/src/monitor.rs` 测试模块中导入覆盖层尺寸，并修改高 DPI 居中测试：

```rust
use crate::overlay::OVERLAY_SIZE;

#[test]
fn centers_logical_overlay_on_negative_coordinate_hidpi_monitor() {
    let monitor = monitor("left", -2560, 2560, 2.0, false);

    let point = centered_overlay_position(&monitor, OVERLAY_SIZE);

    assert_eq!(point, PhysicalPoint { x: -1536, y: 464 });
}
```

- [ ] **Step 2: 运行测试并确认 RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor::tests::centers_logical_overlay_on_negative_coordinate_hidpi_monitor -- --exact`

Expected: 编译失败，因为 `OVERLAY_SIZE` 仍是私有常量；公开后若仍为 128px，坐标断言会失败。

- [ ] **Step 3: 最小化实现覆盖层尺寸**

在 `src-tauri/src/overlay.rs` 中把常量改为：

```rust
pub(crate) const OVERLAY_SIZE: f64 = 256.0;
```

- [ ] **Step 4: 运行测试并确认 GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor::tests::centers_logical_overlay_on_negative_coordinate_hidpi_monitor -- --exact`

Expected: PASS，256px 逻辑窗口在 2× 缩放、负坐标显示器上的左上角为 `(-1536, 464)`。

- [ ] **Step 5: 运行全量验证**

Run: `pnpm test`

Expected: 全部前端测试 PASS。

Run: `pnpm build`

Expected: TypeScript 检查与 Vite 构建成功。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 全部 Rust 测试 PASS。

- [ ] **Step 6: 提交覆盖层变更**

```bash
git add src-tauri/src/overlay.rs src-tauri/src/monitor.rs
git commit -m "功能：放大准星覆盖层窗口"
```
