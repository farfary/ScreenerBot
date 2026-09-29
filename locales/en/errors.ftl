# API error messages. Each key names the failed operation; technical causes are
# carried separately in the response `details` and are never part of a message.

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
errors-security-token-required = Security token required. This endpoint is only accessible from within ScreenerBot.

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
errors-telegram-disabled = Telegram is not enabled
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
errors-wallet-watch-approval-failed = Helius approval could not be updated
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
errors-tools-router-disabled = { $router } is disabled in Settings > Swaps
errors-tools-router-unknown = Unknown swap router '{ $router }'
errors-tools-session-type-mismatch = Session is { $actual } not { $expected }
errors-tools-session-not-found = Session not found
errors-tools-session-complete = Session is already complete
