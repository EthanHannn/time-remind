# Changelog

## 0.1.0 Beta - 2026-06-09

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
