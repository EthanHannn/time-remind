# Changelog

## 0.1.11 Beta - 2026-10-12

### Added

- Added a Windows installer startup page in all 10 installer languages, with opt-in choices for starting with Windows and starting silently in the system tray. Both default off for new installations.
- Applied installer choices to the app's settings on first launch. Silent startup only hides the main window on Windows sign-in; manual and post-install launches still show it.

### Improved

- Kept existing startup preferences by default during upgrades and reinstalls, including installations that have not launched the app yet. Silent, passive, and update installations preserve preferences without prompting.
- Preserved autostart registration across maintenance uninstall and installation path changes. A one-time request ID prevents leftover installer files from undoing later in-app changes.

## 0.1.10 Beta - 2026-10-12

### Fixed

- Removed a startup deadlock between the scheduler's tray icon update and the main window's database reads. Tray updates now dispatch to the UI thread without waiting while holding the database lock.
- Added a 15-second timeout and retry for initial event registration and reminder loading, with localized guidance instead of an indefinite loading screen.
- Cleaned up failed and late event subscriptions, cancelled pending load timers on unmount, and prevented timed-out responses from replacing a newer retry's data.

## 0.1.9 Beta - 2026-10-10

### Fixed

- Honored disabled action countdowns and migrated legacy reminder fields only once.
- Initialized defaults only for new databases, preserving deleted reminders and saved preferences across restarts.
- Recorded completion when an action actually finishes, persisted active action progress, and kept interrupted actions out of completed counts.
- Unified completion rates, daily totals, trends, and streaks around final outcomes; preserved disabled reminders' history and stopped automatic deletion of older logs. Previously deleted or incorrectly completed historical records are not reconstructed.
- Validated imported settings, reminders, and logs before applying changes; aligned backup reads, writes, and automatic snapshots to the same size limit.
- Preserved form input on save failures, prevented duplicate submissions, and serialized settings saves with retryable errors.
- Kept due, queued, paused, and do-not-disturb states consistent with the scheduler; made pause compensation atomic and followed system theme changes immediately.
- Cleared stale hover and focus state when a notification closes so the next reminder can dismiss automatically.
- Fixed concurrent release creation and added recovery for missing macOS ARM64 DMG uploads.

### Improved

- Implemented the approved Light companion, System simple, and Illustrated companion reminder layouts with soft blue, green, amber, and violet category colors. Existing Soft settings are retained; legacy Vivid settings open the new Illustrated layout.
- Restored the main window's translucent glass background with soft blue and warm amber light shared across the title bar and content, and tightened the Illustrated companion layout with a 40-pixel reduction in window height.
- Replaced all seven reminder icons and unified action buttons, accessible labels, keyboard focus, and modal dismissal.
- Made the next reminder time prominent while retaining progress bars and individual switches; moved global pause and reminder management into the top toolbar and removed duplicate routine status labels.
- Added reminder editing with retryable save errors and confirmed deletion, a direct five-minute snooze with all saved durations still available, and a collapsible action timer.
- Matched native notification sizing to each layout and countdown state, including previews and monitor positioning. Pointer and keyboard interaction pause automatic dismissal.
- Added custom reminder previews and consistent category colors in statistics; localized the new controls in all 19 languages.

### Verified

- 69 frontend tests and 58 Rust tests passed for the implemented changes.
- TypeScript checking, production build, ESLint, and Rust formatting passed; layouts were checked across all 19 languages, light and dark themes, and narrow windows.
- Native Windows multi-monitor DPI, fullscreen, lock, sleep, and wake acceptance remains pending for these changes. macOS and Linux remain community previews without real-device verification.

## 0.1.8 Beta - 2026-09-29

### Added

- Two independently selectable reminder popup styles: Soft colors and Vivid gradients.
- Three cat illustration styles with transparent artwork and instant reminder previews.
- Eight additional languages: Spanish, Brazilian Portuguese, Indonesian, Italian, Russian, Turkish, Arabic, and Hindi, bringing the total to 19.
- Arabic right-to-left layouts, localized chart dates and labels, and translated native tray menus and default reminders.
- English and Simplified Chinese screenshot galleries in the corresponding READMEs.
- SHA256SUMS.txt for downloadable release assets.

### Improved

- Reorganized settings into compact groups, with inline theme controls and a two-column layout in wider windows, while retaining the glass appearance.
- Replaced the native language dropdown with a scrollable, keyboard-accessible selector.
- Completed missing translations in existing languages and added checks for translation keys and placeholders.
- Refined popup spacing, action controls, countdowns, and artwork across light and dark themes.

### Verified

- 43 frontend tests and 40 Rust tests passed, covering translations, persistence, reminder scheduling, and native language support.
- TypeScript checking, production build, source ESLint, and Rust formatting passed.
- Windows tests confirmed that preview and scheduled reminder windows stay above maximized applications on a dual-monitor, high-DPI desktop.
- Settings layouts checked at the 400 × 500 minimum window size, including Arabic and long translations.
- macOS and Linux remain community previews; their native behavior has not been verified on real devices.

## 0.1.7 Beta - 2026-09-20

### Fixed

- Isolated notification instances and versioned atomic queue snapshots to reject delayed events and stale responses.
- Preserved reminder schedules across pause, application restart, lock, sleep, and wake; corrected overlapping manual, system, and temporary do-not-disturb compensation.
- Used the frozen schedule clock when creating, enabling, editing, or importing reminders during a pause.
- Rechecked fullscreen, lock, pause, and do-not-disturb conditions before showing queued notifications; excluded Explorer desktop windows from fullscreen detection.
- Hid cleared native notification windows and serialized native display with queue cancellation.
- Displayed global pause status and frozen countdowns consistently, including delayed overview responses.

- Made the notification window transparent, disabled its native shadow, and excluded it from native window materials so rounded corners no longer sit inside an opaque rectangular frame.
- Selected the X11 backend at startup for Linux Wayland sessions with XWayland available, allowing the custom notification window to retain bottom-right positioning on Ubuntu GNOME.

### Verified

- 15 frontend tests and 39 Rust tests passed.
- ESLint, TypeScript checking, production build, Rust checking, and formatting passed.
- Real-device fullscreen, lock/sleep/wake, and multi-monitor interaction acceptance remains pending.

## 0.1.6 Beta - 2026-09-02

### Added

- Added four selectable custom reminder icons and reused the saved choice in reminder cards, notifications, and statistics.

### Fixed

- Made native select options readable in dark mode.
- Increased the main panel opacity so background content no longer interferes with readability.
- Repositioned the notification window after it is mapped on Linux, with a delayed second positioning pass for GTK window managers.

### Verified

- `node_modules/.bin/eslint .`
- `node_modules/.bin/vitest run`
- `node_modules/.bin/vue-tsc --noEmit`
- `node_modules/.bin/vite build`

## 0.1.5 Beta - 2026-07-22

### Fixed

- Restored the notification window's always-on-top state each time a reminder is shown.
- Completed Traditional Chinese, Japanese, and Korean platform capability messages.

### Changed

- GitHub Releases now use the matching version entry from this changelog as their release notes.

### Verified

- `pnpm lint`
- `pnpm test`
- `pnpm build`
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `pnpm tauri build --no-bundle`
- `pnpm tauri build --bundles nsis`

## 0.1.4 Beta - 2026-07-17

### Fixed

- Preserved each enabled reminder's remaining schedule during temporary Do Not Disturb.

## 0.1.3 Beta - 2026-07-13

### Fixed

- Corrected notification placement on Windows multi-monitor setups with mixed display scaling.
- Positioned notification windows before showing them and recorded placement failures in the runtime log.

### Verified

- `cargo fmt -- --check`
- `cargo test scheduler::tests::notification_position_uses_target_monitor_physical_coordinates`
- `cargo check`

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
