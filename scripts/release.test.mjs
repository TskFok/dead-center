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

describe("发布编排", () => {
  it("预检只更新当前远端分支且不抓取标签", () => {
    const { calls } = releaseHarness(["--current"]);
    const fetchCall = calls.find(
      ([command, subcommand]) => command === "git" && subcommand === "fetch",
    );
    expect(fetchCall).toEqual([
      "git",
      "fetch",
      "--no-tags",
      "origin",
      "refs/heads/main:refs/remotes/origin/main",
    ]);
    expect(fetchCall).not.toContain("--tags");
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
