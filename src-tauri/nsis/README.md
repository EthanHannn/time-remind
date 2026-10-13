# Windows installer startup choices

`installer.nsi` is the upstream Tauri NSIS template from **tauri-cli-v2.11.2**:

https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.2/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi

The upstream template is used under the MIT license in `TAURI-LICENSE-MIT`.
Keep the template aligned with the locked `@tauri-apps/cli` version when upgrading Tauri.
Our additions, besides the attribution comment, are three macro calls and a
local-language branch for Khmer:

- `TIME_REMIND_STARTUP_PAGE` after the directory page, before installation.
- `TIME_REMIND_STARTUP_LANGUAGES` after the installer languages are loaded.
- `TIME_REMIND_STARTUP_INIT` after `SetContext` in `.onInit`, before maintenance can uninstall an older version.
- `MUI_LANGUAGEEX` loads `languages/native/Khmer.nlf` and `Khmer.nsh` from the
  repository. The NSIS distribution has no Khmer resources; no installed NSIS
  files are modified. These resources use Khmer (Cambodia), LCID 1107, and the
  Leelawadee UI font. Their structure follows the NSIS English language files.

The installer offers the same 19 languages as the app. Tauri-specific messages
for Khmer, Indonesian and Hindi are supplied alongside the existing Korean,
Thai and Malay overrides. The remaining languages use Tauri's bundled messages.
All languages also cover the startup-options page and installer-to-app marker.

The existing `hooks.nsh` still records the installer language and now also applies
the startup choices after installation. The custom page is necessary because a
post-install hook cannot insert a page into the middle of Tauri's wizard.

## Behavior

- Install for the current Windows user only, matching the app's autostart plugin
  and Roaming AppData location.
- On a new interactive installation, autostart and silent startup default off.
- Silent startup can only be chosen with autostart. It hides the main window on
  Windows sign-in, while reminders and the tray continue running.
- Opening the app manually or using the finish-page launch option shows its window.
- If an existing database or pending installer choice is present, the page defaults
  to keeping current settings; users can uncheck that option to choose new ones.
- Silent, passive, and update installs skip the page and preserve preferences.
- Preserve an existing Run registration across maintenance uninstall and installation
  path changes without resetting Windows' StartupApproved state. Explicitly enabling
  autostart resets that state, matching the app's autostart plugin.

## Installer-to-app contract

For an explicit choice, the installer configures the current user's Windows Run
entry immediately. The command is `"<installed executable>" --autostart`, with the
registry value named after `productName`, just like `tauri-plugin-autostart`.

It also writes `%APPDATA%/com.hpk.time-remind/installer-startup.json`:

```json
{ "schema": 1, "request_id": "{a GUID}", "auto_start": true, "silent_start": true }
```

`src/installer.rs` consumes this once, after database initialization and before
autostart restoration or the window visibility decision. Both preferences and the
request ID are committed in one SQLite transaction. A repeated ID is ignored, so a
marker that could not be deleted cannot overwrite later settings changes. Invalid
or oversized markers are logged and do not prevent the app from starting.

## Validation

Run `cargo test --locked` in `src-tauri` and `pnpm tauri build --debug --bundles nsis`
from the repository root. Changes to the template also need an interactive Windows
installation check, including fresh install, upgrade, Back navigation, autostart
on/off, silent on/off, and manual versus sign-in launch. Use a separate Windows test
account or VM to avoid changing a developer's real data and autostart settings.
