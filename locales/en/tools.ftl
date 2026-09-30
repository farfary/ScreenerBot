# Tools page: the shell, the token tools and the trading tools.
# Wallet and multi-wallet tools are added by their own sections.

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
tools-analyzer-pools-column-address = Pool Address
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
