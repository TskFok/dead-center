// @vitest-environment node

import { access, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanTargets, getCleanTargets } from "./clean.mjs";

const temporaryRoots = [];

afterEach(async () => {
  await Promise.all(
    temporaryRoots.splice(0).map((root) => rm(root, { recursive: true, force: true })),
  );
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
