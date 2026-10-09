# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = Position created


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = Open
positions-status-closed = Closed
positions-status-archived = Archived
# Empty positions table per status (POSITION_EMPTY_LABELS)
positions-open-empty = No open positions
    .message = A position appears here when the auto trader or a manual buy opens one.
positions-closed-empty = No closed positions
    .message = A position moves here once it is fully sold.
positions-archived-empty = No archived positions
    .message = Positions you remove with Archive are kept here.
positions-origin-copy = Copy
positions-origin-manual = Manual
positions-origin-wallet = Wallet
positions-origin-copy-link =
    .title = Open the copy task that opened this position
positions-holding-frozen = Frozen
    .title = The mint authority froze this token account - the balance cannot be transferred or sold
positions-toolbar-total = Total
positions-toolbar-delete-all = Delete all
positions-search-placeholder = Search by symbol or mint...
positions-filter-origin = Origin
positions-filter-origin-all = All origins
positions-filter-origin-auto = Auto Trader
positions-filter-origin-copy = Copy Trading
positions-delete-all-tooltip = Permanently delete all archived positions

## Columns

positions-column-token = Token
positions-column-archived-at = Archived
positions-column-entry-time = Entry Time
positions-column-exit-time = Exit Time
positions-column-avg-entry = Avg Entry ({ -sol })
positions-column-avg-exit = Avg Exit ({ -sol })
positions-column-current-price = Current ({ -sol })
positions-column-total-invested = Total Invested
positions-column-proceeds = Proceeds
positions-column-pnl = P&L
positions-column-pnl-percent = P&L %
positions-column-size = Size
positions-column-dca = DCA
positions-column-exits = Exits
positions-column-unrealized-pnl = Unrealized P&L
positions-column-unrealized-percent = Unrealized %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = No cost basis in this wallet's history (airdrop, USD-quoted fill, or a swap with no SOL leg)
positions-unknown-history = This round does not reconcile with the on-chain balance
positions-dca-count =
    { $count ->
        [one] { $count } DCA
       *[other] { $count } DCAs
    }
positions-exit-count =
    { $count ->
        [one] { $count } exit
       *[other] { $count } exits
    }

## Row actions

positions-action-add =
    .title = Add to position (DCA)
    .aria-label = Add to position
positions-action-sell =
    .title = Sell (full or % partial)
    .aria-label = Sell position
positions-action-sell-frozen = Frozen by the mint authority - this holding cannot be sold
positions-action-remove =
    .title = Remove (archive or delete)
    .aria-label = Remove position
positions-action-restore =
    .title = Restore to Open/Closed
    .aria-label = Restore position
positions-action-delete =
    .title = Delete permanently
    .aria-label = Delete permanently
positions-action-in-progress = In progress…

## Live state of a row

positions-caption-buying = Buying
# $step is the label of the current action step.
positions-caption-buying-step = Buying · { $step }
positions-caption-selling = Selling
positions-caption-selling-step = Selling · { $step }
positions-caption-closing = Closing
positions-caption-failed = Failed
# $error is the failure text of the action.
positions-caption-failed-detail = Failed · { $error }
positions-step-adding = Adding
positions-pending-buying = Buying…
positions-pending-buy-failed = Buy failed

## Messages and confirmations

positions-load-failed = Could not refresh positions
positions-toast-not-found = Position data not found
positions-toast-deleted = Position deleted
positions-toast-archived = Position archived
positions-toast-restored = Position restored
positions-buy-adds-to-archived = This token already has an open position in the archive. The buy is added to that position, and it moves back to the open positions. It keeps its current management mode.
positions-action-failed = Action failed
positions-delete-title = Delete position permanently
# $symbol is the token symbol.
positions-delete-message = Permanently delete { $symbol }? This removes the position and its history from the database and cannot be undone. Your transactions and token data are not affected.
positions-delete-confirm = Delete permanently
positions-delete-all-title = Delete all archived positions
positions-delete-all-message =
    { $count ->
        [one] Permanently delete all { $count } archived position? This cannot be undone. Transactions and token data are not affected.
       *[other] Permanently delete all { $count } archived positions? This cannot be undone. Transactions and token data are not affected.
    }
positions-delete-all-message-empty = Permanently delete all archived positions? This cannot be undone.
positions-delete-all-confirm = Delete all
positions-delete-all-done =
    { $count ->
        [one] Deleted { $count } archived position
       *[other] Deleted { $count } archived positions
    }
positions-delete-all-failed = Failed to delete archived positions

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = Remove position
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>This position is still open.</strong> The bot is holding this token. Removing it frees the trade slot and stops tracking — but it does <strong>not</strong> sell. Sell first if you want your { -sol } back.
positions-remove-modes =
    .aria-label = Removal mode
positions-remove-archive = Archive
positions-remove-recommended = Recommended
positions-remove-archive-description = Hide it into the Archived tab. Reversible anytime — nothing is sold and all trades stay on record.
positions-remove-delete = Delete permanently
positions-remove-delete-description = Erase this position and its full history from the database.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = This permanently removes the position and its history. <strong>This cannot be undone.</strong> Your transactions and token data are not affected.
positions-remove-confirm-archive = Archive position

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = Position management set to { $mode }
positions-details-load-failed = Failed to load position details
positions-details-mint-label = Mint address
positions-details-management-failed = Failed to update position management
positions-details-favorite-add =
    .title = Add to favorites
    .aria-label = Add to favorites
positions-details-favorite-remove =
    .title = Remove from favorites
    .aria-label = Remove from favorites
positions-details-view-solscan =
    .title = View on { -solscan }
    .aria-label = View token on { -solscan }
positions-details-close =
    .title = Close (Esc)
    .aria-label = Close
positions-details-chart-section =
    .aria-label = Price chart
positions-details-loading-chart = Loading chart...
positions-details-activity-section =
    .aria-label = Activity
positions-details-activity-title = Activity
positions-details-split-handle =
    .aria-label = Resize chart and activity
positions-details-activity-pane =
    .aria-label = Activity pane
positions-details-activity-expand =
    .title = Expand activity
    .aria-label = Expand activity
positions-details-summary-section =
    .aria-label = Position summary
positions-details-loading = Loading position...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = Auto Trader
positions-management-user-only = User Only
positions-management-copy-task = Copy Task
positions-management-hybrid = Hybrid
positions-pane-show-chart = Show chart
positions-pane-show-activity = Show activity
positions-pane-restore-activity = Restore activity
positions-pane-expand-chart =
    .title = Expand chart
    .aria-label = Expand chart

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = Low risk
positions-risk-medium = Medium risk
positions-risk-high = High risk
positions-risk-unknown = Risk unknown
positions-busy-buying = Buy in progress…
positions-busy-selling = Sell in progress…
positions-busy-closing = Close in progress…
positions-header-avg-entry = Avg entry
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
        [one] { $count } buy
       *[other] { $count } buys
    }
positions-header-exit-price = Exit price
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = closed { $ago }
positions-header-realized-pnl = Realized P&L
positions-header-usd-note = USD at today's { -sol } price
positions-header-returned = Returned
# $amount is the formatted SOL amount invested.
positions-header-of-invested = of { $amount } invested
positions-header-price = Price
positions-header-last-price = Last price
positions-header-pool-ago = pool · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = Unrealized P&L
positions-header-pnl-last-price = P&L at last price
positions-header-value = Value
positions-header-last-value = Last value
positions-header-invested = { $amount } invested
positions-header-origin-hint = How this position was opened
positions-header-risk-hint = { -rugcheck } score — lower is safer
positions-header-frozen = Frozen
    .title = The mint authority froze this holding
positions-header-managed-by = Managed by
positions-header-management-select =
    .aria-label = Position management

## Entry origin shown in the header badge

positions-origin-unknown = unknown
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = Copied · task { $task }
positions-origin-manual-entry = Manual entry
positions-origin-wallet-entry = Wallet entry
# $strategy is the strategy id.
positions-origin-auto-strategy = Auto · { $strategy }
positions-origin-auto-entry = Auto entry

## Swaps that are submitted and not yet booked

positions-pending-adding = Adding
positions-pending-adding-amount = Adding { $amount }
positions-pending-selling = Selling
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = Selling { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · confirming
    .title = Submitted and waiting for on-chain confirmation. The figures update once it is verified.

## Trade controls

positions-trade-add = Add
    .title = Add to position
positions-trade-sell = Sell
    .title = Sell part of the position
positions-trade-close = Close position
    .title = Sell everything and close
positions-trade-token = Token details
    .title = Open token details

## Favorites

positions-favorite-token-fallback = Token
# $symbol is the token symbol.
positions-favorite-added = { $symbol } added to favorites
positions-favorite-removed = { $symbol } removed from favorites
positions-favorite-add-failed = Failed to add favorite
positions-favorite-remove-failed = Failed to remove favorite
positions-favorite-update-failed = Failed to update favorites

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = Position
positions-summary-price-path = Price path
positions-summary-network-fees = Network fees
positions-summary-risk = Risk
positions-summary-market = Market
positions-summary-market-now = Market now
positions-summary-links = Links
positions-fact-tokens-fallback = tokens
positions-fact-bought = Bought
positions-fact-holding = Holding
positions-fact-sold = Sold
positions-fact-realized = Realized
positions-fact-opened = Opened
positions-fact-closed = Closed
positions-fact-reason = Reason
positions-fact-archived = Archived
positions-fact-entry = Entry
positions-fact-exit = Exit
positions-fact-total = Total
positions-fact-verified = Verified on chain
positions-fact-confirming = Confirming
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = { $percent } of bought
positions-fact-share-of-invested = { $percent } of invested
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 1 entry
        [one] 1 entry + { $count } add
       *[other] 1 entry + { $count } adds
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } partial exit · { $returned } back
       *[other] { $count } partial exits · { $returned } back
    }
# $age is the elapsed time of the hold.
positions-fact-held = held { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = { $percent } vs entry
positions-fact-exit-vs-peak = Exit vs peak
positions-fact-now-vs-peak = Now vs peak
positions-fact-entry-range = Entry range
positions-range-low = Low
positions-range-peak = Peak
positions-range-now = Now
positions-range-label-exit = Entry and exit price between the low and the peak
positions-range-label-now = Entry and now price between the low and the peak
positions-fact-mint-authority = Mint authority
positions-fact-freeze-authority = Freeze authority
positions-fact-active = Active
positions-fact-pool = Pool
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = { $amount } { -sol } liquidity
positions-fact-market-cap = Market cap
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Liquidity
positions-fact-volume-24h = Volume 24h
positions-fact-price-change = Price change
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = Holders
positions-link-website = Website
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = Activity could not be loaded
positions-activity-loading = Loading activity...
positions-activity-empty = Nothing has happened to this token in this wallet yet
positions-activity-filter-empty = No activity matches this filter
positions-activity-round-count =
    { $count ->
        [one] { $count } round
       *[other] { $count } rounds
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } event
       *[other] { $count } events
    }
positions-activity-pending-count = { $count } pending
positions-activity-failed-count = { $count } failed
positions-filter-all = All
positions-filter-trades = Trades
positions-filter-buys = Buys
positions-filter-sells = Sells
positions-filter-wallet = Wallet
positions-filter-issues = Issues
positions-activity-filters =
    .aria-label = Filter activity
positions-activity-totals =
    .aria-label = All rounds on this token
positions-activity-realized-all = Realized, all rounds
positions-activity-invested = Invested
positions-activity-returned = Returned
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = Opened { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = Position { $index }
positions-activity-this-position = This position
positions-activity-dates-unavailable = Dates unavailable
positions-activity-wallet-title = Wallet transactions
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
        [one] Outside any position · { $range } · { $count } event
       *[other] Outside any position · { $range } · { $count } events
    }
positions-details-signature-label = Signature

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = Position open
positions-state-closing = Position closing
positions-state-closed = Position closed
positions-state-exit-pending = Position exit pending
positions-state-exit-failed = Position exit failed
positions-state-phantom = Position phantom
positions-state-reconciling = Position reconciling

## Activity events

positions-event-kind-entry = Entry
positions-event-kind-dca = Add
positions-event-kind-partial-exit = Partial exit
positions-event-kind-exit = Exit
positions-event-kind-buy = Wallet buy
positions-event-kind-sell = Wallet sell
positions-event-kind-transfer = Transfer
positions-event-kind-ata = Token account
positions-event-kind-other = Transaction
positions-event-state-pending = Pending
positions-event-state-failed = Failed
positions-event-state-synthetic = Synthetic
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = Failed: { $error }
positions-event-tokens-fallback = tokens
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = Submitted buy for { $amount }
positions-event-entry-for = Bought { $amount } for { $sol }
positions-event-entry = Bought { $amount }
positions-event-dca-submitted = Submitted add for { $amount }
positions-event-dca-for = Added { $amount } for { $sol }
positions-event-dca = Added { $amount }
positions-event-partial-exit-submitted-percent = Submitted { $percent } partial exit for { $amount }
positions-event-partial-exit-submitted = Submitted partial exit for { $amount }
positions-event-sold-percent-for = Sold { $amount } ({ $percent }) for { $sol }
positions-event-sold-percent = Sold { $amount } ({ $percent })
positions-event-sold-for = Sold { $amount } for { $sol }
positions-event-sold = Sold { $amount }
positions-event-exit-submitted = Submitted full position exit
positions-event-exit-for = Closed with { $amount } sold for { $sol }
positions-event-exit-closed = Closed the position
positions-event-wallet-bought = Wallet bought { $amount } elsewhere
positions-event-wallet-sold = Wallet sold { $amount } elsewhere
positions-event-received = Received { $amount }
positions-event-sent = Sent { $amount }
positions-event-transferred = Transferred { $amount }
positions-event-ata = Token account activity
positions-event-wallet-transaction = Wallet transaction involving { $amount }
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / token
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = { $amount } wallet change
positions-event-after-title = Position after this event
positions-event-capital-invested = Capital invested
positions-event-average-entry = Average entry
positions-event-transfers-title = Token transfers
positions-event-transfer-amount = Amount
positions-event-transfer-mint = Mint
positions-event-transfer-from = From
positions-event-transfer-to = To
positions-event-no-signature = No on-chain signature
positions-event-click-to-copy = Click to copy
positions-event-solscan = { -solscan }
positions-event-token-amount = Token amount
positions-event-trade-price = Trade price
positions-event-native-amount = { -sol } amount
positions-event-cost-basis = Cost basis
positions-event-usd-value = USD value
positions-event-network-fee = Network fee
positions-event-router = Router
positions-event-slot = Slot
positions-event-chain-status = Chain status
positions-event-transaction-type = Transaction type
positions-event-direction = Direction
positions-event-wallet-native-change = Wallet { -sol } change
positions-event-instructions = Instructions
positions-event-compute-units = Compute units
positions-event-accounts = Accounts
positions-event-record-id = Record ID
positions-event-time-unavailable = Time unavailable
positions-event-details = Details
positions-event-hide-details = Hide details

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = Candles
positions-chart-type-line = Line
positions-chart-type-area = Area
positions-chart-type-group =
    .aria-label = Chart type
positions-chart-overlays-group =
    .aria-label = Chart overlays
positions-chart-ema = EMA
    .title = Exponential moving averages, 9 and 21
positions-chart-fit = Fit
    .title = Frame this position's lifetime
positions-chart-timeframes-group =
    .aria-label = Timeframe
positions-chart-pane-group =
    .aria-label = Chart pane
positions-chart-unavailable = Chart engine unavailable
positions-chart-collecting = Collecting chart data…
positions-chart-no-data = No chart data for this token yet
positions-chart-avg-entry = Avg Entry
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Avg entry
positions-chart-legend-avg-entry-off-scale = Avg entry (off scale)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } event without a candle on this timeframe
       *[other] { $count } events without a candle on this timeframe
    }
positions-chart-level = Level
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } is above this view
positions-chart-level-below = { $label } { $price } is below this view
positions-chart-scale-hint = Drag the price axis to scale out to it
positions-chart-pnl-at-bar = P&L @ Bar
positions-chart-click-to-locate = Click to locate
