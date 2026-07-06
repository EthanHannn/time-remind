# Time Remind 发布就绪清单

## 当前结论

当前版本可以作为 `0.1.2 Beta` 发给少量可信用户试用。

公开发布前仍建议完成长时间稳定性、显示环境、重复启动和休眠唤醒等实机回归。

## 构建产物

- Release executable: `src-tauri/target/release/time-remind.exe`
- NSIS installer: `src-tauri/target/release/bundle/nsis/Time Remind_0.1.2_x64-setup.exe`

当前已验证发布产物为 Windows NSIS 安装包。macOS/Linux 产物会作为社区预览资产随 Release 发布，用于开源反馈，不视为已验证 stable 支持。

预览构建目标：

- macOS `.app`: `src-tauri/target/release/bundle/macos/Time Remind.app`
- macOS `.dmg`: `src-tauri/target/release/bundle/dmg/Time Remind_0.1.2_*.dmg`
- macOS ARM64 GitHub Actions artifact: `Time-Remind_0.1.2_macOS_aarch64_app.zip`
- macOS ARM64 GitHub Actions DMG diagnostics: `Time-Remind_0.1.2-macos-aarch64-dmg-diagnostics`
- Linux `.deb`: `src-tauri/target/release/bundle/deb/*.deb`
- Linux `.AppImage`: `src-tauri/target/release/bundle/appimage/*.AppImage`

最近一次安装包构建结果：

- 构建命令：`pnpm tauri build`
- 安装包大小：约 `5.01 MB`
- 可执行文件大小：约 `14.10 MB`
- 安装器语言：英文、简体中文、繁体中文、日文、韩文、法文、德文、越南文、泰文、马来文；默认匹配系统语言，无法匹配时回退英文。当前 NSIS 工具包未内置高棉语，安装阶段无法提供高棉语选项。

## 已通过的命令检查

- [x] `pnpm lint`
- [x] `pnpm test`
- [x] `pnpm build`
- [x] `cargo fmt --check`
- [x] `cargo check`
- [x] `cargo test`
- [x] `pnpm tauri build --no-bundle`
- [x] `pnpm tauri build`

## 跨平台预览状态

- [ ] macOS `.app` 构建
- [ ] macOS x64 `.dmg` 构建
- [ ] macOS ARM64 `.dmg` 构建或诊断 artifact
- [ ] macOS ARM64 `.app.zip` 构建
- [ ] Linux `.deb` 构建
- [ ] Linux `.AppImage` 构建
- [ ] macOS 实机主流程验证
- [ ] Linux GNOME Wayland 主流程验证
- [ ] Linux GNOME X11 托盘验证
- [ ] Linux KDE Plasma 托盘验证

当前限制：

- macOS/Linux 发布为社区预览资产，尚未完成实机验证。
- macOS 签名与公证尚未规划完成。
- Linux 托盘行为需按桌面环境分别验证。
- 非 Windows 平台的全屏检测、锁屏检测、自启动和托盘行为必须以实机结果为准。
- 当前公开安装包尚未记录 SHA256 校验值。
- 版本号需保持 `package.json`、`src-tauri/tauri.conf.json`、发布标签和发布说明一致。

## 已完成的实机确认

- [x] NSIS 安装包可生成
- [x] NSIS 安装器已开启语言选择器
- [x] 首次默认提醒会按系统语言创建，无法匹配时回退英文
- [x] 开机自动启动已通过重启验证
- [x] 普通最大化窗口不屏蔽通知
- [x] 重复启动应用时只唤起已有主窗口，不新增托盘图标和调度实例
- [x] 设置项修改后重启应用仍保持
- [x] 完成一次提醒触发、完成、延后、跳过和超时回归
- [x] 完成一次导出和导入回归
- [x] 托盘退出正常
- [x] 导出配置、卸载、重装、导入配置流程已实现并具备回归条件

## 发布前必须回归

- [ ] PPT 或游戏真全屏时自动延后提醒
- [ ] 电脑休眠 10 分钟后唤醒，提醒不会立即连续触发
- [ ] 4K 显示器 + 150% 缩放下主窗口、设置页、统计页和通知窗口布局正常
- [ ] 双显示器环境下通知窗口出现在预期屏幕位置
- [ ] 托盘图标在浅色和深色任务栏上可识别

## 长时间稳定性

发布前建议完成 24 小时内存采样。

```powershell
pwsh -File scripts/measure-memory.ps1 -ProcessName time-remind -DurationHours 24 -IntervalSeconds 60 -OutputPath logs/memory-24h.csv
```

验收重点：

- `WorkingSetMB` 不持续单方向增长
- `PrivateMemoryMB` 不持续单方向增长
- `Handles` 不持续单方向增长
- 不出现 `Status=not_found`

快速自检可先运行：

```powershell
pwsh -File scripts/measure-memory.ps1 -ProcessName time-remind -DurationMinutes 5 -IntervalSeconds 10 -OutputPath logs/memory-smoke.csv
```

## 对外发布注意事项

- 当前安装包未做代码签名，Windows 可能显示未知发布者。
- 当前版本建议标记为 `Beta`。
- 首批用户建议控制在 2 到 5 人，优先覆盖不同 Windows 版本和显示器配置。
- 对外材料必须把 macOS/Linux 描述为社区预览，不得描述为已验证 stable 支持。
- 发包前需要为公开安装包生成 SHA256 校验值。
- 不在同一正式标签下替换已发布安装包；修复阻断问题时发布新的 PATCH 版本。
- 发包时同时提供：
  - 安装包
  - README
  - CHANGELOG
  - SHA256 校验值
  - 已知限制说明

## 推荐发布步骤

1. 完成发布前必须回归清单。
2. 完成 24 小时内存采样。
3. 如有问题，修复后重新执行命令检查和安装包构建。
4. 更新 `CHANGELOG.md` 中的验证状态。
5. 将 `docs/planning/implementation-plan.md` 中对应验证项改为已完成。
6. 确认 `package.json` 与 `src-tauri/tauri.conf.json` 版本号一致。
7. 为安装包生成 SHA256 校验值。
8. 创建版本提交并推送到 GitHub。
9. 创建并推送 `v0.1.2` 形式的标签，触发 GitHub Actions 自动发布构建。
10. 检查 Release 说明是否明确标注 Windows 已验证、macOS/Linux 为社区预览。
11. 分发 `Time Remind_0.1.2_x64-setup.exe` 和对应校验值给试用用户。
