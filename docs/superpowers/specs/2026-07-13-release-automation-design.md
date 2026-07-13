# 自动发布设计

## 目标

为 Dead Center 增加一条可重复、可恢复的发布链路：开发者在当前分支运行 `pnpm release` 后，脚本完成本地校验、版本递增、中文发布提交、分支推送和 Git Tag 推送；Tag 推送触发 GitHub Actions，在 Windows、macOS 和 Linux 上构建安装包并发布到 GitHub Release。

命令还必须支持：

- `pnpm release`：将当前稳定版本的补丁号加一，例如 `0.1.0` 变为 `0.1.1`。
- `pnpm release 1.2.3`：发布显式指定的、更高的稳定 SemVer 版本。
- `pnpm release --current`：不修改版本、不创建版本提交，重新发布当前版本。

## 范围

本次实现包含：

- 跨平台 Node.js 发布脚本及其自动化测试。
- `package.json` 中的 `release` 命令。
- 三个版本源及 Rust 锁文件的同步。
- 现有 GitHub Actions 发布工作流的重发能力和并发保护。
- README 中的发布命令说明和失败恢复说明。

本次不包含自动更新客户端、代码签名凭据配置、变更日志生成、预发布版本或应用商店发布。

## 方案选择

采用自包含的 Node.js ESM 脚本，不新增运行时或开发依赖。

未采用 Shell 脚本，因为发布命令应能从 Windows、macOS 和 Linux 开发环境运行，Shell 方言和转义规则会降低可移植性。未采用 Changesets 或 release-it，因为当前仓库只有一个应用，额外依赖和配置无法抵消其维护成本，而且当前版本重发仍需自定义 Git Tag 处理。

## 组件与职责

### `scripts/release-core.mjs`

提供无外部副作用、可单元测试的核心逻辑：

- 解析空参数、显式版本和 `--current`。
- 校验严格的稳定 SemVer，格式仅允许 `x.y.z`，命令行版本不带 `v`。
- 计算下一个补丁版本。
- 比较版本，拒绝显式发布相同或更低版本。
- 读取并校验 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 和 `src-tauri/Cargo.lock` 中的版本。
- 生成前三个版本文件的新内容；Cargo 锁文件由 Cargo 命令同步，不在核心逻辑中手工改写。

### `scripts/release.mjs`

负责命令编排和所有外部副作用：

- 运行 Git、pnpm 和 Cargo 子进程。
- 检查工作区、分支、远端和同步状态。
- 运行完整本地校验。
- 写入版本文件，并调用 Cargo 同步锁文件。
- 创建中文提交、推送分支、创建并推送 Tag。
- 在可安全恢复时回滚未提交的版本文件。

子进程使用参数数组启动，不通过 Shell 拼接用户输入。

### `scripts/release.test.mjs`

使用现有 Vitest 测试核心逻辑和静态工作流契约，不访问网络，也不实际创建提交或 Tag。

### `.github/workflows/release.yml`

保留现有测试任务和三平台构建矩阵，并做以下调整：

- 仅由 `v*` Tag 推送触发发布。
- 使用 `tauri-apps/tauri-action@v1`。
- 以完整 Git 引用作为并发分组；同一个 Tag 的新工作流取消旧工作流。
- 继续使用 `github.ref_name` 查找或创建 Release。
- 当前 Tag 已存在 Release 时复用该 Release；Tauri Action 在上传前删除同名旧资产，从而支持重新发布。

## 普通发布流程

1. 解析命令参数。无参数时计算补丁版本；显式版本必须是严格的稳定 SemVer 且高于当前版本。
2. 要求 Git 工作区干净，当前处于具名分支，并且存在 `origin` 远端。
3. 从 `origin` 获取当前分支和 Tag 信息。
4. 要求本地当前分支与对应远端分支没有领先、落后或分叉。
5. 要求四个版本记录一致；目标版本对应的本地或远端 Tag 均不得存在。
6. 依次运行：
   - `pnpm test`
   - `pnpm build`
   - `cargo test --manifest-path src-tauri/Cargo.toml`
7. 更新 `package.json`、`src-tauri/tauri.conf.json` 和 `src-tauri/Cargo.toml`。
8. 运行 Cargo metadata 命令，使 `src-tauri/Cargo.lock` 与新包版本同步。
9. 再次检查四个版本记录完全一致。
10. 仅暂存四个版本文件，创建中文提交 `发布：v<版本>`。
11. 推送当前分支到 `origin`。
12. 在已推送的提交上创建带说明的 `v<版本>` Tag，说明为 `发布 v<版本>`。
13. 推送 Tag 到 `origin`，由 GitHub Actions 构建并发布安装包。

分支推送和 Tag 推送故意分为两步。这样 GitHub 上的 Tag 不会指向尚未发布的版本提交；如果 Tag 推送失败，已推送的版本提交可通过 `pnpm release --current` 恢复发布。

## 当前版本重发流程

`pnpm release --current` 执行与普通发布相同的工作区、分支、远端同步、版本一致性检查和完整本地测试，但不会修改文件或创建提交。

校验通过后，脚本在当前提交上重新创建带说明的 `v<当前版本>` Tag，并使用强制 Tag 推送更新远端。如果 Tag 尚不存在，则该命令创建它；如果已经存在，则移动它。GitHub 的 Tag `push` 事件触发新工作流，工作流复用现有 Release 并覆盖同名构建资产。

此能力服从仓库安全策略。受保护 Tag 或 GitHub Immutable Releases 禁止强制更新时，脚本报告远端拒绝并退出，不尝试绕过策略。

## 错误处理与恢复

- 参数、Git 状态、远端同步或版本一致性检查失败：退出且不修改文件。
- 前端测试、前端构建或 Rust 测试失败：退出且不修改版本文件。
- 写入版本后、创建提交前发生错误：恢复脚本开始时读取的版本文件内容。
- 提交创建失败：撤销四个版本文件的暂存并恢复其原始内容；保留用户在脚本运行前已有的 Git 状态不变。
- 分支推送失败：保留本地版本提交，不自动重写 Git 历史；提示解决远端问题、推送该提交，再运行 `pnpm release --current`。
- Tag 推送失败：保留已推送的版本提交和本地 Tag；提示运行 `pnpm release --current`。
- GitHub Actions 构建失败：保留 Tag 和 Release 状态；修复构建问题并提交、推送后，运行 `pnpm release --current`。
- 同一 Tag 的旧工作流仍在运行：新工作流取消旧工作流，防止并发覆盖同一组 Release 资产。

脚本以非零状态码表示失败，并在错误信息中说明已经完成到哪一步以及下一条恢复命令。

## 测试策略

### 核心逻辑测试

覆盖以下行为：

- 空参数选择补丁递增。
- `0.1.9` 正确递增为 `0.1.10`。
- `pnpm release 1.2.3` 选择显式版本。
- `--current` 选择当前版本且禁止与其他版本参数组合。
- 非严格 SemVer、带 `v`、预发布版本、相同版本和降级版本被拒绝。
- 三个清单和 Cargo 锁文件版本一致时读取成功。
- 任一版本源不一致时给出包含文件名和版本值的错误。
- 版本内容更新只改变目标版本字段并保持有效 JSON/TOML 结构。

### 工作流契约测试

读取 `.github/workflows/release.yml` 并验证：

- 存在 `v*` Tag push 触发器。
- Windows NSIS、macOS Universal DMG、Linux AppImage 和 DEB 构建仍然存在。
- 使用 `tauri-apps/tauri-action@v1`。
- Release Tag 使用 `github.ref_name`。
- 同 Tag 并发组启用 `cancel-in-progress`。

### 完整验证

实现完成后运行：

```bash
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

另外以只读方式检查 `pnpm release` 的帮助或参数错误路径，确保验证过程不会产生提交、Tag 或网络发布。

## 文档与使用示例

README 将增加以下示例：

```bash
# 自动发布下一个补丁版本
pnpm release

# 发布指定版本
pnpm release 1.2.3

# 重新发布当前版本
pnpm release --current
```

文档明确说明命令会创建提交、推送当前分支和 Tag，要求调用者已经配置可写的 `origin` Git 凭据；不要求安装 GitHub CLI。

## 外部行为依据

- GitHub Actions 的 `push` 事件支持 Tag 推送触发工作流：[GitHub 文档](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)。
- Tauri Action 可以按 Tag 查找已有 Release，并在上传同名资产前删除旧资产：[Tauri Action 文档与源码](https://github.com/tauri-apps/tauri-action)。

## 完成标准

- 三条发布命令的行为符合本文定义。
- 普通发布的所有版本源一致，提交信息为简体中文。
- Tag 推送触发 Windows、macOS 和 Linux 构建并将产物上传到 GitHub Release。
- `--current` 能强制移动同名 Tag，并由工作流复用 Release、覆盖同名资产。
- 所有新增测试及现有前端、Rust 测试通过。
- 工作区不存在与本功能无关的修改。
