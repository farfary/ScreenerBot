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
wallets-modal-close =
    .aria-label = Close modal
wallets-summary-native = { -sol }
wallets-copied-private-key = Private key

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Main Wallet
wallets-tab-other = Other Wallets
wallets-tab-watched = Watched
wallets-refresh-failed = Could not refresh wallets
wallets-action-failed = Failed
wallets-toast-failed = Failed: { $reason }
wallets-export-busy = Decrypting...
wallets-export-revealed = Key revealed - handle with care

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
wallets-watch-budget-label-signatures = Signatures Checked per Check
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

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Token
wallets-holdings-col-balance = Balance
wallets-holdings-col-value = Value ({ -sol })
wallets-holdings-col-type = Type
wallets-holdings-col-decimals = Decimals
wallets-holdings-empty-title = No token holdings
wallets-holdings-unreadable-title = Holdings unknown
wallets-holdings-empty-message = Tokens held by this wallet will appear here.
wallets-funding-title = Fund the trading wallet to start
wallets-funding-message = This wallet holds no { -sol }. Send { -sol } to this address from an exchange or another wallet; each buy spends its amount plus the network fees.
wallets-holdings-no-main = No main wallet
wallets-holdings-main-title = Main Wallet
wallets-holdings-tokens = Tokens
wallets-holdings-last-used = Last used
wallets-holdings-never = Never
wallets-holdings-search =
    .placeholder = Search by symbol or mint...
wallets-holdings-export = Export Key
wallets-holdings-export-tooltip = Export this wallet's private key
wallets-holdings-replace-tooltip = Trade with another wallet; this wallet's key and history are kept
wallets-list-col-name = Name
wallets-list-col-balance = Balance ({ -sol })
wallets-list-col-type = Type
wallets-list-col-created = Created
wallets-list-col-replaced = Replaced
wallets-list-replaced-on = On { $date }
wallets-replace-abandoned-title = Wallet replacement cancelled
wallets-replace-abandoned-open-positions = The trading wallet had open positions ({ $count }) at the restart, so the replacement was cancelled. The trading wallet is unchanged; the new key is kept under Other Wallets.
wallets-replace-abandoned-positions-unreadable = The positions store could not be read at the restart, so the replacement was cancelled. The trading wallet is unchanged; the new key is kept under Other Wallets.
wallets-replace-abandoned-vault-changed = The wallet vault no longer matched the staged replacement at the restart, so the replacement was cancelled. The trading wallet is unchanged; the new key is kept under Other Wallets.
wallets-list-col-actions = Actions
wallets-list-action-export = Export private key
wallets-list-count = Wallets
wallets-list-search =
    .placeholder = Search by name or address...
wallets-list-loading-title = Loading wallets…
wallets-list-loading-description = Preparing the selected wallet view.
wallets-list-kept-empty-message = Replaced trading wallets and wallets from earlier versions appear here so you can still export their keys.
wallets-list-previous-trading-wallet = Previous trading wallet
wallets-other-empty-title = No other wallets

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
