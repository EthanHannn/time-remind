# Changelog

## 0.1.2 Beta - 2026-07-06

### Changed
- Added tag-triggered GitHub release builds for Windows, macOS, and Linux assets.
- Published macOS and Linux packages as community preview assets for open-source feedback.
- Clarified release documentation for version tags, checksums, and preview platform limits.

## 0.1.1 Beta - 2026-07-03

### Added

- Added explicit platform support status: Windows is the current verified release platform, while macOS and Linux are preview builds that need real-device validation.
- Added cross-platform bundle targets for macOS `.app`/`.dmg` and Linux `.deb`/`.AppImage` preview builds.
- Added platform capability detection through `get_platform_capabilities`.
- Added frontend platform capability API and typed capability model.
- Added settings-page gating for unsupported platform features.
- Added macOS and Linux preview build output notes and verification checklists.

### Changed

- Updated Windows release build guidance to use the explicit NSIS bundle target.
- Updated settings behavior so unsupported autostart, silent start, and fullscreen detection controls are disabled with a platform limitation message.
- Updated architecture and release documentation to avoid presenting macOS/Linux support as complete before real device verification.
- Updated the manual release workflow to publish assets against the `v0.1.1` tag, package macOS ARM64 as a zipped `.app` preview build, and run ARM64 DMG packaging as a separate diagnostic job.

### Verified

- `pnpm build`
- `pnpm lint`
- `cargo test --manifest-path src-tauri\Cargo.toml`
- `pnpm tauri build --no-bundle`
- `pnpm tauri build --bundles nsis`
- `pnpm tauri build`

### Known Limitations

- macOS/Linux packages are preview builds and require real-device validation.
- macOS ARM64 DMG packaging may still fail on hosted runners; the preview release keeps a zipped `.app` bundle and records DMG diagnostics.
- macOS signing and notarization are not yet complete.
- Linux tray behavior still requires desktop-environment-specific validation.
- Non-Windows fullscreen detection, lock detection, autostart, and tray behavior must be verified on real devices before being marked supported.

## 0.1.0 Beta Initial Build - 2026-06-09

### Added

- Added default drink, rest, and eye care reminders.
- Added custom reminders with configurable name, message, interval, and action countdown.
- Added desktop notification window with complete, postpone, skip, and timeout handling.
- Added serial notification queue to avoid overlapping reminder popups.
- Added system tray menu and close-to-tray behavior.
- Added start-on-login setting.
- Added light, dark, and system theme modes.
- Added do-not-disturb schedule and temporary quiet mode.
- Added presentation and game fullscreen detection with automatic reminder postponing.
- Added sleep and wake handling to recalibrate active reminders.
- Added reminder sound presets, volume control, and preview playback.
- Added daily stats, trend chart, and streak display.
- Added configuration export and import.
- Added local database recovery path for corrupted database files.
- Added multilingual interface.
- Added NSIS installer build target.

### Changed

- Updated default reminder templates:
  - Drink reminder: 90 minutes.
  - Rest reminder: 60 minutes with 5 minute action countdown.
  - Eye care reminder: 20 minutes with 20 second action countdown.
- Unified rest, eye care, and custom reminders under the action countdown model.
- Refined fullscreen detection so ordinary maximized windows do not suppress notifications.
- Replaced the initial scaffold README with product and release documentation.

### Verified

- `pnpm lint`
- `pnpm test`
- `pnpm build`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `pnpm tauri build --no-bundle`
- `pnpm tauri build`
- NSIS installer generation.
- Start-on-login behavior after Windows restart.

### Known Limitations

- Current release is intended for beta testing.
- Installer is not code-signed, so Windows may show an unknown publisher warning.
- 24 hour memory sampling still needs to be completed on a target device.
- 4K scaling, dual-monitor positioning, repeated launch behavior, setting persistence after restart, and sleep/wake behavior still need final manual regression.
- Local custom audio files and fullscreen overlay mode are not included in this release.
