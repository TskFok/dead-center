# 屏幕百分比准星实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将全部准星的整体尺寸改为目标屏幕百分比，并让“细旗空心菱形”从目标屏幕四边按百分比向中心缺口延伸。

**Architecture:** 使用单个与目标显示器完全重合的透明 Tauri 窗口作为渲染画布。普通预设按画布短边计算正方形尺寸；`fine-diamond` 填满画布并把百分比解释为四条边缘旗臂的向心进度。设置协议升级到版本 2，并在 Rust 加载层把版本 1 的 `sizePx` 按目标屏幕逻辑短边迁移为 `sizePercent`。

**Tech Stack:** React 19、TypeScript 5.8、CSS、Vitest 4、Testing Library、Rust stable、Tauri 2、Serde

## Global Constraints

- 默认在当前分支修改，不创建新分支。
- 所有 Git 提交信息使用简体中文。
- 不增加运行时依赖、数据库、网络请求、遥测或用户跟踪。
- 禁止在循环遍历中查询 SQL；本功能不引入 SQL。
- “整体尺寸”固定为 `0%` 到 `100%`、步进 `1%`、新安装默认 `3%`。
- 普通预设以目标屏幕逻辑短边为基准并保持正方形比例。
- `fine-diamond` 外端始终贴屏幕四边；`0%` 保留线宽大小的旗头；`100%` 停在中心缺口／空心菱形外缘。
- 线宽和中心缺口继续使用逻辑 CSS 像素。
- 透明、无边框、始终置顶、不可聚焦、鼠标穿透、多屏回退和托盘行为保持不变。

## 文件职责

- `src/shared/settings.ts`、`src-tauri/src/config.rs`：版本 2 设置协议、百分比默认值与边界；Rust 额外定义版本 1 兼容结构和纯迁移函数。
- `src-tauri/src/state.rs`：识别配置版本、解析迁移屏幕、保存迁移结果和预检配置文件。
- `src-tauri/src/monitor.rs`：逻辑短边与全屏覆盖层几何纯函数。
- `src-tauri/src/overlay.rs`：把完整屏幕几何应用到 Tauri 透明窗口。
- `src/components/Crosshair.tsx`、`src/components/Crosshair.css`：普通预设短边尺寸与细旗四边向心几何。
- `src/SettingsApp.tsx`、`src/SettingsApp.css`：百分比滑杆和实际解析屏幕比例预览画布。
- `src/global.css`：把覆盖层根元素声明为尺寸容器。
- 对应 `*.test.*` 文件：先锁定协议、迁移、几何和 UI 行为，再实现生产代码。
- `README.md`：记录新的百分比尺寸语义。

---

### Task 1：升级百分比设置协议并迁移版本 1 配置

**Files:**
- Modify: `src/shared/settings.ts:25-113`
- Modify: `src/shared/settings.test.ts:1-88`
- Modify: `src/SettingsApp.tsx:1-11,182-189`
- Modify: `src/SettingsApp.test.tsx:56-103`
- Modify: `src/components/Crosshair.tsx:20-40`
- Modify: `src/components/Crosshair.css:1-6`
- Modify: `src/components/Crosshair.test.tsx:255-267`
- Modify: `src/global.css:37-45`
- Modify: `src/SettingsApp.css:226-238`
- Modify: `src-tauri/src/config.rs:1-140`
- Modify: `src-tauri/src/state.rs:1-200`
- Modify: `src-tauri/src/monitor.rs:72-107,133-185`

**Interfaces:**
- Consumes: 版本 1 JSON 的 `visual.sizePx` 与 `MonitorGeometry { width, height, scale_factor }`。
- Produces: `VisualSettings.sizePercent`、`AppSettings.version = 2`、`logical_short_edge(&MonitorGeometry) -> Option<f64>`、`resolved_logical_short_edge(&[MonitorGeometry], Option<&str>) -> Option<f64>`、`migrate_v1_settings(LegacyAppSettingsV1, Option<f64>) -> AppSettings`。

- [ ] **Step 1：写前端百分比协议和设置页失败测试**

在 `src/shared/settings.test.ts` 把默认值与边界用例改为：

```ts
import {
  CROSSHAIR_SIZE_PERCENT_MAX,
  CROSSHAIR_SIZE_PERCENT_MIN,
  DEFAULT_SETTINGS,
  isDiamondPreset,
  isHexColor,
  normalizeVisualSettings,
} from "./settings";

it("默认使用版本 2 和 3% 尺寸", () => {
  expect(DEFAULT_SETTINGS.version).toBe(2);
  expect(DEFAULT_SETTINGS.visual).toMatchObject({
    preset: "classic-cross",
    sizePercent: 3,
  });
  expect(DEFAULT_SETTINGS.visual).not.toHaveProperty("sizePx");
});

it("把百分比尺寸限制到 0 至 100", () => {
  expect(CROSSHAIR_SIZE_PERCENT_MIN).toBe(0);
  expect(CROSSHAIR_SIZE_PERCENT_MAX).toBe(100);
  expect(
    normalizeVisualSettings({
      ...DEFAULT_SETTINGS.visual,
      sizePercent: 120,
    }).sizePercent,
  ).toBe(100);
  expect(
    normalizeVisualSettings({
      ...DEFAULT_SETTINGS.visual,
      sizePercent: -1,
    }).sizePercent,
  ).toBe(0);
});
```

保留颜色、预设守卫、透明度、线宽和缺口用例，只把其中的设置字段从 `sizePx` 改为 `sizePercent`。

在 `src/SettingsApp.test.tsx` 首个用例加入：

```tsx
expect(screen.getByLabelText("整体尺寸")).toHaveAttribute("min", "0");
expect(screen.getByLabelText("整体尺寸")).toHaveAttribute("max", "100");
expect(screen.getByText("3%")).toBeVisible();
```

并新增：

```tsx
it("按百分比调整整体尺寸并防抖保存", async () => {
  vi.useFakeTimers();
  const bridge = makeBridge();
  render(<SettingsApp bridge={bridge} />);
  await act(async () => undefined);

  fireEvent.change(screen.getByLabelText("整体尺寸"), {
    target: { value: "25" },
  });

  expect(bridge.updateVisual).not.toHaveBeenCalled();
  await act(async () => vi.advanceTimersByTime(100));
  expect(bridge.updateVisual).toHaveBeenCalledWith(
    expect.objectContaining({ sizePercent: 25 }),
  );
});
```

把 `src/components/Crosshair.test.tsx` 的行内尺寸断言改为：

```tsx
expect(crosshair).toHaveStyle({ opacity: "0.8" });
expect(crosshair.style.getPropertyValue("--crosshair-size")).toBe("3cqmin");
```

- [ ] **Step 2：运行前端目标测试并确认 RED**

Run: `pnpm test -- src/shared/settings.test.ts src/SettingsApp.test.tsx src/components/Crosshair.test.tsx`

Expected: FAIL；`sizePercent` 和新常量不存在，滑杆仍为 12–192 px，Crosshair 仍写入 `32px`。

- [ ] **Step 3：写 Rust 版本 2、迁移和预检失败测试**

在 `src-tauri/src/config.rs` 测试模块新增：

```rust
fn legacy_defaults() -> LegacyAppSettingsV1 {
    LegacyAppSettingsV1 {
        version: 1,
        visual: LegacyVisualSettingsV1 {
            preset: CrosshairPreset::ClassicCross,
            primary_color: "#4DFFB8".into(),
            accent_color: "#F4FF4D".into(),
            opacity: 0.8,
            size_px: 32.0,
            stroke_px: 3.0,
            gap_px: 8.0,
        },
        target_monitor_id: None,
        toggle_shortcut: "Alt+Shift+X".into(),
        launch_at_login: false,
        show_on_launch: true,
    }
}

#[test]
fn default_and_serialized_settings_use_percent_size() {
    let settings = AppSettings::default();
    assert_eq!(settings.version, 2);
    assert_eq!(settings.visual.size_percent, 3.0);
    let value = serde_json::to_value(&settings.visual).unwrap();
    assert_eq!(value["sizePercent"], 3.0);
    assert!(value.get("sizePx").is_none());
}

#[test]
fn validation_clamps_percent_size() {
    let mut visual = AppSettings::default().visual;
    visual.size_percent = 120.0;
    visual.normalize();
    assert_eq!(visual.size_percent, 100.0);
    visual.size_percent = -1.0;
    visual.normalize();
    assert_eq!(visual.size_percent, 0.0);
}

#[test]
fn migrates_pixels_against_logical_short_edge_and_preserves_fields() {
    let mut legacy = legacy_defaults();
    legacy.visual.preset = CrosshairPreset::FineDiamond;
    legacy.target_monitor_id = Some("secondary".into());
    legacy.toggle_shortcut = "Alt+Shift+Y".into();
    legacy.launch_at_login = true;
    legacy.show_on_launch = false;
    let settings = migrate_v1_settings(legacy, Some(800.0));
    assert_eq!(settings.version, 2);
    assert_eq!(settings.visual.size_percent, 4.0);
    assert_eq!(settings.visual.preset, CrosshairPreset::FineDiamond);
    assert_eq!(settings.target_monitor_id.as_deref(), Some("secondary"));
    assert_eq!(settings.toggle_shortcut, "Alt+Shift+Y");
    assert!(settings.launch_at_login);
    assert!(!settings.show_on_launch);
}

#[test]
fn migration_without_monitor_uses_three_percent() {
    assert_eq!(
        migrate_v1_settings(legacy_defaults(), None)
            .visual
            .size_percent,
        3.0
    );
}

#[test]
fn migration_clamps_converted_percent() {
    let mut legacy = legacy_defaults();
    legacy.visual.size_px = 2000.0;
    assert_eq!(
        migrate_v1_settings(legacy, Some(800.0))
            .visual
            .size_percent,
        100.0
    );
}
```

在 `src-tauri/src/monitor.rs` 测试模块加入：

```rust
#[test]
fn logical_short_edge_accounts_for_scale_factor() {
    let monitor = monitor("retina", 0, 2560, 2.0, true);
    assert_eq!(logical_short_edge(&monitor), Some(720.0));
}

#[test]
fn resolved_short_edge_uses_target_fallback_and_empty_default() {
    let monitors = vec![
        monitor("primary", 0, 1920, 1.0, true),
        monitor("secondary", 1920, 2560, 2.0, false),
    ];
    assert_eq!(
        resolved_logical_short_edge(&monitors, Some("secondary")),
        Some(720.0)
    );
    assert_eq!(
        resolved_logical_short_edge(&monitors, Some("missing")),
        Some(1440.0)
    );
    assert_eq!(resolved_logical_short_edge(&[], None), None);
}
```

在 `src-tauri/src/state.rs` 测试模块加入版本 1 fixture，并替换版本预检测试：

```rust
fn version_one_value() -> Value {
    serde_json::json!({
        "version": 1,
        "visual": {
            "preset": "classic-cross",
            "primaryColor": "#4DFFB8",
            "accentColor": "#F4FF4D",
            "opacity": 0.8,
            "sizePx": 32.0,
            "strokePx": 3.0,
            "gapPx": 8.0
        },
        "targetMonitorId": null,
        "toggleShortcut": "Alt+Shift+X",
        "launchAtLogin": false,
        "showOnLaunch": true
    })
}

#[test]
fn valid_store_accepts_versions_one_and_two() {
    for settings in [
        version_one_value(),
        serde_json::to_value(AppSettings::default()).unwrap(),
    ] {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "settings": settings
        }))
        .unwrap();
        assert!(is_valid_settings_store(&bytes));
    }
}

#[test]
fn unsupported_settings_version_is_not_valid() {
    let mut settings = serde_json::to_value(AppSettings::default()).unwrap();
    settings["version"] = 3.into();
    let bytes = serde_json::to_vec(&serde_json::json!({ "settings": settings })).unwrap();
    assert!(!is_valid_settings_store(&bytes));
}
```

- [ ] **Step 4：运行 Rust 测试并确认 RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests`

Expected: FAIL；`size_percent`、兼容结构和迁移函数不存在。

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor::tests`

Expected: FAIL；`logical_short_edge` 不存在。

Run: `cargo test --manifest-path src-tauri/Cargo.toml state::tests`

Expected: FAIL；版本 1 仍不能通过预检。

- [ ] **Step 5：实现前端版本 2 协议与百分比滑杆**

在 `src/shared/settings.ts` 使用：

```ts
export interface VisualSettings {
  preset: CrosshairPreset;
  primaryColor: string;
  accentColor: string;
  opacity: number;
  sizePercent: number;
  strokePx: number;
  gapPx: number;
}

export interface AppSettings {
  version: 2;
  visual: VisualSettings;
  targetMonitorId: string | null;
  toggleShortcut: string;
  launchAtLogin: boolean;
  showOnLaunch: boolean;
}

export const CROSSHAIR_SIZE_PERCENT_MIN = 0;
export const CROSSHAIR_SIZE_PERCENT_MAX = 100;
```

把 `DEFAULT_SETTINGS.version` 设为 `2`，把 `sizePx: 32` 替换为 `sizePercent: 3`。在 `normalizeVisualSettings` 使用：

```ts
sizePercent: clamp(
  value.sizePercent,
  CROSSHAIR_SIZE_PERCENT_MIN,
  CROSSHAIR_SIZE_PERCENT_MAX,
),
```

在 `src/SettingsApp.tsx` 导入新常量，并替换尺寸控件：

```tsx
<RangeControl
  label="整体尺寸"
  max={CROSSHAIR_SIZE_PERCENT_MAX}
  min={CROSSHAIR_SIZE_PERCENT_MIN}
  suffix="%"
  value={visual.sizePercent}
  onChange={(value) => setVisual({ ...visual, sizePercent: value })}
/>
```

在 `src/components/Crosshair.tsx` 的 `CrosshairStyle` 增加 `"--crosshair-size": string`，并把行内宽高替换为：

```ts
"--crosshair-size": `${settings.sizePercent}cqmin`,
```

在 `src/components/Crosshair.css` 的 `.crosshair` 中加入：

```css
width: var(--crosshair-size);
height: var(--crosshair-size);
```

在 `src/global.css` 的 `.overlay-root` 和 `src/SettingsApp.css` 的 `.preview-stage` 中加入 `container-type: size;`。

- [ ] **Step 6：实现 Rust 版本 2、兼容结构与纯迁移函数**

在 `src-tauri/src/config.rs` 定义：

```rust
pub const SETTINGS_VERSION: u8 = 2;
pub const DEFAULT_SIZE_PERCENT: f64 = 3.0;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSettings {
    pub preset: CrosshairPreset,
    pub primary_color: String,
    pub accent_color: String,
    pub opacity: f64,
    pub size_percent: f64,
    pub stroke_px: f64,
    pub gap_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyVisualSettingsV1 {
    pub(crate) preset: CrosshairPreset,
    pub(crate) primary_color: String,
    pub(crate) accent_color: String,
    pub(crate) opacity: f64,
    pub(crate) size_px: f64,
    pub(crate) stroke_px: f64,
    pub(crate) gap_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyAppSettingsV1 {
    pub(crate) version: u8,
    pub(crate) visual: LegacyVisualSettingsV1,
    pub(crate) target_monitor_id: Option<String>,
    pub(crate) toggle_shortcut: String,
    pub(crate) launch_at_login: bool,
    pub(crate) show_on_launch: bool,
}
```

把 Rust 默认版本设为 `SETTINGS_VERSION`，默认尺寸设为 `DEFAULT_SIZE_PERCENT`，规范化改为 `self.size_percent = self.size_percent.clamp(0.0, 100.0);`。加入：

```rust
pub(crate) fn migrate_v1_settings(
    legacy: LegacyAppSettingsV1,
    logical_short_edge: Option<f64>,
) -> AppSettings {
    let size_percent = logical_short_edge
        .filter(|edge| edge.is_finite() && *edge > 0.0)
        .map(|edge| legacy.visual.size_px / edge * 100.0)
        .unwrap_or(DEFAULT_SIZE_PERCENT);
    let mut visual = VisualSettings {
        preset: legacy.visual.preset,
        primary_color: legacy.visual.primary_color,
        accent_color: legacy.visual.accent_color,
        opacity: legacy.visual.opacity,
        size_percent,
        stroke_px: legacy.visual.stroke_px,
        gap_px: legacy.visual.gap_px,
    };
    visual.normalize();
    AppSettings {
        version: SETTINGS_VERSION,
        visual,
        target_monitor_id: legacy.target_monitor_id,
        toggle_shortcut: legacy.toggle_shortcut,
        launch_at_login: legacy.launch_at_login,
        show_on_launch: legacy.show_on_launch,
    }
}
```

在 `src-tauri/src/monitor.rs` 加入：

```rust
pub fn logical_short_edge(monitor: &MonitorGeometry) -> Option<f64> {
    (monitor.scale_factor.is_finite() && monitor.scale_factor > 0.0).then(|| {
        f64::from(monitor.width.min(monitor.height)) / monitor.scale_factor
    })
}

pub fn resolved_logical_short_edge(
    monitors: &[MonitorGeometry],
    target_monitor_id: Option<&str>,
) -> Option<f64> {
    if monitors.is_empty() {
        return None;
    }
    logical_short_edge(resolve_monitor(monitors, target_monitor_id).monitor)
}
```

- [ ] **Step 7：实现配置版本分派和迁移后保存**

在 `src-tauri/src/state.rs` 导入迁移函数、兼容结构、版本常量和显示器 helper，加入：

```rust
fn settings_version(value: &Value) -> Option<u64> {
    value.get("version").and_then(Value::as_u64)
}

fn legacy_short_edge<R: Runtime>(
    app: &AppHandle<R>,
    target_monitor_id: Option<&str>,
) -> Option<f64> {
    let monitors = collect_monitors(app).ok()?;
    resolved_logical_short_edge(&monitors, target_monitor_id)
}

fn decode_settings<R: Runtime>(
    app: &AppHandle<R>,
    value: Value,
) -> Option<(AppSettings, bool)> {
    match settings_version(&value)? {
        1 => {
            let legacy = serde_json::from_value::<LegacyAppSettingsV1>(value).ok()?;
            let short_edge = legacy_short_edge(app, legacy.target_monitor_id.as_deref());
            Some((migrate_v1_settings(legacy, short_edge), true))
        }
        version if version == u64::from(SETTINGS_VERSION) => {
            let mut settings = serde_json::from_value::<AppSettings>(value).ok()?;
            settings.visual.normalize();
            Some((settings, false))
        }
        _ => None,
    }
}
```

在 `load_settings` 中把已有配置分支替换为：

```rust
if let Some(value) = store.get("settings") {
    if let Some((settings, migrated)) = decode_settings(app, value) {
        if migrated {
            store.set(
                "settings",
                serde_json::to_value(&settings).map_err(|error| error.to_string())?,
            );
            store.save().map_err(|error| error.to_string())?;
        }
        return Ok(settings);
    }
}
```

用以下函数完成预检版本分派：

```rust
fn is_supported_settings_value(value: Value) -> bool {
    match settings_version(&value) {
        Some(1) => serde_json::from_value::<LegacyAppSettingsV1>(value).is_ok(),
        Some(version) if version == u64::from(SETTINGS_VERSION) => {
            serde_json::from_value::<AppSettings>(value).is_ok()
        }
        _ => false,
    }
}
```

`is_valid_settings_store` 继续解析 `HashMap<String, Value>`，把 `settings` 值 `cloned().map(is_supported_settings_value).unwrap_or(true)` 作为结果。

- [ ] **Step 8：运行协议和迁移回归并确认 GREEN**

Run: `pnpm test -- src/shared/settings.test.ts src/SettingsApp.test.tsx src/components/Crosshair.test.tsx`

Expected: PASS。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS；版本 1/2 预检、迁移、逻辑短边和规范化测试通过。

Run: `pnpm build`

Expected: exit code 0。

- [ ] **Step 9：提交百分比协议与迁移**

```bash
git add src/shared/settings.ts src/shared/settings.test.ts src/SettingsApp.tsx src/SettingsApp.test.tsx src/components/Crosshair.tsx src/components/Crosshair.css src/components/Crosshair.test.tsx src/global.css src/SettingsApp.css src-tauri/src/config.rs src-tauri/src/state.rs src-tauri/src/monitor.rs
git commit -m "功能：升级准星百分比尺寸配置"
```

---

### Task 2：让覆盖层窗口完整匹配目标屏幕

**Files:**
- Modify: `src-tauri/src/monitor.rs:16-25,97-106,133-185`
- Modify: `src-tauri/src/overlay.rs:1-89`
- Modify: `src-tauri/src/commands.rs:52-70`
- Modify: `src-tauri/src/lib.rs:58-64`

**Interfaces:**
- Consumes: `MonitorGeometry`、`resolve_monitor`、`AppState.settings.target_monitor_id`。
- Produces: `OverlayGeometry { position, physical_width, physical_height, logical_width, logical_height }`、`full_screen_overlay_geometry(&MonitorGeometry) -> Option<OverlayGeometry>`、`refresh_overlay_geometry(&AppHandle<R>) -> Result<(), String>`。

- [ ] **Step 1：写全屏覆盖层几何失败测试**

把 `src-tauri/src/monitor.rs` 的测试 helper 改为：

```rust
fn monitor(
    id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale_factor: f64,
    primary: bool,
) -> MonitorGeometry {
    MonitorGeometry {
        id: id.into(),
        name: id.into(),
        x,
        y,
        width,
        height,
        scale_factor,
        primary,
    }
}
```

更新原有 helper 调用参数，并把旧的 256px 居中用例替换为：

```rust
#[test]
fn full_screen_overlay_uses_physical_origin_and_logical_size() {
    let monitor = monitor("left", -2560, -180, 2560, 1440, 2.0, false);
    assert_eq!(
        full_screen_overlay_geometry(&monitor),
        Some(OverlayGeometry {
            position: PhysicalPoint { x: -2560, y: -180 },
            physical_width: 2560,
            physical_height: 1440,
            logical_width: 1280.0,
            logical_height: 720.0,
        })
    );
}

#[test]
fn full_screen_overlay_rejects_invalid_scale_factor() {
    let monitor = monitor("invalid", 0, 0, 1920, 1080, 0.0, true);
    assert_eq!(full_screen_overlay_geometry(&monitor), None);
}

#[test]
fn full_screen_overlay_preserves_portrait_dimensions() {
    let monitor = monitor("portrait", 1920, 0, 1080, 1920, 1.25, false);
    let geometry = full_screen_overlay_geometry(&monitor).unwrap();
    assert_eq!(geometry.physical_width, 1080);
    assert_eq!(geometry.physical_height, 1920);
    assert_eq!(geometry.logical_width, 864.0);
    assert_eq!(geometry.logical_height, 1536.0);
}
```

- [ ] **Step 2：运行几何测试并确认 RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor::tests`

Expected: FAIL；`OverlayGeometry` 和 `full_screen_overlay_geometry` 未定义。

- [ ] **Step 3：实现全屏几何纯函数**

在 `src-tauri/src/monitor.rs` 定义：

```rust
#[derive(Debug, PartialEq)]
pub struct OverlayGeometry {
    pub position: PhysicalPoint,
    pub physical_width: u32,
    pub physical_height: u32,
    pub logical_width: f64,
    pub logical_height: f64,
}

pub fn full_screen_overlay_geometry(
    monitor: &MonitorGeometry,
) -> Option<OverlayGeometry> {
    (monitor.scale_factor.is_finite() && monitor.scale_factor > 0.0).then(|| {
        OverlayGeometry {
            position: PhysicalPoint {
                x: monitor.x,
                y: monitor.y,
            },
            physical_width: monitor.width,
            physical_height: monitor.height,
            logical_width: f64::from(monitor.width) / monitor.scale_factor,
            logical_height: f64::from(monitor.height) / monitor.scale_factor,
        }
    })
}
```

删除 `centered_overlay_position` 和 `OVERLAY_SIZE` 测试依赖。

- [ ] **Step 4：创建隐藏窗口后应用完整屏幕几何**

在 `src-tauri/src/overlay.rs` 导入 `PhysicalSize` 和新几何函数，删除 `OVERLAY_SIZE`。把 `create_or_refresh_overlay` 改为：

```rust
pub fn create_or_refresh_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let should_show = app
        .state::<AppState>()
        .status
        .lock()
        .map_err(|error| error.to_string())?
        .visible;
    let created = app.get_webview_window(OVERLAY_LABEL).is_none();

    if created {
        let mut builder = WebviewWindowBuilder::new(
            app,
            OVERLAY_LABEL,
            WebviewUrl::App(PathBuf::from("index.html?view=overlay")),
        )
        .title("Dead Center Crosshair")
        .inner_size(1.0, 1.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .focusable(false)
        .skip_taskbar(true)
        .visible(false);

        #[cfg(target_os = "macos")]
        {
            builder = builder.visible_on_all_workspaces(true);
        }

        let window = builder.build().map_err(|error| error.to_string())?;
        window
            .set_ignore_cursor_events(true)
            .map_err(|error| error.to_string())?;
    }

    refresh_overlay_geometry(app)?;
    if created && should_show {
        if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
            window.show().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
```

把 `refresh_overlay_position` 重命名为 `refresh_overlay_geometry`。解析目标屏幕后使用：

```rust
let geometry = full_screen_overlay_geometry(resolved.monitor)
    .ok_or_else(|| "目标显示器缩放比例无效".to_string())?;

if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
    window
        .set_position(PhysicalPosition::new(
            geometry.position.x,
            geometry.position.y,
        ))
        .map_err(|error| error.to_string())?;
    window
        .set_size(PhysicalSize::new(
            geometry.physical_width,
            geometry.physical_height,
        ))
        .map_err(|error| error.to_string())?;
    window
        .set_always_on_top(true)
        .map_err(|error| error.to_string())?;
    window
        .set_focusable(false)
        .map_err(|error| error.to_string())?;
}
```

保留函数后半段对 `resolved_monitor_id`、`using_fallback_monitor`、`error` 和运行时事件的更新。

- [ ] **Step 5：切换完整几何刷新调用并确认 GREEN**

在 `src-tauri/src/commands.rs` 的 `select_monitor` 中使用 `overlay::refresh_overlay_geometry(&app)?;`；在 `src-tauri/src/lib.rs` 的两秒轮询中使用 `overlay::refresh_overlay_geometry(&handle)` 并保留现有错误记录分支。

Run: `rg -n "OVERLAY_SIZE|centered_overlay_position|refresh_overlay_position" src-tauri/src`

Expected: 无输出。

Run: `cargo test --manifest-path src-tauri/Cargo.toml monitor::tests`

Expected: PASS。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS。

- [ ] **Step 6：提交全屏覆盖层**

```bash
git add src-tauri/src/monitor.rs src-tauri/src/overlay.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "功能：覆盖层匹配目标屏幕尺寸"
```

---

### Task 3：实现细旗四边锚定、向心进度与屏幕比例预览

**Files:**
- Modify: `src/components/Crosshair.tsx:20-53`
- Modify: `src/components/Crosshair.css:1-10,83-104,149-183`
- Modify: `src/components/Crosshair.test.tsx:16-24,160-192,255-267`
- Modify: `src/SettingsApp.tsx:108-162`
- Modify: `src/SettingsApp.test.tsx:56-103`
- Modify: `src/SettingsApp.css:226-259`
- Verify: `src/OverlayApp.tsx:26-30`
- Verify: `src/global.css:37-46`

**Interfaces:**
- Consumes: `sizePercent`、`strokePx`、`gapPx`、`status.resolvedMonitorId`、`MonitorInfo.width/height`。
- Produces: `--fine-arm-length`、`crosshair--zero`、`aria-label="目标屏幕预览画布"`、`data-monitor-id`。

- [ ] **Step 1：写细旗 0%、100% 和普通预设零值失败测试**

让 `src/components/Crosshair.test.tsx` 的 helper 接收覆盖值：

```tsx
const renderPreset = (
  preset: CrosshairPreset,
  visual: Partial<typeof DEFAULT_SETTINGS.visual> = {},
) =>
  render(
    <Crosshair
      settings={{
        ...DEFAULT_SETTINGS.visual,
        ...visual,
        preset,
      }}
    />,
  );
```

把旧的 fine-diamond 容器边缘用例拆成以下行为断言，并继续保留原四向向心 `clip-path` 循环：

```tsx
it("fine-diamond 在 0% 时铺满画布并保留线宽旗头", () => {
  renderPreset("fine-diamond", {
    sizePercent: 0,
    strokePx: 3,
    gapPx: 8,
  });
  const crosshair = screen.getByLabelText("细旗空心菱形");
  expect(crosshair).toHaveClass("crosshair--zero");
  expect(crosshair.style.getPropertyValue("--fine-arm-length")).toBe(
    "max(3px, calc(0% - 0px))",
  );
  expect(readPresetBlock("fine-diamond")).toContain("width: 100cqw");
  expect(readPresetBlock("fine-diamond")).toContain("height: 100cqh");
});

it("fine-diamond 在 100% 时停在菱形和缺口外缘", () => {
  renderPreset("fine-diamond", {
    sizePercent: 100,
    strokePx: 3,
    gapPx: 8,
  });
  expect(
    screen
      .getByLabelText("细旗空心菱形")
      .style.getPropertyValue("--fine-arm-length"),
  ).toBe("max(3px, calc(50% - 14px))");
});

it("fine-diamond 在 100% 时响应中心缺口", () => {
  const noGap = renderPreset("fine-diamond", {
    sizePercent: 100,
    gapPx: 0,
  });
  expect(
    screen
      .getByLabelText("细旗空心菱形")
      .style.getPropertyValue("--fine-arm-length"),
  ).toContain("calc(50% - 10px)");
  noGap.unmount();

  renderPreset("fine-diamond", {
    sizePercent: 100,
    gapPx: 24,
  });
  expect(
    screen
      .getByLabelText("细旗空心菱形")
      .style.getPropertyValue("--fine-arm-length"),
  ).toContain("calc(50% - 22px)");
});

it("普通预设在 0% 时隐藏但 fine-diamond 仍保留中心", () => {
  const ordinary = renderPreset("classic-cross", { sizePercent: 0 });
  expect(screen.getByLabelText("缺口十字")).toHaveClass("crosshair--zero");
  expect(crosshairCss).toContain(
    ".crosshair--zero:not(.crosshair--fine-diamond)",
  );
  ordinary.unmount();

  renderPreset("fine-diamond", { sizePercent: 0 });
  expect(document.querySelector(".crosshair__diamond")).toBeInTheDocument();
});
```

- [ ] **Step 2：写实际解析屏幕比例预览失败测试**

在 `src/SettingsApp.test.tsx` 首个用例加入：

```tsx
const viewport = screen.getByLabelText("目标屏幕预览画布");
expect(viewport).toHaveAttribute("data-monitor-id", "primary");
expect(viewport).toHaveStyle({ aspectRatio: "2560 / 1440" });
```

新增回退用例：

```tsx
it("预览使用运行时实际解析出的回退屏幕比例", async () => {
  const fallbackSnapshot: AppSnapshot = {
    ...snapshot,
    status: {
      ...snapshot.status,
      resolvedMonitorId: "fallback",
      usingFallbackMonitor: true,
    },
    monitors: [
      ...snapshot.monitors,
      {
        id: "fallback",
        name: "回退屏幕",
        isPrimary: false,
        width: 1920,
        height: 1200,
        scaleFactor: 1,
      },
    ],
  };
  const bridge = makeBridge();
  vi.mocked(bridge.getSnapshot).mockResolvedValue(fallbackSnapshot);
  render(<SettingsApp bridge={bridge} />);

  const viewport = await screen.findByLabelText("目标屏幕预览画布");
  expect(viewport).toHaveAttribute("data-monitor-id", "fallback");
  expect(viewport).toHaveStyle({ aspectRatio: "1920 / 1200" });
});
```

- [ ] **Step 3：运行目标测试并确认 RED**

Run: `pnpm test -- src/components/Crosshair.test.tsx src/SettingsApp.test.tsx`

Expected: FAIL；细旗进度变量、零值类、100cqw/100cqh 和预览画布未实现。

- [ ] **Step 4：实现细旗线性进度和零值行为**

在 `src/components/Crosshair.tsx` 定义 `const FINE_DIAMOND_HALF_EXTENT_PX = 10;`，在 `CrosshairStyle` 增加 `"--fine-arm-length": string`，并在组件内计算：

```ts
const progress = settings.sizePercent / 100;
const fineCenterClearancePx =
  FINE_DIAMOND_HALF_EXTENT_PX + settings.gapPx / 2;
const scaledFineClearancePx = progress * fineCenterClearancePx;
```

在 `style` 加入：

```ts
"--fine-arm-length": `max(${settings.strokePx}px, calc(${settings.sizePercent / 2}% - ${scaledFineClearancePx}px))`,
```

把根类名改为：

```tsx
className={`crosshair crosshair--${settings.preset} ${
  settings.sizePercent === 0 ? "crosshair--zero" : ""
}`}
```

在 `src/components/Crosshair.css` 加入：

```css
.crosshair--zero:not(.crosshair--fine-diamond) {
  visibility: hidden;
}
```

把细旗变量块改为：

```css
.crosshair--fine-diamond {
  width: 100cqw;
  height: 100cqh;
  --flag-inset: 0%;
  --flag-thickness: min(max(var(--crosshair-stroke), 1px), 12%);
  --flag-clearance-cap: 24%;
  --diamond-clearance: 12%;
  --diamond-size: 14px;
  --diamond-stroke-cap: 1px;
}
```

在通用旗臂定位规则后加入：

```css
.crosshair--fine-diamond
  :is(.crosshair__flag--left, .crosshair__flag--right) {
  width: var(--fine-arm-length);
}

.crosshair--fine-diamond
  :is(.crosshair__flag--top, .crosshair__flag--bottom) {
  height: var(--fine-arm-length);
}
```

保留现有包含 `.crosshair--fine-diamond` 的四向向心 `clip-path` 分组。

- [ ] **Step 5：实现目标屏幕比例内部预览画布**

在 `src/SettingsApp.tsx` 增加类型导入和预览样式类型：

```ts
import type { CSSProperties } from "react";

type PreviewViewportStyle = CSSProperties & {
  "--preview-width-by-height": string;
  "--preview-height-by-width": string;
};
```

在返回 JSX 前计算实际解析屏幕和能完整适配外层预览区的尺寸变量：

```ts
const previewMonitor =
  snapshot.monitors.find(
    (monitor) => monitor.id === snapshot.status.resolvedMonitorId,
  ) ??
  snapshot.monitors.find((monitor) => monitor.isPrimary) ??
  snapshot.monitors[0];
const previewWidth = previewMonitor?.width ?? 16;
const previewHeight = previewMonitor?.height ?? 9;
const previewRatio = previewWidth / previewHeight;
const previewStyle: PreviewViewportStyle = {
  aspectRatio: `${previewWidth} / ${previewHeight}`,
  "--preview-width-by-height": `${previewRatio * 100}cqh`,
  "--preview-height-by-width": `${(1 / previewRatio) * 100}cqw`,
};
```

把预览内容改为：

```tsx
<div className="preview-stage">
  <div
    aria-label="目标屏幕预览画布"
    className="preview-viewport"
    data-monitor-id={previewMonitor?.id ?? "unknown"}
    style={previewStyle}
  >
    <div className="preview-grid" />
    <Crosshair settings={visual} />
  </div>
</div>
```

在 `src/SettingsApp.css` 用以下规则让外层提供容器尺寸、内层按屏幕比例完整适配：

```css
.preview-stage {
  height: 322px;
  margin-top: 18px;
  position: relative;
  display: grid;
  place-items: center;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 14px;
  background: #070a0f;
  container-type: size;
}

.preview-viewport {
  position: relative;
  width: min(100cqw, var(--preview-width-by-height));
  height: min(100cqh, var(--preview-height-by-width));
  display: grid;
  place-items: center;
  overflow: hidden;
  container-type: size;
  background:
    radial-gradient(circle at center, rgba(48, 65, 87, 0.75), transparent 37%),
    linear-gradient(145deg, #111824, #070a0f);
}
```

保留 `.preview-stage::after` 和 `.preview-grid`；确认 `src/global.css` 的 `.overlay-root` 仍包含 `container-type: size`，`src/OverlayApp.tsx` 仍直接在其中渲染 Crosshair。

- [ ] **Step 6：运行前端回归并确认 GREEN**

Run: `pnpm test -- src/components/Crosshair.test.tsx src/SettingsApp.test.tsx`

Expected: PASS。

Run: `pnpm test`

Expected: PASS；全部前端测试通过。

Run: `pnpm build`

Expected: exit code 0。

- [ ] **Step 7：提交细旗与预览画布**

```bash
git add src/components/Crosshair.tsx src/components/Crosshair.css src/components/Crosshair.test.tsx src/SettingsApp.tsx src/SettingsApp.test.tsx src/SettingsApp.css src/global.css
git commit -m "样式：细旗从屏幕四边向心延伸"
```

---

### Task 4：更新文档并完成全量验证

**Files:**
- Modify: `README.md:5-12,25-32`
- Modify: `docs/superpowers/plans/2026-07-13-screen-percent-crosshair.md`

**Interfaces:**
- Consumes: 最终版本 2 设置协议、全屏覆盖层和细旗几何。
- Produces: 用户文档、自动化验证记录和桌面视觉验证状态。

- [x] **Step 1：更新 README**

把功能列表中的尺寸说明写成：

```markdown
- 调节准星颜色、中心标记颜色、透明度、屏幕百分比尺寸、线宽和中心缺口。
- “细旗空心菱形”从目标屏幕四边向中心延伸，0% 保留边缘短旗头，100% 停在中心缺口或空心菱形外缘。
```

把使用步骤 2 写成：

```markdown
2. 在设置页选择样式和参数；普通预设的“整体尺寸”按目标屏幕短边百分比计算，“细旗空心菱形”则按百分比控制四臂从屏幕边缘向中心的延伸长度，修改会实时生效并自动保存。
```

- [x] **Step 2：检查旧字段和旧窗口接口**

Run: `rg -n "sizePx|size_px|CROSSHAIR_SIZE_(MIN|MAX)|OVERLAY_SIZE|refresh_overlay_position" src src-tauri README.md`

Expected: `sizePx`／`size_px` 仅出现在版本 1 兼容结构、迁移 fixture 和迁移测试中；旧常量与旧窗口刷新函数无输出。

- [x] **Step 3：运行格式与全量自动化验证**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: exit code 0。若失败，运行 `cargo fmt --manifest-path src-tauri/Cargo.toml`，再重新执行检查。

Run: `pnpm test`

Expected: PASS；全部 Vitest 测试通过。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS；全部 Rust 测试通过。

Run: `pnpm build`

Expected: exit code 0。

Run: `git diff --check`

Expected: 无输出，退出码为 0。

- [ ] **Step 4：运行桌面应用进行定向视觉验证**

状态：自动化验证完成，桌面视觉验证需在有图形会话的环境执行。

Run: `pnpm tauri dev`

Expected checklist:

1. 默认缺口十字显示 `3%`，在目标屏幕中央保持正方形。
2. 普通预设在 `0%` 时不可见，在 `100%` 时边长等于目标屏幕短边且不变形。
3. “细旗空心菱形”在 `0%` 时四边各显示一个贴边短旗头，中心空心菱形可见。
4. 细旗在 `100%` 时四臂停在中心缺口／空心菱形外缘，不覆盖中心。
5. 调整中心缺口时，四臂终点同步改变。
6. 切换不同分辨率或缩放比例的屏幕后，覆盖层与目标屏幕四边对齐，预览宽高比同步改变。
7. 鼠标点击可穿透覆盖层，快捷键显示／隐藏和托盘行为正常。

完成后停止开发进程。若没有可用图形会话，在 `progress.md` 记录“自动化验证完成，桌面视觉验证需在有图形会话的环境执行”，不得声称已完成视觉验证。

- [x] **Step 5：更新勾选并提交文档**

在本计划勾选已完成步骤，并在 `progress.md` 记录测试结果和视觉验证状态，然后执行：

```bash
git add README.md docs/superpowers/plans/2026-07-13-screen-percent-crosshair.md
git commit -m "文档：说明准星百分比尺寸"
```

- [x] **Step 6：最终检查提交范围**

结果：`git log -5 --oneline` 包含 README 说明、细旗向心样式和覆盖层安全修复；由于实施期间增加了审查补强提交，百分比配置迁移与全屏覆盖层主提交位于最近 10 条记录中，相关提交信息均为简体中文。

Run: `git status --short --branch`

Expected: 当前分支未变化；除本地规划记录 `task_plan.md`、`findings.md`、`progress.md` 外，没有未提交的产品代码或文档。

Run: `git log -5 --oneline`

Expected: 最近提交包含百分比配置迁移、全屏覆盖层、细旗向心样式和 README 说明，提交信息全部为简体中文。
