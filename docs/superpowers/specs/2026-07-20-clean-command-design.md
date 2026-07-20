# 清理命令设计

## 目标

为 Dead Center 增加 `pnpm clean`，让开发者能够在重新安装依赖或排查本地构建问题前，一次清除 JavaScript 依赖目录和 Tauri 的 Rust 构建产物。

## 范围

本次实现只删除以下两个项目内目录：

- `node_modules`
- `src-tauri/target`

命令不删除 `pnpm-lock.yaml`、`src-tauri/Cargo.lock`、`dist`、源码、配置或任何项目外文件。目录不存在时命令仍应成功完成。

## 方案选择

采用 Node.js ESM 清理脚本，并由 `package.json` 的 `clean` 脚本调用。

未采用内联 `rm -rf`，因为该命令不支持 Windows。未采用 `rimraf`，因为 Node.js 22 已提供满足需求的 `fs.rm`，无须增加依赖。

## 组件与职责

### `package.json`

新增：

```json
"clean": "node scripts/clean.mjs"
```

### `scripts/clean.mjs`

使用 `import.meta.url` 计算仓库根目录，以绝对路径定位两个固定目标。通过 Node 内置的 `fs/promises.rm` 递归删除目录，并设置 `force: true`，使缺失目录不会导致失败。脚本只接受固定目标，不解析用户传入路径。

### `scripts/clean.test.mjs`

从清理脚本导出的纯函数获取目标目录列表，并验证：

- 目标恰为仓库根目录下的 `node_modules` 与 `src-tauri/target`；
- 调用清理函数会删除两个临时目录及其内容；
- 任一目标不存在时清理仍正常完成。

测试仅使用临时目录，不触及仓库实际依赖或构建缓存。

### `README.md`

在本地开发部分增加 `pnpm clean` 示例，说明它会删除 `node_modules` 与 `src-tauri/target`，并提示清理后可运行 `pnpm install` 恢复依赖。

## 错误处理

- 删除权限不足或发生 I/O 错误时，脚本以非零状态退出并保留 Node 原始错误信息。
- 目录不存在不视为错误。
- 因为删除目标由脚本固定生成，命令行参数不能扩大清理范围。

## 验证

实现后执行：

```bash
pnpm test
```

通过临时测试目录验证清理函数，而不在验证过程中对工作区运行实际清理。最后检查 `package.json` 脚本、README 说明与测试均符合本文定义。

## 完成标准

- `pnpm clean` 可跨平台删除固定的两个目录。
- 目录缺失时命令成功。
- 清理范围不包含锁文件、源码或项目外路径。
- 新增测试和现有测试通过，README 已说明使用方式。
