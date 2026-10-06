# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = In-memory configuration differs from disk version
system-result-config-matches = In-memory configuration matches disk version

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    Successfully imported { $count ->
        [one] { $count } section
       *[other] { $count } sections
    }
system-result-config-imported-with-warnings =
    Imported { $count ->
        [one] { $count } section
       *[other] { $count } sections
    } with { $warnings ->
        [one] { $warnings } warning
       *[other] { $warnings } warnings
    }: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = Search settings...
system-config-export-title =
    .title = Export configuration to file
system-config-import-title =
    .title = Import configuration from file
system-config-reload = Reload from Disk
system-config-reset-defaults = Reset to Defaults
system-config-select-section = Select a configuration section
system-config-select-section-details = Select a configuration section to view details.
system-config-no-metadata = No metadata for <code>{ $section }</code>
system-config-technical-settings = Technical Settings
system-config-expand-title = Expand every section and every nested sub-config
system-config-collapse-title = Collapse every section and every nested sub-config
system-config-toolbar-no-changes = No section changes
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> change in section
       *[other] <strong>{ $count }</strong> changes in section
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> total change
       *[other] <strong>{ $count }</strong> total changes
    }

## Config page: state banner

system-config-loading = Loading configuration…
system-config-refreshing = Refreshing configuration…
system-config-saving-title = Saving changes…
system-config-saving-detail = Updating configuration
system-config-validation-issues = <strong>Validation issues detected.</strong> Please review highlighted fields.

## Config page: section header and category chips

system-config-save-changes = Save Changes
system-config-saving = Saving…
system-config-compare = Compare with Disk
system-config-revert-section = Revert Section
system-config-summary-critical = { $count } critical
system-config-summary-performance = { $count } performance
system-config-summary-pending =
    { $count ->
        [one] { $count } pending change
       *[other] { $count } pending changes
    }
system-config-summary-none = No metadata summary
system-config-fields-count =
    { $count ->
        [one] { $count } field
       *[other] { $count } fields
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · { $pending } pending
system-config-chip-visible = { $visible } of { $fields }

## Config page: field rows

system-config-field-unit = Unit: { $unit }
system-config-field-default = Default: { $value }
system-config-field-reset = Reset to default
system-config-array-invalid-title = Invalid array entry
system-config-json-invalid-title = Invalid JSON
system-config-list-separator = { ", " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
        [one] Line { $lines } must be a valid integer.
       *[other] Lines { $lines } must be a valid integer.
    }
system-config-array-invalid-number =
    { $count ->
        [one] Line { $lines } must be a valid number.
       *[other] Lines { $lines } must be a valid number.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] Line { $lines } must be a valid boolean.
       *[other] Lines { $lines } must be a valid boolean.
    }
system-config-array-invalid-value =
    { $count ->
        [one] Line { $lines } must be a valid value.
       *[other] Lines { $lines } must be a valid value.
    }

## Config page: Telegram actions

system-config-telegram-actions = Actions
system-config-telegram-test-title = Test Connection
system-config-telegram-test-description = Send a test message to verify your { -telegram } configuration is working
system-config-telegram-send-test = Send Test Message
system-config-telegram-sending = Sending...
system-config-telegram-configure-token-title = Configure bot token first
system-config-telegram-configure-token-status = Configure bot token above to enable testing
system-config-telegram-test-sent-status = Test message sent successfully! Check your { -telegram }.
system-config-telegram-test-sent = { -telegram } test message sent
system-config-telegram-test-failed = Failed to send test message
system-config-telegram-auth-title = Bot Authentication
system-config-telegram-totp-title = Two-Factor Authentication (TOTP)
system-config-telegram-totp-configured = Configured
system-config-telegram-totp-not-configured = Not Configured
system-config-telegram-totp-active = Two-factor authentication is active. Expired { -telegram } sessions require TOTP code from your authenticator app.
system-config-telegram-totp-inactive = Enable two-factor authentication in Security settings to protect { -telegram } commands.
system-config-telegram-totp-note = TOTP is shared with the dashboard lockscreen. Configure it in Security settings.
system-config-telegram-require-2fa = Require 2FA for commands
# $status is the HTTP status code.
system-config-telegram-save-rejected = Save rejected ({ $status })
system-config-telegram-save-failed = Could not save { -telegram } setting

## Config page: operations

system-config-saved = Configuration saved
system-config-save-failed = Could not save configuration
system-config-reloaded = Configuration reloaded from disk
system-config-reload-failed = Could not reload configuration
system-config-diff-title = Configuration diff
system-config-diff-console = Written to the browser console
system-config-diff-failed = Could not calculate diff
system-config-reset-title = Reset Configuration
system-config-reset-message =
    This will reset the entire configuration to embedded default values. All current settings will be lost.

    This action cannot be undone.
system-config-reset-done-title = Configuration reset
system-config-reset-done-message = All settings restored to default values
system-config-reset-failed = Could not reset configuration
system-config-load-failed = Could not load configuration
system-config-metadata-failed = Could not load configuration metadata

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = Close
system-config-select-none = Select None
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } change
       *[other] { $count } changes
    }
system-config-sections-count =
    { $count ->
        [one] { $count } section
       *[other] { $count } sections
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-chains = Chain enablement, RPC endpoints and swap routing
system-config-section-hint-trader = Trading rules and automation
system-config-section-hint-positions = Position management settings
system-config-section-hint-filtering = Token filtering rules and thresholds
system-config-section-hint-tokens = Token discovery and data sources
system-config-section-hint-events = Event recording settings
system-config-section-hint-services = Background service settings
system-config-section-hint-monitoring = System monitoring configuration
system-config-section-hint-ohlcv = Candlestick data settings
system-config-section-hint-gui = Dashboard and UI settings
system-config-section-hint-telegram = { -telegram } bot configuration

## Export dialog

system-config-export-dialog-title = Export Configuration
system-config-export-intro = Select which configuration sections to export. The exported file can be imported later to restore or share settings.
system-config-export-sections = Sections
system-config-export-timestamp = Include export timestamp
system-config-sections-selected =
    { $count ->
        [one] { $count } section selected
       *[other] { $count } sections selected
    }
system-config-exporting = Exporting...
system-config-export-invalid-response = Invalid response from server
system-config-exported-title = Configuration Exported
system-config-exported-message =
    { $count ->
        [one] Exported { $count } section
       *[other] Exported { $count } sections
    }
system-config-export-failed-title = Export Failed
system-config-export-failed = Failed to export configuration

## Import dialog

system-config-import-dialog-title = Import Configuration
system-config-import-upload-intro = Upload a previously exported configuration file. You'll be able to preview and select which sections to import.
system-config-import-dropzone-title = Drop config file here
system-config-import-dropzone-hint = or click to browse
system-config-import-analyzing = Analyzing configuration...
system-config-import-preview = Preview
system-config-import-preview-intro = Review the configuration sections below. Select which sections to import.
system-config-import-sections = Sections in File
system-config-import-select-valid = Select All Valid
system-config-import-merge-label = Merge with existing
system-config-import-merge-hint = Only update fields present in the file. Unchecked = replace entire sections.
system-config-import-save-label = Save to disk
system-config-import-save-hint = Persist changes to config.toml after import
system-config-import-selected = Import Selected
system-config-import-warnings =
    { $count ->
        [one] { $count } Warning
       *[other] { $count } Warnings
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = Unknown section "{ $section }" will be ignored
system-config-import-warning-sensitive-field = Importing { $field } may overwrite authentication settings
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Not in file
system-config-import-status-invalid = Invalid configuration
system-config-import-status-unchanged = No changes
system-config-import-not-included = Not included in file
system-config-import-show-changes = Show changes
system-config-import-hide-changes = Hide changes
system-config-import-value-current = Current value
system-config-import-value-new = New value
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } more change
       *[other] +{ $count } more changes
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } item
       *[other] { $count } items
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } key
       *[other] { $count } keys
    }{ "}" }
system-config-importing = Importing...
system-config-import-failed = Import failed
system-config-import-invalid-file-title = Invalid File
system-config-import-invalid-file = Failed to parse configuration file
system-config-imported-title = Configuration Imported
system-config-imported-message =
    { $count ->
        [one] Imported { $count } section
       *[other] Imported { $count } sections
    }
system-config-import-failed-title = Import Failed
system-config-import-failed-message = Failed to import configuration
