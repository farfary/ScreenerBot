# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = Automatic installation is disabled. The update is ready and will be applied when you choose.
updates-defer-trading-active = A position, trade, or tool operation is active, so the restart is deferred. The update applies automatically when the app is idle.
updates-defer-needs-installer = This release also updates the desktop shell, so the installer has to run once.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = Downloading update v{ $version }...
updates-apply-started = Installing the update. { -brand } restarts and reconnects automatically.
updates-install-opened = Verified update installer opened. Complete the operating-system installer.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = Installer opened
updates-installer-toast-message = { -brand } will quit cleanly now.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = Status
updates-tab-release-notes = Release Notes
updates-tab-preferences = Preferences
updates-tab-sections = Update sections
updates-checking-installation = Checking this installation...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = Ready to check for updates
updates-phase-idle-detail = { -brand } v{ $version } is installed.
updates-phase-up-to-date-headline = You are up to date
updates-phase-up-to-date-detail = { -brand } v{ $version } is the latest version.
updates-phase-checking-headline = Checking for updates
updates-phase-checking-detail = Looking for the latest published release.
updates-phase-available-headline = Version { $version } is available
updates-phase-downloading-headline = Downloading v{ $version }
updates-phase-verifying-headline = Verifying v{ $version }
updates-phase-verifying-detail = Checking the download against its published checksum.
updates-phase-ready-to-apply-headline = Version { $version } is ready
updates-phase-ready-to-apply-detail = The update can be installed now with a short restart, or automatically on the next start.
updates-phase-ready-to-install-headline = Version { $version } is ready
updates-phase-ready-to-install-detail = The desktop installer is ready to finish this update.
updates-phase-applying-headline = Installing update
updates-phase-applying-detail = { -brand } is restarting onto the new version.
updates-phase-applied-headline = Updated to v{ $version }
updates-phase-applied-detail = The update was installed. Nothing else is needed.
updates-phase-failed-headline = The update did not finish
updates-phase-failed-detail = Try the update again.
updates-phase-check-failed-headline = Could not check for updates
updates-phase-check-failed-detail = The release service could not be reached.
updates-status-unavailable-headline = Update status is unavailable
updates-phase-unrecognized-detail = The reported update state is not recognized.
updates-status-load-failed-detail = The installation status could not be loaded.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = Core update · { $size } · short restart
updates-kind-full = Desktop update · { $size } · installer required
updates-size-unknown = unknown size

updates-action-check-now = Check now
updates-action-check-again = Check again
updates-action-try-again = Try again
updates-action-download = Download update
updates-action-restart = Restart to update
updates-action-open-installer = Open installer

updates-busy-checking = Checking...
updates-busy-resuming = Resuming download...
updates-busy-starting-download = Starting download...
updates-busy-restarting = Restarting...
updates-busy-opening-installer = Opening installer...

updates-progress-downloading = Downloading update
updates-progress-verifying = Verifying update
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } of { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Installation details
updates-detail-installed-version = Installed version
updates-detail-system = System
updates-detail-last-checked = Last checked
updates-detail-never = Never
updates-detail-download-size = Download size

updates-version-installed = Installed
updates-version-available = Available

updates-notes-highlights = Highlights
updates-notes-empty-title = No release notes yet
updates-notes-empty-error = The release history could not be loaded. Check the connection and try again.
updates-notes-empty-none = Release notes will appear here once a release has been published.
updates-notes-history-notice = Showing what this installation already knows — the release history could not be loaded.
updates-release-empty = No changes were listed for this release.
updates-release-changes =
    { $count ->
        [one] { $count } change
       *[other] { $count } changes
    }

updates-preferences-unavailable-title = Update preferences are unavailable
updates-preferences-unavailable-detail = The update configuration could not be loaded.
updates-preference-fallback-name = update preference
updates-preference-save-failed = Could not save { $preference }

updates-request-failed = Request failed
updates-check-request-failed = Could not check for updates
updates-resume-failed = Could not resume update download
updates-download-failed = Could not start update download
updates-apply-failed = Could not install update
updates-install-failed = Could not open update installer
updates-apply-confirm-title = Install v{ $version }
updates-apply-confirm-message = { -brand } restarts onto the new version. Trading stops for a few seconds and resumes automatically; open positions are untouched.
updates-install-confirm-title = Run the installer
updates-install-confirm-message = The verified installer opens and { -brand } quits cleanly. Complete the installer, then reopen { -brand }.

# A release version as displayed.
updates-version-number = v{ $version }

# The Home update notice, shown while a release is in play.
updates-notice-region =
    .aria-label = Update status
updates-notice-view = View update
updates-notice-whats-new = What's new
updates-notice-available-detail = See what changed and install it from Settings.
updates-notice-updated-detail = See what changed in this version.
# A headless installation cannot download or install a release itself.
updates-headless-install-detail = Headless installations are updated outside the dashboard. On Linux, run { "screenerbot-manager update" }.
