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
strategies-editor-enabled =
    .aria-label = Strategy enabled
    .title = Strategy enabled
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
strategies-summary-period-seconds = Period: { $amount } sec
strategies-summary-period-minutes = Period: { $amount } min
strategies-summary-period-hours = Period: { $amount } hrs

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
strategies-unit-hours = hrs
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = Search conditions...
strategies-catalog-search-clear =
    .aria-label = Clear search
strategies-catalog-fold-all = Fold All
strategies-catalog-unfold-all = Unfold All
strategies-catalog-no-description = No description available

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
strategies-toast-enabled = Strategy Enabled
    .message = "{ $name }" enabled
strategies-toast-disabled = Strategy Disabled
    .message = "{ $name }" disabled
strategies-toast-toggle-failed = Toggle Failed
    .message = Failed to update strategy status
strategies-toast-load-failed = Load Failed
    .message = Failed to load strategies from server
strategies-toast-load-strategy-failed = Failed to load strategy
strategies-toast-no-strategy = No Strategy Created
    .message = Add at least one condition or click 'New Strategy' to create a strategy first
strategies-toast-no-conditions-save = No Conditions
    .message = Add at least one condition to the strategy before saving
strategies-toast-name-required = Name Required
    .message = Enter a strategy name before saving
strategies-toast-saved = Strategy Saved
    .message = "{ $name }" saved successfully
strategies-toast-save-failed = Save Failed
    .message = Failed to save strategy to database
strategies-toast-no-strategy-validate = No strategy to validate
strategies-toast-no-conditions-validate = No Conditions
    .message = Add at least one condition before validating
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
strategies-toast-condition-added = Condition Added
    .message = { $name } added to strategy


## Conditions

strategies-condition-candle-size = Candle Size Pattern
    .description = Detect specific candle patterns: large body, small body (doji), long wicks
strategies-condition-candle-size-param-pattern = Pattern Type
    .description = Candle pattern to detect
strategies-condition-candle-size-param-pattern-option-large-body = Large Body (Strong Move)
strategies-condition-candle-size-param-pattern-option-small-body = Small Body (Doji/Indecision)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Long Upper Wick (Rejection)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Long Lower Wick (Support)
strategies-condition-candle-size-param-threshold = Size Threshold %
    .description = Percentage threshold for pattern detection

strategies-condition-consecutive-candles = Consecutive Candles
    .description = Detect consecutive green (bullish) or red (bearish) candles with minimum size filter
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
    .description = Detect price breaking above resistance (period high) or below support (period low)
strategies-condition-price-breakout-param-lookback = Lookback Period
    .description = Number of candles to find support/resistance level
strategies-condition-price-breakout-param-direction = Breakout Direction
    .description = Direction of the breakout
strategies-condition-price-breakout-param-direction-option-upward = Upward (Resistance Break)
strategies-condition-price-breakout-param-direction-option-downward = Downward (Support Break)
strategies-condition-price-breakout-param-confirmation = Confirmation %
    .description = How far past the level to confirm breakout (avoids false signals)

strategies-condition-price-change-percent = Price Change %
    .description = Check if price changed by a percentage threshold within a time period
strategies-condition-price-change-percent-param-percentage = Change Threshold %
    .description = Percentage price change to trigger (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Direction
    .description = Price movement direction
strategies-condition-price-change-percent-param-direction-option-above = Gain (+%)
strategies-condition-price-change-percent-param-direction-option-below = Loss (-%)
strategies-condition-price-change-percent-param-direction-option-within = Within Range (±%)
strategies-condition-price-change-percent-param-time-value = Time Period
    .description = Lookback period value (1-3600 for seconds, 1-1440 for minutes, 1-720 for hours)
strategies-condition-price-change-percent-param-time-unit = Time Unit
    .description = Time unit for lookback period
strategies-condition-price-change-percent-param-time-unit-option-seconds = Seconds
strategies-condition-price-change-percent-param-time-unit-option-minutes = Minutes
strategies-condition-price-change-percent-param-time-unit-option-hours = Hours

strategies-condition-price-to-ma = Price vs Moving Average
    .description = Check if price is above, below, or within range of its Simple Moving Average
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
    .description = Detect volume spikes compared to average volume (indicates increased interest)
strategies-condition-volume-spike-param-lookback = Lookback Period
    .description = Number of candles to calculate average volume
strategies-condition-volume-spike-param-multiplier = Volume Multiplier
    .description = How many times above average (e.g., 2.0 = 200% of average)

## Shared by every condition

strategies-condition-param-timeframe = Timeframe
    .description = Candle timeframe to analyze (defaults to strategy timeframe if not set)
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

# Indicators

strategies-error-indicator-consecutive-candles = consecutive candles
strategies-error-indicator-moving-average = moving average
strategies-error-indicator-price-breakout = price breakout
strategies-error-indicator-price-change-lookback = price change lookback
strategies-error-indicator-volume-spike = volume spike

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = Branch node missing conditions
strategies-error-rule-branch-node-missing-operator = Branch node missing operator
strategies-error-rule-branch-node-must-have-at-least-one-child = Branch node must have at least one child
strategies-error-rule-invalid-rule-tree-structure = Invalid rule tree structure
strategies-error-rule-leaf-node-missing-condition = Leaf node missing condition
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT operator must have exactly one child
