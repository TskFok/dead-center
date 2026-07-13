# Dead Center

Dead Center 是一个轻量的跨平台屏幕准星工具。它在目标显示器正中心提供稳定的视觉锚点，帮助缓解第一人称或第三人称 3D 游戏中的晕动不适。

## 功能

- 圆环与中心点、缺口十字、柔和同心标记，以及细旗空心菱形、内向旗空心菱形、长旗空心菱形、长旗实心菱形，共七种准星；默认使用缺口十字。
- 调节准星颜色、中心标记颜色、透明度、屏幕百分比尺寸、线宽和中心缺口。
- “细旗空心菱形”从目标屏幕四边向中心延伸，0% 保留边缘短旗头，100% 停在中心缺口或空心菱形外缘。
- 透明、无边框、始终置顶、鼠标穿透，不会拦截游戏点击。
- 多显示器手动选择；目标显示器断开时回退主屏，重新连接后自动恢复。
- 系统托盘常驻、全局快捷键显示或隐藏、可选开机启动。
- 配置仅保存在本机，不使用数据库、网络服务、遥测或用户跟踪。

## 平台兼容性

| 平台 | 支持情况 |
| --- | --- |
| Windows 10/11 x64 | 完整支持窗口化和无边框全屏 |
| macOS 12+ Intel/Apple Silicon | 完整支持，并在所有桌面空间显示 |
| Ubuntu 22.04/24.04 X11 | 完整支持 |
| Linux Wayland | 尽力支持，置顶和全局快捷键可能受桌面合成器限制 |

Dead Center 不注入游戏进程，因此不会主动影响反作弊系统。独占全屏可能绕过桌面窗口合成器，建议把游戏切换为“无边框全屏”或“窗口化”。

## 使用

1. 启动应用后，准星默认显示在主屏幕中心。
2. 在设置页选择样式和参数；普通预设的“整体尺寸”按目标屏幕短边百分比计算，“细旗空心菱形”则按百分比控制四臂从屏幕边缘向中心的延伸长度，修改会实时生效并自动保存。
3. 使用 `Alt+Shift+X` 显示或隐藏准星，也可以在设置中改成其他组合键。
4. 关闭设置窗口后应用继续在托盘运行；从托盘菜单可以重新打开或彻底退出。

如果快捷键被其他应用占用，Dead Center 会保留原快捷键并显示错误。配置文件损坏时，应用会把原文件备份为 `settings.corrupt-<时间戳>.json` 并恢复默认值。

## 本地开发

需要 Node.js 22+、pnpm 10+、Rust stable 和 [Tauri 2 的平台依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
pnpm install
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
```

构建当前平台安装包：

```bash
pnpm tauri build
```

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

- Windows x64 NSIS 安装包
- macOS Intel/Apple Silicon 通用 DMG
- Linux x64 AppImage 和 DEB

没有签名凭据时会生成未签名产物，Windows SmartScreen 或 macOS Gatekeeper 可能显示安全提示。仓库配置对应证书密钥后，工作流会启用平台签名。

### 可选签名密钥

- Windows：`WINDOWS_CERTIFICATE`、`WINDOWS_CERTIFICATE_PASSWORD`、`WINDOWS_CERTIFICATE_THUMBPRINT`
- macOS：`APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`

## 隐私

应用不发送任何遥测数据，不读取游戏进程，不包含自动更新或云同步。所有设置都存放在操作系统分配的应用数据目录中。
