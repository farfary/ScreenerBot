# API error messages. Each key names the failed operation; technical causes are
# carried separately in the response `details` and are never part of a message.

# Framing for an operation message followed by its technical cause.
errors-with-details = { $message }: { $details }

# Configuration
errors-config-save-failed = Failed to save configuration

# Authentication
errors-auth-current-password-incorrect = Current password is incorrect
errors-auth-current-password-required = Current password is required to change password
errors-auth-password-too-short = Password must be at least 4 characters
errors-auth-password-too-long = Password must be at most 128 characters
errors-auth-hash-failed = Failed to hash password
errors-auth-not-enabled = Authentication is not enabled
errors-auth-no-password = No password has been configured
errors-auth-password-incorrect = Incorrect password
errors-auth-required = Authentication required. Please log in to access this endpoint.
errors-auth-totp-invalid = Invalid or expired 2FA code
errors-auth-totp-verify-failed = Failed to verify 2FA code
errors-auth-totp-password-required = Password must be set before enabling 2FA
errors-auth-totp-uri-failed = Failed to generate TOTP URI
errors-auth-totp-qr-failed = Failed to generate QR code
errors-auth-totp-secret-required = Secret is required
errors-auth-totp-save-failed = Failed to save TOTP configuration
errors-auth-totp-code-invalid = Invalid verification code. Please check the code and try again.
errors-auth-totp-code-verify-failed = Failed to verify code

# Request security
errors-security-invalid-local-request = Request must originate from the local dashboard
errors-security-invalid-token = Invalid security token
errors-security-token-required = Security token required. This endpoint is only accessible from within { -brand }.

# Lockscreen
errors-lockscreen-no-password-set = No password has been set
errors-lockscreen-invalid-type = Invalid password type. Must be 'pin4', 'pin6', or 'text'
errors-lockscreen-invalid-format = Password does not match the selected type
errors-lockscreen-no-current-password = No password is currently set
errors-lockscreen-password-incorrect = Password is incorrect
errors-lockscreen-enable-needs-password = Cannot enable lockscreen without setting a password first

# Account
errors-account-signin-failed = Sign-in failed
# The reason is the account server's own wording, shown exactly as sent.
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = Sign-in is unavailable
errors-account-browser-open-failed = Could not open your browser. Open your default browser and try again.
errors-account-signup-open-failed = Could not open the sign-up page. Open screenerbot.io/signup in your browser.
errors-account-credentials-required = Enter your email address and password.
errors-account-signout-failed = Sign-out failed
errors-account-gateway-update-failed = Could not update the gateway setting

# Localization
errors-i18n-locale-not-registered = Locale is not registered
errors-i18n-catalog-encode-failed = Catalog could not be encoded

# System
errors-system-paths-init-failed = Could not create the application directories
errors-system-open-data-failed = Could not open the data folder
errors-system-url-empty = URL cannot be empty
errors-system-open-url-failed = Could not open the URL

# Initialization
errors-initialization-required = Bot initialization is required before accessing this endpoint. Please complete the initialization process through the web interface.
errors-initialization-explore-unavailable = Explore Mode has no wallet connected. Complete setup to use wallets and copy trading.
errors-initialization-onboarding-update-failed = Failed to update onboarding state
errors-initialization-validation-required = Credential verification is required before saving setup
errors-initialization-encrypt-failed = Failed to encrypt private key

# Dashboard state
errors-ui-state-save-failed = Failed to save state
errors-ui-state-clear-failed = Failed to clear state

# Agent control
errors-agent-config-failed = Agent-control configuration failed
errors-agent-invalid-parameters = Invalid parameters
errors-agent-wallet-key-material = Wallet key material is not agent-accessible
errors-agent-store-failed = Agent-control store failed
errors-agent-invalid-pairing-request = Invalid pairing request
errors-agent-pairing-rejected = Pairing credential rejected
errors-agent-disabled = Agent control is disabled
errors-agent-approval-not-pending = Approval is no longer pending
errors-agent-approval-not-found = Approval not found
errors-agent-bridge-task-failed = Agent-control bridge task failed
errors-agent-task-failed = agent-control task failed
errors-agent-pairing-not-found = No active pairing with that id
errors-agent-permissions-update-failed = Failed to update permissions

# Connectivity
errors-connectivity-endpoint-not-found = Endpoint '{ $endpoint }' not found or not monitored

# Copy trading
errors-copy-task-not-found = Copy task not found
errors-copy-holding-not-found = No open paper holding in this token
errors-copy-live-confirmation-required = Arming live copy trading requires explicit confirmation
errors-copy-task-invalid = Invalid copy task
errors-copy-request-rejected = Copy trading request rejected
errors-copy-task-limit = Maximum active copy tasks reached
errors-copy-watch-rejected = Copy target could not be watched
errors-copy-live-unavailable = Live copy trading is unavailable
errors-copy-task-live = Pause the live task before deleting it
errors-copy-task-owns-positions = Copy task still owns open positions
errors-copy-request-failed = Copy trading request failed

# Strategies
errors-strategies-not-found = Strategy not found
errors-strategies-invalid-type = Invalid strategy type. Must be ENTRY or EXIT
errors-strategies-list-failed = Failed to get strategies
errors-strategies-get-failed = Failed to get strategy
errors-strategies-serialize-rules-failed = Failed to serialize rules
errors-strategies-invalid-rules-json = Invalid rules JSON
errors-strategies-already-exists = Strategy with ID '{ $id }' already exists
errors-strategies-validation-failed = Strategy validation failed
errors-strategies-create-failed = Failed to create strategy
errors-strategies-update-failed = Failed to update strategy
errors-strategies-update-enabled-failed = Failed to update strategy enabled state
errors-strategies-delete-failed = Failed to delete strategy
errors-strategies-deploy-failed = Failed to deploy strategy
errors-strategies-no-performance = No performance data available for this strategy
errors-strategies-performance-failed = Failed to get performance stats
errors-strategies-schemas-failed = Failed to get condition schemas
errors-strategies-evaluation-failed = Strategy evaluation failed

# Transactions
errors-transactions-own-wallet-unavailable = The main wallet is not configured
errors-transactions-invalid-subject = Transaction subject is not a valid Solana address
errors-transactions-subject-not-watched = Transaction subject is not a watched wallet
errors-transactions-watch-store-unavailable = Watched wallets are not available

# Wallet
errors-wallet-unavailable = Main wallet is not available
errors-wallet-changed = The main wallet changed; refresh and try again
errors-wallet-qr-failed = Could not generate the wallet QR code

# Updates
errors-updates-none-available = No update available to download
errors-updates-version-changed = The available update changed; check for updates again
errors-updates-check-failed = Update check failed
errors-updates-download-failed = Update download could not be started
errors-updates-history-unavailable = Release history is unavailable
errors-updates-apply-failed = Could not apply the update
errors-updates-install-failed = Could not open the update installer

# Telegram
errors-telegram-settings-update-failed = Failed to update settings
errors-telegram-disabled = { -telegram } is not enabled
errors-telegram-not-configured = Bot token or chat ID not configured
errors-telegram-send-failed = Failed to send message
errors-telegram-notifier-failed = Failed to create notifier
errors-telegram-token-required = Bot token must be configured first
errors-telegram-discovery-failed = Failed to start discovery
errors-telegram-chat-select-failed = Failed to select chat

# Assistant chat
errors-chat-message-empty = Message cannot be empty
errors-chat-message-too-long = Message exceeds maximum length of 10,000 characters
errors-chat-database-unavailable = Chat database not initialized
errors-chat-session-not-found = Chat session { $id } not found
errors-chat-session-validate-failed = Failed to validate session
errors-chat-engine-unavailable = Chat engine not initialized
errors-chat-process-failed = Failed to process chat message
errors-chat-stream-serialize-failed = Failed to serialize chat event
errors-chat-sessions-list-failed = Failed to list chat sessions
errors-chat-session-create-failed = Failed to create chat session
errors-chat-session-get-failed = Failed to get chat session
errors-chat-messages-get-failed = Failed to get chat messages
errors-chat-session-delete-failed = Failed to delete chat session
errors-chat-messages-load-failed = Failed to get messages
errors-chat-summarize-empty = Cannot summarize empty chat session
errors-chat-provider-invalid = Invalid provider: { $provider }
errors-chat-summary-save-failed = Failed to save summary
errors-chat-title-empty-session = Cannot generate title for empty chat session
errors-chat-no-user-message = No user messages found in session
errors-chat-title-save-failed = Failed to update session title
errors-chat-confirmation-save-failed = Failed to save confirmation response
errors-chat-confirmation-failed = Failed to process confirmation
errors-chat-summary-failed = Failed to generate summary

# Assistant automation
errors-automation-database-unavailable = Database not initialized
errors-automation-tasks-list-failed = Failed to list tasks
errors-automation-name-empty = Task name cannot be empty
errors-automation-instruction-empty = Task instruction cannot be empty
errors-automation-schedule-type-invalid = Invalid schedule_type. Must be: interval, daily, or weekly
errors-automation-schedule-value-invalid = Invalid schedule_value
errors-automation-task-create-failed = Failed to create task
errors-automation-task-not-found = Task not found
errors-automation-task-get-failed = Failed to get task
errors-automation-schedule-invalid = Invalid schedule
errors-automation-tool-permissions-invalid = tool_permissions must be 'full' or 'readonly'
errors-automation-priority-invalid = priority must be 'low', 'medium', or 'high'
errors-automation-task-update-failed = Failed to update task
errors-automation-task-running-delete = Cannot delete task while it is running
errors-automation-task-delete-failed = Failed to delete task
errors-automation-task-toggle-failed = Failed to toggle task
errors-automation-task-disabled = Cannot run a disabled task
errors-automation-task-already-running = Task is already running
errors-automation-runs-list-failed = Failed to list runs
errors-automation-recent-runs-failed = Failed to list recent runs
errors-automation-run-not-found = Run not found
errors-automation-run-get-failed = Failed to get run
errors-automation-stats-failed = Failed to get stats

# LLM providers
errors-llm-config-update-failed = Failed to update LLM config
errors-llm-provider-unknown = Unknown provider: { $provider }
errors-llm-manager-unavailable = LLM manager not initialized
errors-llm-provider-disabled = Provider '{ $provider }' is not configured or disabled
errors-llm-provider-config-update-failed = Failed to update provider config
errors-llm-provider-test-failed = Provider test failed
# The reason is the provider's own wording, shown exactly as sent.
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = Failed to update analysis config
errors-llm-analysis-unavailable = Analysis engine not initialized
errors-llm-analysis-disabled = LLM features are disabled. Enable [llm] first.
errors-llm-analysis-priority-invalid = Invalid priority: '{ $priority }'. Use 'high', 'medium', or 'low'.
errors-llm-analysis-evaluation-failed = Model analysis failed
errors-llm-analysis-instructions-list-failed = Failed to list instructions
errors-llm-analysis-instruction-not-found = Instruction { $id } not found
errors-llm-analysis-instruction-get-failed = Failed to get instruction
errors-llm-analysis-instruction-created-retrieve-failed = Failed to retrieve created instruction
errors-llm-analysis-instruction-create-failed = Failed to create instruction
errors-llm-analysis-instruction-updated-retrieve-failed = Failed to retrieve updated instruction
errors-llm-analysis-instruction-update-failed = Failed to update instruction
errors-llm-analysis-instruction-delete-failed = Failed to delete instruction
errors-llm-analysis-instructions-reorder-failed = Failed to reorder instructions
errors-llm-analysis-decisions-list-failed = Failed to list decision history
errors-llm-analysis-decision-not-found = Decision { $id } not found
errors-llm-analysis-decision-get-failed = Failed to get decision

# Wallets
errors-wallets-list-failed = Failed to list wallets
errors-wallets-name-empty = Wallet name cannot be empty
errors-wallets-create-failed = Failed to create wallet
errors-wallets-key-empty = Private key cannot be empty
errors-wallets-already-exists = Wallet already exists
errors-wallets-key-invalid = Invalid private key format
errors-wallets-import-failed = Failed to import wallet
errors-wallets-summary-failed = Failed to get wallets summary
errors-wallets-no-main-wallet = No main wallet configured
errors-wallets-main-get-failed = Failed to get main wallet
errors-wallets-not-found = Wallet not found
errors-wallets-get-failed = Failed to get wallet
errors-wallets-update-failed = Failed to update wallet
errors-wallets-delete-failed = Failed to delete wallet
errors-wallets-export-failed = Failed to export wallet
errors-wallets-set-main-failed = Failed to set main wallet
errors-wallets-archive-failed = Failed to archive wallet
errors-wallets-restore-failed = Failed to restore wallet
errors-wallets-export-format-unsupported = Only CSV format is currently supported
# The confirmation is the exact phrase the request must carry.
errors-wallets-export-confirmation-required = You must confirm by providing: "{ $confirmation }"
errors-wallets-export-no-ids = No wallet IDs provided
errors-wallets-export-bulk-failed = Failed to export wallets
errors-wallets-export-no-match = No wallets found matching the provided IDs
errors-wallets-import-file-too-large = File exceeds maximum size of { $megabytes }MB
errors-wallets-import-read-failed = Failed to read uploaded file
errors-wallets-import-no-file = No file uploaded. Use 'file' field in multipart form
errors-wallets-import-encoding-invalid = CSV file must be UTF-8 encoded
errors-wallets-import-csv-parse-failed = Failed to parse CSV file
errors-wallets-import-excel-parse-failed = Failed to parse Excel file
errors-wallets-import-format-unsupported = Unsupported file format. Use .csv, .xlsx, or .xls
errors-wallets-import-file-empty = File contains no data rows
errors-wallets-import-existing-check-failed = Failed to check existing wallets
errors-wallets-import-mapping-invalid = Missing required columns: { $columns }
errors-wallets-import-session-not-found = Import session not found or expired. Please upload the file again
errors-wallets-import-no-valid-rows = No valid rows to import

# Wallet watching
errors-wallet-watch-list-failed = Failed to list watch targets
errors-wallet-watch-address-empty = Address cannot be empty
errors-wallet-watch-add-failed = Failed to add watch target
errors-wallet-watch-remove-failed = Failed to remove watch target
errors-wallet-watch-update-failed = Failed to update watch target
errors-wallet-watch-budget-failed = Watch budget could not be updated
errors-wallet-watch-resume-failed = Watch could not be resumed
errors-wallet-watch-approval-failed = { -helius } approval could not be updated
errors-wallet-watch-status-failed = Failed to get watch status

# Tools
errors-tools-wallet-failed = Failed to get wallet
errors-tools-wallet-address-failed = Failed to get wallet address
errors-tools-accounts-scan-failed = Failed to scan accounts
errors-tools-token-accounts-scan-failed = Failed to scan token accounts
errors-tools-token-accounts-get-failed = Failed to get token accounts
errors-tools-cleanup-failed = Cleanup failed
errors-tools-cache-clear-failed = Failed to clear cache
errors-tools-no-tokens = No tokens selected for burning
errors-tools-burn-failed = Failed to burn tokens
errors-tools-favorites-list-failed = Failed to get favorites
errors-tools-favorite-type-invalid = Invalid tool type. Must be one of: { $types }
errors-tools-favorite-add-failed = Failed to add favorite
errors-tools-favorite-not-found = Favorite not found
errors-tools-favorite-update-failed = Failed to update favorite
errors-tools-favorite-delete-failed = Failed to delete favorite
errors-tools-favorite-use-failed = Failed to update use count
errors-tools-pool-search-failed = Pool search failed for token { $mint }
errors-tools-watched-list-failed = Failed to list watched tokens
errors-tools-watched-add-failed = Failed to add watched token
errors-tools-watched-delete-failed = Failed to delete watched token
errors-tools-mint-invalid = Invalid token mint address
errors-tools-wallets-get-failed = Failed to get wallets
errors-tools-balance-failed = Failed to get wallet balance
errors-tools-session-active = Another multi-wallet operation is already in progress
# The reason is the tool configuration check's own wording, shown exactly as produced.
errors-tools-config-invalid = Invalid tool configuration: { $reason }
errors-tools-config-rejected = Invalid tool configuration
errors-tools-consolidate-failed = Failed to consolidate wallets
errors-tools-ata-cleanup-failed = Failed to cleanup ATAs
errors-tools-routers-unavailable = Swap routers are not ready yet
errors-tools-router-disabled-chain-settings = { $router } is disabled in Settings > Chains
errors-tools-router-unknown = Unknown swap router '{ $router }'
errors-tools-session-type-mismatch = Session is { $actual } not { $expected }
errors-tools-session-not-found = Session not found
errors-tools-session-complete = Session is already complete

# Configuration import and reload
errors-config-reload-failed = Failed to reload config
errors-config-reset-failed = Failed to reset config
errors-config-disk-parse-failed = Failed to parse disk config
errors-config-disk-read-failed = Failed to read disk config
errors-config-update-failed = Failed to update config
errors-config-import-not-object = Config must be a JSON object
errors-config-import-no-sections = No valid sections found to import
errors-config-import-validation-failed = Config validation failed. No changes were applied.
errors-config-import-commit-failed = Failed to commit config changes
errors-config-import-failed = Failed to import config

# Filtering
errors-filtering-analytics-failed = Failed to fetch analytics
errors-filtering-refresh-failed = Failed to rebuild filtering snapshot
errors-filtering-rejection-stats-failed = Failed to fetch rejection statistics
errors-filtering-rejected-tokens-failed = Failed to fetch rejected tokens
errors-filtering-csv-header-failed = Failed to write CSV header
errors-filtering-csv-record-failed = Failed to write CSV record
errors-filtering-csv-finalize-failed = Failed to finalize CSV
errors-filtering-export-response-failed = Failed to build response

# OHLCV
errors-ohlcv-fetch-failed = Failed to fetch OHLCV data
errors-ohlcv-pools-failed = Failed to fetch pools
errors-ohlcv-gaps-failed = Failed to fetch gaps
errors-ohlcv-refresh-failed = Failed to refresh
errors-ohlcv-monitor-start-failed = Failed to start monitoring
errors-ohlcv-monitor-stop-failed = Failed to stop monitoring
errors-ohlcv-activity-failed = Failed to record activity
errors-ohlcv-list-failed = Failed to list OHLCV tokens
errors-ohlcv-delete-failed = Failed to delete token data
errors-ohlcv-clear-failed = Failed to clear OHLCV cache
errors-ohlcv-cleanup-failed = Failed to cleanup inactive tokens

# Trader and manual trading
errors-trade-already-running = trader is already running
errors-trade-already-stopped = trader is already stopped
errors-trade-config-update-failed = trader config update failed
errors-trade-trader-unavailable = complete wallet and RPC setup before using the auto trader
errors-trade-force-stop-active = the emergency stop is active; clear it first
errors-trade-template-not-found = no trader template named { $template }
errors-trade-manual-force-stopped = manual trading is disabled while the emergency stop is active
errors-trade-core-services-not-ready = core services are not ready for trading: { $pending }
errors-trade-mint-invalid = invalid token mint address { $mint }
errors-trade-blacklisted = token { $mint } is blacklisted
errors-trade-slippage-invalid = slippage { $slippage }% must be in (0, { $maximum }]
errors-trade-percentage-invalid = sell percentage { $percentage } must be in (0, 100]
errors-trade-record-failed = could not record manual trade
errors-trade-task-cancelled = the manual trade stopped before it answered because the app is shutting down; its position holds the outcome
errors-trade-no-open-position = no open position for token { $mint }
errors-trade-size-invalid = Invalid trade size { $amount } { -sol }
errors-trade-management-invalid = invalid position management { $management }
errors-trade-strategy-evaluation-failed = strategy evaluation for token { $mint } failed
errors-trade-token-data-missing = token data unavailable for { $mint }
errors-trade-endpoints-unhealthy = no healthy endpoints available
errors-trade-dependency-failed = { $dependency } dependency failed
errors-trade-storage-failed = The trade request could not be completed
errors-trade-manual-failed = Manual trade failed
# The reason is the trader's own wording for a refused trade, shown exactly as produced.
errors-trade-manual-refused = { $reason }
errors-trade-swap-too-large = No swap route could build a transaction small enough to send
    .hint = The best route needed more accounts than one transaction can carry, so nothing was sent and nothing was spent. Try again in a moment for a different route, or enable another swap router.
errors-trade-wallet-not-configured = Wallet not configured
errors-trade-amount-sol-invalid = amount_sol is required for buy and must be positive
errors-trade-no-tokens-in-wallet = No tokens found in wallet for this position. Token balance is 0; the position cannot be closed via swap.
errors-trade-percentage-range = percentage must be in (0, 100]
errors-trade-amount-tokens-invalid = amount_tokens must be positive
errors-trade-sell-amount-zero = Computed sell amount is zero

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = Swap routing is not ready yet
    .hint = The swap service is still starting. Wait for services to become ready, then retry.
errors-trade-quote-no-routers-enabled = No swap providers are enabled
    .hint = Enable at least one swap router in Trader settings, then try again.
errors-trade-quote-not-tradable = This token isn't tradable right now
    .hint = No liquidity or swap route is available. The token may be unlaunched, abandoned, or have no pool. Try again later or choose another token.
errors-trade-quote-no-route = No swap route available
    .hint = No provider could route this trade at the requested amount. Try a smaller amount, or try again in a moment.
errors-trade-quote-rate-limited = Swap providers are rate limiting us
    .hint = The swap providers are throttling requests. Wait a few seconds and retry.
errors-trade-quote-timeout = Quote request timed out
    .hint = The swap providers didn't respond in time. Check your connection and retry.
errors-trade-quote-router-rejected = The quote was refused
    .hint = A provider returned a quote that failed our safety checks and was discarded. Retry to fetch a fresh one.
errors-trade-quote-not-offered-exact-out = No enabled swap route quotes an exact output amount
    .hint = The enabled swap routes price a trade only from the amount spent. Enter the amount to spend, or enable another swap router.
errors-trade-quote-not-offered-unsupported-venue = No enabled swap route trades this token's pool
    .hint = The token trades on an exchange the enabled swap routes do not support yet. Enable another swap router, then try again.
errors-trade-quote-unavailable = Couldn't fetch a quote
    .hint = The swap providers couldn't quote this trade. Try again in a moment.

# Positions
errors-positions-not-found = Position not found
errors-positions-already-closed = Position is already closed
errors-positions-force-close-failed = Failed to force-close position
errors-positions-already-archived = Position is already archived
errors-positions-unverified-entry-archive = This position's buy is not confirmed yet. Archive it after it confirms.
errors-positions-not-archived = Position is not archived
errors-positions-archive-failed = Failed to archive position
errors-positions-unarchive-failed = Failed to unarchive position
errors-positions-unarchive-duplicate-open = This token has more than one open position ({ $positions }), and only one of them can be active. Close or archive the others first.
errors-positions-management-invalid = Copy-owned management requires a copy-origin position
errors-positions-management-failed = Failed to update position management
errors-positions-delete-failed = Failed to delete position
errors-positions-bulk-delete-failed = Failed to delete archived positions
errors-positions-detail-failed = Failed to load position details
errors-positions-resolve-failed = Failed to resolve position
errors-positions-wrapped-sol-activity = Wrapped SOL has no token activity

# Tokens
errors-tokens-database-unavailable = Token database not available
errors-tokens-blacklist-failed = Failed to blacklist token
errors-tokens-blacklist-internal = Internal error during blacklist operation
errors-tokens-unblacklist-failed = Failed to remove from blacklist
errors-tokens-unblacklist-internal = Internal error during unblacklist operation
errors-tokens-blacklist-status-failed = Failed to check blacklist status
errors-tokens-blacklist-status-internal = Internal error during blacklist status check
errors-tokens-favorites-fetch-failed = Failed to fetch favorites
errors-tokens-favorite-add-failed = Failed to add favorite
errors-tokens-favorite-remove-failed = Failed to remove favorite
errors-tokens-favorite-update-failed = Failed to update favorite
errors-tokens-detail-not-found = Token not found in database or external sources
errors-tokens-fetch-failed = Failed to fetch token
errors-tokens-refresh-all-failed = All data sources failed
errors-tokens-refresh-failed = Failed to refresh token
errors-tokens-search-query-required = Search query 'q' is required
errors-tokens-search-failed = Token search failed

# Actions and services
errors-actions-not-found = Action { $id } not found
errors-services-not-found = Service '{ $name }' not found
