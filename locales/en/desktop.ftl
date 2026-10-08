# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = OK

## Splash and loading status.

desktop-splash-starting = Starting { -brand }
desktop-splash-restarting = Restarting { -brand }
desktop-splash-recovering = Recovering
desktop-splash-opening-dashboard = Opening dashboard
desktop-splash-checking-dependencies = Checking dependencies
desktop-splash-installing-dependencies = Installing system dependencies
desktop-splash-installing-dependencies-detail = { -brand } needs the Microsoft Visual C++ Redistributable to run.
desktop-splash-resetting-wallet = Resetting wallet data
desktop-splash-resetting-wallet-detail = The existing wallet data is backed up before it is cleared.
desktop-splash-updating = Updating to v{ $version }
desktop-splash-updating-detail = Your settings and data stay exactly as they are.
desktop-splash-restoring = Restoring v{ $version }
desktop-splash-restoring-detail = Update v{ $failed } did not start, so the previous version is taking over.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = { -brand } could not start
desktop-boot-detail-fallback = The backend stopped unexpectedly.
desktop-boot-remedy-label = How to fix
desktop-boot-log-file-label = Log file:
desktop-boot-action-reset-wallet = Reset wallet data & restart
desktop-boot-action-working = Working...
desktop-boot-action-open-logs = Open logs folder
desktop-boot-action-copy = Copy details
desktop-boot-action-copied = Copied
desktop-boot-action-quit = Quit
desktop-boot-subtitle-wallet-mismatch = A different wallet was detected
desktop-boot-subtitle-port-in-use = A required network port is busy
desktop-boot-subtitle-lock-held = { -brand } is already running
desktop-boot-subtitle-config-invalid = Configuration problem
desktop-boot-subtitle-directory-setup = Storage problem
desktop-boot-subtitle-storage-upgrade = Database upgrade problem
desktop-boot-subtitle-generic = Startup error

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = { -brand } could not start
desktop-boot-error-remedy = Open the logs folder to see what happened, then restart the app. If the problem persists, contact support at t.me/screenerbotio_support.
desktop-boot-error-default = The backend stopped unexpectedly before the dashboard was ready.
desktop-boot-error-restore-failed = The updated backend failed and the previous version could not be restored ({ $error }).
desktop-boot-error-spawn-failed = The backend program could not be started ({ $error }).
desktop-boot-error-spawn-missing = The backend program could not be started. It may be missing or blocked by security software.
desktop-boot-error-exited-running = The backend stopped while the dashboard was running (exit code { $code }).
desktop-boot-error-exited-early = The backend stopped before the dashboard was ready (exit code { $code }).
desktop-boot-error-dashboard-load = The dashboard failed to load ({ $description }, { $code }).
desktop-boot-error-renderer-gone = The dashboard renderer stopped ({ $reason }).
desktop-boot-error-unresponsive = The dashboard became unresponsive.
desktop-boot-error-url-failed = The dashboard URL could not be loaded ({ $error }).
desktop-boot-error-relaunch-setup = Could not relaunch the backend after setup.
desktop-boot-error-relaunch-recovery = Could not relaunch the backend for recovery.
desktop-boot-error-restart-offline = The backend did not come back online after restart.
desktop-boot-error-recovery-offline = Recovery finished but the backend did not become ready.
desktop-boot-error-start-timeout = The backend did not finish starting in time. This can happen on a slow first run or if another program is blocking the connection.

## System tray.

desktop-tray-tooltip = { -brand } - Solana Trading Bot
desktop-tray-show = Show { -brand }
desktop-tray-open-dashboard = Open Dashboard
desktop-tray-quit = Quit { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = Open Data Folder
desktop-menu-open-logs-folder = Open Logs Folder
desktop-menu-documentation = Documentation
desktop-menu-telegram-support = { -telegram } Support
desktop-menu-check-updates = Check for Updates...

## Application menu.

desktop-menu-file = File
desktop-menu-edit = Edit
desktop-menu-view = View
desktop-menu-window = Window
desktop-menu-help = Help
desktop-menu-reset-zoom = Reset Zoom
desktop-menu-zoom-in = Zoom In
desktop-menu-zoom-out = Zoom Out
desktop-menu-keyboard-shortcuts = Keyboard Shortcuts
desktop-menu-telegram-channel = { -telegram } Channel
desktop-menu-telegram-community = { -telegram } Community
desktop-menu-follow-x = Follow on { -x } ({ -twitter })
desktop-menu-visit-website = Visit Website
desktop-menu-about = About { -brand }

## About dialog.

desktop-about-title = About { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Version { $version }

    Advanced Solana wallet management and auto-trading bot.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = Keyboard Shortcuts
desktop-shortcuts-message = { -brand } Keyboard Shortcuts
desktop-shortcuts-body-mac =
    Keyboard Shortcuts:

    Window Controls:
      Cmd+M          Minimize
      Cmd+W          Close Window
      Cmd+Q          Quit
      Cmd+Ctrl+F     Toggle Fullscreen

    Zoom:
      Cmd++          Zoom In
      Cmd+-          Zoom Out
      Cmd+0          Reset Zoom

    Navigation:
      Cmd+R          Reload Dashboard
      Cmd+Shift+D    Open Data Folder

    Other:
      F1             Open Documentation
      Cmd+Alt+I      Toggle DevTools
desktop-shortcuts-body-other =
    Keyboard Shortcuts:

    Window Controls:
      Alt+F4         Quit
      F11            Toggle Fullscreen

    Zoom:
      Ctrl++         Zoom In
      Ctrl+-         Zoom Out
      Ctrl+0         Reset Zoom

    Navigation:
      Ctrl+R         Reload Dashboard
      Ctrl+Shift+D   Open Data Folder

    Other:
      F1             Open Documentation
      Ctrl+Shift+I   Toggle DevTools

## Close confirmation (Windows and Linux).

desktop-close-title = Close { -brand }
desktop-close-message = What would you like to do?
desktop-close-detail = { -brand } can continue running in the background. The trading bot will keep monitoring and trading while minimized to the system tray.
desktop-close-minimize = Minimize to Tray
desktop-close-quit = Quit Completely
desktop-close-cancel = Cancel

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = Missing Dependency
desktop-vcredist-missing-message = Visual C++ Redistributable is missing
desktop-vcredist-missing-detail = { -brand } requires Microsoft Visual C++ Redistributable to run. Would you like to install it now?
desktop-vcredist-install = Install & Fix
desktop-vcredist-exit = Exit
desktop-vcredist-not-found-title = Installer Not Found
desktop-vcredist-not-found-message = Could not locate { $name } correctly.
desktop-vcredist-done-title = Installation Complete
desktop-vcredist-done-message = Dependencies installed successfully.
desktop-vcredist-done-detail = { -brand } will now start.
desktop-vcredist-failed-title = Installation Failed
desktop-vcredist-failed-message = Please install Visual C++ Redistributable manually.
