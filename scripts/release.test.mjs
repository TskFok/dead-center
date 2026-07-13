// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  getConsistentVersion,
  parseReleaseArgs,
  resolveTargetVersion,
  updateVersionContents,
} from "./release-core.mjs";
import { runRelease } from "./release.mjs";

describe("发布参数", () => {
  it("无参数时递增补丁号", () => {
    expect(resolveTargetVersion(parseReleaseArgs([]), "0.1.9")).toBe("0.1.10");
  });

  it("接受更高的显式稳定版本", () => {
    expect(resolveTargetVersion(parseReleaseArgs(["1.2.3"]), "0.1.9")).toBe("1.2.3");
  });

  it("精确比较超出安全整数范围的版本段", () => {
    expect(
      resolveTargetVersion(
        parseReleaseArgs(["1.0.9007199254740993"]),
        "1.0.9007199254740992",
      ),
    ).toBe("1.0.9007199254740993");
  });

  it("精确递增超出安全整数范围的补丁号", () => {
    expect(resolveTargetVersion(parseReleaseArgs([]), "1.0.9007199254740992")).toBe(
      "1.0.9007199254740993",
    );
  });

  it("current 模式沿用当前版本", () => {
    expect(resolveTargetVersion(parseReleaseArgs(["--current"]), "0.1.9")).toBe("0.1.9");
  });

  it.each(["v1.2.3", "1.2", "1.2.3-beta.1", "01.2.3"])(
    "拒绝非法版本 %s",
    (version) => expect(() => parseReleaseArgs([version])).toThrow("稳定 SemVer"),
  );

  it("拒绝 current 与其他参数组合", () => {
    expect(() => parseReleaseArgs(["--current", "1.2.3"])).toThrow(
      "不能与其他参数组合",
    );
  });

  it.each(["0.1.9", "0.1.8"])("拒绝相同或更低版本 %s", (version) => {
    expect(() => resolveTargetVersion(parseReleaseArgs([version]), "0.1.9")).toThrow(
      "必须高于",
    );
  });
});

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

  it.each([
    ["packageJson", "package.json"],
    ["tauriConfig", "src-tauri/tauri.conf.json"],
  ])("%s JSON 损坏时报告文件路径并保留原始原因", (key, filePath) => {
    const invalid = { ...manifests, [key]: "{" };

    try {
      getConsistentVersion(invalid);
      throw new Error("预期 JSON 解析失败");
    } catch (error) {
      expect(error.message).toContain(filePath);
      expect(error.cause).toBeInstanceOf(SyntaxError);
      expect(error.message).toContain(error.cause.message);
    }
  });

  it("只更新三个清单的版本字段", () => {
    const updated = updateVersionContents(manifests, "0.1.1");
    expect(updated.packageJson).toBe(manifests.packageJson.replace("0.1.0", "0.1.1"));
    expect(updated.tauriConfig).toBe(manifests.tauriConfig.replace("0.1.0", "0.1.1"));
    expect(updated.cargoToml).toBe(manifests.cargoToml.replace("0.1.0", "0.1.1"));
    expect(updated.cargoLock).toBe(manifests.cargoLock);
  });

  it("只更新顶层 JSON 版本字段", () => {
    const nestedVersionFirst = {
      ...manifests,
      packageJson:
        '{\n  "metadata": {\n    "version": "9.9.9"\n  },\n  "version": "0.1.0"\n}\n',
    };

    expect(updateVersionContents(nestedVersionFirst, "0.1.1").packageJson).toBe(
      '{\n  "metadata": {\n    "version": "9.9.9"\n  },\n  "version": "0.1.1"\n}\n',
    );
  });
});

function releaseHarness(
  args,
  {
    branch = "main",
    failOn,
    runtime,
    state = {},
    status = "",
    syncResults = ["0\t0"],
  } = {},
) {
  const files = new Map([
    ["/repo/package.json", manifests.packageJson],
    ["/repo/src-tauri/tauri.conf.json", manifests.tauriConfig],
    ["/repo/src-tauri/Cargo.toml", manifests.cargoToml],
    ["/repo/src-tauri/Cargo.lock", manifests.cargoLock],
  ]);
  const calls = [];
  const events = [];
  let syncResultIndex = 0;
  state.files = files;
  state.calls = calls;
  state.events = events;
  const fileSystem = {
    readFileSync: (path) => files.get(path),
    writeFileSync: (path, value) => {
      events.push(["write", path]);
      files.set(path, value);
    },
  };
  const execute = (command, commandArgs) => {
    calls.push([command, ...commandArgs]);
    events.push(["command", command, ...commandArgs]);
    const key = [command, ...commandArgs].join(" ");
    if (key === failOn) throw new Error(`模拟失败：${key}`);
    if (key === "git status --porcelain") return status;
    if (key === "git symbolic-ref --short HEAD") return branch;
    if (key === "git branch --show-current") return branch;
    if (key === "git remote get-url origin") return "git@github.com:owner/dead-center.git";
    if (key === `git rev-list --left-right --count HEAD...origin/${branch}`) {
      const result = syncResults[Math.min(syncResultIndex, syncResults.length - 1)];
      syncResultIndex += 1;
      return result;
    }
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
    runtime,
  });
  return { calls, events, files, result };
}

describe("发布编排", () => {
  it("通过成功返回空分支名的查询给出 detached HEAD 专用提示", () => {
    const state = {};

    expect(() => releaseHarness(["--current"], { branch: "", state })).toThrow(
      "detached HEAD",
    );
    expect(state.calls).toContainEqual(["git", "branch", "--show-current"]);
    expect(state.calls).not.toContainEqual(["git", "symbolic-ref", "--short", "HEAD"]);
  });

  it.each([
    ["脏工作区", "工作区不干净", { status: " M package.json" }],
    [
      "缺失 origin",
      "模拟失败：git remote get-url origin",
      { failOn: "git remote get-url origin" },
    ],
    ["本地领先", "未完全同步", { syncResults: ["1\t0"] }],
    ["远端领先", "未完全同步", { syncResults: ["0\t1"] }],
    ["分叉", "未完全同步", { syncResults: ["2\t3"] }],
  ])("危险预检拒绝%s且不产生发布副作用", (_scenario, message, options) => {
    const state = {};

    expect(() => releaseHarness([], { ...options, state })).toThrow(message);
    expect(
      state.calls.some(([command]) => command === "pnpm" || command === "cargo"),
    ).toBe(false);
    expect(state.events.some(([type]) => type === "write")).toBe(false);
    expect(
      state.calls.some(
        ([command, subcommand, ...commandArgs]) =>
          command === "git" &&
          (["commit", "tag"].includes(subcommand) ||
            (subcommand === "push" &&
              commandArgs.some((argument) => argument.startsWith("refs/tags/")))),
      ),
    ).toBe(false);
  });

  it("Windows 通过 Node 和 npm_execpath 启动 pnpm 检查", () => {
    const runtime = {
      platform: "win32",
      nodePath: "C:\\Program Files\\nodejs\\node.exe",
      npmExecPath: "C:\\corepack\\pnpm.cjs",
    };
    const { calls } = releaseHarness([], { runtime });

    expect(calls).toContainEqual([
      runtime.nodePath,
      runtime.npmExecPath,
      "test",
    ]);
    expect(calls).toContainEqual([
      runtime.nodePath,
      runtime.npmExecPath,
      "build",
    ]);
    expect(calls.some(([command]) => command === "pnpm")).toBe(false);
  });

  it("Windows 缺少 npm_execpath 时给出明确错误", () => {
    expect(() =>
      releaseHarness([], {
        runtime: {
          platform: "win32",
          nodePath: "C:\\Program Files\\nodejs\\node.exe",
          npmExecPath: "",
        },
      }),
    ).toThrow("npm_execpath");
  });

  it("非 Windows 保留直接 pnpm 参数数组调用", () => {
    const { calls } = releaseHarness([], {
      runtime: {
        platform: "linux",
        nodePath: "/usr/bin/node",
        npmExecPath: "/opt/pnpm.cjs",
      },
    });

    expect(calls).toContainEqual(["pnpm", "test"]);
    expect(calls).toContainEqual(["pnpm", "build"]);
  });

  it("预检只更新当前远端分支且不抓取标签", () => {
    const { calls } = releaseHarness(["--current"]);
    const fetchCalls = calls.filter(
      ([command, subcommand]) => command === "git" && subcommand === "fetch",
    );
    expect(fetchCalls).toHaveLength(2);
    for (const fetchCall of fetchCalls) {
      expect(fetchCall).toEqual([
        "git",
        "fetch",
        "--no-tags",
        "origin",
        "refs/heads/main:refs/remotes/origin/main",
      ]);
      expect(fetchCall).not.toContain("--tags");
    }
  });

  it("current 在测试后发现远端前进时不执行任何标签命令", () => {
    const state = {};

    expect(() =>
      releaseHarness(["--current"], {
        state,
        syncResults: ["0\t0", "0\t1"],
      }),
    ).toThrow("未完全同步");
    expect(
      state.calls.filter(
        ([command, subcommand]) => command === "git" && subcommand === "fetch",
      ),
    ).toHaveLength(2);
    expect(
      state.calls.some(
        ([command, subcommand]) => command === "git" && subcommand === "tag",
      ),
    ).toBe(false);
    expect(
      state.calls.some(
        ([command, subcommand, option]) =>
          command === "git" && subcommand === "push" && option === "--force",
      ),
    ).toBe(false);
  });

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

  it("严格保护普通发布的安全执行顺序", () => {
    const { events } = releaseHarness([]);
    const relevantEvents = events.filter(
      ([type, command, subcommand, option]) =>
        type === "write" ||
        (command === "git" &&
          (["status", "rev-list", "add", "commit", "push"].includes(subcommand) ||
            (subcommand === "tag" && option === "-a"))) ||
        command === "pnpm" ||
        command === "cargo",
    );

    expect(relevantEvents).toEqual([
      ["command", "git", "status", "--porcelain"],
      [
        "command",
        "git",
        "rev-list",
        "--left-right",
        "--count",
        "HEAD...origin/main",
      ],
      ["command", "pnpm", "test"],
      ["command", "pnpm", "build"],
      [
        "command",
        "cargo",
        "test",
        "--manifest-path",
        "src-tauri/Cargo.toml",
      ],
      ["write", "/repo/package.json"],
      ["write", "/repo/src-tauri/tauri.conf.json"],
      ["write", "/repo/src-tauri/Cargo.toml"],
      ["write", "/repo/src-tauri/Cargo.lock"],
      [
        "command",
        "cargo",
        "metadata",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--format-version",
        "1",
        "--no-deps",
      ],
      [
        "command",
        "git",
        "add",
        "--",
        "package.json",
        "src-tauri/tauri.conf.json",
        "src-tauri/Cargo.toml",
        "src-tauri/Cargo.lock",
      ],
      ["command", "git", "commit", "-m", "发布：v0.1.1"],
      ["command", "git", "push", "origin", "main"],
      ["command", "git", "tag", "-a", "v0.1.1", "-m", "发布 v0.1.1"],
      ["command", "git", "push", "origin", "refs/tags/v0.1.1"],
    ]);
  });

  it("current 不写版本或提交并强推当前标签", () => {
    const { calls, files, result } = releaseHarness(["--current"]);
    expect(result).toEqual({ mode: "current", version: "0.1.0" });
    expect(files.get("/repo/package.json")).toBe(manifests.packageJson);
    expect(calls.some((call) => call.includes("commit"))).toBe(false);
    expect(calls).toContainEqual([
      "git",
      "tag",
      "-f",
      "-a",
      "v0.1.0",
      "-m",
      "发布 v0.1.0",
    ]);
    expect(calls).toContainEqual([
      "git",
      "push",
      "--force",
      "origin",
      "refs/tags/v0.1.0",
    ]);
  });

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
});

function stripYamlComments(source) {
  return source
    .split("\n")
    .filter((line) => !/^\s*#/.test(line))
    .map((line) => line.replace(/\s+#.*$/, ""))
    .join("\n");
}

function yamlBlock(source, key, indentation = 0) {
  const lines = source.split("\n");
  const header = `${" ".repeat(indentation)}${key}:`;
  const start = lines.findIndex((line) => line.trimEnd() === header);
  if (start === -1) return "";

  let end = start + 1;
  while (end < lines.length) {
    const line = lines[end];
    if (line.trim() === "") {
      end += 1;
      continue;
    }
    const currentIndentation = line.match(/^ */)[0].length;
    if (currentIndentation <= indentation) break;
    end += 1;
  }
  return lines.slice(start, end).join("\n").trimEnd();
}

describe("发布工作流契约", () => {
  const workflow = stripYamlComments(
    readFileSync(new URL("../.github/workflows/release.yml", import.meta.url), "utf8"),
  );
  const triggerBlock = yamlBlock(workflow, "on");
  const concurrencyBlock = yamlBlock(workflow, "concurrency");
  const buildBlock = yamlBlock(workflow, "build", 2);

  it("只由版本标签触发并取消同标签旧任务", () => {
    expect(triggerBlock).toBe('on:\n  push:\n    tags:\n      - "v*"');
    expect(concurrencyBlock).toMatch(/^  group: release-\$\{\{ github\.ref \}\}$/m);
    expect(concurrencyBlock).toMatch(/^  cancel-in-progress: true$/m);
  });

  it("注释不能伪装触发器或构建依赖", () => {
    const misleading = stripYamlComments(`on:
  workflow_dispatch:
  # push:
  #   tags:
  #     - "v*"
jobs:
  build:
    # needs: test
    runs-on: ubuntu-latest
`);

    expect(yamlBlock(misleading, "on")).not.toBe(
      'on:\n  push:\n    tags:\n      - "v*"',
    );
    expect(yamlBlock(misleading, "build", 2)).not.toMatch(/^    needs: test$/m);
  });

  it("构建任务必须依赖测试任务", () => {
    expect(buildBlock).toMatch(/^    needs: test$/m);
  });

  it("使用 Tauri Action v1 和当前标签发布", () => {
    expect(buildBlock).toMatch(/^      - uses: tauri-apps\/tauri-action@v1$/m);
    expect(buildBlock).toMatch(/^          tagName: \$\{\{ github\.ref_name \}\}$/m);
  });

  it.each([
    "--bundles nsis",
    "--target universal-apple-darwin --bundles dmg",
    "--bundles appimage,deb",
  ])("保留平台构建参数 %s", (args) =>
    expect(buildBlock).toMatch(new RegExp(`^ {12}args: ${args.replace(",", "\\,")}$`, "m")),
  );
});
