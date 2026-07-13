# 自动发布 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 增加 `pnpm release`、显式版本发布和 `pnpm release --current`，并让 Tag 推送可靠触发三平台构建及 GitHub Release 发布或重发。

**Architecture:** 使用无新增依赖的 Node.js ESM 脚本；纯函数模块负责参数、SemVer 和清单内容，命令模块通过可注入的文件与子进程接口编排 Git、pnpm 和 Cargo。GitHub Actions 继续使用 Tag 驱动的 Tauri 构建矩阵，升级到可复用已有 Release 并覆盖同名资产的 `tauri-action@v1`。

**Tech Stack:** Node.js 22 ESM、Vitest 4、pnpm 10、Git、Cargo、Tauri 2、GitHub Actions

## Global Constraints

- 默认在当前分支修改，不新建分支。
- 所有新增 Git 提交信息使用简体中文。
- `pnpm release` 默认仅递增补丁号。
- `pnpm release 1.2.3` 仅接受高于当前版本的稳定 `x.y.z` SemVer。
- `pnpm release --current` 不修改版本或创建提交，强制移动当前版本 Tag。
- 发布前必须运行 `pnpm test`、`pnpm build` 和 `cargo test --manifest-path src-tauri/Cargo.toml`。
- 不新增 npm 依赖，不依赖 GitHub CLI。
- 禁止在循环遍历中查询 SQL；本实现不访问数据库。

---

## 文件结构

- Create: `scripts/release-core.mjs` — 参数解析、SemVer、版本读取和内容更新。
- Create: `scripts/release.mjs` — 发布命令编排、Git/测试/Cargo 调用和失败恢复。
- Create: `scripts/release.test.mjs` — 核心逻辑、命令编排和工作流契约测试。
- Modify: `package.json` — 注册 `release` 命令。
- Modify: `.github/workflows/release.yml` — Tag 专用触发、并发控制、Tauri Action v1。
- Modify: `README.md` — 使用方法、前置条件和失败恢复。

---

### Task 1: 纯发布核心

**Files:**
- Create: `scripts/release-core.mjs`
- Create: `scripts/release.test.mjs`

**Interfaces:**
- Produces: `parseReleaseArgs(args: string[]): ReleaseRequest`
- Produces: `resolveTargetVersion(request: ReleaseRequest, current: string): string`
- Produces: `getConsistentVersion(contents: ManifestContents): string`
- Produces: `updateVersionContents(contents: ManifestContents, version: string): ManifestContents`
- Consumes: 四个清单文件的 UTF-8 字符串，不执行 I/O。

- [ ] **Step 1: 写参数与 SemVer 的失败测试**

Create `scripts/release.test.mjs`:

```js
import { describe, expect, it } from "vitest";
import {
  parseReleaseArgs,
  resolveTargetVersion,
} from "./release-core.mjs";

describe("发布参数", () => {
  it("无参数时递增补丁号", () => {
    expect(resolveTargetVersion(parseReleaseArgs([]), "0.1.9")).toBe("0.1.10");
  });

  it("接受更高的显式稳定版本", () => {
    expect(resolveTargetVersion(parseReleaseArgs(["1.2.3"]), "0.1.9")).toBe("1.2.3");
  });

  it("current 模式沿用当前版本", () => {
    expect(resolveTargetVersion(parseReleaseArgs(["--current"]), "0.1.9")).toBe("0.1.9");
  });

  it.each(["v1.2.3", "1.2", "1.2.3-beta.1", "01.2.3"])(
    "拒绝非法版本 %s",
    (version) => expect(() => parseReleaseArgs([version])).toThrow("稳定 SemVer"),
  );

  it("拒绝 current 与其他参数组合", () => {
    expect(() => parseReleaseArgs(["--current", "1.2.3"])).toThrow("不能组合");
  });

  it.each(["0.1.9", "0.1.8"])("拒绝相同或更低版本 %s", (version) => {
    expect(() => resolveTargetVersion(parseReleaseArgs([version]), "0.1.9")).toThrow(
      "必须高于",
    );
  });
});
```

- [ ] **Step 2: 运行测试并确认因模块缺失而失败**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: FAIL，错误包含 `Failed to load url ./release-core.mjs`。

- [ ] **Step 3: 实现最小参数与 SemVer 核心**

Create `scripts/release-core.mjs`:

```js
const STABLE_SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

function parseVersion(version) {
  const match = STABLE_SEMVER.exec(version);
  if (!match) {
    throw new Error(`版本 ${version} 不是稳定 SemVer（格式必须为 x.y.z）`);
  }
  return match.slice(1).map(Number);
}

function compareVersions(left, right) {
  const leftParts = parseVersion(left);
  const rightParts = parseVersion(right);
  for (let index = 0; index < leftParts.length; index += 1) {
    if (leftParts[index] !== rightParts[index]) {
      return leftParts[index] - rightParts[index];
    }
  }
  return 0;
}

export function parseReleaseArgs(args) {
  if (args.length === 0) return { mode: "next-patch" };
  if (args.length === 1 && args[0] === "--current") return { mode: "current" };
  if (args.includes("--current")) throw new Error("--current 不能与其他参数组合");
  if (args.length !== 1) throw new Error("用法：pnpm release [x.y.z | --current]");
  parseVersion(args[0]);
  return { mode: "explicit", version: args[0] };
}

export function resolveTargetVersion(request, current) {
  const [major, minor, patch] = parseVersion(current);
  if (request.mode === "current") return current;
  if (request.mode === "next-patch") return `${major}.${minor}.${patch + 1}`;
  if (compareVersions(request.version, current) <= 0) {
    throw new Error(`目标版本 ${request.version} 必须高于当前版本 ${current}`);
  }
  return request.version;
}
```

- [ ] **Step 4: 运行参数测试并确认通过**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: PASS，所有参数测试通过。

- [ ] **Step 5: 写清单一致性与更新的失败测试**

Append to `scripts/release.test.mjs` and add `getConsistentVersion`、`updateVersionContents` to its import:

```js
const manifests = {
  packageJson: '{\n  "name": "dead-center",\n  "version": "0.1.0"\n}\n',
  tauriConfig: '{\n  "productName": "Dead Center",\n  "version": "0.1.0"\n}\n',
  cargoToml: '[package]\nname = "dead-center"\nversion = "0.1.0"\nedition = "2021"\n',
  cargoLock: '[[package]]\nname = "dead-center"\nversion = "0.1.0"\ndependencies = []\n',
};

describe("版本清单", () => {
  it("读取四个一致的版本源", () => {
    expect(getConsistentVersion(manifests)).toBe("0.1.0");
  });

  it("报告不一致的文件和值", () => {
    const inconsistent = {
      ...manifests,
      tauriConfig: manifests.tauriConfig.replace("0.1.0", "0.2.0"),
    };
    expect(() => getConsistentVersion(inconsistent)).toThrow(
      "src-tauri/tauri.conf.json=0.2.0",
    );
  });

  it("只更新三个清单的版本字段", () => {
    const updated = updateVersionContents(manifests, "0.1.1");
    expect(updated.packageJson).toBe(manifests.packageJson.replace("0.1.0", "0.1.1"));
    expect(updated.tauriConfig).toBe(manifests.tauriConfig.replace("0.1.0", "0.1.1"));
    expect(updated.cargoToml).toBe(manifests.cargoToml.replace("0.1.0", "0.1.1"));
    expect(updated.cargoLock).toBe(manifests.cargoLock);
  });
});
```

- [ ] **Step 6: 运行测试并确认缺少清单接口**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: FAIL，错误指向未导出的 `getConsistentVersion` 或 `updateVersionContents`。

- [ ] **Step 7: 实现清单读取与更新**

Append to `scripts/release-core.mjs`:

```js
const VERSION_PATHS = {
  packageJson: "package.json",
  tauriConfig: "src-tauri/tauri.conf.json",
  cargoToml: "src-tauri/Cargo.toml",
  cargoLock: "src-tauri/Cargo.lock",
};

function jsonVersion(content, path) {
  const value = JSON.parse(content).version;
  if (typeof value !== "string") throw new Error(`${path} 缺少字符串 version`);
  parseVersion(value);
  return value;
}

function packageBlock(content, marker) {
  if (marker === "cargoToml") {
    const match = content.match(/\[package\][\s\S]*?(?=\n\[|$)/);
    if (!match) throw new Error("src-tauri/Cargo.toml 缺少 [package]");
    return match[0];
  }
  const blocks = content.match(/\[\[package\]\][\s\S]*?(?=\n\[\[package\]\]|$)/g) ?? [];
  const matches = blocks.filter((block) => /^name\s*=\s*"dead-center"\s*$/m.test(block));
  if (matches.length !== 1) throw new Error("src-tauri/Cargo.lock 必须包含一个 dead-center 包");
  return matches[0];
}

function tomlVersion(content, marker) {
  const block = packageBlock(content, marker);
  const match = block.match(/^version\s*=\s*"([^"]+)"\s*$/m);
  if (!match) throw new Error(`${VERSION_PATHS[marker]} 缺少 package version`);
  parseVersion(match[1]);
  return match[1];
}

export function getConsistentVersion(contents) {
  const versions = {
    packageJson: jsonVersion(contents.packageJson, VERSION_PATHS.packageJson),
    tauriConfig: jsonVersion(contents.tauriConfig, VERSION_PATHS.tauriConfig),
    cargoToml: tomlVersion(contents.cargoToml, "cargoToml"),
    cargoLock: tomlVersion(contents.cargoLock, "cargoLock"),
  };
  const unique = new Set(Object.values(versions));
  if (unique.size !== 1) {
    const detail = Object.entries(versions)
      .map(([key, value]) => `${VERSION_PATHS[key]}=${value}`)
      .join("，");
    throw new Error(`版本不一致：${detail}`);
  }
  return versions.packageJson;
}

function replaceJsonVersion(content, version, path) {
  jsonVersion(content, path);
  return content.replace(/("version"\s*:\s*")[^"]+("\s*[,}])/, `$1${version}$2`);
}

function replaceCargoVersion(content, version) {
  const block = packageBlock(content, "cargoToml");
  const updatedBlock = block.replace(
    /^(version\s*=\s*")[^"]+("\s*)$/m,
    `$1${version}$2`,
  );
  return content.replace(block, updatedBlock);
}

export function updateVersionContents(contents, version) {
  parseVersion(version);
  return {
    packageJson: replaceJsonVersion(contents.packageJson, version, VERSION_PATHS.packageJson),
    tauriConfig: replaceJsonVersion(
      contents.tauriConfig,
      version,
      VERSION_PATHS.tauriConfig,
    ),
    cargoToml: replaceCargoVersion(contents.cargoToml, version),
    cargoLock: contents.cargoLock,
  };
}
```

- [ ] **Step 8: 运行核心测试并提交**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: PASS，所有参数与清单测试通过。

```bash
git add scripts/release-core.mjs scripts/release.test.mjs
git commit -m "测试：覆盖发布版本逻辑"
```

---

### Task 2: 发布命令编排与恢复

**Files:**
- Create: `scripts/release.mjs`
- Modify: `scripts/release.test.mjs`
- Modify: `package.json`

**Interfaces:**
- Consumes: Task 1 的四个核心函数。
- Produces: `runRelease({ args, cwd, execute, fileSystem, output }): { version, mode }`
- Produces: package script `release = node scripts/release.mjs`。

- [ ] **Step 1: 写普通发布与 current 重发的失败测试**

Extend imports in `scripts/release.test.mjs`:

```js
import { runRelease } from "./release.mjs";
```

Append:

```js
function releaseHarness(args) {
  const files = new Map([
    ["/repo/package.json", manifests.packageJson],
    ["/repo/src-tauri/tauri.conf.json", manifests.tauriConfig],
    ["/repo/src-tauri/Cargo.toml", manifests.cargoToml],
    ["/repo/src-tauri/Cargo.lock", manifests.cargoLock],
  ]);
  const calls = [];
  const fileSystem = {
    readFileSync: (path) => files.get(path),
    writeFileSync: (path, value) => files.set(path, value),
  };
  const execute = (command, commandArgs) => {
    calls.push([command, ...commandArgs]);
    const key = [command, ...commandArgs].join(" ");
    if (key === "git status --porcelain") return "";
    if (key === "git symbolic-ref --short HEAD") return "main";
    if (key === "git remote get-url origin") return "git@github.com:owner/dead-center.git";
    if (key === "git rev-list --left-right --count HEAD...origin/main") return "0\t0";
    if (key === "git tag --list v0.1.1") return "";
    if (key === "git ls-remote --tags origin refs/tags/v0.1.1") return "";
    if (command === "cargo" && commandArgs[0] === "metadata") {
      files.set(
        "/repo/src-tauri/Cargo.lock",
        files.get("/repo/src-tauri/Cargo.lock").replace("0.1.0", "0.1.1"),
      );
    }
    return "";
  };
  const result = runRelease({
    args,
    cwd: "/repo",
    execute,
    fileSystem,
    output: { log() {}, error() {} },
  });
  return { calls, files, result };
}

describe("发布编排", () => {
  it("校验、提交并推送新版本和标签", () => {
    const { calls, result } = releaseHarness([]);
    expect(result).toEqual({ mode: "next-patch", version: "0.1.1" });
    expect(calls).toContainEqual(["pnpm", "test"]);
    expect(calls).toContainEqual(["pnpm", "build"]);
    expect(calls).toContainEqual([
      "cargo",
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
    ]);
    expect(calls).toContainEqual(["git", "commit", "-m", "发布：v0.1.1"]);
    expect(calls).toContainEqual(["git", "push", "origin", "main"]);
    expect(calls).toContainEqual(["git", "tag", "-a", "v0.1.1", "-m", "发布 v0.1.1"]);
    expect(calls).toContainEqual(["git", "push", "origin", "refs/tags/v0.1.1"]);
  });

  it("current 不写版本或提交并强推当前标签", () => {
    const { calls, files, result } = releaseHarness(["--current"]);
    expect(result).toEqual({ mode: "current", version: "0.1.0" });
    expect(files.get("/repo/package.json")).toBe(manifests.packageJson);
    expect(calls.some((call) => call.includes("commit"))).toBe(false);
    expect(calls).toContainEqual(["git", "tag", "-f", "-a", "v0.1.0", "-m", "发布 v0.1.0"]);
    expect(calls).toContainEqual([
      "git",
      "push",
      "--force",
      "origin",
      "refs/tags/v0.1.0",
    ]);
  });
});
```

- [ ] **Step 2: 运行测试并确认命令模块缺失**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: FAIL，错误包含 `Failed to load url ./release.mjs`。

- [ ] **Step 3: 实现发布命令**

Create `scripts/release.mjs` with these exact responsibilities and signatures:

```js
import * as nodeFs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  getConsistentVersion,
  parseReleaseArgs,
  resolveTargetVersion,
  updateVersionContents,
} from "./release-core.mjs";

const FILES = {
  packageJson: "package.json",
  tauriConfig: "src-tauri/tauri.conf.json",
  cargoToml: "src-tauri/Cargo.toml",
  cargoLock: "src-tauri/Cargo.lock",
};
const VERSION_FILES = Object.values(FILES);

export function systemExecute(command, args, { cwd, capture = false } = {}) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const detail = capture ? (result.stderr || result.stdout || "").trim() : "";
    throw new Error(`${command} ${args.join(" ")} 执行失败${detail ? `：${detail}` : ""}`);
  }
  return capture ? result.stdout.trim() : "";
}

function readContents(cwd, fileSystem) {
  return Object.fromEntries(
    Object.entries(FILES).map(([key, relativePath]) => [
      key,
      fileSystem.readFileSync(path.join(cwd, relativePath), "utf8"),
    ]),
  );
}

function writeContents(cwd, fileSystem, contents) {
  for (const [key, relativePath] of Object.entries(FILES)) {
    fileSystem.writeFileSync(path.join(cwd, relativePath), contents[key], "utf8");
  }
}

function repositoryPreflight(cwd, execute) {
  const run = (command, args, capture = false) => execute(command, args, { cwd, capture });
  if (run("git", ["status", "--porcelain"], true) !== "") {
    throw new Error("工作区不干净，请先提交或暂存现有修改");
  }
  const branch = run("git", ["symbolic-ref", "--short", "HEAD"], true);
  if (!branch) throw new Error("当前处于 detached HEAD，不能发布");
  run("git", ["remote", "get-url", "origin"], true);
  run("git", ["fetch", "origin", branch, "--tags"]);
  const sync = run(
    "git",
    ["rev-list", "--left-right", "--count", `HEAD...origin/${branch}`],
    true,
  );
  if (!/^0\s+0$/.test(sync)) {
    throw new Error(`当前分支与 origin/${branch} 未完全同步：${sync}`);
  }
  return branch;
}

function ensureNewTag(tag, cwd, execute) {
  const local = execute("git", ["tag", "--list", tag], { cwd, capture: true });
  const remote = execute("git", ["ls-remote", "--tags", "origin", `refs/tags/${tag}`], {
    cwd,
    capture: true,
  });
  if (local || remote) throw new Error(`标签 ${tag} 已存在；重发当前版本请使用 --current`);
}

function runChecks(cwd, execute) {
  execute("pnpm", ["test"], { cwd });
  execute("pnpm", ["build"], { cwd });
  execute("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml"], { cwd });
}

export function runRelease({
  args,
  cwd = process.cwd(),
  execute = systemExecute,
  fileSystem = nodeFs,
  output = console,
}) {
  const request = parseReleaseArgs(args);
  const original = readContents(cwd, fileSystem);
  const current = getConsistentVersion(original);
  const version = resolveTargetVersion(request, current);
  const tag = `v${version}`;
  const branch = repositoryPreflight(cwd, execute);
  if (request.mode !== "current") ensureNewTag(tag, cwd, execute);

  output.log(`准备发布 ${tag}，开始本地校验……`);
  runChecks(cwd, execute);

  if (request.mode === "current") {
    execute("git", ["tag", "-f", "-a", tag, "-m", `发布 ${tag}`], { cwd });
    execute("git", ["push", "--force", "origin", `refs/tags/${tag}`], { cwd });
    output.log(`${tag} 已重新推送，GitHub Actions 将重新构建 Release。`);
    return { mode: request.mode, version };
  }

  const updated = updateVersionContents(original, version);
  writeContents(cwd, fileSystem, updated);
  execute(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--format-version",
      "1",
      "--no-deps",
    ],
    { cwd, capture: true },
  );
  const synchronized = readContents(cwd, fileSystem);
  if (getConsistentVersion(synchronized) !== version) {
    throw new Error("Cargo 锁文件未同步到目标版本");
  }
  execute("git", ["add", "--", ...VERSION_FILES], { cwd });
  execute("git", ["commit", "-m", `发布：${tag}`], { cwd });
  execute("git", ["push", "origin", branch], { cwd });
  execute("git", ["tag", "-a", tag, "-m", `发布 ${tag}`], { cwd });
  execute("git", ["push", "origin", `refs/tags/${tag}`], { cwd });
  output.log(`${tag} 已推送，GitHub Actions 将构建并发布 Release。`);
  return { mode: request.mode, version };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    runRelease({ args: process.argv.slice(2) });
  } catch (error) {
    console.error(`发布失败：${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }
}
```

- [ ] **Step 4: 注册命令并运行编排测试**

Modify `package.json` scripts:

```json
"release": "node scripts/release.mjs"
```

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: PASS，普通发布和 current 编排测试通过。

- [ ] **Step 5: 写提交失败恢复测试**

Change the harness signature to expose state and inject one command failure:

```js
function releaseHarness(args, { failOn, state = {} } = {}) {
  const files = new Map([
    ["/repo/package.json", manifests.packageJson],
    ["/repo/src-tauri/tauri.conf.json", manifests.tauriConfig],
    ["/repo/src-tauri/Cargo.toml", manifests.cargoToml],
    ["/repo/src-tauri/Cargo.lock", manifests.cargoLock],
  ]);
  const calls = [];
  state.files = files;
  state.calls = calls;
  const fileSystem = {
    readFileSync: (path) => files.get(path),
    writeFileSync: (path, value) => files.set(path, value),
  };
  const execute = (command, commandArgs) => {
    calls.push([command, ...commandArgs]);
    const key = [command, ...commandArgs].join(" ");
    if (key === failOn) throw new Error(`模拟失败：${key}`);
    if (key === "git status --porcelain") return "";
    if (key === "git symbolic-ref --short HEAD") return "main";
    if (key === "git remote get-url origin") return "git@github.com:owner/dead-center.git";
    if (key === "git rev-list --left-right --count HEAD...origin/main") return "0\t0";
    if (key === "git tag --list v0.1.1") return "";
    if (key === "git ls-remote --tags origin refs/tags/v0.1.1") return "";
    if (command === "cargo" && commandArgs[0] === "metadata") {
      files.set(
        "/repo/src-tauri/Cargo.lock",
        files.get("/repo/src-tauri/Cargo.lock").replace("0.1.0", "0.1.1"),
      );
    }
    return "";
  };
  const result = runRelease({
    args,
    cwd: "/repo",
    execute,
    fileSystem,
    output: { log() {}, error() {} },
  });
  return { calls, files, result };
}
```

Append the test:

```js
it("提交失败时撤销暂存并恢复四个版本文件", () => {
  const state = {};
  expect(() =>
    releaseHarness([], { failOn: "git commit -m 发布：v0.1.1", state }),
  ).toThrow("模拟失败");
  expect(state.calls).toContainEqual([
    "git",
    "restore",
    "--staged",
    "--",
    "package.json",
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
  ]);
  expect(state.files.get("/repo/package.json")).toBe(manifests.packageJson);
  expect(state.files.get("/repo/src-tauri/tauri.conf.json")).toBe(manifests.tauriConfig);
  expect(state.files.get("/repo/src-tauri/Cargo.toml")).toBe(manifests.cargoToml);
  expect(state.files.get("/repo/src-tauri/Cargo.lock")).toBe(manifests.cargoLock);
});
```

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: FAIL，因为生产代码尚未撤销暂存或恢复文件。

- [ ] **Step 6: 实现提交前恢复**

Replace the manifest update and commit section with:

```js
let wroteFiles = false;
let stagedFiles = false;
let committed = false;
try {
  const updated = updateVersionContents(original, version);
  wroteFiles = true;
  writeContents(cwd, fileSystem, updated);
  execute(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--format-version",
      "1",
      "--no-deps",
    ],
    { cwd, capture: true },
  );
  const synchronized = readContents(cwd, fileSystem);
  if (getConsistentVersion(synchronized) !== version) {
    throw new Error("Cargo 锁文件未同步到目标版本");
  }
  execute("git", ["add", "--", ...VERSION_FILES], { cwd });
  stagedFiles = true;
  execute("git", ["commit", "-m", `发布：${tag}`], { cwd });
  committed = true;
} catch (error) {
  if (!committed) {
    if (stagedFiles) {
      try {
        execute("git", ["restore", "--staged", "--", ...VERSION_FILES], { cwd });
      } catch {
        output.error("无法自动撤销版本文件的暂存，请检查 git status。");
      }
    }
    if (wroteFiles) writeContents(cwd, fileSystem, original);
  }
  throw error;
}
```

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: PASS，提交失败恢复测试通过。

- [ ] **Step 7: 写推送失败恢复提示测试**

Append:

```js
it("分支推送失败时给出 current 恢复命令", () => {
  expect(() =>
    releaseHarness([], { failOn: "git push origin main" }),
  ).toThrow("推送提交后执行 pnpm release --current");
});

it("标签推送失败时给出 current 重试命令", () => {
  expect(() =>
    releaseHarness([], { failOn: "git push origin refs/tags/v0.1.1" }),
  ).toThrow("pnpm release --current");
});
```

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: FAIL，因为子进程原始错误尚未包含恢复命令。

- [ ] **Step 8: 实现推送失败提示**

Replace the three normal-release push/tag calls with:

```js
try {
  execute("git", ["push", "origin", branch], { cwd });
} catch (error) {
  throw new Error(
    `${error instanceof Error ? error.message : String(error)}；请先推送提交后执行 pnpm release --current`,
  );
}
execute("git", ["tag", "-a", tag, "-m", `发布 ${tag}`], { cwd });
try {
  execute("git", ["push", "origin", `refs/tags/${tag}`], { cwd });
} catch (error) {
  throw new Error(
    `${error instanceof Error ? error.message : String(error)}；请执行 pnpm release --current 重试`,
  );
}
```

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: PASS，推送失败提示测试通过。

- [ ] **Step 9: 运行全量前端测试并提交**

Run: `pnpm test`

Expected: PASS，现有 React 测试和新增发布测试全部通过。

```bash
git add package.json scripts/release.mjs scripts/release.test.mjs
git commit -m "功能：新增自动发布命令"
```

---

### Task 3: GitHub Release 重发工作流

**Files:**
- Modify: `scripts/release.test.mjs`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `v*` Tag push 和 GitHub `contents: write` token。
- Produces: Windows x64 NSIS、macOS Universal DMG、Linux x64 AppImage/DEB Release 资产。

- [ ] **Step 1: 写工作流契约失败测试**

Add `readFileSync` import and append:

```js
import { readFileSync } from "node:fs";

describe("发布工作流契约", () => {
  const workflow = readFileSync(
    new URL("../.github/workflows/release.yml", import.meta.url),
    "utf8",
  );

  it("只由版本标签触发并取消同标签旧任务", () => {
    expect(workflow).toContain('- "v*"');
    expect(workflow).not.toContain("workflow_dispatch:");
    expect(workflow).toContain("group: release-${{ github.ref }}");
    expect(workflow).toContain("cancel-in-progress: true");
  });

  it("使用 Tauri Action v1 和当前标签发布", () => {
    expect(workflow).toContain("uses: tauri-apps/tauri-action@v1");
    expect(workflow).toContain("tagName: ${{ github.ref_name }}");
  });

  it.each([
    "--bundles nsis",
    "--target universal-apple-darwin --bundles dmg",
    "--bundles appimage,deb",
  ])("保留平台构建参数 %s", (args) => expect(workflow).toContain(`args: ${args}`));
});
```

- [ ] **Step 2: 运行契约测试并确认失败**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: FAIL，至少报告仍存在 `workflow_dispatch`、缺少 `concurrency`、仍使用 `@v0`。

- [ ] **Step 3: 最小修改发布工作流**

Modify `.github/workflows/release.yml`:

```yaml
on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: true
```

Keep the existing test job and build matrix unchanged. In the Tauri step use:

```yaml
- uses: tauri-apps/tauri-action@v1
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  with:
    tagName: ${{ github.ref_name }}
    releaseName: Dead Center ${{ github.ref_name }}
    releaseBody: |
      跨平台防晕 3D 屏幕准星。

      - Windows：NSIS 安装包
      - macOS：Intel / Apple Silicon 通用 DMG
      - Linux：AppImage 与 DEB
    releaseDraft: false
    prerelease: false
    args: ${{ matrix.args }}
```

- [ ] **Step 4: 运行契约和全量测试并提交**

Run: `pnpm vitest run scripts/release.test.mjs`

Expected: PASS，工作流契约全部通过。

Run: `pnpm test`

Expected: PASS，全部前端与发布测试通过。

```bash
git add .github/workflows/release.yml scripts/release.test.mjs
git commit -m "发布：支持同版本重新构建"
```

---

### Task 4: 发布文档与最终验证

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: Task 2 的三个命令和 Task 3 的 Tag 工作流。
- Produces: 开发者可直接复制执行的发布与恢复说明。

- [ ] **Step 1: 更新 README 自动发布章节**

Replace the start of `## 自动发布` with:

```markdown
## 自动发布

发布命令要求工作区干净、当前分支已与 `origin` 完全同步，并且 Git 凭据具有分支和 Tag 推送权限。命令会先运行前端测试、前端构建和 Rust 测试。

```bash
# 自动递增补丁版本，例如 0.1.0 -> 0.1.1
pnpm release

# 发布指定的更高版本
pnpm release 1.2.3

# 不改版本，重新发布当前版本
pnpm release --current
```

普通发布会同步更新前端、Tauri 和 Rust 版本，创建中文版本提交并推送当前分支，然后推送 `v<版本>` Tag。`--current` 会强制把当前版本 Tag 移到当前提交；受保护 Tag 或 Immutable Releases 禁止强推时，远端会拒绝该操作。

如果分支推送后 Tag 推送失败，修复远端问题后执行 `pnpm release --current`。如果 GitHub Actions 构建失败，提交并推送修复后同样执行 `pnpm release --current`。工作流会复用已有 Release 并覆盖同名安装包。

推送 `v*` 标签会触发 GitHub Actions，测试通过后生成：
```

Keep the existing platform artifact and signing-secret lists below it.

- [ ] **Step 2: 运行格式与完整验证**

Run: `git diff --check`

Expected: 无输出，退出码 0。

Run: `pnpm test`

Expected: PASS，全部测试通过。

Run: `pnpm build`

Expected: PASS，TypeScript 和 Vite 构建成功。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS，全部 Rust 测试通过。

Run: `node scripts/release.mjs --current unexpected`

Expected: 输出 `发布失败：--current 不能与其他参数组合`，退出码 1；Git 状态、提交和 Tag 不变。

- [ ] **Step 3: 检查最终差异并提交**

Run: `git status --short`

Expected: 只有 `README.md` 尚未提交。

Run: `git diff -- README.md`

Expected: 只包含自动发布用法、前置条件和恢复说明。

```bash
git add README.md
git commit -m "文档：补充自动发布用法"
```

- [ ] **Step 4: 完成前复验**

Run: `git status --short --branch`

Expected: 工作区干净；当前分支仍为开始实施时的分支。

Run: `pnpm test`

Expected: PASS。

Run: `pnpm build`

Expected: PASS。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS。

记录三条命令的测试数量和退出状态，再进入完成分支工作流。
