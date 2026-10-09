# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = Live market data
tokens-result-source-unavailable = { $label } unavailable — retrying
tokens-result-source-not-listed = Not listed on { $label }
tokens-result-security-available = Security report available
tokens-result-security-missing = No { -rugcheck } report
tokens-result-chart-available = Chart data available
tokens-result-chart-missing = No chart data yet

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = Couldn't load data
tokens-state-offline = You appear to be offline.
tokens-state-request-failed = The request failed after several attempts.
tokens-state-waiting = Waiting for data…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = Entry
tokens-chart-level-stop-loss = Stop Loss
tokens-chart-level-take-profit = Take Profit

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = Loading transactions…
tokens-transactions-empty-title = No transactions
tokens-transactions-empty-history = No wallet transaction history is available for this token.
tokens-transactions-empty-data = No transaction data is available for this token.
tokens-transactions-error-title = Couldn't load transactions
tokens-transactions-error-message = Transaction history is temporarily unavailable.
tokens-transactions-activity-title = 24h activity
tokens-transactions-activity-subtitle = Hourly wallet transactions
tokens-transactions-metric-total = Total
tokens-transactions-metric-buys = Buys
tokens-transactions-metric-sells = Sells
tokens-transactions-recent-title = Recent transactions
# $count is the number of rows listed.
tokens-transactions-shown = { $count } shown
tokens-transactions-column-time = Time
tokens-transactions-column-type = Type
tokens-transactions-column-price = Price
tokens-transactions-column-total = Total
tokens-transactions-chart-missing = Chart library missing
tokens-transactions-view-solscan = View transaction on { -solscan }

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = No position
tokens-positions-no-token = No token selected.
tokens-positions-empty-message = No position for this token yet. Use Buy to open one.
tokens-positions-loading = Loading position…
tokens-positions-from-wallet-history = From wallet history
tokens-positions-frozen = Frozen — cannot be sold
tokens-positions-no-cost-basis = No cost basis
tokens-positions-history-incomplete = History incomplete
# $count is the number of DCA buys.
tokens-positions-dca-count = DCA { $count }
# $count is the number of partial exits.
tokens-positions-exit-count = Exits { $count }
tokens-positions-fact-avg-entry = Avg Entry
tokens-positions-fact-current = Current
tokens-positions-fact-tokens = Tokens
tokens-positions-fact-opened = Opened
tokens-positions-fact-exit-price = Exit Price
tokens-positions-fact-native-received = { -sol } Received
tokens-positions-fact-closed-reason = Closed Reason
tokens-positions-fact-target-min = Profit Target Min
tokens-positions-fact-target-max = Profit Target Max
tokens-positions-fact-highest = Highest Price
tokens-positions-fact-lowest = Lowest Price
tokens-positions-section-range = Targets & range
tokens-positions-section-market = Market & holdings
tokens-positions-kicker = Position
tokens-positions-fallback-symbol = Token
tokens-positions-realized-pnl = Realized P&L
tokens-positions-unrealized-pnl = Unrealized P&L
tokens-positions-size = Size

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = { -rugcheck } analysis in progress...
tokens-security-analyzing = Analyzing security…
tokens-security-pulse-title = Security Pulse
tokens-security-pending-caption = Risk signals are still being collected.
tokens-security-control-title = Token Control
tokens-security-control-meta = Authority status
# $time is the formatted time of the last security update.
tokens-security-updated = Updated { $time }
tokens-security-score-caption = Normalized token risk score out of 100.
tokens-security-score-label = Score
tokens-security-rugged = Rugged
tokens-security-grade-analyzing = Analyzing
tokens-security-grade-shielded = Shielded
tokens-security-grade-safe = Safe
tokens-security-grade-caution = Caution
tokens-security-grade-vulnerable = Vulnerable
tokens-security-grade-unknown = Unknown
tokens-security-metric-token-type = Token Type
tokens-security-metric-total-holders = Total Holders
tokens-security-metric-lp-providers = LP Providers
tokens-security-metric-graph-insiders = Graph Insiders
# $count is the number of insider wallets found in the holder graph.
tokens-security-insiders-detected = Detected ({ $count })
tokens-security-insiders-clean = Clean
tokens-security-authority-mint = Mint
tokens-security-authority-freeze = Freeze
tokens-security-authority-immutable = Immutable
tokens-security-authority-mutable = Mutable
tokens-security-authority-revoked = Revoked
tokens-security-authority-active = Active
tokens-security-holder-health-title = Holder Health
tokens-security-holders-unique = unique
tokens-security-creator-share = Creator Share
tokens-security-gauge-top-10 = Top 10
tokens-security-concentration-unknown = Unknown
tokens-security-concentration-critical = Critical
tokens-security-concentration-high = High
tokens-security-concentration-moderate = Moderate
tokens-security-concentration-healthy = Healthy
tokens-security-transfer-title = Transfer Tax
tokens-security-transfer-no-fee = No Fee
tokens-security-transfer-fee-percentage = Fee Percentage
tokens-security-transfer-max-fee = Max Fee Amount
tokens-security-transfer-authority = Fee Authority
# $percent is the formatted transfer fee percentage.
tokens-security-transfer-note = A { $percent } fee is charged on every transfer.
tokens-security-transfer-none = No transfer fees detected.
tokens-security-risks-title = Security Risks
tokens-security-risks-none = No security risks detected.
tokens-security-risk-fallback-name = Security signal
tokens-security-risks-critical = { $count } critical
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } warning
       *[other] { $count } warnings
    }
tokens-security-risks-info = { $count } info
tokens-security-risks-incidents =
    { $count ->
        [one] { $count } incident found
       *[other] { $count } incidents found
    }
tokens-security-top-holders-title = Top Holders
# $percent is the formatted share of supply held by the top holders.
tokens-security-top-holders-concentration = { $percent } concentration
tokens-security-insider = Insider

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = Checking data…
tokens-overview-banner-open = Open token banner
tokens-overview-headline-label = Headline market metrics
tokens-overview-price = Price
tokens-overview-market-cap = Market Cap
tokens-overview-liquidity = Liquidity
tokens-overview-volume = Volume
tokens-overview-volume-24h = Vol 24H
tokens-overview-no-tags = No tags
tokens-overview-info-title = Token Info
tokens-overview-profile = Published profile
tokens-overview-fact-mint = Mint
tokens-overview-fact-decimals = Decimals
tokens-overview-fact-age = Age
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = Holders
tokens-overview-fact-top-10 = Top 10 Hold
tokens-overview-tags = Tags
tokens-overview-liquidity-title = Liquidity & Market
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-native = Pool { -sol }
tokens-overview-fact-pool-token = Pool Token
tokens-overview-pool = Pool
tokens-overview-pulse-title = Market Pulse
tokens-overview-activity-title = Transaction Activity
# $percent is the formatted share of buys among the 24h transactions.
tokens-overview-buy-share = { $percent } Buy
# $ratio is the formatted buy-to-sell ratio.
tokens-overview-buy-sell-ratio = { $ratio } B/S
tokens-overview-buys-24h = Buys 24H
tokens-overview-sells-24h = Sells 24H
tokens-overview-net-flow = Net Flow
tokens-overview-total-24h = 24H Total
tokens-overview-average-24h = 24H Avg
tokens-overview-spike-5m = 5M Spike
# $amount is the formatted average number of transactions per hour.
tokens-overview-rate-per-hour = { $amount }/h
# $amount is the formatted average number of transactions per minute.
tokens-overview-rate-per-minute = { $amount }/m
# $factor is the formatted ratio of the 5-minute rate to the 1-hour rate.
tokens-overview-spike-factor = { $factor }×
# Tooltip of one activity row. Counts and percentages are formatted; "—" marks a missing value.
tokens-overview-flow-counts = Buys: { $buys } ({ $buyPercent }), Sells: { $sells } ({ $sellPercent }), Total: { $total }
tokens-overview-flow-no-data = No transaction data

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = No pools
tokens-pools-empty-message = No liquidity pools have been detected for this token.
tokens-pools-unknown = Unknown
tokens-pools-unknown-dex = Unknown DEX
tokens-pools-liquidity = Liquidity
tokens-pools-volume-24h = 24h Volume
tokens-pools-base-role = Base Role
tokens-pools-quote-role = Quote Role
tokens-pools-canonical-title = Canonical pool
tokens-pools-canonical = Canonical
tokens-pools-dex = DEX
tokens-pools-summary-title = Pool summary
tokens-pools-breakdown-title = DEX breakdown
tokens-pools-all-title = All pools
tokens-pools-updated = Updated
tokens-pools-role-base = Base
tokens-pools-role-quote = Quote
tokens-pools-role-unknown = Unknown
tokens-pools-reserves = Reserve accounts
tokens-pools-no-reserves = No reserve accounts
tokens-pools-address-pool = Pool
    .title = Copy pool
tokens-pools-address-base = Base mint
    .title = Copy base mint
tokens-pools-address-quote = Quote mint
    .title = Copy quote mint
    .title = Copy paired mint

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = No official website or social links are available for this token.
tokens-links-info-title = Token info
tokens-links-mint-address = Mint address
tokens-links-data-source = Data source
tokens-links-security = Security
tokens-links-profile-title = Token profile
tokens-links-profile-published-title = Published profile content
tokens-links-profile-published-note = Media, description, and official links are paid profile content reviewed before publication. This does not verify ownership or token safety.
tokens-links-profile-create-note = Add a reviewed logo, project description, and official links to this token's public profile.
tokens-links-profile-update-hint = Update this token profile on screenerbot.io
tokens-links-profile-create-hint = Create a token profile on screenerbot.io
tokens-links-profile-update = Update profile
tokens-links-profile-create = Create profile
tokens-links-media-title = Media assets
tokens-links-media-fallback-symbol = Token
tokens-links-media-logo = Logo
tokens-links-media-banner = Banner
# $symbol is the token symbol.
tokens-links-media-banner-alt = { $symbol } banner
tokens-links-media-open = Open image
tokens-links-description-title = Description
tokens-links-explorers-title = Explorers & analytics
tokens-links-websites-title = Official websites
tokens-links-socials-title = Social media
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = Social

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = Overview
tokens-dialog-tab-security = Security
tokens-dialog-tab-positions = Positions
tokens-dialog-tab-pools = Pools
tokens-dialog-tab-links = Links
tokens-dialog-tab-transactions = Txns
tokens-dialog-sections = Token details sections
tokens-dialog-close =
    .title = Close (ESC)
    .aria-label = Close token details
tokens-dialog-unknown-symbol = Unknown
tokens-dialog-unknown-name = Unknown Token
tokens-dialog-market-summary = Market summary
tokens-dialog-price-loading = Loading price
tokens-dialog-unit-native = { -sol }
tokens-dialog-market-metrics = Market metrics
tokens-dialog-metric-market-cap = Market cap
tokens-dialog-metric-volume-24h = 24h volume
# $change is the formatted 24 hour price change.
tokens-dialog-change-24h = 24 hour change { $change }
tokens-dialog-buy = Buy
    .title = Buy this token
tokens-dialog-sell = Sell
    .title = Sell position
tokens-dialog-sell-unavailable = No open position to sell
tokens-dialog-details = Details
tokens-dialog-sources = Sources
tokens-dialog-sources-status = Data source status
tokens-dialog-updated-label = Updated
tokens-dialog-just-now = Just now
# $time is a relative or clock time.
tokens-dialog-updated-at = Updated { $time }
tokens-dialog-updated-unavailable = Update time unavailable
tokens-dialog-error-title = Couldn't load token data
tokens-dialog-waiting-token = Waiting for token data…
tokens-dialog-loading-overview = Loading overview…
tokens-dialog-loading-security = Loading security…
tokens-dialog-loading-pools = Loading pools…
tokens-dialog-loading-links = Loading links…
tokens-dialog-chart-still-checking = No chart data available yet — still checking…
tokens-dialog-no-data = No data available
tokens-dialog-source-token = Token
tokens-dialog-source-market = Market
tokens-dialog-source-security = Security
tokens-dialog-source-chart = Chart
tokens-dialog-status-pending = Waiting
tokens-dialog-status-loading = Loading
tokens-dialog-status-ready = Ready
tokens-dialog-status-unavailable = Unavailable
tokens-dialog-status-cached = Cached
# $source is a data source name and $status its state, for example "Market data: Ready".
tokens-dialog-source-summary = { $source } data: { $status }
tokens-dialog-badge-pool-price = Pool price
tokens-dialog-badge-pool-price-hint = Price from real-time on-chain pool
tokens-dialog-badge-api-price = API price
tokens-dialog-badge-api-price-hint = Price from cached market-data (API)
tokens-dialog-badge-profile = Published profile
    .title = Paid profile content reviewed for publication; not an audit or ownership verification.
tokens-dialog-badge-low-risk-hint = Low risk according to the current { -rugcheck } score; not identity verification.
tokens-dialog-badge-immutable = Immutable
tokens-dialog-badge-mutable = Mutable
tokens-dialog-badge-position = Position
tokens-dialog-badge-blacklisted = Blacklisted

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = Favorites
tokens-view-pool = Pool Service
tokens-view-no-market = No Market Data
tokens-view-all = All Tokens
tokens-view-passed = Passed
tokens-view-rejected = Rejected
tokens-view-blacklisted = Blacklisted
tokens-view-positions = Positions
tokens-view-recent = Recent
tokens-view-ohlcv = OHLCV Data
# Empty token table per view (TOKEN_VIEW_EMPTY_LABELS)
tokens-view-pool-empty = No priced tokens yet
    .message = Tokens appear here once they pass filtering and their pool price is computed.
tokens-view-no-market-empty = No tokens without market data
    .message = Tokens appear here while the market data sources have not listed them yet.
tokens-view-all-empty = No tokens discovered yet
    .message = Every token found by discovery appears here, whatever its filtering result.
tokens-view-passed-empty = No tokens passed filtering
    .message = Tokens that pass every active filter appear here. Review the Filtering page if this stays empty.
tokens-view-rejected-empty = No rejected tokens
    .message = Tokens that fail a filter appear here with the reason.
tokens-view-blacklisted-empty = No blacklisted tokens
    .message = Tokens excluded from trading, by you or by the safety checks, appear here.
tokens-view-positions-empty = No tokens in positions
    .message = Tokens held in open positions appear here.
tokens-view-recent-empty = No recent tokens
    .message = Newly discovered tokens appear here as they are found.
tokens-ohlcv-empty = No chart data yet
    .message = Tokens appear here once their candles are being collected.

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = Click to enlarge
# $boosts is the formatted active boost count.
tokens-boost-title = Boosted { $boosts } on screenerbot.io
tokens-cell-action-add =
    .title = Add to position (DCA)
    .aria-label = Add to position
tokens-cell-action-sell =
    .title = Sell (full or % partial)
    .aria-label = Sell token
tokens-cell-action-buy =
    .title = Buy position
    .aria-label = Buy token
tokens-cell-external-links =
    .title = External links
    .aria-label = External links

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = Loading tokens…
tokens-table-loading-description = Preparing the selected token view.
tokens-table-retry-hint = Switch tabs or try again.
tokens-filter-all = All

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = Favorites could not be loaded
tokens-favorites-load-failed-toast = Could not load favorites
tokens-favorites-total = Total Favorites
tokens-favorites-empty-title = No Favorites Yet
    .message = Star a token in any list to keep it here.

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = Token
tokens-column-status = Status
tokens-ohlcv-delete =
    .title = Delete OHLCV data
    .aria-label = Delete OHLCV data
tokens-ohlcv-status-active = Active
tokens-ohlcv-status-inactive = Inactive
tokens-ohlcv-priority-critical = Critical
tokens-ohlcv-priority-high = High
tokens-ohlcv-priority-medium = Medium
tokens-ohlcv-priority-low = Low
tokens-ohlcv-column-priority = Priority
tokens-ohlcv-column-backfill = Backfill
tokens-ohlcv-column-data-span = Data Span
tokens-ohlcv-column-gaps = Gaps
tokens-ohlcv-column-pools = Pools
tokens-ohlcv-column-last-fetch = Last Fetch
# $timeframe is a timeframe code such as 1h.
tokens-ohlcv-timeframe-complete = { $timeframe }: Complete
tokens-ohlcv-timeframe-pending = { $timeframe }: Pending
tokens-ohlcv-load-failed-title = OHLCV data could not be loaded
tokens-ohlcv-load-failed-toast = Could not load OHLCV data
tokens-ohlcv-total = Total Tokens
tokens-ohlcv-active = Active
tokens-ohlcv-db-size = DB Size
tokens-ohlcv-cleanup = Cleanup Inactive
tokens-ohlcv-delete-title = Delete OHLCV Data
# $token is the token symbol.
tokens-ohlcv-delete-token-message = Delete all OHLCV data for { $token }?
tokens-ohlcv-delete-done =
    Deleted: { $candles ->
        [one] { $candles } candle
       *[other] { $candles } candles
    }, { $pools ->
        [one] { $pools } pool
       *[other] { $pools } pools
    }
tokens-ohlcv-delete-failed = Failed to delete OHLCV data
tokens-ohlcv-cleanup-title = Delete Inactive Tokens
tokens-ohlcv-cleanup-message = Delete inactive tokens older than specified hours
tokens-ohlcv-cleanup-placeholder = Hours...
tokens-ohlcv-cleanup-invalid = Please enter a positive number
tokens-ohlcv-cleanup-done =
    Cleaned up { $count ->
        [one] { $count } inactive token
       *[other] { $count } inactive tokens
    }
tokens-ohlcv-cleanup-failed = Failed to cleanup OHLCV data

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = Total
tokens-summary-pool-priced = With Pool Price
tokens-summary-positions = Positions
tokens-summary-blacklisted = Blacklisted
tokens-search-placeholder = Search by symbol or mint...
tokens-table-waiting-title = Still loading tokens...
tokens-table-waiting-description = Waiting for the backend to respond. We will retry automatically.
tokens-load-failed-toast = Could not load tokens
tokens-row-data-missing = Token data not found
tokens-column-price-sol = Price ({ -sol })
tokens-column-liquidity = Liquidity
tokens-column-volume-24h = 24h Vol
tokens-column-fdv = FDV
tokens-column-market-cap = Mkt Cap
tokens-column-change-1h = 1h
tokens-column-change-24h = 24h
tokens-column-txns-5m = Txns 5m
tokens-column-txns-1h = Txns 1h
tokens-column-txns-6h = Txns 6h
tokens-column-txns-24h = Txns 24h
tokens-column-risk-score = Risk Score
tokens-column-reject-reason = Reject Reason
tokens-column-blacklist-reason = Blacklist Reason
tokens-column-updated = Updated
tokens-column-birth = Birth
tokens-column-first-seen = First Seen
tokens-badge-price = Price
tokens-badge-ohlcv = OHLCV
tokens-badge-position = Position
tokens-badge-blacklisted = Blacklisted
tokens-badge-blacklisted-title = Blacklisted token
# $reasons is the list of blacklist categories, reasons and details.
tokens-badge-blacklisted-reasons = Blacklisted: { $reasons }
tokens-links-menu-copy-mint = Copy Mint
tokens-links-copy-failed = Failed to copy mint
tokens-lightbox-token-age = Token Age

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = Search name, symbol or mint...
tokens-search-input-label = Search tokens
tokens-search-results-label = Search results
tokens-search-tip-nav = nav
tokens-search-tip-open = open
tokens-search-tip-close = close
tokens-search-failed = Search failed
# $message is the failure text.
tokens-search-error = Error: { $message }
tokens-search-clear =
    .title = Clear search
    .aria-label = Clear search
tokens-search-recent = Recent
tokens-search-recent-label = Recent searches
tokens-search-lists-label = Token lists
tokens-search-tab-trending = Trending
tokens-search-kinds = Name · symbol · mint
tokens-search-empty-trending = Trending tokens appear once the bot has priced its first pools.
tokens-search-empty-positions = No open positions right now.
tokens-search-empty-favorites = Star a token and it waits here for the next search.
tokens-search-empty-boosted = No token is boosted right now.
tokens-search-list-failed = This list could not be loaded.
tokens-search-searching = Searching markets…
# $count is the number of tokens found.
tokens-search-result-count =
    { $count ->
        [one] { $count } result
       *[other] { $count } results
    }
tokens-search-order = Best match first, then 24h volume
tokens-search-metric-mc = MC
    .title = Market cap
tokens-search-metric-fdv = FDV
    .title = Fully diluted valuation
tokens-search-metric-liq = Liq
    .title = Liquidity
tokens-search-metric-vol = Vol
    .title = 24h volume
tokens-search-more =
    .title = More actions
    .aria-label = More actions
# $query is the text the user typed.
tokens-search-no-match = No token matches “{ $query }”.

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = Boosted
tokens-featured-category-jupiter-organic = { -jupiter } Top Organic
tokens-featured-category-jupiter-traded = { -jupiter } Top Traded
tokens-featured-category-dexscreener-trending = { -dexscreener } Trending
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = Promoted by their teams
tokens-featured-security-risky = Risky
tokens-featured-load-failed = Failed to load featured
# $message is the failure text.
tokens-featured-network-error = Network error: { $message }
tokens-featured-title = Featured
tokens-featured-subtitle = Boosted tokens first, then trending across Solana
tokens-featured-boost = Boost a Token
tokens-featured-close =
    .title = Close (ESC)
tokens-featured-loading = Loading featured & trending...
tokens-featured-error-hint = Check connection or try again
tokens-featured-empty = No tokens available right now
tokens-featured-count =
    { $count ->
        [one] { $count } token
       *[other] { $count } tokens
    }
tokens-featured-stat-market-cap = Market Cap
tokens-featured-stat-liquidity = Liquidity
tokens-featured-stat-volume = Vol 24H
tokens-featured-stat-holders = Holders
tokens-featured-stat-txns = Txns 24H
# $symbol is the token symbol.
tokens-featured-buy = Buy
    .title = Buy { $symbol }
# $score is the normalized security score out of 100, where higher is safer.
tokens-featured-security-score = Security score: { $score }/100
tokens-featured-social-website = Website
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = All
    .title = Open the full Featured view
tokens-featured-row-scroll-start =
    .aria-label = Show previous tokens
tokens-featured-row-scroll-end =
    .aria-label = Show more tokens
tokens-featured-row-empty = No featured tokens
# $name and $symbol identify the token; $boosts is the formatted active boost count.
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — boosted { $boosts }

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = Select Pool
tokens-pool-selector-loading = Loading pools...
tokens-pool-selector-empty = No pools found for this token
# $message is the failure text.
tokens-pool-selector-load-failed = Failed to load pools: { $message }
tokens-pool-selector-count =
    { $count ->
        [one] { $count } pool found
       *[other] { $count } pools found
    }
# $amount is the formatted pool liquidity in USD.
tokens-pool-selector-liquidity = { $amount } liq
    .title = Liquidity
# $amount is the formatted 24 hour pool volume in USD.
tokens-pool-selector-volume = { $amount } 24h
    .title = 24h Volume

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = Unknown asset
tokens-identity-copy-address =
    .title = Copy address
    .aria-label = Copy address
tokens-identity-copy-signature =
    .title = Copy signature
    .aria-label = Copy signature

# Rugcheck risk names and descriptions, keyed by the provider's English risk name in RUGCHECK_RISK_LABELS (ui/rugcheck_risk.js). A risk the map does not list shows the provider's text.
tokens-rugcheck-risk-single-holder-ownership = Single holder ownership
    .description = One holder owns a large share of the token supply.
tokens-rugcheck-risk-low-liquidity = Low liquidity
    .description = The token's pool holds little liquidity.
tokens-rugcheck-risk-few-lp-providers = Few LP providers
    .description = Only a few users provide liquidity.
tokens-rugcheck-risk-high-holder-concentration = High holder concentration
    .description = The top 10 holders own more than 50% of the token supply.
tokens-rugcheck-risk-top-10-holders-high-ownership = Top 10 holders high ownership
    .description = The top 10 holders own more than 70% of the token supply.
tokens-rugcheck-risk-high-ownership = High ownership
    .description = The top holders own more than 80% of the token supply.
tokens-rugcheck-risk-creator-rug-history = Creator rug history
    .description = The creator has a history of rugging tokens.
tokens-rugcheck-risk-large-lp-unlocked = Large amount of LP unlocked
    .description = A large amount of LP tokens is unlocked, allowing the owner to remove liquidity at any time.
tokens-rugcheck-risk-mutable-metadata = Mutable metadata
    .description = The owner can change the token metadata.
tokens-rugcheck-risk-few-holders = Few holders
    .description = Not many wallets hold the token.
tokens-rugcheck-risk-copycat-token = Copycat token
    .description = This token uses the symbol of a verified token.
tokens-rugcheck-risk-fee-config-enabled = Fee configuration enabled
    .description = The owner can change the fees at any time.
tokens-rugcheck-risk-high-holder-correlation = High holder correlation
    .description = The top holders hold similar amounts of the supply.
tokens-rugcheck-risk-freeze-authority-enabled = Freeze authority enabled
    .description = Tokens can be frozen and blocked from trading.
tokens-rugcheck-risk-mint-authority-enabled = Mint authority enabled
    .description = The owner can mint more tokens.
tokens-rugcheck-risk-missing-file-metadata = Missing file metadata
    .description = No metadata file is linked to this token.
tokens-rugcheck-risk-high-market-cap-per-holder = High market cap per holder
    .description = The market cap is very high relative to the number of holders.
tokens-rugcheck-risk-symbol-mismatch = Symbol mismatch
    .description = The token symbol does not match its metadata file.
tokens-rugcheck-risk-name-mismatch = Name mismatch
    .description = The token name does not match its metadata file.
tokens-rugcheck-risk-permanent-control-enabled = Permanent control enabled
    .description = The token creator can permanently control all tokens.
tokens-rugcheck-risk-missing-metadata = Missing metadata
    .description = No metadata was found for this token.
tokens-rugcheck-risk-lp-unlock-soon = LP unlocking soon
    .description = LP tokens will unlock soon, allowing the owner to remove liquidity.
tokens-rugcheck-risk-lp-vault-unlocked = LP vault unlocked
    .description = LP tokens in the vault can be reclaimed.
tokens-rugcheck-risk-mint-authority-locked = Mint authority locked
    .description = Minting new tokens is locked.
tokens-rugcheck-risk-high-transfer-fee = High transfer fee
    .description = Every transfer of this token pays a high tax.
