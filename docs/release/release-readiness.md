# Time Remind 发布就绪清单

## 当前结论

当前版本可以作为 `0.1.1 Beta` 发给少量可信用户试用。

公开发布前仍建议完成长时间稳定性、显示环境、重复启动和休眠唤醒等实机回归。

## 构建产物

- Release executable: `src-tauri/target/release/time-remind.exe`
- NSIS installer: `src-tauri/target/release/bundle/nsis/Time Remind_0.1.1_x64-setup.exe`

当前正式发布产物仅包含 Windows NSIS 安装包。macOS/Linux 产物仅作为后续预览验证目标，不纳入当前 `0.1.1 Beta` 对外分发范围。

预览构建目标：

- macOS `.app`: `src-tauri/target/release/bundle/macos/Time Remind.app`
- macOS `.dmg`: `src-tauri/target/release/bundle/dmg/Time Remind_0.1.1_*.dmg`
- macOS ARM64 GitHub Actions preview: `Time-Remind_0.1.1_macOS_aarch64_app.zip`
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
- [ ] macOS ARM64 `.app.zip` 构建
- [ ] Linux `.deb` 构建
- [ ] Linux `.AppImage` 构建
- [ ] macOS 实机主流程验证
- [ ] Linux GNOME Wayland 主流程验证
- [ ] Linux GNOME X11 托盘验证
- [ ] Linux KDE Plasma 托盘验证

当前限制：

- macOS/Linux 暂未发布安装包。
- macOS 签名与公证尚未规划完成。
- Linux 托盘行为需按桌面环境分别验证。
- 非 Windows 平台的全屏检测、锁屏检测、自启动和托盘行为必须以实机结果为准。

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
- 对外材料不得把 macOS/Linux 描述为已正式支持。
- 发包时同时提供：
  - 安装包
  - README
  - CHANGELOG
  - 已知限制说明

## 推荐发布步骤

1. 完成发布前必须回归清单。
2. 完成 24 小时内存采样。
3. 如有问题，修复后重新执行命令检查和安装包构建。
4. 更新 `CHANGELOG.md` 中的验证状态。
5. 将 `docs/planning/implementation-plan.md` 中对应验证项改为已完成。
6. 创建版本提交和标签。
7. 分发 `Time Remind_0.1.1_x64-setup.exe` 给试用用户。
