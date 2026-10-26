# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = All
strategies-filter-entry = Entry
strategies-filter-exit = Exit
strategies-type-entry = Entry
strategies-type-exit = Exit
strategies-list-empty-title = No strategies yet
strategies-list-empty-hint = Create your first strategy
strategies-new = New Strategy
strategies-import =
    .title = Import Strategy
    .aria-label = Import Strategy
strategies-item-enable =
    .title = Enable
strategies-item-disable =
    .title = Disable

# Name given to a strategy before it is saved.
strategies-new-name = New Strategy

## Editor

strategies-editor-name =
    .placeholder = Strategy name
strategies-editor-dirty =
    .title = Unsaved changes
strategies-action-validate = Validate
strategies-editor-empty = Select a strategy to edit, or create a new one
strategies-conditions-empty-title = No conditions yet
strategies-conditions-empty-hint = Use "{ strategies-add-condition }" to start building
strategies-add-condition = Add Condition
strategies-modal-close =
    .aria-label = Close
strategies-card-move-up =
    .title = Move up
strategies-card-move-down =
    .title = Move down
strategies-card-duplicate =
    .title = Duplicate
strategies-card-delete =
    .title = Delete
# $name is the condition name.
strategies-card-delete-confirm = Remove condition
    .message = Remove "{ $name }" from this strategy?

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = No parameters
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Strategy setting ({ $value })
strategies-summary-period-seconds = Lookback: { $amount } s
strategies-summary-period-minutes = Lookback: { $amount } min
strategies-summary-period-hours = Lookback: { $amount } h

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } hour
       *[other] { $amount } hours
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } candle
       *[other] { $amount } candles
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = h
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = Search conditions...
    .aria-label = Search conditions
strategies-catalog-search-clear =
    .aria-label = Clear search
strategies-catalog-no-description = No description available

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Active
strategies-editor-timeframe = Default Timeframe
    .title = Candle timeframe used by conditions that do not set their own
strategies-editor-priority = Priority
    .title = Evaluation order from 1 to 100; lower numbers are evaluated first
strategies-editor-description = Description
strategies-editor-description-field =
    .placeholder = What this strategy does
    .aria-label = Strategy description
strategies-action-export = Export

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Match { $mode } of these conditions
strategies-group-match = Match { $mode } of these
strategies-match-all = all
strategies-match-any = any
strategies-match-mode =
    .aria-label = Match all or any
strategies-invert = Invert
    .title = Invert: true when this does not match
strategies-chip-off = Off
strategies-chip-inverted = Inverted
strategies-row-switch =
    .title = Switch off to keep it saved but not evaluated
    .aria-label = Evaluated by this strategy
strategies-row-expand =
    .aria-label = Show or hide parameters
strategies-add-group = Add Group
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Remove group
    .message =
        { $count ->
            [one] Remove this group and its condition?
           *[other] Remove this group and its { $count } conditions?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } problem to fix: { $first }
       *[other] { $count } problems to fix: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Add at least one condition that is switched on
strategies-problem-group-empty = A group needs at least one condition that is switched on
strategies-problem-not-applicable = This condition cannot be used in this type of strategy
strategies-problem-unknown-condition = Unknown condition type: { $type }
strategies-problem-too-deep = Rules can nest { $max } levels at most; an inverted group counts as one more level
strategies-problem-required = Enter a number
strategies-problem-range = Must be between { $min } and { $max }
strategies-problem-invalid-option = Choose one of the listed options

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = All
strategies-catalog-categories =
    .aria-label = Categories
strategies-catalog-empty = No condition matches your search
strategies-catalog-exit-only = Exit strategies only
strategies-catalog-entry-only = Entry strategies only
strategies-catalog-candles-note = { $description } Reads candles only.
strategies-data-candles = Candles
strategies-data-pool = Pool
strategies-data-position = Position
strategies-data-market = Market activity

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = trades
strategies-value-trades =
    { $count ->
        [one] { $amount } trade
       *[other] { $amount } trades
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Too Many Levels
    .message = Rules can nest { $max } levels at most
strategies-toast-priority-invalid = Invalid Priority
    .message = Enter a whole number from 1 to 100
strategies-toast-exported = Strategy exported
# $name is the strategy name.
strategies-unsaved-confirm = Discard unsaved changes?
    .message = "{ $name }" has changes that are not saved.
strategies-unsaved-discard = Discard
strategies-import-error-json = The file is not valid JSON
strategies-import-error-shape = The file does not describe a strategy
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = The file uses a condition this version does not have: { $type }
strategies-import-error-not-applicable = { $name } cannot be used in this type of strategy

## New strategy dialog

strategies-create-title = Create New Strategy
strategies-create-prompt = Choose the type of strategy you want to create:
strategies-create-entry-name = Entry Strategy
strategies-create-entry-description = Define conditions for when to buy a token
strategies-create-exit-name = Exit Strategy
strategies-create-exit-description = Define conditions for when to sell a token

## Delete dialog

strategies-delete-title = Delete Strategy
# $name is the strategy name.
strategies-delete-message = Delete strategy "{ $name }"? This action cannot be undone.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Please fix validation errors before saving
strategies-toast-load-failed = Load Failed
    .message = Failed to load strategies from server
strategies-toast-load-strategy-failed = Failed to load strategy
strategies-toast-no-strategy = No Strategy Created
    .message = Add at least one condition or click 'New Strategy' to create a strategy first
strategies-toast-name-required = Name Required
    .message = Enter a strategy name before saving
strategies-toast-saved = Strategy Saved
    .message = "{ $name }" saved successfully
strategies-toast-save-failed = Save Failed
    .message = Failed to save strategy to database
strategies-toast-no-strategy-validate = No strategy to validate
strategies-toast-valid = Strategy is valid
strategies-toast-invalid = Strategy has errors
strategies-toast-validation-failed = Validation failed
strategies-toast-item-enabled = Strategy enabled
strategies-toast-item-disabled = Strategy disabled
strategies-toast-item-toggle-failed = Failed to toggle strategy
strategies-toast-deleted = Strategy Deleted
    .message = "{ $name }" removed successfully
strategies-toast-delete-failed = Delete Failed
    .message = Failed to delete strategy from database
strategies-toast-imported = Strategy imported
strategies-toast-import-failed = Failed to import strategy
strategies-toast-unknown-condition = Unknown Condition
    .message = Condition type not found
strategies-toast-create-first = Create Strategy First
    .message = Click 'New Strategy' to create a strategy before adding conditions

## Conditions

strategies-condition-candle-size = Candle Size Pattern
    .description = Detect specific candle patterns: large body, small body (doji), long wicks.
strategies-condition-candle-size-param-pattern = Pattern Type
    .description = Candle pattern to detect
strategies-condition-candle-size-param-pattern-option-large-body = Large Body (Strong Move)
strategies-condition-candle-size-param-pattern-option-small-body = Small Body (Doji/Indecision)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Long Upper Wick (Rejection)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Long Lower Wick (Support)
strategies-condition-candle-size-param-threshold = Size Threshold %
    .description = Percentage threshold for pattern detection

strategies-condition-consecutive-candles = Consecutive Candles
    .description = Detect consecutive green (bullish) or red (bearish) candles with minimum size filter.
strategies-condition-consecutive-candles-param-count = Candle Count
    .description = Number of consecutive candles required
strategies-condition-consecutive-candles-param-direction = Candle Direction
    .description = Color/direction of consecutive candles
strategies-condition-consecutive-candles-param-direction-option-green = Green (Bullish)
strategies-condition-consecutive-candles-param-direction-option-red = Red (Bearish)
strategies-condition-consecutive-candles-param-minimum-change = Minimum Change %
    .description = Minimum % change for each candle (filters noise)

strategies-condition-liquidity-level = Pool Liquidity Level
    .description = Check pool liquidity in { -sol } (Entry: ensure sufficient liquidity, Exit: detect liquidity drain)
strategies-condition-liquidity-level-param-threshold = Liquidity Threshold ({ -sol })
    .description = Pool liquidity level in { -sol }
strategies-condition-liquidity-level-param-comparison = Comparison
    .description = How to compare pool liquidity to threshold
strategies-condition-liquidity-level-param-comparison-option-greater-than = Greater Than (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Greater or Equal (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Less Than ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Less or Equal (≤)

strategies-condition-position-holding-time = Position Holding Time
    .description = Check how long a position has been held (for exit strategies - time-based exits)
strategies-condition-position-holding-time-param-hours = Time Threshold (Hours)
    .description = Duration in hours since position opened
strategies-condition-position-holding-time-param-comparison = Comparison
    .description = How to compare position age to threshold
strategies-condition-position-holding-time-param-comparison-option-greater-than = Older Than (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = At Least (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Younger Than ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = At Most (≤)

strategies-condition-price-breakout = Price Breakout
    .description = Detect price breaking above resistance (period high) or below support (period low).
strategies-condition-price-breakout-param-lookback = Lookback Period
    .description = Number of candles to find support/resistance level
strategies-condition-price-breakout-param-direction = Breakout Direction
    .description = Direction of the breakout
strategies-condition-price-breakout-param-direction-option-upward = Upward (Resistance Break)
strategies-condition-price-breakout-param-direction-option-downward = Downward (Support Break)
strategies-condition-price-breakout-param-confirmation = Confirmation %
    .description = How far past the level to confirm breakout (avoids false signals)

strategies-condition-price-change-percent = Price Change %
    .description = Check if price changed by a percentage threshold within a time period.
strategies-condition-price-change-percent-param-percentage = Change Threshold %
    .description = Percentage price change to trigger (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Direction
    .description = Price movement direction
strategies-condition-price-change-percent-param-direction-option-above = Gain (+%)
strategies-condition-price-change-percent-param-direction-option-below = Loss (-%)
strategies-condition-price-change-percent-param-direction-option-within = Within Range (±%)
strategies-condition-price-change-percent-param-time-value = Lookback
    .description = How far back the change is measured, in the unit picked inside the field (1-3600 s, 1-1440 min, 1-720 h)
strategies-condition-price-change-percent-param-time-unit = Time Unit
    .description = Time unit for lookback period
strategies-condition-price-change-percent-param-time-unit-option-seconds = s
strategies-condition-price-change-percent-param-time-unit-option-minutes = min
strategies-condition-price-change-percent-param-time-unit-option-hours = h

strategies-condition-price-to-ma = Price vs Moving Average
    .description = Check if price is above, below, or within range of its Simple Moving Average.
strategies-condition-price-to-ma-param-period = MA Period
    .description = Number of candles for moving average calculation
strategies-condition-price-to-ma-param-position = Position
    .description = Price position relative to MA
strategies-condition-price-to-ma-param-position-option-above = Above MA
strategies-condition-price-to-ma-param-position-option-below = Below MA
strategies-condition-price-to-ma-param-position-option-within = Within Range
strategies-condition-price-to-ma-param-distance = Distance %
    .description = Minimum distance from MA (for ABOVE/BELOW) or maximum range (for WITHIN)

strategies-condition-volume-spike = Volume Spike
    .description = Detect volume spikes compared to average volume (indicates increased interest).
strategies-condition-volume-spike-param-lookback = Lookback Period
    .description = Number of candles to calculate average volume
strategies-condition-volume-spike-param-multiplier = Volume Multiplier
    .description = How many times above average (e.g., 2.0 = 200% of average)

strategies-condition-rsi = RSI (Relative Strength Index)
    .description = Fires when the RSI, with Wilder smoothing on closed candles, is above or below a threshold.
strategies-condition-rsi-param-period = RSI Period
    .description = Number of candles the RSI is calculated over; the condition needs one more candle than this
strategies-condition-rsi-param-comparison = Comparison
    .description = Whether the RSI must be below or above the threshold
strategies-condition-rsi-param-comparison-option-below = Below (Oversold)
strategies-condition-rsi-param-comparison-option-above = Above (Overbought)
strategies-condition-rsi-param-threshold = RSI Threshold
    .description = RSI level from 1 to 99 (30 and 70 are the usual oversold and overbought levels)

strategies-condition-ma-cross = Moving Average Cross
    .description = Fires when a fast moving average crosses a slow one within the last few candles.
strategies-condition-ma-cross-param-ma-type = Average Type
    .description = Exponential averages weight recent candles more; simple averages weight all candles equally
strategies-condition-ma-cross-param-ma-type-option-ema = Exponential (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Simple (SMA)
strategies-condition-ma-cross-param-fast-period = Fast Period
    .description = Number of candles in the fast average
strategies-condition-ma-cross-param-slow-period = Slow Period
    .description = Number of candles in the slow average; must be larger than the fast period
strategies-condition-ma-cross-param-direction = Cross Direction
    .description = Which way the fast average crosses the slow one
strategies-condition-ma-cross-param-direction-option-bullish = Bullish (Fast Crosses Above)
strategies-condition-ma-cross-param-direction-option-bearish = Bearish (Fast Crosses Below)
strategies-condition-ma-cross-param-within-candles = Cross Within
    .description = How many of the latest candles may hold the cross; 1 means the latest candle only

strategies-condition-position-profit = Position Profit
    .description = Fires when the open position's unrealized profit is above or below a percentage of the entry price, measured on the pool price. Exit strategies only.
strategies-condition-position-profit-param-comparison = Comparison
    .description = Whether the profit must be at least or at most the threshold
strategies-condition-position-profit-param-comparison-option-above = At Least (≥)
strategies-condition-position-profit-param-comparison-option-below = At Most (≤)
strategies-condition-position-profit-param-threshold-pct = Profit Threshold %
    .description = Unrealized profit as a percentage of the entry price; negative values are losses

strategies-condition-drawdown-from-peak = Drawdown From Peak
    .description = Fires when the pool price has fallen by a percentage from the highest price seen since entry, once that peak stood far enough above the entry price. Reads pool prices only. Exit strategies only.
strategies-condition-drawdown-from-peak-param-drop-pct = Drop From Peak %
    .description = How far below the peak the current price must be
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Arm After Profit %
    .description = The peak must have been at least this far above the entry price before the condition can fire; 0 means always armed

strategies-condition-buy-pressure = Buy Pressure
    .description = Fires when buys make up at least a share of the transactions in a time window and the window has enough transactions. Reads the token's buy and sell counts.
strategies-condition-buy-pressure-param-window = Time Window
    .description = Period the buy and sell counts cover
strategies-condition-buy-pressure-param-window-option-5m = 5 Minutes
strategies-condition-buy-pressure-param-window-option-1h = 1 Hour
strategies-condition-buy-pressure-param-window-option-6h = 6 Hours
strategies-condition-buy-pressure-param-window-option-24h = 24 Hours
strategies-condition-buy-pressure-param-min-buy-share-pct = Minimum Buy Share %
    .description = Buys as a percentage of all transactions in the window
strategies-condition-buy-pressure-param-min-transactions = Minimum Transactions
    .description = Windows with fewer transactions never fire the condition

## Shared by every condition

strategies-condition-param-timeframe = Timeframe
    .description = Candle size: the length of each candle the condition reads (the strategy timeframe when not set)
strategies-condition-timeframe-option-1m = 1 Minute
strategies-condition-timeframe-option-5m = 5 Minutes
strategies-condition-timeframe-option-15m = 15 Minutes
strategies-condition-timeframe-option-1h = 1 Hour
strategies-condition-timeframe-option-4h = 4 Hours
strategies-condition-timeframe-option-12h = 12 Hours
strategies-condition-timeframe-option-1d = 1 Day

## Condition categories

strategies-condition-category-price-analysis = Price Analysis
strategies-condition-category-candle-patterns = Candle Patterns
strategies-condition-category-technical-indicators = Technical Indicators
strategies-condition-category-market-context = Market Context
strategies-condition-category-position-performance = Position & Performance
strategies-condition-category-volume-analysis = Volume Analysis

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = The { $field } parameter is missing
strategies-error-parameter-type = The { $field } parameter must be { $expected }
strategies-error-invalid-value = "{ $value }" is not a valid { $field }
strategies-error-missing-data = { $data } is not available
strategies-error-no-candle-data = The { $timeframe } timeframe has no candle data
strategies-error-insufficient-history = Not enough history for { $indicator }: { $available } s available, { $required } s needed
strategies-error-insufficient-candles = Not enough candles for { $indicator }: have { $available }, need { $required }
strategies-error-stale-candle-data = The { $timeframe } candle data is stale: its age of { $age } s exceeds { $max } s
strategies-error-invalid-rule-tree = Invalid rule tree: { $reason }
strategies-error-evaluation-timeout = Strategy evaluation timed out after { $timeout } ms
strategies-error-condition-not-applicable = { $condition } cannot be used in { $strategy } strategies
strategies-error-invalid-rules = The rules could not be read: { $reason }

# Parameter names

strategies-error-field-average-volume = average volume
strategies-error-field-candle-open = candle open
strategies-error-field-comparison = comparison
strategies-error-field-condition-type = condition type
strategies-error-field-confirmation = confirmation
strategies-error-field-count = count
strategies-error-field-current-price = current price
strategies-error-field-direction = direction
strategies-error-field-distance = distance
strategies-error-field-hours = hours
strategies-error-field-lookback = lookback
strategies-error-field-minimum-change = minimum change
strategies-error-field-multiplier = multiplier
strategies-error-field-pattern = pattern
strategies-error-field-percentage = percentage
strategies-error-field-period = period
strategies-error-field-position = position
strategies-error-field-threshold = threshold
strategies-error-field-time-unit = time unit
strategies-error-field-time-value = time value
strategies-error-field-timeframe = timeframe
strategies-error-field-arm-after-profit = arm after profit
strategies-error-field-drop-percentage = drop percentage
strategies-error-field-entry-price = entry price
strategies-error-field-fast-period = fast period
strategies-error-field-minimum-buy-share = minimum buy share
strategies-error-field-minimum-transactions = minimum transactions
strategies-error-field-moving-average-type = moving average type
strategies-error-field-peak-price = peak price
strategies-error-field-slow-period = slow period
strategies-error-field-window = window
strategies-error-field-within-candles = within candles

# Expected parameter types

strategies-error-expected-boolean = a boolean
strategies-error-expected-number = a number
strategies-error-expected-string = a string

# Missing context data

strategies-error-data-current-price = Current price
strategies-error-data-liquidity-data = Liquidity data
strategies-error-data-market-data = Market data
strategies-error-data-ohlcv-data = OHLCV data
strategies-error-data-position-data = Position data
strategies-error-data-peak-price = Peak price
strategies-error-data-transaction-counts = Transaction counts

# Indicators

strategies-error-indicator-consecutive-candles = consecutive candles
strategies-error-indicator-moving-average = moving average
strategies-error-indicator-price-breakout = price breakout
strategies-error-indicator-price-change-lookback = price change lookback
strategies-error-indicator-volume-spike = volume spike
strategies-error-indicator-moving-average-cross = moving average cross
strategies-error-indicator-rsi = RSI

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = Branch node missing conditions
strategies-error-rule-branch-node-missing-operator = Branch node missing operator
strategies-error-rule-branch-node-must-have-at-least-one-child = Branch node must have at least one child
strategies-error-rule-invalid-rule-tree-structure = Invalid rule tree structure
strategies-error-rule-leaf-node-missing-condition = Leaf node missing condition
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT operator must have exactly one child
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = A group needs at least one condition switched on
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = NOT operator cannot have a switched-off child
strategies-error-rule-rules-nest-deeper-than-four-levels = Rules nest deeper than four levels
strategies-error-rule-the-root-group-cannot-be-switched-off = The root group cannot be switched off
