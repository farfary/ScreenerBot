# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Tools
tools-category-wallet = Wallet
tools-category-token = Token
tools-category-single-token = Single Token
tools-category-utilities = Utilities
tools-sidebar-hint = Select a tool to get started
tools-help-button =
    .aria-label = Show help for this tool
tools-help-unavailable = Help not available
tools-placeholder-title = Select a Tool
tools-placeholder-subtitle = Choose a tool from the sidebar to get started
tools-placeholder-hint-wallets = Wallet tools help manage your Solana wallets
tools-placeholder-hint-secure = All operations are secured and reversible where possible

# Status dot tooltip of a tool in the navigation. Ids are the nav `data-status` values.
tools-status-ready = Ready to use
tools-status-coming = Coming soon
tools-status-beta = Beta - may have bugs
tools-status-disabled = Currently disabled
# Badge of a tool that is not available yet.
tools-status-badge-coming = Coming Soon
tools-status-badge-beta = Beta
tools-toast-coming-soon = This tool is coming soon
tools-toast-disabled = This tool is currently disabled

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = Wallet Cleanup
tools-tool-wallet-cleanup-summary = Close empty ATAs
tools-tool-wallet-cleanup-description = Close empty Associated Token Accounts to reclaim { -sol }
tools-tool-burn-tokens-title = Burn Tokens
tools-tool-burn-tokens-summary = Permanently destroy tokens
tools-tool-burn-tokens-description = Permanently destroy tokens from your wallet
tools-tool-token-analyzer-title = Token Analyzer
tools-tool-token-analyzer-summary = Deep token analysis
tools-tool-token-analyzer-description = Deep analysis of any Solana token with multi-dimensional insights
tools-tool-create-token-title = Create Token
tools-tool-create-token-summary = Deploy new SPL token
tools-tool-create-token-description = Deploy a new SPL token on Solana
tools-tool-trade-watcher-title = Trade Watcher
tools-tool-trade-watcher-summary = Monitor trades & auto-action
tools-tool-trade-watcher-description = Monitor token trades and trigger automatic buy/sell actions
tools-tool-token-watch-title = Holder Watch
tools-tool-token-watch-summary = Track new token holders
tools-tool-token-watch-description = Track and monitor new token holders in real-time
tools-tool-buy-multi-wallets-title = Multi-Buy
tools-tool-buy-multi-wallets-summary = Coordinate buys across wallets
tools-tool-buy-multi-wallets-description = Execute coordinated buy orders across multiple wallets with randomized amounts
tools-tool-sell-multi-wallets-title = Multi-Sell
tools-tool-sell-multi-wallets-summary = Coordinate sells across wallets
tools-tool-sell-multi-wallets-description = Execute coordinated sell orders across multiple wallets with { -sol } consolidation
tools-tool-wallet-consolidation-title = Wallet Consolidation
tools-tool-wallet-consolidation-nav-title = Consolidation
tools-tool-wallet-consolidation-summary = Consolidate wallet funds
tools-tool-wallet-consolidation-description = Consolidate { -sol } and tokens from sub-wallets back to main wallet
tools-tool-airdrop-checker-title = Airdrop Checker
tools-tool-airdrop-checker-summary = Check pending airdrops
tools-tool-airdrop-checker-description = Check for pending airdrops and claimable rewards
tools-tool-wallet-generator-title = Wallet Generator
tools-tool-wallet-generator-summary = Generate new keypairs
tools-tool-wallet-generator-description = Generate new Solana keypairs securely

## Shared by the tools

tools-validation-mint-required = Please enter a token mint address
tools-validation-mint-format = Invalid token mint address format
tools-validation-mint-invalid = Please enter a valid mint address

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Token Details
tools-create-token-name-label = Token Name
tools-create-token-name-input =
    .placeholder = My Token
tools-create-token-symbol-label = Symbol
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Decimals
tools-create-token-supply-label = Initial Supply
tools-create-token-description-label = Description
tools-create-token-description-input =
    .placeholder = Token description...
tools-create-token-image-title = Token Image
tools-create-token-image-drop = Drop image here or click to upload
tools-create-token-image-hint = Recommended: 512x512 PNG
tools-create-token-action-preview = Preview
tools-create-token-action-create = Create Token

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Loading settings...
tools-holder-watch-saved = Holder Watch settings saved
tools-holder-watch-save-failed = Failed to save settings
tools-holder-watch-save-error = Error saving settings
tools-holder-watch-settings-title = Holder Watch Settings
tools-holder-watch-enabled-label = Enable Holder Watching
tools-holder-watch-interval-label = Check Interval (seconds)
tools-holder-watch-interval-hint = How often to check holder counts (10-3600s)
tools-holder-watch-max-tokens-label = Max Watched Tokens
tools-holder-watch-max-tokens-hint = Maximum tokens to watch simultaneously
tools-holder-watch-notify-new-label = Notify on New Holders
tools-holder-watch-notify-drop-label = Notify on Holder Drop
tools-holder-watch-min-change-label = Min Holder Change
tools-holder-watch-min-change-hint = Minimum holder change to trigger notification
tools-holder-watch-drop-percent-label = Holder Drop Threshold (%)
tools-holder-watch-drop-percent-hint = Percentage drop to trigger alert
tools-holder-watch-action-save = Save Settings
tools-holder-watch-tokens-title = Watched Tokens
tools-holder-watch-token-input =
    .placeholder = Enter token mint address...
tools-holder-watch-empty = No tokens being watched
tools-holder-watch-empty-hint = Add a token mint address above to start watching
tools-holder-watch-coming-soon = Token watching feature coming soon

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Analyze Token
tools-analyzer-mint-input =
    .placeholder = Paste token mint address...
tools-analyzer-action-analyze = Analyze
tools-analyzer-action-analyzing = Analyzing...
tools-analyzer-action-copy-report = Copy Report
tools-analyzer-loading = Analyzing token...
tools-analyzer-failed = Failed to analyze token
tools-analyzer-empty = Enter a token mint address to analyze
tools-analyzer-empty-hint = Get comprehensive insights on any Solana token
tools-analyzer-tab-overview = Overview
tools-analyzer-tab-security = Security
tools-analyzer-tab-market = Market
tools-analyzer-tab-liquidity = Liquidity
tools-analyzer-unknown-token = Unknown Token

# Token header actions.
tools-analyzer-favorite-add =
    .title = Add to Favorites
    .aria-label = Add to Favorites
tools-analyzer-favorite-already = Already in Favorites
# $symbol is the token symbol.
tools-analyzer-favorite-added = Added { $symbol } to favorites
tools-analyzer-favorite-failed = Failed to add to favorites
tools-analyzer-blacklist-add =
    .title = Add to Blacklist
    .aria-label = Add to Blacklist
tools-analyzer-blacklist-title = Blacklist Token
# $symbol is the token symbol.
tools-analyzer-blacklist-message = Blacklist { $symbol }? This token will be excluded from trading.
tools-analyzer-blacklist-confirm = Blacklist
tools-analyzer-blacklisted = Blacklisted
# $symbol is the token symbol.
tools-analyzer-blacklist-done = Blacklisted { $symbol }
tools-analyzer-blacklist-failed = Failed to blacklist token

# Overview tab.
tools-analyzer-card-quick-stats = Quick Stats
tools-analyzer-card-market-summary = Market Summary
tools-analyzer-card-token-info = Token Information
tools-analyzer-stat-holders = Holders
tools-analyzer-stat-decimals = Decimals
tools-analyzer-stat-safety-score = Safety Score
tools-analyzer-stat-pools = Pools
tools-analyzer-stat-volume-24h = 24h Volume
tools-analyzer-stat-change-24h = 24h Change
tools-analyzer-stat-market-cap = Market Cap
tools-analyzer-stat-liquidity = Liquidity
tools-analyzer-info-mint = Mint Address
tools-analyzer-info-description = Description
tools-analyzer-info-supply = Supply

# Security tab.
tools-analyzer-security-empty = No security data available
tools-analyzer-security-empty-hint = Security analysis is not available for this token
tools-analyzer-card-safety-score = Safety Score
tools-analyzer-score-good = Good
tools-analyzer-score-moderate = Moderate
tools-analyzer-score-risky = Risky
# $score is the formatted raw risk score.
tools-analyzer-raw-score = Raw Risk Score: { $score }
tools-analyzer-card-authorities = Token Authorities
tools-analyzer-authority-mint = Mint Authority
tools-analyzer-authority-freeze = Freeze Authority
tools-analyzer-authority-transfer-fee = Transfer Fee
tools-analyzer-authority-mutable = Mutable
tools-analyzer-authority-active = Active
tools-analyzer-authority-revoked = Revoked
tools-analyzer-card-holder-concentration = Holder Concentration
tools-analyzer-top-holders = held by top 10 holders
# $count is the formatted number of risks.
tools-analyzer-risks-title = Security Risks ({ $count })
tools-analyzer-risks-title-none = Security Risks
tools-analyzer-risks-none = No security risks detected

# Market tab.
tools-analyzer-market-empty = No market data available
tools-analyzer-market-empty-hint = Market data is not available for this token
tools-analyzer-card-price = Current Price
tools-analyzer-card-price-changes = Price Changes
tools-analyzer-card-volume = Trading Volume
tools-analyzer-card-transactions = 24h Transactions
tools-analyzer-card-valuation = Valuation
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = 1h Volume
tools-analyzer-stat-volume-6h = 6h Volume
tools-analyzer-stat-fdv = Fully Diluted Value
tools-analyzer-txn-buys = Buys
tools-analyzer-txn-sells = Sells

# Liquidity tab.
tools-analyzer-liquidity-empty = No liquidity data available
tools-analyzer-liquidity-empty-hint = No pools found for this token
tools-analyzer-card-total-liquidity = Total Liquidity
tools-analyzer-card-pools = Pools
tools-analyzer-active-pools =
    { $count ->
        [one] Active Pool
       *[other] Active Pools
    }
tools-analyzer-card-pool-details = Pool Details
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Liquidity ({ -sol })
tools-analyzer-pools-column-status = Status
tools-analyzer-pool-primary = Primary

# Copied report. Each line is one message; values arrive already formatted.
tools-analyzer-report-empty = No analysis to copy
tools-analyzer-report-label = Analysis report
tools-analyzer-report-title = Token Analysis Report
tools-analyzer-report-token = Token: { $symbol } ({ $name })
tools-analyzer-report-mint = Mint: { $mint }
# $sol is the price with the SOL unit.
tools-analyzer-report-price = Price: { $sol }
tools-analyzer-report-price-with-usd = Price: { $sol } ({ $usd })
tools-analyzer-report-security = Security:
tools-analyzer-report-safety-score = - Safety Score: { $score }/100
tools-analyzer-report-mint-authority = - Mint Authority: { $state }
tools-analyzer-report-freeze-authority = - Freeze Authority: { $state }
tools-analyzer-report-risks = - Risks: { $count }
tools-analyzer-report-market = Market:
tools-analyzer-report-volume = - 24h Volume: { $amount }
tools-analyzer-report-change = - 24h Change: { $amount }
tools-analyzer-report-market-cap = - Market Cap: { $amount }
tools-analyzer-report-liquidity = Liquidity:
tools-analyzer-report-liquidity-total = - Total: { $amount }
tools-analyzer-report-pools = - Pools: { $count }
# $time is the formatted time the analysis was fetched.
tools-analyzer-report-generated = Generated: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

# Ids are the watch types. `notify` is the short badge of the notify-only type.
tools-watch-type-buy-on-sell = Buy on Sell
tools-watch-type-sell-on-buy = Sell on Buy
tools-watch-type-notify = Notify
tools-watch-type-notify-only = Notify Only

tools-trade-watcher-setup-title = Setup Watch
tools-trade-watcher-mint-label = Token Mint Address
tools-trade-watcher-mint-input =
    .placeholder = Enter token mint address...
tools-trade-watcher-action-search-pools = Search Pools
tools-trade-watcher-pool-label = Selected Pool
tools-trade-watcher-pool-none = No pool selected
tools-trade-watcher-pool-clear =
    .title = Clear pool
# $dex is the DEX name, $base and $quote are the pair symbols.
tools-trade-watcher-pool-selected = Selected pool: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Watch Type
tools-trade-watcher-type-hint = Buy on Sell: Automatically buy when someone sells. Sell on Buy: Automatically sell when someone buys.
tools-trade-watcher-trigger-label = Trigger Amount ({ -sol })
tools-trade-watcher-trigger-hint = Minimum trade size in { -sol } to trigger the action
tools-trade-watcher-action-amount-label = Action Amount ({ -sol })
tools-trade-watcher-action-amount-hint = Amount to buy/sell when triggered
tools-trade-watcher-slippage-label = Slippage (%)
tools-trade-watcher-slippage-hint = Maximum acceptable slippage for trades
tools-trade-watcher-active-title = Active Watches
tools-trade-watcher-empty = No active watches
tools-trade-watcher-empty-hint = Configure a watch above and click "Start Watch" to begin monitoring
tools-trade-watcher-action-start = Start Watch
tools-trade-watcher-action-starting = Starting...
tools-trade-watcher-action-stop-all = Stop All
tools-trade-watcher-action-stopping = Stopping...
# $token is the token symbol or the start of its mint address.
tools-trade-watcher-started = Watch started for { $token }...
tools-trade-watcher-start-failed = Failed to start watch
tools-trade-watcher-stopped = Watch stopped
tools-trade-watcher-stop-failed = Failed to stop watch
tools-trade-watcher-stopped-all = All watches stopped
tools-trade-watcher-stop-all-failed = Failed to stop watches
tools-trade-watcher-load-failed = Failed to load watches
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Type
tools-trade-watcher-column-trigger = Trigger
tools-trade-watcher-column-action = Action
tools-trade-watcher-column-triggered = Triggered
tools-trade-watcher-stop-watch =
    .title = Stop watch

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = Cannot burn { -sol }
tools-burn-failure-open-position = Cannot burn tokens from open positions
tools-burn-failure-account-not-found = Token account not found
tools-burn-failure-zero-balance = Token balance is already zero
tools-burn-failure-transaction = Transaction failed
tools-burn-warning-open-position = Cannot burn tokens from open positions
tools-burn-warning-closed-position = Leftover from closed position
# $amount is the token value in SOL with six decimals.
tools-burn-warning-worth = Worth ~{ $amount } { -sol }
# $needed and $have are SOL amounts with four decimals.
tools-multi-buy-warning-insufficient = Insufficient balance. Need { $needed } { -sol }, have { $have } { -sol }
# $needed and $limit are SOL amounts with four decimals.
tools-multi-buy-warning-over-limit = Total { -sol } needed ({ $needed }) exceeds limit ({ $limit })
tools-multi-sell-warning-no-wallets = No secondary wallets found
tools-multi-sell-warning-no-balance = No wallets have token balance
tools-multi-op-buy-failed = Buy failed
tools-multi-op-sell-failed = Sell failed
tools-multi-op-transfer-failed = Transfer failed
tools-multi-op-balance-failed = Failed to get balance
tools-multi-op-mint-invalid = Invalid mint address
tools-multi-buy-session-failed = Multi-buy failed
tools-multi-sell-session-failed = Multi-sell failed
tools-multi-session-aborted = Operation aborted by user

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Scan Wallet
tools-wallet-action-scanning = Scanning...
# $reason is the technical cause of the failure.
tools-wallet-scan-failed = Scan failed: { $reason }
# $amount is an amount with its unit.
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Selected: { $count } wallet
       *[other] Selected: { $count } wallets
    }
tools-wallet-transfer-failed = Transfer failed: { $reason }
tools-wallet-cleanup-failed = Cleanup failed: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Scan Results
tools-wallet-cleanup-stat-empty = Empty ATAs
tools-wallet-cleanup-stat-reclaimable = Reclaimable { -sol }
tools-wallet-cleanup-stat-failed = Failed (cached)
tools-wallet-cleanup-prompt = Click "Scan Wallet" to find empty ATAs
tools-wallet-cleanup-prompt-hint = This will check all token accounts in your wallet
tools-wallet-cleanup-action-cleanup = Cleanup All
tools-wallet-cleanup-action-cleaning = Cleaning...
tools-wallet-cleanup-scanning = Scanning wallet...
# $amount is the reclaimable rent with its unit.
tools-wallet-cleanup-found =
    { $count ->
        [one] Found { $count } empty ATA worth ~{ $amount }
       *[other] Found { $count } empty ATAs worth ~{ $amount }
    }
tools-wallet-cleanup-clean = No empty ATAs found - wallet is clean!
tools-wallet-cleanup-scan-failed = Failed to scan ATAs
tools-wallet-cleanup-done =
    { $count ->
        [one] Cleaned { $count } ATA
       *[other] Cleaned { $count } ATAs
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Burn Tokens
tools-burn-info-title = What is burning?
tools-burn-info-body = Burning permanently destroys tokens, making them unrecoverable. After burning, run Wallet Cleanup to close empty ATAs and reclaim ~0.002 { -sol } rent per token.
tools-burn-stat-total = Total Tokens
tools-burn-stat-selected = Selected
tools-burn-stat-rent = Rent Reclaimable
tools-burn-prompt = Click "Scan Wallet" to find tokens
tools-burn-scanning = Scanning wallet for tokens...
tools-burn-scan-failed = Failed to scan tokens
tools-burn-empty = No tokens found in wallet
# $count is the number of selected tokens.
tools-burn-action-burn = Burn Selected ({ $count })
tools-burn-action-burning = Burning...
tools-burn-cannot-burn = Cannot burn
tools-burn-no-value = No value

# Category titles and descriptions. Ids are the token categories of the scan.
tools-burn-category-open-position = Open Positions
tools-burn-category-has-value = Has Value
tools-burn-category-closed-position = Closed Positions
tools-burn-category-zero-liquidity = Zero Liquidity
tools-burn-category-hint-open-position = Cannot burn tokens from open positions
tools-burn-category-hint-has-value = Consider selling instead of burning
tools-burn-category-hint-closed-position = Leftovers from closed trades
tools-burn-category-hint-zero-liquidity = Safe to burn - no market value

tools-burn-confirm-title = Confirm Burn
tools-burn-confirm-message =
    { $count ->
        [one] Are you sure you want to burn <strong>{ $count }</strong> token?
       *[other] Are you sure you want to burn <strong>{ $count }</strong> tokens?
    }
# $amount is the estimated value with its unit.
tools-burn-confirm-value = Total estimated value: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Continue
tools-burn-final-title = Final Warning
tools-burn-final-headline = This action is IRREVERSIBLE!
tools-burn-final-message =
    { $count ->
        [one] The following { $count } token will be permanently destroyed and cannot be recovered under any circumstances.
       *[other] The following { $count } tokens will be permanently destroyed and cannot be recovered under any circumstances.
    }
tools-burn-final-confirm = Yes, Burn Tokens
# $successful and $total count tokens; $amount is the reclaimable rent with its unit.
tools-burn-toast-burned =
    { $total ->
        [one] Burned { $successful }/{ $total } token. Run Wallet Cleanup to reclaim ~{ $amount }
       *[other] Burned { $successful }/{ $total } tokens. Run Wallet Cleanup to reclaim ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
        [one] { $count } token failed to burn
       *[other] { $count } tokens failed to burn
    }
tools-burn-failed = Burn failed: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] { $count } token could not be burned
       *[other] { $count } tokens could not be burned
    }
tools-burn-failure-unknown = No reason was reported

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = About
tools-airdrop-about-body = Check for pending airdrops, claimable rewards, and unclaimed allocations across popular Solana protocols.
tools-airdrop-list-title = Available Airdrops
tools-airdrop-prompt = Click "Check Airdrops" to scan for available claims
tools-airdrop-action-check = Check Airdrops
tools-airdrop-action-claim-all = Claim All

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Generator Options
tools-generator-warning-title = Store your private keys securely!
tools-generator-warning-body = Generated keypairs are created locally and never transmitted. Always backup your keys in a secure location.
tools-generator-count-label = Number of Wallets
tools-generator-vanity-label = Vanity Address (starts with specific characters)
tools-generator-prefix-label = Prefix
tools-generator-prefix-input =
    .placeholder = e.g., SOL
tools-generator-prefix-hint = Longer prefixes take exponentially longer to generate
tools-generator-list-title = Generated Wallets
tools-generator-empty = No wallets generated yet
tools-generator-action-generate = Generate
tools-generator-action-generating = Generating...
tools-generator-count-invalid = Please enter a number between 1 and 10
tools-generator-no-keypairs = No keypairs returned
tools-generator-generated =
    { $count ->
        [one] Generated { $count } wallet
       *[other] Generated { $count } wallets
    }
tools-generator-failed = Failed to generate wallets: { $reason }
tools-generator-copy-public-key =
    .title = Copy public key
tools-generator-copy-private-key =
    .title = Copy private key
tools-generator-remove =
    .title = Remove from list
tools-generator-reveal =
    .title = Reveal private key
tools-generator-public-key-label = Public Key:
tools-generator-private-key-label = Private Key:
# Names the copied value in the shared copied toast.
tools-generator-public-key-name = Public key
tools-generator-private-key-copied = Private key copied
tools-generator-private-key-warning = Anyone with this key controls the wallet
tools-generator-export-empty = No wallets to export
tools-generator-exported = Wallets exported - store securely

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Summary
tools-consolidation-stat-wallets = Sub-wallets
tools-consolidation-stat-native = Total { -sol }
tools-consolidation-stat-tokens = Token Types
tools-consolidation-stat-rent = Reclaimable Rent
tools-consolidation-wallets-title = Wallets
tools-consolidation-loading-wallets = Loading wallets...
tools-consolidation-loading-data = Loading wallet data...
tools-consolidation-action-transfer-native = Transfer { -sol }
tools-consolidation-action-transfer-tokens = Transfer All Tokens
tools-consolidation-action-cleanup = Cleanup ATAs
tools-consolidation-action-transferring = Transferring...
tools-consolidation-column-name = Name
tools-consolidation-column-native = { -sol } Balance
tools-consolidation-column-tokens = Tokens
tools-consolidation-column-atas = Empty ATAs
tools-consolidation-empty = No sub-wallets found
tools-consolidation-empty-hint = Create sub-wallets using Multi-Buy to get started
tools-consolidation-load-failed = Failed to load: { $reason }
tools-consolidation-select-prompt = Select wallets to consolidate
# $amount is the selected balance with its unit.
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } token
       *[other] { $tokens } tokens
    } | { $atas ->
        [one] { $atas } empty ATA
       *[other] { $atas } empty ATAs
    }
# $amount is the transferred balance with its unit.
tools-consolidation-transferred-native = Transferred { $amount } to main wallet
tools-consolidation-transferred-tokens =
    { $count ->
        [one] Transferred { $count } token to main wallet
       *[other] Transferred { $count } tokens to main wallet
    }
# $amount is the reclaimed rent with its unit.
tools-consolidation-cleaned =
    { $count ->
        [one] Closed { $count } ATA, reclaimed { $amount }
       *[other] Closed { $count } ATAs, reclaimed { $amount }
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Token Mint Address
tools-multi-mint-input =
    .placeholder = Paste token mint address...
tools-multi-execution-title = Execution Settings
tools-multi-delay-min-label = Delay Min (ms)
tools-multi-delay-max-label = Delay Max (ms)
tools-multi-concurrency-label = Concurrency
tools-multi-concurrency-sequential = { $count } (Sequential)
tools-multi-concurrency-parallel = { $count } parallel
tools-multi-slippage-label = Slippage (%)
tools-multi-router-label = Router
tools-multi-router-auto = Auto (Best Route)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Direct Pool
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Progress
tools-multi-progress-preparing = Preparing...
# $label is the session state, $completed and $total count wallet operations.
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Wallet
tools-multi-column-route = Route
tools-multi-column-status = Status
tools-multi-op-completed = Completed
tools-multi-op-failed = Failed
tools-multi-action-stop = Stop
tools-multi-action-loading = Loading...
# $reason is the technical cause of the failure.
tools-multi-start-failed = Failed to start: { $reason }

# Session states. Ids are the states of a multi-wallet session.
tools-multi-state-pending = Pending
tools-multi-state-funding = Funding
tools-multi-state-executing = Executing
tools-multi-state-consolidating = Consolidating
tools-multi-state-completed = Completed
tools-multi-state-failed = Failed
tools-multi-state-aborted = Aborted

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = The token you want to buy across multiple wallets
tools-multi-buy-wallets-title = Wallet Settings
tools-multi-buy-wallet-count-label = Wallet Count
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } wallet
       *[other] { $count } wallets
    }
tools-multi-buy-wallet-count-hint = Number of sub-wallets to use
tools-multi-buy-buffer-label = { -sol } Buffer per Wallet
tools-multi-buy-buffer-hint = Reserved for fees (0.015 { -sol } min)
tools-multi-buy-amounts-title = Amount Settings
tools-multi-buy-min-label = Min { -sol } per Wallet
tools-multi-buy-min-hint = Minimum buy amount
tools-multi-buy-max-label = Max { -sol } per Wallet
tools-multi-buy-max-hint = Maximum buy amount
tools-multi-buy-limit-label = Total { -sol } Limit (optional)
tools-multi-buy-limit-hint = Maximum total spend
tools-multi-buy-preview-title = Preview
tools-multi-buy-preview-create = Wallets to Create
tools-multi-buy-preview-amount = Amount per Wallet
# $min and $max are amounts with their unit.
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Total { -sol } Needed
tools-multi-buy-preview-balance = Main Balance
tools-multi-buy-action-preview = Preview
tools-multi-buy-action-start = Start Multi-Buy
tools-multi-buy-executing = Executing buys...
tools-multi-buy-column-spent = { -sol } Spent
tools-multi-buy-column-tokens = Tokens
tools-multi-buy-preview-failed = Preview failed: { $reason }
tools-multi-buy-started = Multi-buy started
tools-multi-buy-stopped = Multi-buy stopped
tools-multi-buy-completed = Multi-buy completed! { $successful }/{ $total } successful

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Enter a token address to scan for wallets holding it
tools-multi-sell-action-scan = Scan
tools-multi-sell-settings-title = Sell Settings
tools-multi-sell-percent-label = Sell Percentage
tools-multi-sell-percent-hint = % of tokens to sell per wallet
tools-multi-sell-min-fee-label = Min { -sol } for Fee
tools-multi-sell-min-fee-hint = Minimum { -sol } needed for tx fee
tools-multi-sell-topup-label = Auto topup if needed
tools-multi-sell-topup-hint = Transfer { -sol } from main wallet if sub-wallet has insufficient balance
tools-multi-sell-post-title = Post-Sell Actions
tools-multi-sell-consolidate-label = Consolidate { -sol } to main wallet
tools-multi-sell-consolidate-hint = Transfer all { -sol } from sub-wallets back to main wallet
tools-multi-sell-close-atas-label = Close token ATAs after sell
tools-multi-sell-close-atas-hint = Reclaim ~0.002 { -sol } per ATA
tools-multi-sell-wallets-title = Wallets with Token
tools-multi-sell-empty = No sub-wallets hold this token
tools-multi-sell-column-tokens = Tokens
tools-multi-sell-column-native = { -sol } Balance
tools-multi-sell-column-topup = Needs Topup
tools-multi-sell-none-selected = No wallets selected
tools-multi-sell-select-required = Please select at least one wallet
tools-multi-sell-action-start = Start Multi-Sell
tools-multi-sell-executing = Executing sells...
tools-multi-sell-column-sold = Tokens Sold
tools-multi-sell-column-received = { -sol } Received
tools-multi-sell-started = Multi-sell started
tools-multi-sell-stopped = Multi-sell stopped
# $amount is the received amount with its unit.
tools-multi-sell-completed = Multi-sell completed! { $amount } received

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Favorites
tools-favorites-saved = Saved Favorites
tools-favorites-save-current = Save Current
tools-favorites-empty = No favorites saved yet
tools-favorites-no-label = No label
# $count is how many times the favorite was used.
tools-favorites-uses = { $count }x
tools-favorites-remove = Remove
# $name is the favorite's label or symbol.
tools-favorites-loaded = Loaded favorite: { $name }
tools-favorites-default-name = Config
tools-favorites-mint-required = Please enter a token mint address first
tools-favorites-add-title = Add Favorite
tools-favorites-add-message = Enter a label for this favorite
tools-favorites-add-placeholder = Label (optional)...
tools-favorites-saved-toast = Saved to favorites
tools-favorites-save-failed = Failed to save favorite
tools-favorites-remove-title = Remove Favorite
tools-favorites-remove-message = Remove this favorite?
tools-favorites-removed-toast = Favorite removed
tools-favorites-remove-failed = Failed to remove favorite
