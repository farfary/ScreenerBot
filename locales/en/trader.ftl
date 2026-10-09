# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Stop Loss
trader-exit-type-take-profit = Take Profit
trader-exit-type-roi = ROI Target
trader-exit-type-roi-exit = ROI Target
trader-exit-type-trailing-stop = Trailing Stop
trader-exit-type-time-override = Time Override
trader-exit-type-time-rule = Time Rule
trader-exit-type-manual = Manual
trader-exit-type-manual-close = Manual
trader-exit-type-dca = DCA
trader-exit-type-unknown = Unknown

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = Stats
trader-tab-strategy-control = Strategy Control
trader-tab-strategies = Strategies
trader-tab-stop-loss = Stop Loss
trader-tab-trailing-stop = Trailing Stop
trader-tab-roi = Take Profit
trader-tab-time-rules = Time Rules
trader-tab-dca = DCA
trader-tab-settings = Settings

## Feature status badges and their messages

trader-feature-coming-soon = Coming Soon
    .message = This feature is coming soon and not yet available.
trader-feature-beta = Beta
trader-feature-disabled = Disabled
    .message = This feature is currently disabled.

## Status bar and trading controls

trader-status-title = Auto Trader
trader-status-loading = Loading...
trader-status-running = Running
trader-status-stopped = Stopped
trader-status-setup-required = Setup required
trader-status-unavailable = Complete wallet and RPC setup to use Auto Trader
trader-toggle-on = ON
trader-toggle-off = OFF
trader-toggle-unavailable = UNAVAILABLE
trader-toggle-start-failed = Failed to start trader
trader-toggle-stop-failed = Failed to stop trader
trader-controls-title = Trading Controls
trader-halt-title = TRADING HALTED
trader-halt-reason-default = Manual force stop
trader-halt-resume = Resume
trader-monitor-entry = Entry Monitor
trader-monitor-exit = Exit Monitor
trader-monitor-master-off = Auto Trader off
trader-loss-limit-title = Period Loss Limit
trader-loss-limit-resume = Resume trading
trader-loss-limit-reset = Reset period
trader-loss-limit-off = Off
trader-loss-limit-none = No period loss limit configured
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = Resets in { $hours } { $minutes }
trader-loss-limit-reached = LIMIT REACHED
trader-force-stop = Force Stop Everything

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = Force Stop Trading
    .message = This will immediately halt ALL trading operations. Continue?
    .confirm = Stop Trading
trader-loss-limit-resume-confirm = Resume After Loss Limit
    .message = The period loss limit stopped new entries. Resuming lets the trader open positions again before the period resets. Continue?
trader-loss-limit-reset-confirm = Reset Loss Limit Period
    .message = This clears the accumulated loss for the current period and starts a new one. Continue?

## Toasts

trader-toast-control-failed = Auto Trader control failed
trader-toast-force-stop-on = Force stop activated
trader-toast-force-stop-failed = Could not activate force stop
trader-toast-force-stop-cleared = Force stop cleared
trader-toast-resume-failed = Could not resume trading
trader-toast-loss-limit-reset-failed = Could not reset the loss limit
trader-toast-entry-monitor-failed = Could not toggle the entry monitor
trader-toast-exit-monitor-failed = Could not toggle the exit monitor
trader-toast-load-failed = Load Failed
    .message = Failed to load trader configuration
trader-toast-saved = Configuration Saved
    .message = Trader settings applied successfully
trader-toast-save-failed = Save Failed
    .message = Failed to save trader configuration
trader-toast-feature-enabled = Feature Enabled
trader-toast-feature-disabled = Feature Disabled
trader-toast-feature-applied = Auto Trader setting applied
trader-toast-strategy-enabled = Strategy Enabled
    .message = Strategy is active
trader-toast-strategy-disabled = Strategy Disabled
    .message = Strategy is inactive
trader-toast-strategy-failed = Update Failed
    .message = Failed to update strategy status

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Statistics window
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = Realized Performance
trader-metric-net-pnl = Net P&L
trader-metric-win-rate = Win Rate
trader-metric-profit-factor = Profit Factor
trader-metric-max-drawdown = Max Drawdown
trader-metric-capital = Capital at Work
trader-metric-avg-win-loss = Avg Win / Loss
trader-metric-closed-trades = Closed Trades
trader-metric-median-hold = Median Hold
trader-stats-empty = No closed trades in this window
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = { $won } won · { $lost } lost
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } win
       *[other] { $amount } wins
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } loss
       *[other] { $amount } losses
    }
# $amount is a formatted SOL amount.
trader-stats-expected = { $amount } expected per trade
trader-stats-profit-factor-basis = Gross won ÷ gross lost
trader-stats-drawdown-basis = Deepest realized peak-to-trough
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
        [one] { $used } of { $max } position slot used
       *[other] { $used } of { $max } position slots used
    }
trader-stats-avg-basis = Average outcome of a winning vs losing trade
trader-stats-closed =
    { $count ->
        [one] { $amount } position closed
       *[other] { $amount } positions closed
    }
# $span is a formatted duration.
trader-stats-hold-average = { $span } average
trader-stats-excluded =
    { $count ->
        [one] { $amount } closed round excluded — no complete cost basis, so no honest P&L.
       *[other] { $amount } closed rounds excluded — no complete cost basis, so no honest P&L.
    }

## Stats: daily P&L and extremes

trader-daily-title = Daily P&L
trader-daily-subtitle = Realized { -sol } per day, with the running total
trader-daily-loading = Loading daily P&L...
trader-daily-chart = Daily realized profit and loss in { -sol }
trader-extreme-best = Best trade
trader-extreme-worst = Worst trade

## Stats: exit breakdown

trader-exit-title = Exit Strategy Breakdown
trader-exit-subtitle = How positions were closed, and what each exit returned
trader-exit-loading = Loading exit data...
trader-exit-empty-day = No closed trades in the last 24 hours
trader-exit-empty-days =
    { $count ->
        [one] No closed trades in the last { $amount } day
       *[other] No closed trades in the last { $amount } days
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
        [one] { $amount } trade · { $share } of exits
       *[other] { $amount } trades · { $share } of exits
    }
# $value is a formatted average percentage.
trader-exit-average = { $value } avg

## Shared example vocabulary

trader-impact-label = Impact:
trader-current-label = Current:
trader-readable-label = Readable:
trader-example-how-it-works = How It Works
trader-step-entry = Entry
trader-step-initial-position = Initial position
trader-step-auto-exit = Auto Exit
trader-step-exit = Exit
trader-step-full-exit = Full position exit
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = +{ $value }% profit

## Stop loss

trader-stop-loss-title = Stop Loss
trader-stop-loss-subtitle = Automatically exit a position when its loss exceeds your threshold
# $threshold is the threshold as typed.
trader-stop-loss-impact = Exit when down { $threshold }% from entry
trader-stop-loss-hold-immediate = Immediate
# $span is a formatted duration.
trader-stop-loss-hold-delay = { $span } delay
trader-stop-loss-price-falls = Price Falls
trader-stop-loss-threshold-reached = Threshold reached
trader-stop-loss-partial = Partial exits allowed
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = Loss limited to <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Note:</strong> Stop loss protects against larger losses by exiting early

## Trailing stop

trader-trailing-title = Trailing Stop
trader-trailing-subtitle = Automatically protect profits by trailing the price as it rises
# $value is the activation percentage as typed.
trader-trailing-activation-impact = Starts trailing at +{ $value }% profit
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = Exits at -{ $value }% from peak
trader-trailing-activation = Activation
trader-trailing-peak = Peak
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% final
# $value is a formatted percentage.
trader-trailing-summary-protected = Protected <strong>{ $value }</strong> profit
# $value is a formatted percentage.
trader-trailing-summary-avoided = Avoided <strong>{ $value }</strong> loss from peak

## Take profit

trader-roi-title = Take Profit
trader-roi-subtitle = Automatically exit the entire position when profit reaches your target
# $target is the target percentage as typed.
trader-roi-impact = Exit at +{ $target }% profit
trader-roi-example-title = Example Scenario
trader-roi-initial-buy = Initial buy
trader-roi-target-hit = Target Hit
trader-roi-full-position = Full Position
trader-roi-sold = 100% sold
# $target is the target percentage as typed.
trader-roi-summary = Locked in <strong>+{ $target }%</strong> profit

## Time-based exit

trader-time-title = Time-Based Exit
trader-time-subtitle = Automatically exit positions after a maximum hold time if loss exceeds the threshold
trader-time-unit-seconds = seconds
trader-time-unit-minutes = minutes
trader-time-unit-hours = hours
trader-time-unit-days = days
# Shown before the configured duration loads.
trader-time-conversion-default = 168 hours = 7 days
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } second
       *[other] { $amount } seconds
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } minute
       *[other] { $amount } minutes
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } hour
       *[other] { $amount } hours
    }
trader-duration-days =
    { $count ->
        [one] { $amount } day
       *[other] { $amount } days
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = Exit if down { $value }% or more after hold period
# $day is the day number of the example.
trader-time-day = Day { $day }
trader-time-position-opened = Position opened
trader-time-limit = Time Limit
trader-time-hold-reached = Hold period reached
trader-time-loss-met = Loss threshold met
trader-time-note = <strong>Note:</strong> Positions at profit or smaller losses will NOT be exited
trader-time-positions-title = Current Positions Status
trader-time-positions-loading = Loading positions...
trader-time-positions-empty = No open positions
trader-time-positions-token = Token
trader-time-positions-hold = Hold Time
trader-time-positions-roi = ROI

## Strategy control

trader-strategy-entry-title = Entry Strategies
trader-strategy-entry-subtitle = Signals that can open a new position.
trader-strategy-exit-title = Exit Strategies
trader-strategy-exit-subtitle = Signals that can close or protect an open position.
trader-strategy-active-unknown = -- active
trader-strategy-active = { $enabled }/{ $total } active
trader-strategy-loading = Loading strategies...
trader-strategy-load-failed = Could not load strategies
trader-strategy-empty = No strategies defined
trader-strategy-no-description = No description provided.
trader-strategy-unnamed = Unnamed strategy
trader-strategy-priority-auto = Auto
trader-strategy-priority = Priority { $priority }

## Dollar-cost averaging

trader-dca-title = Dollar-Cost Averaging
trader-dca-subtitle = Automatically add to losing positions to lower your average entry price
trader-dca-example-title = DCA Example
trader-dca-example = 0.01 { -sol } initial → DCA #1: 0.005 { -sol } @ -10% → DCA #2: 0.005 { -sol } @ -10% more
trader-dca-info-title = DCA Strategy Info
trader-dca-info-subtitle = Important considerations for DCA trading
trader-dca-how-title = How DCA Works
trader-dca-how-trigger = <strong>Trigger:</strong> Position drops below DCA threshold (e.g., -10%)
trader-dca-how-action = <strong>Action:</strong> Add more { -sol } to reduce average cost basis
trader-dca-how-repeat = <strong>Repeat:</strong> Can DCA multiple times based on max count
trader-dca-risk-title = Risk Warnings
trader-dca-risk-exposure = <strong>Increased Exposure:</strong> DCA increases total capital at risk per position
trader-dca-risk-knife = <strong>Falling Knife:</strong> DCA won't help if token continues downward trend
trader-dca-risk-cooldown = <strong>Cooldown:</strong> Use cooldown to avoid rapid-fire DCA entries

## General settings

trader-sizing-title = Position Sizing
trader-sizing-subtitle = Control how much to invest per position
trader-timing-title = Timing & Cooldowns
trader-timing-subtitle = Control timing between operations
trader-timing-close-cooldown = Position Close Cooldown
trader-timing-close-cooldown-hint = Minutes to wait before reopening the same token
trader-timing-concurrency = Entry Check Concurrency
trader-timing-concurrency-hint = Number of tokens to check simultaneously (higher = faster but more CPU)
trader-timing-unit-minutes = min
trader-timing-unit-tokens = tokens
