# Time Remind

[English](README.md)

Time Remind 是一款轻量级桌面健康提醒工具，帮助长时间使用电脑的人保持饮水、休息和护眼节奏。

![Time Remind 主界面](assets/main-preview.png)

![Time Remind 设置页](assets/setting-preview.png)

## 平台支持状态

| 平台 | 当前状态 | 说明 |
|------|----------|------|
| Windows | 已支持/已验证 | 当前正式发布安装包面向 Windows 10/11。 |
| macOS | 预览构建 | 已提供给社区试用，但托盘、自启动、锁屏和全屏行为仍需实机验证。 |
| Linux | 预览构建 | 已提供给社区试用，但托盘兼容性和桌面环境差异仍需验证。 |

Windows 是当前已验证发布平台。macOS 和 Linux 构建先用于社区试用和问题反馈。

## 主要功能

- 饮水、休息、护眼三类默认提醒。
- 自定义提醒名称、文案、间隔和行动倒计时。
- 通知弹窗支持完成、延后、跳过和超时处理。
- 免打扰时段。
- Windows 演示或游戏真全屏时自动延后提醒。
- 支持平台上的开机自启动和静默启动。
- 系统托盘常驻和快捷操作。
- 今日统计、历史趋势和连续打卡。
- 配置导出和导入。
- 亮色、暗色和跟随系统主题。
- 多语言界面。

## 安装

从 [最新 GitHub Release](https://github.com/EthanHannn/time-remind/releases/latest) 下载安装包。

推荐下载：

- Windows：`Time Remind_0.1.0_x64-setup.exe`
- macOS：`.dmg` 或 `.app` 包
- Linux：`.deb` 或 `.AppImage` 包

当前 Windows 安装包未做代码签名，可能提示未知发布者。macOS 安装包暂未签名和公证，可能出现 Gatekeeper 安全提示。

## 从源码构建

环境要求：

- Node.js 20 或更高版本
- pnpm 9 或更高版本
- Rust stable
- Windows：Visual Studio Build Tools 2022
- macOS：Xcode Command Line Tools
- Linux：WebKitGTK、GTK 3、AppIndicator、librsvg 及发行版对应依赖

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

预期产物：

- Windows：`src-tauri/target/release/bundle/nsis/Time Remind_0.1.0_x64-setup.exe`
- macOS：`src-tauri/target/release/bundle/macos/Time Remind.app`、`src-tauri/target/release/bundle/dmg/Time Remind_0.1.0_*.dmg`
- Linux：`src-tauri/target/release/bundle/deb/*.deb`、`src-tauri/target/release/bundle/appimage/*.AppImage`

## 数据与隐私

- 数据保存在本机。
- 不需要账号。
- 提醒数据不依赖网络服务。
- 提醒、设置、统计日志和导入导出数据都在本地处理。

建议在重装系统或迁移设备前使用设置页的导出功能备份配置。

## 已知限制

- 当前版本为 `0.1.0 Beta`。
- Windows 安装包未签名。
- macOS 签名和公证尚未完成。
- Linux 托盘行为会受桌面环境影响。
- 非 Windows 平台的锁屏检测、全屏检测、托盘行为和开机自启动仍需实机验证。
- 本地音频文件作为提示音、全屏遮罩模式尚未完成。

## 参与反馈

欢迎提交 Issue。若你测试的是 macOS 或 Linux 构建，请附上系统版本、桌面环境、安装包类型和具体现象。

## 变更记录

见 [CHANGELOG.md](CHANGELOG.md)。
