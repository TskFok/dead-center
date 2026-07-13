import { describe, expect, it } from "vitest";
import {
  getConsistentVersion,
  parseReleaseArgs,
  resolveTargetVersion,
  updateVersionContents,
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
});
