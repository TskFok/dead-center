# 细旗空心菱形贴边向心实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让“细旗空心菱形”的四条细旗臂从准星容器边缘开始，并以尖头朝向中央空心菱形。

**Architecture:** 保持现有 React 结构、设置协议和其他预设不变，只修改 `fine-diamond` 的 CSS 几何声明及其所属裁剪规则。测试直接读取 CSS，锁定零内缩和四向向心多边形，避免样式回退。

**Tech Stack:** React 19、TypeScript 5.8、CSS、Vitest 4、Testing Library

## Global Constraints

- 默认在当前分支修改，不新建分支。
- 只改变 `fine-diamond`；其他六种准星保持不变。
- 保留现有细旗厚度、中心净空、空心菱形尺寸与描边规则。
- 不增加设置字段、运行时依赖或 SQL 查询。
- 所有 Git 提交信息使用简体中文。

---

### Task 1：锁定并实现细旗贴边向心几何

**Files:**
- Modify: `src/components/Crosshair.test.tsx`
- Modify: `src/components/Crosshair.css`

**Interfaces:**
- Consumes: `readFlagGeometry(preset: DiamondCrosshairPreset): FlagGeometry`、`crosshairCss: string`，以及现有 `.crosshair__flag--top|right|bottom|left` DOM 类名。
- Produces: `fine-diamond` 的 `--flag-inset: 0%` 声明，以及左、右、上、下分别指向中心的 `clip-path` 多边形；不产生新的运行时接口。

- [ ] **Step 1：添加失败测试，锁定贴边与四向向心形状**

在 `src/components/Crosshair.test.tsx` 的 `describe("Crosshair", ...)` 中、菱形结构测试之后加入：

```tsx
  it("fine-diamond 从容器边缘向中心延伸", () => {
    expect(readFlagGeometry("fine-diamond").insetPercent).toBe(0);

    const inwardFineDiamondRules = [
      [
        "left",
        "polygon(0 0, 82% 0, 100% 50%, 82% 100%, 0 100%)",
      ],
      [
        "right",
        "polygon(18% 0, 100% 0, 100% 100%, 18% 100%, 0 50%)",
      ],
      [
        "top",
        "polygon(0 0, 100% 0, 100% 82%, 50% 100%, 0 82%)",
      ],
      [
        "bottom",
        "polygon(50% 0, 100% 18%, 100% 100%, 0 100%, 0 18%)",
      ],
    ] as const;

    for (const [direction, clipPath] of inwardFineDiamondRules) {
      const matchingRule = [...crosshairCss.matchAll(/([^{}]+)\{([^{}]+)\}/g)]
        .find(
          ([, selector, declarations]) =>
            selector.includes(".crosshair--fine-diamond") &&
            selector.includes(`.crosshair__flag--${direction}`) &&
            declarations.includes(`clip-path: ${clipPath}`),
        );
      expect(matchingRule, `${direction} 旗臂应以尖头朝向中心`).toBeDefined();
    }
  });
```

- [ ] **Step 2：运行目标测试并确认按预期失败**

Run: `pnpm test -- src/components/Crosshair.test.tsx`

Expected: FAIL；`insetPercent` 当前为 `22`，且 `fine-diamond` 当前属于四向尖头朝外的裁剪规则。

- [ ] **Step 3：最小修改细旗几何与裁剪分组**

在 `src/components/Crosshair.css` 中将细旗内缩改为零：

```css
.crosshair--fine-diamond {
  --flag-inset: 0%;
  --flag-thickness: min(max(var(--crosshair-stroke), 1px), 12%);
  --flag-clearance-cap: 24%;
  --diamond-clearance: 12%;
  --diamond-size: clamp(4px, 18%, 14px);
  --diamond-stroke-cap: 1px;
}
```

把四条朝外规则的选择器从：

```css
:is(.crosshair--fine-diamond, .crosshair--long-diamond)
```

改为：

```css
.crosshair--long-diamond
```

共修改左、右、上、下四处。再把四条朝内规则的选择器从：

```css
:is(.crosshair--inward-diamond, .crosshair--solid-diamond)
```

改为：

```css
:is(
  .crosshair--fine-diamond,
  .crosshair--inward-diamond,
  .crosshair--solid-diamond
)
```

同样修改左、右、上、下四处。保留所有 `clip-path` 坐标和其他变量原值。

- [ ] **Step 4：运行组件测试并确认通过**

Run: `pnpm test -- src/components/Crosshair.test.tsx`

Expected: PASS；组件测试全部通过，新增用例同时确认零内缩与四向向心尖头。

- [ ] **Step 5：运行完整前端回归测试和生产构建**

Run: `pnpm test`

Expected: PASS；全部 Vitest 测试通过。

Run: `pnpm build`

Expected: exit code 0；TypeScript 检查和 Vite 生产构建成功。

- [ ] **Step 6：检查改动范围并提交**

Run: `git diff --check`

Expected: 无输出，退出码为 0。

Run: `git status --short`

Expected: 仅显示 `src/components/Crosshair.css`、`src/components/Crosshair.test.tsx` 和本计划的执行勾选状态（如有更新）。

```bash
git add src/components/Crosshair.css src/components/Crosshair.test.tsx docs/superpowers/plans/2026-07-13-fine-diamond-inward-edge.md
git commit -m "样式：细旗菱形改为贴边向心"
```
