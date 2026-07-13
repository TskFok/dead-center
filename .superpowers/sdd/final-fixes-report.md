# 自动发布最终审查修复报告

## 范围与基线

- 起始提交：`368fd2a34f58910b1dc91146d145f3d11f2e470b`
- 工作方式：在当前 `main` 分支修改，未新建分支。
- 安全边界：未执行真实 `git push`、`git tag` 或发布命令；发布编排全部通过注入的内存文件系统和命令记录器测试。

## 审查清单逐项对应

### Critical：Windows 无法直接启动 pnpm shim

- 根因：`runChecks` 固定执行 `execute("pnpm", args)`；无 shell 的 `spawnSync` 在 Windows/Corepack 环境不能可靠启动 `pnpm.cmd`。
- RED：`pnpm vitest run scripts/release.test.mjs -t "Windows|非 Windows"`，退出状态 1；执行 3 项，2 失败、1 通过。失败显示 Windows 仍调用 `pnpm`，且缺少 `npm_execpath` 时没有报错；非 Windows 基线通过。
- 修复：增加 `resolvePnpmCommand`/`runPnpm`。Windows 使用注入的 `nodePath` 执行注入的 `npmExecPath`，参数保持数组；缺少入口时抛出包含 `npm_execpath` 的中文错误。非 Windows 继续直接执行 `pnpm`。
- GREEN：同一命令退出状态 0；3 通过、27 跳过（当时套件共 30 项）。

### Important：`--current` 测试后可能标记落后提交

- 根因：远端同步检查只存在于耗时检查之前，`runChecks` 后直接创建强制 Tag。
- RED：`pnpm vitest run scripts/release.test.mjs -t "current 在测试后"`，退出状态 1；1 失败、33 跳过。第一次同步为 `0 0`、第二次配置为 `0 1` 时，脚本没有抛错。
- 修复：抽取 `ensureBranchSynchronized`；预检和 `--current` 测试后均执行 `git fetch --no-tags origin refs/heads/<branch>:refs/remotes/origin/<branch>`，随后校验 `HEAD...origin/<branch>` 必须为 `0 0`。第二次校验位于任何 `git tag`/Tag push 之前。
- GREEN：同一命令退出状态 0；1 通过、33 跳过。断言两次 fetch，且复检失败时没有 Tag 创建或强推。

### Important：关键安全顺序没有被测试保护

- 现状：生产实现的顺序本已符合设计；缺口是命令调用和文件写入分属两个记录面，测试无法保护跨边界顺序。
- RED：`pnpm vitest run scripts/release.test.mjs -t "严格保护普通发布"`，退出状态 1；1 失败、34 跳过。事件时间线缺少四次版本文件写入，无法证明检查先于写入。
- 修复：测试 harness 将命令与文件写入记录到统一 `events` 序列，并严格断言：预检 → pnpm test → pnpm build → cargo test → 四个版本文件写入 → cargo metadata → git add → commit → push 分支 → 创建 Tag → push Tag。
- GREEN：同一命令退出状态 0；1 通过、34 跳过。

### Important：危险预检拒绝路径缺少测试

- RED：`pnpm vitest run scripts/release.test.mjs -t "危险预检"`，退出状态 1；执行 5 项，1 失败、4 通过、35 跳过。失败项证明旧 harness 不能注入脏工作区状态；缺失 origin、本地领先、远端领先、分叉用例已捕获现有拒绝行为。
- 修复：扩展 harness 的工作区状态、命令失败和同步计数注入；表驱动覆盖脏工作区、缺失 origin、本地领先 `1 0`、远端领先 `0 1`、分叉 `2 3`。
- GREEN：同一命令退出状态 0；5 通过、35 跳过。每项均断言没有 pnpm/Cargo 检查、没有版本写入、没有 commit、Tag 创建或 Tag push。

### Minor：JSON 错误缺文件路径

- 根因：`jsonVersion` 直接透传 `JSON.parse` 的 SyntaxError。
- RED：`pnpm vitest run scripts/release.test.mjs -t "JSON 损坏"`，退出状态 1；2 失败、30 跳过。两项分别缺少 `package.json` 与 `src-tauri/tauri.conf.json`。
- 修复：在 JSON 清单边界包装解析错误，消息包含文件路径和原始错误文本，并通过 `Error.cause` 保留原始 SyntaxError。
- GREEN：同一命令退出状态 0；2 通过、30 跳过。

### Minor：detached HEAD 专用提示不可达

- 根因：`git symbolic-ref --short HEAD` 在 detached HEAD 下非零退出，通用执行错误先于专用分支判断。
- RED：`pnpm vitest run scripts/release.test.mjs -t "detached HEAD"`，退出状态 1；1 失败、32 跳过。调用仍为 `symbolic-ref`，未使用成功返回空串的查询。
- 修复：改用 `git branch --show-current`；空输出由脚本显式抛出“当前处于 detached HEAD，不能发布”。
- GREEN：同一命令退出状态 0；1 通过、32 跳过。

### Minor：工作流契约字符串断言脆弱

- 现状：实际 `.github/workflows/release.yml` 从起始提交起已满足设计，本项只需强化测试，不改工作流生产配置。
- 修复：测试先去除 YAML 注释，再按缩进和区块边界提取顶层 `on`、`concurrency` 及 `jobs.build`；精确断言触发区块只有 Tag push，并断言 `build` 需要 `test`。保留三平台矩阵、Tauri Action v1、`github.ref_name` 与同 Tag 并发契约。另加入“注释伪装触发器/依赖”的负例。
- Mutation RED：临时给实际工作流加入 `workflow_dispatch` 后运行 `pnpm vitest run scripts/release.test.mjs -t "发布工作流契约"`，退出状态 1；1 失败、6 通过、35 跳过，失败明确指出触发区块多出 dispatch。
- 恢复 GREEN：立即恢复工作流后运行同一命令，退出状态 0；7 通过、35 跳过。最终工作流与起始提交内容一致。

## 完整验证

| 命令 | 数量/关键输出 | 退出状态 |
| --- | --- | --- |
| `pnpm vitest run scripts/release.test.mjs` | 1 个测试文件；42/42 通过 | 0 |
| `pnpm test` | 4 个测试文件；52/52 通过 | 0 |
| `pnpm build` | `tsc && vite build`；39 个模块转换；Vite 构建完成 | 0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | lib 8/8 通过；main 0 失败；doc tests 0 失败 | 0 |
| `git diff --check` | 无输出 | 0 |
| `node --check scripts/release-core.mjs` | 无输出 | 0 |
| `node --check scripts/release.mjs` | 无输出 | 0 |

## 自审结果

- Windows pnpm 解析只生成命令和参数数组；`systemExecute` 继续以 `spawnSync(command, args)` 无 shell 执行，没有拼接或解释用户版本输入。
- `--current` 第二次同步检查发生在耗时检查之后、任何 Tag 命令之前；失败路径没有 Tag 副作用。
- 普通发布安全顺序有统一事件序列保护；危险预检五类拒绝均有零副作用断言。
- JSON 错误同时保留文件上下文与原始异常；detached HEAD 专用中文提示可达。
- 工作流测试使用去注释后的缩进区块，不依赖跨区块全文字符串命中。
- 未修改 SQL，也不存在循环遍历中查询 SQL 的变化。
- 未执行真实 push、Tag 创建或发布。

## 关注点

- 当前环境不是 Windows；Windows/Corepack 行为通过注入 `win32`、Node 路径和 `npm_execpath` 的编排测试验证，最终真实环境覆盖仍由 Windows GitHub Actions/实际发布机提供。

## 预提交校正

- 首次执行 `git diff --cached --check` 时因本报告末尾多余空行退出 2；移除多余空行后重新暂存并复检。
