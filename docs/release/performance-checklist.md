# Time Remind 压测与发布检查

## 图标检查

1. 运行 `pwsh -File scripts/generate-app-icon.ps1`
2. 运行 `pnpm tauri icon src-tauri/icons/icon-source.png`
3. 检查以下产物是否已更新：
   - `src-tauri/icons/icon.ico`
   - `src-tauri/icons/icon.icns`
   - `src-tauri/icons/32x32.png`
   - `src-tauri/icons/128x128.png`

## 内存压测

1. 启动应用
   - 开发环境：`pnpm tauri dev`
   - 安装包环境：启动已安装应用
2. 在仓库根目录运行：

```powershell
pwsh -File scripts/measure-memory.ps1 -ProcessName time-remind -DurationHours 24 -IntervalSeconds 60 -OutputPath logs/memory-24h.csv
```

快速自检可运行：

```powershell
pwsh -File scripts/measure-memory.ps1 -ProcessName time-remind -DurationMinutes 5 -IntervalSeconds 10 -OutputPath logs/memory-smoke.csv
```

命令链路烟测可运行：

```powershell
pwsh -File scripts/measure-memory.ps1 -ProcessName pwsh -DurationSeconds 20 -IntervalSeconds 5 -OutputPath logs/memory-script-smoke.csv
```

3. 检查输出：
   - 采样文件：`logs/memory-24h.csv`
   - 关注字段：`WorkingSetMB`、`PrivateMemoryMB`、`Handles`
4. 验收建议：
   - 首尾内存变化不应持续单方向增长
   - 空闲状态 `WorkingSetMB` 应稳定在目标范围内
   - `Status=not_found` 说明进程中途退出，需要结合崩溃日志排查

## 发布门禁

提交发布分支前运行：

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
```

验收建议：

1. 所有命令必须通过。
2. `pnpm build` 如出现 UnoCSS 多入口导入提示，先确认是否为已知提示，不应伴随构建失败。
3. `cargo fmt --check` 失败时先运行 `cargo fmt`，再重新执行检查。
4. `pnpm tauri build --no-bundle` 用于验证 release 可执行文件构建，不依赖 NSIS 下载。

## 安装包构建

Windows 正式发布构建运行：

```powershell
pnpm tauri build --bundles nsis
```

当前环境说明：

1. release 可执行文件应生成在 `src-tauri/target/release/time-remind.exe`。
2. NSIS 安装包应生成在 `src-tauri/target/release/bundle/nsis/Time Remind_0.1.2_x64-setup.exe`。
3. NSIS 安装包依赖本机已有 `makensis` 或 Tauri 自动下载 NSIS 工具包。
4. 如果出现 `nsis-3.11.zip` 下载超时，需要先安装 NSIS 或在网络稳定环境中重新构建。
5. 安装器应显示语言选择器，并内置英文、简体中文、繁体中文、日文、韩文、法文、德文、越南文、泰文、马来文。
6. 安装器默认匹配系统语言；系统语言不在内置列表中时回退英文。当前 NSIS 工具包未内置高棉语，安装阶段无法提供高棉语选项。
7. 当前已验证安装包可正常生成、安装和卸载；后续发布仍需按版本重新执行核对。

## 跨平台预览构建

当前正式发布仍只面向 Windows。macOS/Linux 构建仅用于预览验证，不作为对外稳定版本发布。

版本与产物命名要求：

1. `package.json`、`src-tauri/tauri.conf.json`、Git 标签和发布说明中的版本号必须一致。
2. 正式标签使用 `vMAJOR.MINOR.PATCH`，预发布可使用 `vMAJOR.MINOR.PATCH-beta.N` 或 `vMAJOR.MINOR.PATCH-rc.N`。
3. 对外产物命名应包含产品名、版本号、平台、架构和包类型，例如：
   - `Time-Remind_0.1.2_windows_x64_nsis.exe`
   - `Time-Remind_0.1.2_macos_aarch64_app.zip`
   - `Time-Remind_0.1.2_linux_x64_appimage.AppImage`
4. 当前 Tauri 本地产物仍可能使用默认命名；上传 release asset 或 workflow artifact 时应使用规范化名称。
5. 不使用 `latest` 作为安装包文件名，最新版本由发布页和标签表达。

版本号一致性检查可运行：

```powershell
$packageVersion = (Get-Content package.json -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
if ($packageVersion -ne $tauriVersion) {
  throw "Version mismatch: package.json=$packageVersion, tauri.conf.json=$tauriVersion"
}
"Version OK: $packageVersion"
```

发布触发方式：

1. 推荐方式：创建并推送 `v*` 标签，例如 `v0.1.2`，GitHub Actions 会自动运行发布构建。
2. 手动方式：在 GitHub Actions 中运行 `Release build`，输入不带 `v` 的版本号。
3. Tag 发布默认创建 prerelease，用于明确 macOS/Linux 仍为社区预览资产。

发布渠道约束：

| 渠道 | 允许产物 | 要求 |
|---|---|---|
| stable | Windows NSIS | 完成发布前必须回归、基础安装卸载和校验值记录 |
| beta | Windows NSIS，必要时附预览 artifact 说明 | 必须标注已知限制，不得暗示 macOS/Linux 正式支持 |
| preview | macOS/Linux release asset 与 workflow artifact | 用于开源反馈，必须明确标注未验证限制 |

校验与签名要求：

1. 每个公开安装包都应生成 SHA256，并写入发布说明或 `SHA256SUMS.txt`。
2. Windows 当前未签名时，发布说明必须提示未知发布者风险。
3. macOS 未签名/未公证产物只允许作为社区预览资产，发布说明必须提示 Gatekeeper 风险。
4. Linux `.deb` 和 `.AppImage` 进入公开下载前必须记录适用发行版、桌面环境和 SHA256。

Windows 安装包 SHA256 可运行：

```powershell
Get-FileHash -Algorithm SHA256 "src-tauri/target/release/bundle/nsis/Time Remind_0.1.2_x64-setup.exe"
```

macOS 构建命令：

```powershell
pnpm tauri build --bundles app,dmg
```

macOS ARM64 在 GitHub 托管 runner 上优先构建 `.app` 并压缩为 `.zip` 作为验证 artifact，同时独立运行 DMG 诊断构建；DMG 成功时作为 workflow artifact 保留，失败时保留诊断 artifact。

macOS 预期产物：

- `src-tauri/target/release/bundle/macos/Time Remind.app`
- `src-tauri/target/release/bundle/dmg/Time Remind_0.1.2_*.dmg`
- GitHub Actions ARM64 验证 artifact：`Time-Remind_0.1.2_macOS_aarch64_app.zip`
- GitHub Actions ARM64 DMG 诊断产物：`Time-Remind_0.1.2-macos-aarch64-dmg-diagnostics`

Linux 构建命令：

```powershell
pnpm tauri build --bundles deb,appimage
```

Linux 预期产物：

- `src-tauri/target/release/bundle/deb/*.deb`
- `src-tauri/target/release/bundle/appimage/*.AppImage`

跨平台构建前置要求：

1. macOS 需要 Xcode Command Line Tools。
2. Linux 需要 WebKitGTK、GTK 3、AppIndicator 或发行版对应依赖。
3. macOS 未签名/未公证包可能触发 Gatekeeper 安全提示。
4. Linux 托盘依赖桌面环境和 AppIndicator 支持，GNOME Wayland 下需要单独验证。
5. 非 Windows 平台的全屏检测、锁屏检测、自启动和托盘行为必须以实机结果为准。

阻断发布条件：

- 安装后无法启动主窗口或通知窗口。
- 覆盖安装导致提醒、设置、统计或导入导出数据丢失。
- 卸载后残留运行进程或错误的自启动项。
- Windows NSIS 安装器语言、路径或权限流程异常。
- 发布说明的支持平台、签名状态或校验值与实际产物不一致。
- macOS/Linux 预览资产未明确标注未验证限制。

## 安全收口

1. Tauri CSP 不应为空，至少保留：
   - `default-src 'self'`
   - `connect-src ipc: http://ipc.localhost`
   - `script-src 'self'`
   - `object-src 'none'`
2. 导入导出文件命令仅允许 `.json`，并限制单文件大小。
3. 发布前手工验证导入导出流程，确认 CSP 未阻断本地图片、样式和 IPC。

## 发布前手工核对

1. 托盘图标在浅色和深色任务栏上可识别
2. 安装器和便携版显示新图标
3. 通知窗口、主窗口、任务栏图标一致
4. 完成一次导出、导入、提醒触发和托盘操作回归
5. 设置页开启开机自启后，重启 Windows 并确认应用自动运行
6. 应用运行时再次启动程序，只唤起已有主窗口，不新增托盘图标和提醒调度

## macOS 预览验证

- [ ] `.app` 可启动并显示主窗口。
- [ ] x64 `.dmg` 可挂载、拖拽安装并启动。
- [ ] ARM64 `.dmg` 可挂载、拖拽安装并启动。
- [ ] ARM64 `.app.zip` 解压后可启动。
- [ ] 主提醒流程可用：触发、完成、延后、跳过。
- [ ] 设置页中不支持的平台能力处于禁用状态并显示限制说明。
- [ ] 托盘菜单、显示窗口、隐藏窗口和退出行为符合预期。
- [ ] 开机自启动与 `--autostart` 参数完成实机确认。
- [ ] 未签名或已签名包的安全提示符合预期。

## Linux 预览验证

- [ ] `.deb` 或 `.AppImage` 可安装/启动。
- [ ] GNOME Wayland 下主提醒流程可用。
- [ ] GNOME X11 下托盘行为可用。
- [ ] KDE Plasma 下托盘行为可用。
- [ ] 设置页中不支持的平台能力处于禁用状态并显示限制说明。
- [ ] `.desktop` 自启动入口与 `--autostart` 参数完成实机确认。
- [ ] 缺失 AppIndicator 或托盘支持时有可接受降级。
