# Time Remind

Time Remind 是一款 Windows 桌面健康提醒工具，用于帮助长时间使用电脑的人保持饮水、休息和护眼节奏。

它会在后台常驻，通过非侵入式通知提醒你完成简单行动，例如喝水、站起来活动、看远处放松眼睛。

## 平台支持状态

| 平台 | 当前状态 | 说明 |
|------|----------|------|
| Windows | 已支持/已验证 | 当前发布安装包面向 Windows 10/11。 |
| macOS | 计划支持，尚未验证 | 暂未发布安装包，系统托盘、自启动、锁屏和全屏检测仍需实机验证。 |
| Linux | 计划支持，尚未验证 | 暂未发布安装包，托盘兼容性、自启动、锁屏和全屏检测仍需按桌面环境验证。 |

## 主要功能

- 饮水、休息、护眼三类默认提醒
- 自定义提醒名称、文案、间隔和行动倒计时
- 每个提醒可单独启用或暂停
- 通知弹窗支持完成、延后和跳过
- 免打扰时段
- 演示或游戏真全屏时自动延后提醒（Windows 已接入）
- 开机自动启动（Windows 已验证）
- 系统托盘常驻和快捷操作（Windows 已验证）
- 今日统计、历史趋势和连续打卡
- 配置导出和导入
- 亮色、暗色和跟随系统主题
- 多语言界面

## 系统要求

当前正式验证平台：

- Windows 10 1809 或更高版本
- Windows 11
- WebView2 Runtime

多数 Windows 10/11 设备已内置 WebView2。如果无法启动应用，请先安装 Microsoft Edge WebView2 Runtime。

macOS 和 Linux 版本仍在计划支持阶段，当前没有可发布安装包。

## 安装

当前仅提供 Windows 安装包：

1. 下载安装包 `Time Remind_0.1.0_x64-setup.exe`。
2. 双击安装包并按提示完成安装。
3. 启动 Time Remind。
4. 根据需要在设置页开启开机自动启动。

当前安装包未做代码签名。Windows 可能提示未知发布者，这是未签名桌面应用的常见提示。

## 基本使用

1. 启动后主窗口会显示默认提醒。
2. 使用卡片开关启用或暂停单个提醒。
3. 点击添加提醒可创建自定义提醒。
4. 收到提醒时可选择完成、延后或跳过。
5. 关闭主窗口不会退出应用，应用会继续在系统托盘运行。
6. 如需完全退出，请在托盘菜单中选择退出。

## 数据与隐私

- 数据保存在本机。
- 不需要账号。
- 不依赖网络服务。
- 提醒、设置、统计日志和导入导出数据都在本地处理。

建议在重装系统或迁移设备前使用设置页的导出功能备份配置。

## 已知限制

- 当前版本为 `0.1.0 Beta`，适合小范围试用。
- 安装包未签名，可能触发 Windows 安全提示。
- macOS/Linux 暂未发布安装包，平台能力仍未完成实机验证。
- 非 Windows 平台的锁屏检测、全屏检测、托盘行为和开机自启动尚未承诺可用。
- 24 小时长时间内存压测仍需在目标设备上完成。
- 4K 高缩放、双显示器、休眠唤醒等场景仍建议发布前做实机回归。
- 本地音频文件作为提示音、全屏遮罩模式尚未完成。

## 开发与构建

开发环境要求：

- Node.js 20 或更高版本
- pnpm 9 或更高版本
- Rust stable
- Windows 构建：Visual Studio Build Tools 2022
- macOS 构建：Xcode Command Line Tools
- Linux 构建：WebKitGTK、GTK、AppIndicator 或发行版对应依赖

常用命令：

```powershell
pnpm install
pnpm lint
pnpm test
pnpm build
pnpm tauri dev
```

平台打包命令：

```powershell
# Windows
pnpm tauri build --bundles nsis

# macOS
pnpm tauri build --bundles app,dmg

# Linux
pnpm tauri build --bundles deb,appimage
```

macOS/Linux 打包目标仅用于后续预览验证，当前正式发布仍以 Windows NSIS 安装包为准。

发布前建议执行：

```powershell
pnpm lint
pnpm test
pnpm build
Push-Location src-tauri
cargo fmt --check
cargo check
cargo test
Pop-Location
pnpm tauri build --no-bundle
pnpm tauri build --bundles nsis
```

## 版本

当前版本：`0.1.0 Beta`

变更记录见 [CHANGELOG.md](CHANGELOG.md)。
