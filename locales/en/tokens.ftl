# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = Live market data
tokens-result-source-unavailable = { $label } unavailable — retrying
tokens-result-source-not-listed = Not listed on { $label }
tokens-result-security-available = Security report available
tokens-result-security-missing = No Rugcheck report
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
tokens-positions-fact-sol-received = { -sol } Received
tokens-positions-fact-closed-reason = Closed Reason
tokens-positions-fact-target-min = Profit Target Min
tokens-positions-fact-target-max = Profit Target Max
tokens-positions-fact-highest = Highest Price
tokens-positions-fact-lowest = Lowest Price
tokens-positions-section-range = Targets & range
tokens-positions-section-market = Market & holdings
tokens-positions-kicker = Position
tokens-positions-fallback-symbol = Token
tokens-positions-realized-pnl = Realized PnL
tokens-positions-unrealized-pnl = Unrealized PnL
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
tokens-security-severity-critical = Critical
tokens-security-severity-warning = Warning
tokens-security-severity-info = Info
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
tokens-overview-fact-pool-sol = Pool { -sol }
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
tokens-pools-total = Total Pools
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
tokens-pools-address-copy = Copy address
tokens-pools-address-pool = Pool
    .title = Copy pool
tokens-pools-address-base = Base mint
    .title = Copy base mint
tokens-pools-address-quote = Quote mint
    .title = Copy quote mint
tokens-pools-address-paired = Paired mint
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
tokens-links-explorer-gmgn = { -gmgn }
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
tokens-dialog-unit-sol = { -sol }
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
tokens-dialog-copy-mint =
    .title = Copy Mint Address
    .aria-label = Copy Mint Address
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
tokens-dialog-badge-auth = Auth:
tokens-dialog-badge-update-authority = Update Authority:
tokens-dialog-badge-position = Position
tokens-dialog-badge-blacklisted = Blacklisted
