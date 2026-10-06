# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = Couldn't fetch a quote — check your connection and try again
# Fallback title when the quote request failed without a message.
trade-quote-error-title = Couldn't fetch a quote
trade-quote-title = Swap Preview
trade-quote-refresh =
    .aria-label = Refresh quote
    .title = Refresh quote
trade-quote-idle = Choose an amount to preview your swap
trade-quote-loading = Finding the best route…
trade-quote-retry = Try again
trade-quote-pay = You pay
trade-quote-receive = You receive (estimated)
trade-quote-minimum = Guaranteed minimum
    .title = The least you can receive after max slippage. The swap reverts rather than fill below it.
trade-quote-impact = Price impact
trade-quote-slippage = Max slippage
trade-quote-platform-fee = Platform fee
    .title = 0.5% — supports development. Already built into the quote above.
trade-quote-network-fee = Network fee
trade-quote-route = Route
trade-quote-disclaimer = Prices update live from the chain. The swap reverts if it can't fill above your guaranteed minimum, so you never get less than shown.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = Price impact { $impact } is above your { $tolerance }% max slippage — this size moves the pool. A smaller amount fills closer to the market price.

## Units

trade-unit-native = { -sol }
trade-unit-tokens = tokens

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = Buy Token
trade-buy-subtitle = Enter amount in { -sol }
trade-buy-confirm = Execute Buy
trade-buy-hint = Leave empty for config default
trade-sell-title = Sell Position
trade-sell-subtitle = Select sell percentage
trade-sell-confirm = Execute Sell
trade-sell-hint = Enter value between 1-100
trade-sell-input = Custom Percentage
    .placeholder = 1-100
trade-add-title = Add to Position
trade-add-subtitle = DCA into existing position
trade-add-confirm = Add Position
trade-add-hint = Leave empty for the configured DCA size
trade-amount-input = Custom Amount
    .placeholder = Enter { -sol } amount

## Presets

trade-presets-quick-amount = Quick Amount
trade-presets-quick-sell = Quick Sell
trade-presets-match-entry = Match Entry
trade-presets-fixed-amount = Fixed Amount
trade-preset-partial = Partial
trade-preset-half = Half
trade-preset-most = Most
trade-preset-full = Full Exit
# $label is the preset's amount.
trade-preset-select =
    .aria-label = Select { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = Close dialog
trade-input-max = MAX
    .aria-label = Use maximum
trade-slider =
    .aria-label = Amount slider
trade-context-available = Available
trade-context-position-size = Position Size
trade-context-holdings = Holdings
trade-held-badge = Held
    .title = You hold an open position in this token
trade-manage-title = Manual management
trade-manage-description = Auto-trader won't sell or DCA this position. Uncheck to let it manage exits.

## Slippage

trade-slippage-label = Slippage
trade-slippage-presets =
    .aria-label = Slippage preset
trade-slippage-auto = Auto
trade-slippage-custom =
    .placeholder = Custom
    .aria-label = Custom slippage percent
trade-slippage-note-auto = Auto (from settings)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = Auto ({ $pct }% from settings)
# $pct is the override as typed.
trade-slippage-note-override = Override: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = High slippage: you may receive up to { $pct }% less than quoted.
trade-impact-warning-title = High Price Impact Warning
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = This trade has a price impact of <strong>{ $impact }</strong>, which exceeds your slippage tolerance of <strong>{ $tolerance }%</strong>. You may receive significantly less than expected.
trade-impact-warning-proceed = Proceed Anyway

## Validation and verification

trade-error-invalid-number = Invalid number
trade-error-percentage-range = Percentage must be between 1 and 100
trade-error-amount-positive = Amount must be greater than 0
trade-error-amount-minimum = Minimum: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = Insufficient balance (need { $needed } plus { $reserve } for fees, have { $balance })
trade-error-position-closed = This position is no longer open.
trade-error-verify-failed = Could not verify token balance
trade-error-position-missing = Position not found - it may have been closed
# $expected and $current are formatted token amounts.
trade-error-balance-changed = Token balance changed. Expected { $expected }, now { $current }. Please refresh.
trade-error-verify-network = Network error verifying balance

## Quick trade

trade-quick-buy-title = Quick Buy
trade-quick-sell-title = Quick Sell
trade-quick-subtitle = Enter token mint address
trade-quick-mint-label = Enter Token Mint Address
trade-quick-mint-input =
    .placeholder = Enter mint address or search by symbol...
trade-quick-paste =
    .aria-label = Paste from clipboard
trade-quick-recent = Recent:
trade-quick-fetching = Fetching token info...
trade-quick-continue = Continue
trade-quick-token-not-found = Token not found
trade-quick-token-failed = Failed to fetch token
trade-quick-token-not-in-database = Token not found in database
trade-quick-token-info-failed = Failed to fetch token info
trade-quick-no-position = No position found for this token
trade-quick-no-holdings = Position has no remaining tokens
trade-quick-position-failed = Failed to fetch position data

## Manual trade toasts

trade-toast-no-mint = No mint address available
trade-toast-open-failed = Could not open the trade dialog
trade-toast-pending-buy = Buy still running
trade-toast-pending-add = Add still running
trade-toast-pending-sell = Sell still running
trade-toast-pending-message = The browser stopped waiting; watch the position row for the result
trade-toast-failed-buy = Buy failed
trade-toast-failed-add = Add to position failed
trade-toast-failed-sell = Sell failed

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = Strategy Signal
trade-reason-manual-entry = Manual Entry
trade-reason-force-buy = Force Buy
trade-reason-copy-buy = Copy Buy
trade-reason-dca-scheduled = DCA Scheduled
trade-reason-take-profit = Take Profit
trade-reason-stop-loss = Stop Loss
trade-reason-trailing-stop = Trailing Stop
trade-reason-time-override = Time Override
trade-reason-strategy-exit = Strategy Exit
trade-reason-llm-analysis-exit = LLM Analysis Exit
trade-reason-manual-exit = Manual Exit
trade-reason-risk-management = Risk Management
trade-reason-blacklisted = Blacklisted
trade-reason-force-sell = Force Sell
trade-reason-copy-sell = Copy Sell
trade-reason-closed-externally = Closed Externally
trade-reason-wallet-history = Wallet History
trade-reason-exit-retry-pending = Exit Retry Pending
trade-reason-synthetic-exit-permanent-failure = Synthetic Exit Permanent Failure
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (pending verification)
# $note is the operator text of a force close.
trade-reason-force-closed = Force closed: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = No token selected
