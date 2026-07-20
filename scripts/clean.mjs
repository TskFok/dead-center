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
