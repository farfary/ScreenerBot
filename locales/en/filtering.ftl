# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = No decimals in database
filtering-reject-token-too-new = Token too new
filtering-reject-cooldown-filtered = Cooldown filtered
filtering-reject-dex-data-missing = { -dexscreener } data missing
filtering-reject-gecko-data-missing = { -geckoterminal } data missing
filtering-reject-rug-data-missing = { -rugcheck } data missing
filtering-reject-onchain-numeric-symbol = Numeric-only symbol (scam)
filtering-reject-onchain-empty-symbol = Empty symbol (scam)
filtering-reject-onchain-suspicious-symbol = Suspicious symbol (scam)
filtering-reject-onchain-known-scam-authority = Known scam authority
filtering-reject-onchain-immutable-with-freeze = Immutable + freeze authority (scam)
filtering-reject-onchain-high-risk-score = On-chain high risk score
filtering-reject-dex-empty-name = Empty name
filtering-reject-dex-empty-symbol = Empty symbol
filtering-reject-dex-empty-logo = Empty logo URL
filtering-reject-dex-empty-website = Empty website URL
filtering-reject-dex-txn-5m = Low 5m transactions
filtering-reject-dex-txn-1h = Low 1h transactions
filtering-reject-dex-zero-liq = Zero liquidity
filtering-reject-dex-liq-low = Liquidity too low
filtering-reject-dex-liq-high = Liquidity too high
filtering-reject-dex-mcap-low = Market cap too low
filtering-reject-dex-mcap-high = Market cap too high
filtering-reject-dex-vol-low = Volume too low
filtering-reject-dex-vol-missing = Volume missing
filtering-reject-dex-fdv-low = FDV too low
filtering-reject-dex-fdv-high = FDV too high
filtering-reject-dex-vol5m-low = 5m volume too low
filtering-reject-dex-vol5m-missing = 5m volume missing
filtering-reject-dex-vol1h-low = 1h volume too low
filtering-reject-dex-vol1h-missing = 1h volume missing
filtering-reject-dex-vol6h-low = 6h volume too low
filtering-reject-dex-vol6h-missing = 6h volume missing
filtering-reject-dex-price-change-5m-low = 5m price change too low
filtering-reject-dex-price-change-5m-high = 5m price change too high
filtering-reject-dex-price-change-low = Price change too low
filtering-reject-dex-price-change-high = Price change too high
filtering-reject-dex-price-change-6h-low = 6h price change too low
filtering-reject-dex-price-change-6h-high = 6h price change too high
filtering-reject-dex-price-change-24h-low = 24h price change too low
filtering-reject-dex-price-change-24h-high = 24h price change too high
filtering-reject-gecko-liq-low = Liquidity too low
filtering-reject-gecko-liq-high = Liquidity too high
filtering-reject-gecko-mcap-low = Market cap too low
filtering-reject-gecko-mcap-high = Market cap too high
filtering-reject-gecko-vol5m-low = 5m volume too low
filtering-reject-gecko-vol5m-missing = 5m volume missing
filtering-reject-gecko-vol1h-low = 1h volume too low
filtering-reject-gecko-vol1h-missing = 1h volume missing
filtering-reject-gecko-vol24h-low = 24h volume too low
filtering-reject-gecko-vol24h-missing = 24h volume missing
filtering-reject-gecko-price-change-5m-low = 5m price change too low
filtering-reject-gecko-price-change-5m-high = 5m price change too high
filtering-reject-gecko-price-change-1h-low = 1h price change too low
filtering-reject-gecko-price-change-1h-high = 1h price change too high
filtering-reject-gecko-price-change-24h-low = 24h price change too low
filtering-reject-gecko-price-change-24h-high = 24h price change too high
filtering-reject-gecko-pool-count-low = Pool count too low
filtering-reject-gecko-pool-count-high = Pool count too high
filtering-reject-gecko-pool-count-missing = Pool count missing
filtering-reject-gecko-reserve-low = Reserve too low
filtering-reject-gecko-reserve-missing = Reserve missing
filtering-reject-rug-rugged = Rugged token
filtering-reject-rug-score = Risk score too high
filtering-reject-rug-level-danger = Danger risk level
filtering-reject-rug-mint-authority = Mint authority present
filtering-reject-rug-freeze-authority = Freeze authority present
filtering-reject-rug-top-holder = Top holder % too high
filtering-reject-rug-top3-holders = Top 3 holders % too high
filtering-reject-rug-min-holders = Not enough holders
filtering-reject-rug-insider-count = Too many insider holders
filtering-reject-rug-insider-pct = Insider % too high
filtering-reject-rug-creator-pct = Creator balance too high
filtering-reject-rug-transfer-fee-present = Transfer fee present
filtering-reject-rug-transfer-fee-high = Transfer fee too high
filtering-reject-rug-graph-insiders = Graph insiders too high
filtering-reject-rug-lp-providers-low = LP providers too low
filtering-reject-rug-lp-providers-missing = LP providers missing
filtering-reject-rug-lp-lock-low = LP lock too low
filtering-reject-rug-lp-lock-missing = LP lock missing
filtering-reject-llm-analysis-rejected = LLM Analysis Rejected: { $reason } ({ $confidence }% conf, { $provider })
filtering-reject-llm-analysis-rejected-generic = LLM Analysis Rejected
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = FDV missing
filtering-reject-dex-price-change-5m-missing = 5m price change missing
filtering-reject-dex-price-change-missing = Price change missing
filtering-reject-dex-price-change-6h-missing = 6h price change missing
filtering-reject-dex-price-change-24h-missing = 24h price change missing
filtering-reject-gecko-liq-missing = Liquidity missing
filtering-reject-gecko-mcap-missing = Market cap missing
filtering-reject-gecko-price-change-5m-missing = 5m price change missing
filtering-reject-gecko-price-change-1h-missing = 1h price change missing
filtering-reject-gecko-price-change-24h-missing = 24h price change missing
filtering-reject-rug-transfer-fee-missing = Transfer fee data missing

# Rejection categories used to group reasons.
filtering-reject-category-security = Security Issues
filtering-reject-category-distribution = Holder Distribution
filtering-reject-category-liquidity-lock = LP Lock Issues
filtering-reject-category-fees = Transfer Fees
filtering-reject-category-liquidity = Liquidity
filtering-reject-category-volume = Trading Volume
filtering-reject-category-market-cap = Market Cap/FDV
filtering-reject-category-price-action = Price Movement
filtering-reject-category-activity = Trading Activity
filtering-reject-category-data-quality = Missing Data
filtering-reject-category-timing = Timing Filters
filtering-reject-category-market = Market Data
filtering-reject-category-other = Other

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = Status
filtering-tab-analytics = Analytics
filtering-tab-explorer = Explorer
filtering-source-core = Core
filtering-source-onchain = On-Chain
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = LLM Analysis

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = All
filtering-range-all-time = All Time
filtering-range-custom = Custom
filtering-range-now = Now
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = Saving changes...
filtering-footer-refreshing = Refreshing snapshot...
filtering-footer-unsaved = Unsaved changes pending
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = Last saved { $time }
filtering-footer-in-sync = Configuration in sync

## Info bar and status metrics

filtering-info-total = Total
filtering-info-priced = Priced
filtering-info-passed = Passed
filtering-info-positions = Positions
filtering-info-blacklisted = Blacklisted
filtering-info-cache = Cache
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = Building…
filtering-refresh-never = Never

filtering-status-loading = Loading statistics...
filtering-status-total = Total Tokens
filtering-status-total-detail = In filtering cache
filtering-status-total-detail-building = Snapshot building — counts land on the next refresh
filtering-status-priced = With Price
filtering-status-priced-detail = { $share } have pricing
filtering-status-passed = Passed Filters
filtering-status-passed-detail = { $share } passed
filtering-status-positions = Open Positions
filtering-status-positions-detail = Active trades
filtering-status-blacklisted = Blacklisted
filtering-status-blacklisted-detail = Flagged tokens
filtering-status-ohlcv = With OHLCV
filtering-status-ohlcv-detail = Historical data
filtering-status-refresh = Last Refresh
filtering-status-refresh-building = First snapshot in progress
filtering-status-refresh-none = No refresh yet
filtering-status-no-rejections = No rejection data available

## Analytics

filtering-analytics-loading = Loading analytics for { $range }…
filtering-analytics-scanned = Total Scanned
# $time is a relative time such as "5m ago".
filtering-analytics-updated = Updated { $time }
filtering-analytics-passed = Passed Tokens
filtering-analytics-pass-rate = <strong>{ $share }</strong> pass rate
filtering-analytics-rejected = Rejected Tokens
filtering-analytics-rejection-rate = <strong>{ $share }</strong> rejection rate
filtering-analytics-by-category = Rejection by Category
filtering-analytics-by-source = Rejection by Source
filtering-analytics-no-category = No category data
filtering-analytics-no-source = No source data
filtering-analytics-top-reasons = Top Rejection Reasons
filtering-analytics-no-data = No data available
filtering-analytics-column-reason = Reason
filtering-analytics-column-category = Category
filtering-analytics-column-count = Count
filtering-analytics-column-share = %
filtering-analytics-column-impact = Impact
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
        [one] { $amount } token
       *[other] { $amount } tokens
    }

## Explorer

filtering-explorer-top-reasons = Top Reasons
filtering-explorer-recent = Recent Rejections
filtering-explorer-none = No data
filtering-explorer-none-recent = No recent
filtering-explorer-search =
    .placeholder = Search reasons...
filtering-explorer-overview = Overview
filtering-explorer-no-match = No matching reasons
filtering-explorer-column-token = Token
filtering-explorer-column-source = Source
filtering-explorer-column-time = Time
filtering-explorer-page = Page { $page }
filtering-explorer-no-results = No results
filtering-explorer-empty = No tokens found
filtering-explorer-empty-filtered = No tokens found matching filter
filtering-explorer-load-failed = Failed to load tokens

## Configuration panels

filtering-config-loading = Loading configuration…
# $query is the text typed in the filter box.
filtering-config-no-match = No parameter matches “{ $query }”
filtering-config-no-parameters = This source exposes no parameters
# $source is the source name.
filtering-source-off = { $source } filtering is off — these parameters are not evaluated.
filtering-toolbar-filter =
    .placeholder = Filter parameters
    .aria-label = Filter parameters
filtering-toolbar-clear =
    .aria-label = Clear filter
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
        [one] { $amount } parameter
       *[other] { $amount } parameters
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
        [one] { $visible } of { $total } parameter
       *[other] { $visible } of { $total } parameters
    }
filtering-group-enable =
    .aria-label = Enable { $group } checks
filtering-field-min = Min
filtering-field-max = Max
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = Minimum { $label }
filtering-field-max-aria =
    .aria-label = Maximum { $label }
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = Reset to default ({ $default })
    .aria-label = Reset { $label } to default

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = Configuration Saved
    .message = Filtering settings saved and snapshot refreshed
filtering-toast-save-failed = Save Failed
    .message = Failed to save filtering configuration
filtering-toast-reset = Changes Reset
    .message = Configuration restored to last saved state
filtering-toast-refresh-failed = Refresh Failed
    .message = Failed to refresh filtering snapshot
filtering-toast-exported = Configuration Exported
    .message = Filtering settings saved to file
filtering-toast-imported = Configuration Imported
    .message = Filtering settings loaded from file
filtering-toast-import-failed = Import Failed
    .message = Failed to import configuration - invalid file format
filtering-toast-load-failed = Load Failed
    .message = Failed to load filtering configuration
filtering-toast-range-missing = Please select both start and end dates
filtering-toast-range-order = Start time must be before end time
filtering-toast-range-future = End time cannot be in the future
