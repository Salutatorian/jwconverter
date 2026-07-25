# Uninstall full wipe — design (v0.1.3)

## Goal

When a user uninstalls JW Converter, nothing app-related is left on the PC. They get a clear warning first.

## Behavior

1. Uninstall confirm page shows a locked, checked notice that all JW Converter **app data** will be deleted.
2. Leaving the confirm page shows an OK/Cancel warning:
   - App + settings/cache are permanently removed
   - Converted audio files in Downloads/other folders are **not** deleted
3. On uninstall (not during silent updater `/UPDATE`), always wipe:
   - `%APPDATA%\<bundle id>` and `%LOCALAPPDATA%\<bundle id>` (incl. WebView2/EBWebView)
   - `%APPDATA%\JW Converter` and `%LOCALAPPDATA%\JW Converter` leftovers
   - Manufacturer/product registry keys used by the installer
4. Install directory, Start Menu, desktop shortcuts, and Apps & Features entry are removed as before.

## Non-goals

- Do not delete user-converted media outside the app folders.
- Do not wipe app data during in-app updater installs (`/UPDATE`).
