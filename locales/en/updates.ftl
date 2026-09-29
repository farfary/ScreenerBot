# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = Automatic installation is disabled. The update is ready and will be applied when you choose.
updates-defer-trading-active = A position, trade, or tool operation is active, so the restart is deferred. The update applies automatically when the app is idle.
updates-defer-needs-installer = This release also updates the desktop shell, so the installer has to run once.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }
updates-check-failed-legacy = { $cause }

## Progress and outcome of update actions

updates-download-started = Downloading update v{ $version }...
updates-apply-started = Installing the update. ScreenerBot restarts and reconnects automatically.
updates-install-opened = Verified update installer opened. Complete the operating-system installer.
