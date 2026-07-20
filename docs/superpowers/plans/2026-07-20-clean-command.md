# 清理命令 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增跨平台的 `pnpm clean`，仅删除项目的 Node.js 依赖和 Tauri 构建缓存。

**Architecture:** `package.json` 将命令委托给一个 Node.js ESM 脚本。脚本导出可注入根目录的路径解析与删除函数，以便 Vitest 只在临时目录验证删除行为；被直接执行时，它固定清理仓库根目录的两个目标。

**Tech Stack:** Node.js 22 ESM、`node:fs/promises`、Vitest、pnpm。

## Global Constraints

- 运行环境为 Node.js 22+、pnpm 10+。
- 不新增任何运行时或开发依赖。
- 仅允许删除仓库根目录的 `node_modules` 和 `src-tauri/target`。
- 清理范围不得包含锁文件、源码、`dist` 或项目外路径。
- 目录缺失时命令必须成功；实际 I/O 错误必须保留并以非零状态退出。
- 所有提交信息使用简体中文。

---

### Task 1: 实现并测试清理模块

**Files:**
- Create: `scripts/clean.mjs`
- Create: `scripts/clean.test.mjs`

**Interfaces:**
- Produces: `getCleanTargets(rootDirectory: string): string[]`，按顺序返回 `<root>/node_modules` 与 `<root>/src-tauri/target`。
- Produces: `cleanTargets(targets: string[]): Promise<void>`，递归删除所给目录，并允许目录不存在。
- Consumes: Node.js 内置的 `node:fs/promises`、`node:path` 与 `node:url`。

- [ ] **Step 1: 写入失败测试**

创建 `scripts/clean.test.mjs`：

```js
// @vitest-environment node

import { access, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanTargets, getCleanTargets } from "./clean.mjs";

const temporaryRoots = [];

afterEach(async () => {
  await Promise.all(temporaryRoots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function createRoot() {
  const root = await mkdtemp(join(tmpdir(), "dead-center-clean-"));
  temporaryRoots.push(root);
  return root;
}

describe("清理目录", () => {
  it("只定位 Node 依赖和 Tauri 构建目录", () => {
    const root = join("workspace", "dead-center");

    expect(getCleanTargets(root)).toEqual([
      join(root, "node_modules"),
      join(root, "src-tauri", "target"),
    ]);
  });

  it("删除两个目标目录及其内容", async () => {
    const root = await createRoot();
    const [nodeModules, tauriTarget] = getCleanTargets(root);
    await mkdir(nodeModules, { recursive: true });
    await mkdir(tauriTarget, { recursive: true });
    await writeFile(join(nodeModules, "package.txt"), "dependency");
    await writeFile(join(tauriTarget, "artifact.txt"), "build");

    await cleanTargets([nodeModules, tauriTarget]);

    await expect(access(nodeModules)).rejects.toThrow();
    await expect(access(tauriTarget)).rejects.toThrow();
  });

  it("目标目录不存在时仍成功", async () => {
    const root = await createRoot();

    await expect(cleanTargets(getCleanTargets(root))).resolves.toBeUndefined();
  });
});
```

- [ ] **Step 2: 运行测试，确认因模块缺失失败**

Run: `pnpm test -- scripts/clean.test.mjs`

Expected: FAIL，错误指出无法解析 `./clean.mjs`。

- [ ] **Step 3: 写入最小实现**

创建 `scripts/clean.mjs`：

```js
import { rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export function getCleanTargets(rootDirectory) {
  return [join(rootDirectory, "node_modules"), join(rootDirectory, "src-tauri", "target")];
}

export async function cleanTargets(targets) {
  await Promise.all(targets.map((target) => rm(target, { recursive: true, force: true })));
}

if (import.meta.main) {
  const rootDirectory = fileURLToPath(new URL("../", import.meta.url));
  await cleanTargets(getCleanTargets(rootDirectory));
}
```

- [ ] **Step 4: 运行定向测试，确认通过**

Run: `pnpm test -- scripts/clean.test.mjs`

Expected: PASS，3 个清理行为测试通过。

- [ ] **Step 5: 运行全量前端测试**

Run: `pnpm test`

Expected: PASS，既有测试与新增 Node 环境测试全部通过。

### Task 2: 暴露命令并补充使用说明

**Files:**
- Modify: `package.json:6-14`
- Modify: `README.md:39-50`

**Interfaces:**
- Consumes: `scripts/clean.mjs` 作为 `pnpm clean` 的固定入口。
- Produces: 用户可运行的 `pnpm clean` 命令和对应文档。

- [ ] **Step 1: 添加 package 脚本**

在 `package.json` 的 `scripts` 对象中紧接 `build` 后加入：

```json
"clean": "node scripts/clean.mjs",
```

最终相关片段为：

```json
"dev": "vite",
"build": "tsc && vite build",
"clean": "node scripts/clean.mjs",
"release": "node scripts/release.mjs",
```

- [ ] **Step 2: 补充 README 本地开发说明**

在构建安装包示例之后、`## 自动发布` 标题之前加入：

~~~~md
清理本地依赖和 Tauri 构建缓存：

```bash
pnpm clean
```

该命令只会删除 `node_modules` 与 `src-tauri/target`。如需恢复 JavaScript 依赖，请随后运行 `pnpm install`。
~~~~

- [ ] **Step 3: 验证 package 脚本引用的入口存在且文档范围正确**

Run: `node -e 'const pkg = require("./package.json"); if (pkg.scripts.clean !== "node scripts/clean.mjs") process.exit(1)' && rg -n "pnpm clean|node_modules|src-tauri/target" README.md scripts/clean.mjs`

Expected: exit 0；输出同时显示 README 与脚本中两个固定清理目标，且不出现 `pnpm-lock.yaml` 或 `Cargo.lock` 作为删除目标。

- [ ] **Step 4: 运行全量验证**

Run: `pnpm test && pnpm build`

Expected: 两条命令均以 exit 0 完成；测试包含 `scripts/clean.test.mjs` 的 3 个通过用例。

- [ ] **Step 5: 检查改动并提交**

Run: `git diff --check && git status --short`

Expected: 无空白错误；仅显示 `package.json`、`README.md`、`scripts/clean.mjs` 与 `scripts/clean.test.mjs` 的本功能改动。

```bash
git add package.json README.md scripts/clean.mjs scripts/clean.test.mjs
git commit -m "功能：新增 pnpm clean 清理命令"
```
