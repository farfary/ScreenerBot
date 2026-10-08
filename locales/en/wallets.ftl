# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = Generated
wallets-type-imported = Imported
wallets-type-migrated = Migrated

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = Paused by you
wallets-watch-disabled-signature-budget = Paused: reached the { $limit }-signature check limit before catching up
wallets-watch-disabled-unknown = Paused: the saved watch safety reason could not be read
wallets-watch-disabled-helius-unavailable = Paused: high-activity provider is unavailable; cursor preserved
wallets-watch-disabled-processing-failed = Paused: wallet activity could not be processed; cursor preserved

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = High-activity provider is unavailable; watch paused
wallets-watch-error-provider-repeated-failure = { -helius } checks repeatedly failed; watch paused
wallets-watch-error-processing-repeated-failure = Wallet activity processing repeatedly failed; watch paused
wallets-watch-error-position-unreadable = Wallet watch could not read its saved position; retrying
wallets-watch-error-provider-check-failed = High-activity provider check failed; retrying
wallets-watch-error-decode-failed = High-activity transaction could not be decoded; cursor retained
wallets-watch-error-processing-failed = Wallet activity could not be processed; retrying
wallets-watch-error-position-save-failed = Wallet watch could not save its position; retrying

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = Paused by you.
wallets-watch-reason-signature-budget = This wallet has more activity than its current watch can check.
wallets-watch-reason-helius-unavailable = { -helius } checks failed. Saved progress is preserved.
wallets-watch-reason-processing-failed = Wallet activity could not be processed. Saved progress is preserved.

# Vocabulary shared by the wallet tables and dialogs.
wallets-field-name = Wallet Name
wallets-field-notes = Notes
wallets-field-private-key = Private Key
wallets-modal-close =
    .aria-label = Close modal
wallets-this-wallet = this wallet
wallets-summary-native = { -sol }
wallets-copied-private-key = Private key

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Main Wallet
wallets-tab-secondaries = Secondaries
wallets-tab-archive = Archive
wallets-tab-watched = Watched
wallets-refresh-failed = Could not refresh wallets
wallets-action-failed = Failed
wallets-toast-failed = Failed: { $reason }
wallets-create-busy = Creating...
wallets-create-fallback = Creation failed
wallets-create-done = Wallet "{ $name }" created!
wallets-import-busy = Importing...
wallets-import-failed = Import failed
wallets-import-done = Wallet "{ $name }" imported!
wallets-archive-busy = Archiving...
wallets-archive-confirm-text = Are you sure you want to archive <strong>{ $name }</strong>?
wallets-archive-done = Wallet archived
wallets-restore-done = Wallet restored
wallets-export-busy = Decrypting...
wallets-export-revealed = Key revealed - handle with care
wallets-delete-busy = Deleting...
wallets-delete-confirm-text = Are you sure you want to delete <strong>{ $name }</strong>?
wallets-delete-done = Wallet deleted permanently

# wallets.html: Add Wallet dialog.
wallets-add-title = Add Wallet
wallets-add-tab-create = Create New
wallets-add-tab-import = Import Existing
wallets-create-name-input =
    .placeholder = e.g., Trading Wallet
wallets-create-name-hint = A friendly name to identify this wallet
wallets-create-notes-input =
    .placeholder = Optional description or purpose...
wallets-create-submit = Create Wallet
wallets-import-warning-title = Security Warning
wallets-import-warning-body = Only import private keys from trusted sources. Your key will be encrypted and stored securely on this device.
wallets-import-name-input =
    .placeholder = e.g., My Wallet
wallets-import-key-input =
    .placeholder = Base58 string or JSON array [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Toggle private key visibility
wallets-import-key-hint = Supports base58 encoded key or byte array format
wallets-import-notes-input =
    .placeholder = Optional description...
wallets-import-submit = Import Wallet

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = Watch Wallet
wallets-watch-add-address = Wallet Address
wallets-watch-add-address-input =
    .placeholder = Solana address
wallets-watch-add-address-hint = Records the wallet's on-chain activity and sends trade alerts through your { -telegram } settings.
wallets-watch-add-label = Label
wallets-watch-add-label-input =
    .placeholder = Optional name
wallets-watch-add-submit = Add Watch

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = Wallet watch options
wallets-watch-budget-title-restore = Restore wallet watch
wallets-watch-budget-close =
    .aria-label = Close
wallets-watch-budget-label-signatures = Signatures checked per check
wallets-watch-budget-label-transactions = Successful full transactions checked per check
wallets-watch-budget-hint-signatures = Current limit: { $limit }. Choose 500–5,000 signatures per check in steps of 100.
wallets-watch-budget-hint-transactions = Current limit: { $limit }. Choose 500–5,000 successful transactions per check in steps of 100.
wallets-watch-budget-error-range = Choose between 500 and 5,000 records per check in 100-record steps.
wallets-watch-budget-error-ack = Acknowledge that signatures since the last completed check will be skipped.
wallets-watch-budget-save-failed = Watch limit could not be saved.
wallets-watch-budget-save = Save limit
wallets-watch-budget-resume = Resume from now
wallets-watch-budget-resume-notice = This wallet reached its check limit before catching up. Resume from now starts at the latest wallet activity; activity since the last completed check will not be copied.
wallets-watch-budget-resume-tasks = Copy tasks stay paused until you resume each task in Copy Trading.
wallets-watch-budget-resume-ack = I understand missed activity will not be copied.
wallets-watch-budget-resumed = Watch resumed from the current wallet head
wallets-watch-budget-updated = Wallet watch limit updated
wallets-watch-helius-allow = Allow { -helius } catch-up if needed
wallets-watch-helius-try = Try to catch up using { -helius }
wallets-watch-helius-stop = Stop { -helius } catch-up for this wallet
wallets-watch-helius-description-approved = { -helius } catch-up is allowed for this wallet. Turning it off returns to standard checks, which may fall behind on a busy wallet.
wallets-watch-helius-description-available = { -helius } can check successful Solana transactions from the saved position without skipping the unchecked interval. It may use more provider credits and can still fall behind.
wallets-watch-helius-description-unavailable = { -helius } catch-up is unavailable. Configure an enabled { -helius } RPC endpoint to use it.
wallets-watch-helius-description-unsupported = No catch-up provider is supported for this watch. Resume from now is available if the watch reaches its limit.
wallets-watch-helius-allow-title = Allow { -helius } catch-up for this wallet
wallets-watch-helius-allow-message = { -helius } can check successful Solana transactions from the saved position without skipping the unchecked interval. It currently charges 10 credits per 100 full transactions returned, rounded up, with a 10 credit minimum per request. A check can make multiple requests; usage and provider pricing may vary. Copy tasks remain paused until resumed separately.
wallets-watch-helius-allow-confirm = Allow for this wallet
wallets-watch-helius-stop-message = This wallet will return to standard checks. A busy wallet may reach its watch limit and pause again. Other wallets and your { -helius } RPC configuration are unchanged.
wallets-watch-helius-stop-confirm = Stop for this wallet
wallets-watch-helius-stop-keep = Keep allowed
wallets-watch-helius-restored = Watch restored from saved progress; copy tasks remain paused
wallets-watch-helius-allowed = { -helius } catch-up allowed for this wallet when needed
wallets-watch-helius-stopped = { -helius } catch-up stopped for this wallet
wallets-watch-helius-update-failed = Wallet catch-up setting could not be updated

# wallets.html: Export Private Key dialog.
wallets-export-title = Export Private Key
wallets-export-warning-title = Critical Security Warning
wallets-export-warning-body = Never share your private key with anyone. Anyone with access to this key can steal all funds from this wallet.
wallets-export-key-label = Private Key (Base58)
wallets-export-copy =
    .title = Copy to clipboard
    .aria-label = Copy to clipboard
wallets-export-reveal = Reveal Key

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = Archive Wallet
wallets-archive-note = Archived wallets are not used in any operations but can be restored anytime.
wallets-archive-confirm = Yes, Archive
wallets-delete-title = Delete Wallet
wallets-delete-warning-title = This action cannot be undone!
wallets-delete-warning-body = Deleting this wallet will permanently remove it and its encrypted private key from this device.
wallets-delete-confirm = Yes, Delete

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = Import Wallets
wallets-bulk-import-submit = Import Wallets
wallets-bulk-step-upload = Upload File
wallets-bulk-step-map = Map Columns
wallets-bulk-step-results = Results
wallets-bulk-import-file-warning-body = Only import files from trusted sources. Private keys will be encrypted and stored securely on this device.
wallets-bulk-drop-title = Drop your file here
wallets-bulk-drop-subtitle = or click to browse
wallets-bulk-drop-formats = Supports CSV and Excel (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Remove file
wallets-bulk-map-subtitle = Match your file columns to wallet fields
wallets-bulk-preview-title = Preview (First 5 Rows)
wallets-bulk-summary-valid = <strong>{ $count }</strong> valid
wallets-bulk-summary-invalid = <strong>{ $count }</strong> invalid
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> duplicate
       *[other] <strong>{ $count }</strong> duplicates
    }
wallets-bulk-done = Done
wallets-bulk-file-invalid = Invalid file type. Please use CSV or Excel files.
wallets-bulk-preview-busy = Processing...
wallets-bulk-preview-fallback = Failed to process file
wallets-bulk-preview-failed = Failed to process file: { $reason }
wallets-bulk-column-select = -- Select column --
wallets-bulk-preview-empty = No data rows found in file
wallets-bulk-preview-status = Status
wallets-bulk-status-valid = Valid
wallets-bulk-status-duplicate = Duplicate
wallets-bulk-status-invalid = Invalid
wallets-bulk-import-busy = Importing...
wallets-bulk-import-toast =
    { $count ->
        [one] Imported { $count } wallet
       *[other] Imported { $count } wallets
    }
wallets-bulk-import-error = Import failed: { $reason }
wallets-bulk-result-success-title = Import Successful
wallets-bulk-result-success-detail =
    { $count ->
        [one] All { $count } wallet imported successfully
       *[other] All { $count } wallets imported successfully
    }
wallets-bulk-result-partial-title = Partial Success
wallets-bulk-result-partial-detail = { $imported } imported, { $failed } failed
wallets-bulk-result-failed-title = Import Failed
wallets-bulk-result-failed-detail =
    { $count ->
        [one] All { $count } wallet failed to import
       *[other] All { $count } wallets failed to import
    }
wallets-bulk-result-imported = Imported
wallets-bulk-result-failed = Failed

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = Export Wallets
wallets-bulk-export-format = Format
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Include archived wallets
wallets-bulk-export-safe-title = Safe Export
wallets-bulk-export-safe-body = Export wallet addresses and metadata only. No private keys included.
wallets-bulk-export-safe-submit = Export Addresses
wallets-bulk-export-or = or
wallets-bulk-export-danger-title = Dangerous Export
wallets-bulk-export-danger-body = Include private keys in the export. Anyone with this file can steal your funds.
wallets-bulk-export-danger-submit = Export with Private Keys
wallets-bulk-export-busy = Exporting...
wallets-bulk-export-done = Exported wallets to { $filename }
wallets-bulk-export-fallback = Export failed
wallets-bulk-export-error = Export failed: { $reason }
wallets-bulk-confirm-title = Confirm Dangerous Export
wallets-bulk-confirm-warning =
    { $count ->
        [one] You are about to export <strong>{ $count }</strong> private key. This is extremely dangerous!
       *[other] You are about to export <strong>{ $count }</strong> private keys. This is extremely dangerous!
    }
wallets-bulk-confirm-risk-steal = Anyone with this file can steal all funds
wallets-bulk-confirm-risk-share = Never share this file with anyone
wallets-bulk-confirm-risk-delete = Delete the file immediately after use
wallets-bulk-confirm-prompt = Type the phrase below to confirm
wallets-bulk-confirm-submit = Export Keys

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Token
wallets-holdings-col-balance = Balance
wallets-holdings-col-value = Value ({ -sol })
wallets-holdings-col-type = Type
wallets-holdings-col-decimals = Decimals
wallets-holdings-empty-title = No token holdings
wallets-holdings-empty-message = Tokens held by this wallet will appear here.
wallets-holdings-no-main = No main wallet
wallets-holdings-main-tag = Main
wallets-holdings-main-title = Main Wallet
wallets-holdings-tokens = Tokens
wallets-holdings-last-used = Last used
wallets-holdings-never = Never
wallets-holdings-search =
    .placeholder = Search by symbol or mint...
wallets-holdings-export = Export Key
wallets-holdings-export-tooltip = Export this wallet's private key
wallets-list-col-name = Name
wallets-list-col-balance = Balance ({ -sol })
wallets-list-col-type = Type
wallets-list-col-created = Created
wallets-list-col-actions = Actions
wallets-list-action-export = Export private key
wallets-list-action-archive = Archive wallet
wallets-list-action-restore = Restore wallet
wallets-list-action-delete = Delete permanently
wallets-list-count = Wallets
wallets-list-search =
    .placeholder = Search by name or address...
wallets-list-loading-title = Loading wallets…
wallets-list-loading-description = Preparing the selected wallet view.
wallets-secondaries-empty-title = No secondary wallets
wallets-secondaries-empty-message = Create additional wallets to organize your trading activities across multiple accounts.
wallets-secondaries-add = Add Wallet
wallets-archive-empty-title = No archived wallets
wallets-archive-empty-message = Wallets you archive will be safely stored here for future reference.

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = Wallet
wallets-watched-col-status = Status
wallets-watched-col-progress = Progress saved
wallets-watched-col-last-check = Last check
wallets-watched-unlabelled = Unlabelled wallet
wallets-watched-generic-name = wallet
wallets-watched-not-synced = Not synced yet
wallets-watched-not-checked = Not checked yet
wallets-watched-action-copy = Copy trade
    .title = Open this wallet in Copy Trading
wallets-watched-action-restore = Restore watch
wallets-watched-action-options = Watch options
wallets-watched-action-retry = Retry watch
wallets-watched-action-pause = Pause
wallets-watched-action-enable = Enable
wallets-watched-action-remove =
    .title = Remove
    .aria-label = Remove { $name }
wallets-watch-state-paused = Paused
wallets-watch-state-catching-up = Catching up
wallets-watch-state-watching = Watching
wallets-watch-state-streaming = Streaming
wallets-watch-state-polling = Polling
wallets-watched-detail-helius = Checking through { -helius } for this wallet.
wallets-watched-empty-title = No watched addresses
wallets-watched-empty-message = Use Watch Wallet to record a public wallet's on-chain activity.
wallets-watched-count = Watched
wallets-watched-search =
    .placeholder = Search watched wallets...
wallets-watched-add = Watch Wallet
wallets-watched-refresh = Refresh watched wallets
wallets-watched-loading-title = Loading watched wallets...
wallets-watched-loading-description = Fetching observation targets.
wallets-watched-load-error-title = Watched addresses could not be loaded
wallets-watched-load-error-description = Use refresh to try again.
wallets-watched-address-invalid = Enter a valid Solana wallet address.
wallets-watched-added = Wallet watch added
wallets-watched-duplicate = That wallet is already watched.
wallets-watched-add-failed = Wallet watch could not be added.
wallets-watched-retried = Wallet watch restored with its saved cursor
wallets-watched-paused = Wallet watch paused
wallets-watched-enabled = Wallet watch enabled
wallets-watched-removed = Wallet watch removed
wallets-watched-update-failed = Wallet watch could not be updated
wallets-setup-gate-title = Wallets need setup
