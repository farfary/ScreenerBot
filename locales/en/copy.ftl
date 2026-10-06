# Copy trading messages.

# Why a copy was skipped. Ids come from CopySkip in src/trader/copy/types.rs.
copy-skip-not-buy-swap = Wallet activity was not a buy
copy-skip-task-disabled = Task is paused
copy-skip-mode-transition-required = Execution mode must be changed separately
copy-skip-live-confirmation-required = Live execution needs confirmation
copy-skip-unsupported-sizing-mode = Sizing mode is not supported yet
copy-skip-self-copy = The wallet is one of your own
copy-skip-target-below-minimum = Wallet trade below the minimum
copy-skip-target-above-maximum = Wallet trade above the maximum
copy-skip-already-bought = Already bought this token (buy once)
copy-skip-blacklisted = Token is blocked by risk controls
copy-skip-filter-required = Token did not pass Filtering
copy-skip-budget-exhausted = Task budget is spent
copy-skip-token-cap-reached = Per-token limit reached
copy-skip-below-minimum-size = Copy size too small
copy-skip-invalid-sizing = Task sizing is invalid
copy-skip-invalid-slippage = Task slippage is invalid
copy-skip-invalid-exit-policy = Task exit rules are invalid
copy-skip-invalid-price = No usable market price
copy-skip-not-sell-swap = Wallet activity was not a sell
copy-skip-exit-mode-disabled = Wallet sell ignored: the task sells by its own rules
copy-skip-force-stopped = Trading is force-stopped
copy-skip-copy-position-not-found = No position owned by this task
copy-skip-position-user-only = Position is managed by you
copy-skip-position-management-mismatch = Position no longer follows copy sells
copy-skip-latency-kill-switch = Auto-paused: trades detected too late
copy-skip-claim-reconciled-abandoned = Interrupted live submission closed without retry
copy-skip-stale-observation = Replayed after downtime, too old to copy
copy-skip-unknown-observation-time = Replayed trade has no block time
copy-skip-entry-blocked = Entry blocked

# Why a new entry was blocked. Ids come from EntryBlock in src/trader/admission.rs.
copy-entry-block-force-stopped = Trading is force-stopped
copy-entry-block-loss-limit = Loss limit blocks new entries
copy-entry-block-connectivity = Required services are unavailable
copy-entry-block-position-limit = Open-position limit reached
copy-entry-block-already-open = A position is already open
copy-entry-block-reentry-cooldown = Token re-entry cooldown
copy-entry-block-open-cooldown = Global entry cooldown
copy-entry-block-entry-reserved = Another entry is processing
copy-entry-block-blacklisted = Token is blocked by risk controls
copy-entry-block-check-failed = A safety check could not complete

# Why a task stopped copying. Ids come from CopyPauseReason in
# src/trader/copy/types.rs. $average, $threshold and $p95 are seconds with one
# decimal below ten seconds and whole seconds above; $limit is a signature count.
copy-pause-user = Paused by you
copy-pause-latency-kill-switch = Auto-paused: trades arrived { $average }s late on average (limit { $threshold }s)
copy-pause-watch-detached = Auto-paused: the wallet is no longer watched
copy-pause-watch-budget-exceeded = Paused: this wallet reached its { $limit }-signature watch check limit before catching up
copy-pause-helius-unavailable = Paused: { -helius } wallet checks failed
copy-pause-watch-processing-failed = Paused: wallet activity could not be processed
# A paused task with no recorded reason.
copy-pause-unspecified = Paused

# Short pause cause shown after the paused state in the task list.
copy-pause-short-user = by you
copy-pause-short-latency-kill-switch = too slow
copy-pause-short-watch-detached = watch lost
copy-pause-short-watch-budget-exceeded = watch limit
copy-pause-short-helius-unavailable = watch provider
copy-pause-short-watch-processing-failed = watch processing
copy-state-paused = Paused
copy-state-paused-reason = Paused · { $reason }

# Live-readiness checklist: a title and a detail per check. Ids come from
# ReadinessCheck in src/trader/copy/workspace/readiness.rs. $realized is a signed SOL figure.
copy-readiness-history = Paper history
copy-readiness-history-met =
    { $count ->
        [one] { $count } closed paper round, { $needed } needed
       *[other] { $count } closed paper rounds, { $needed } needed
    }
copy-readiness-history-short = { $count } of { $needed } closed paper rounds
copy-readiness-profit = Profitable in paper
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realized over { $count } round, { $wins } won
       *[other] { $realized } { -sol } realized over { $count } rounds, { $wins } won
    }
copy-readiness-latency = Trades detected in time
copy-readiness-latency-detail = p95 arrival { $p95 }s, limit { $limit }s
copy-readiness-latency-none = No arrival samples yet
copy-readiness-priced = Every holding priced
copy-readiness-priced-ok = Every open paper holding has a pool price
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } open holding without a pool price
       *[other] { $count } open holdings without a pool price
    }
copy-readiness-runtime = Live execution available
copy-readiness-runtime-ok = Setup and safety gates allow live copies

# Why live execution is unavailable. Reasons come from control::live_block_reason.
copy-live-block-setup-incomplete = Finish wallet and RPC setup first
copy-live-block-force-stop = The emergency stop is engaged
copy-live-block-copy-trading-disabled = Copy processing is paused globally
copy-live-block-unavailable = Live execution is unavailable

## Task state, mode and exit labels. Ids come from effective_state
## (src/trader/copy/control.rs), CopyMode, ExitMode and PaperExitRule
## (src/trader/copy/types.rs) plus the target_sell exit bucket.

copy-state-system-paused = Paused globally
copy-state-force-stopped = Force stopped
copy-state-entries-blocked = Entries blocked
copy-state-running-live = Running
copy-state-running-paper = Running
copy-mode-paper = Paper
copy-mode-live = Live
copy-exit-mode-buy-only = My exit rules
copy-exit-mode-mirror = Mirror wallet sells
copy-exit-mode-hybrid = Wallet sells and my rules
copy-exit-target-sell = Wallet sold
copy-exit-stop-loss = Stop loss
copy-exit-trailing-stop = Trailing stop
copy-exit-take-profit = Take profit
copy-exit-time-override = Time rule
copy-exit-manual = Closed by hand

## Shared wording

copy-request-failed = Request failed
copy-keep-paused = Keep paused
copy-paused-suffix = · paused
# $mode is the execution mode label.
copy-mode-paused = { $mode } · paused
# $name is the task name and $mode its execution mode label.
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = Realized P&L
copy-metric-unrealized-pnl = Unrealized P&L
copy-metric-win-rate = Win rate
copy-metric-budget-spent = Budget spent
copy-metric-median-arrival = Median arrival
copy-metric-open-holdings = Open holdings
copy-record-won-lost = { $won } won · { $lost } lost
# $spent and $budget are decimal SOL amounts.
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Fills
copy-kind-exits = Exits
copy-kind-skips = Skips
copy-kind-errors = Errors
copy-field-per-trade-cap = Per-trade cap
copy-field-per-token-cap = Per-token cap
copy-field-total-budget = Total budget
copy-field-slippage = Slippage
copy-rules-wallet-sells-only = Wallet sells only
copy-filter-copy-setting-required = Copy setting (required)
copy-filter-copy-setting-not-required = Copy setting (not required)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } closed round
       *[other] { $count } closed rounds
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } open holding
       *[other] { $count } open holdings
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } priced holding · { $unpriced } without a price
       *[other] { $priced } priced holdings · { $unpriced } without a price
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } holding without a price
       *[other] { $count } holdings without a price
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = All
copy-range-label =
    .aria-label = Date range

## Page strip (pages/copy.html, pages/copy/summary.js)

copy-page-title = Copy Trading
copy-page-beta = Beta
copy-strip-loading = Loading
copy-strip-unavailable = Unavailable
copy-strip-pause-all = Pause all
copy-strip-resume = Resume processing
copy-strip-settings = Settings
copy-strip-add-wallet = Add wallet
copy-strip-paused-globally = Paused globally · no new copies, exits still run
copy-strip-force-stopped = Force stopped · nothing is copied
copy-strip-loss-limit = Loss limit · new entries blocked, exits still run
copy-strip-idle-paused =
    { $count ->
        [one] Idle · { $count } task paused
       *[other] Idle · { $count } tasks paused
    }
copy-strip-idle-empty = Idle · no tasks yet
copy-strip-processing = Processing · { $paper } paper
copy-strip-processing-live = Processing · { $live } live · { $paper } paper
copy-figures-label =
    .aria-label = Copy trading totals
copy-figure-marked-at-pool = Marked at the pool price
copy-figure-across-tasks = Across all tasks
copy-figure-budget-lifetime = Lifetime spend of enabled tasks
copy-figure-budget-none = No enabled tasks
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } trade
       *[other] { $count } trades
    }
copy-figure-arrival-none = No samples from enabled tasks

## Page frame (pages/copy.js)

# $error is the failure detail.
copy-load-failed = Copy trading could not be loaded: { $error }
copy-resume-all-title = Resume copy processing
copy-resume-all-message =
    { $count ->
        [one] { $count } live task will submit real swaps when its wallet trades again.
       *[other] { $count } live tasks will submit real swaps when their wallets trade again.
    }
copy-toast-resumed-all = Copy processing resumed
copy-toast-paused-all = All copy processing paused
copy-toast-global-failed = Copy processing could not be changed

## Onboarding (pages/copy.html)

copy-onboarding-title = Copy the wallets you trust, prove them in Paper first
copy-onboarding-body = Every task starts in Paper: target trades are simulated at the pool price with your slippage and fees, and your exit rules run on the paper book. Arm Live per wallet once its paper results earn it.
copy-onboarding-add = Add your first wallet
copy-onboarding-observe = Observe
copy-onboarding-observe-detail = Detect the wallet's swaps without spending { -sol }.
copy-onboarding-evaluate = Evaluate
copy-onboarding-evaluate-detail = Read paper P&L, win rate, skips, detection speed and slippage.
copy-onboarding-arm = Arm
copy-onboarding-arm-detail = Pass the readiness checks, then enable real swaps.

## Wallet list (pages/copy.html, pages/copy/list.js)

copy-list-label =
    .aria-label = Copied wallets
copy-list-title = Wallets
copy-list-compare = Compare
copy-list-sort-label = Sort wallets
copy-list-count = { $active } active · { $total } total
copy-sort-pnl = P&L
copy-sort-state = State
copy-sort-name = Name
copy-compare-label =
    .aria-label = Compare wallets

## Dialog chrome (pages/copy.html)

copy-dialog-close =
    .aria-label = Close
copy-editor-title-add = Add wallet
copy-editor-sub-add = New tasks start in Paper
copy-arm-title = Arm live copying
copy-arm-sub = Real swaps from your wallet
copy-arm-keep-paper = Keep Paper
copy-arm-confirm = Arm live
copy-profile-title = Wallet profile
copy-profile-sub = What this bot has seen of the wallet

## Settings dialog (pages/copy.html, pages/copy/settings.js). Field labels and hints
## come from the config catalog.

copy-settings-title = Copy trading settings
copy-settings-subtitle = Global policy for every task
copy-settings-filter-warning = With the default Filtering setup this rejects almost every token, so nothing is copied. Leave it off unless your filters pass the tokens your wallets trade.
copy-settings-unit-seconds = seconds
copy-settings-unit-trades = trades
copy-settings-unit-tasks = tasks
copy-settings-unit-closed-rounds = closed rounds
copy-settings-save = Save settings
copy-settings-load-failed = Copy settings could not be loaded
copy-settings-saved = Copy trading settings saved

## Workspace (pages/copy/workspace.js)

copy-tab-overview = Overview
copy-tab-holdings = Holdings
copy-tab-activity = Activity
copy-tab-rules = Rules
copy-tab-execution = Execution
copy-tabs-label = Task views
copy-workspace-select = Select a wallet to open its workspace.
copy-workspace-loading = Loading task…
# $error is the failure detail.
copy-workspace-load-failed = This task could not be loaded: { $error }

# What a running or blocked task is doing. Ids are the effective_state values
# without the paused state.
copy-state-detail-paper = Running in Paper · trades are simulated, nothing is spent
copy-state-detail-live = Running live · wallet trades are copied with real swaps
copy-state-detail-system-paused = Waiting · copy processing is paused globally, exits still run
copy-state-detail-entries-blocked = Entries blocked by the loss limit · exits still run
copy-state-detail-force-stopped = Force stopped · nothing is copied

# $reason is the pause wording and $since the time since the pause.
copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Resuming keeps the same limit, so it pauses again while trades still arrive late. Check the RPC stream or raise the arrival limit in Settings.
copy-paused-resume-detached = Resuming watches the wallet again.
copy-paused-holdings-rules =
    { $count ->
        [one] Its exit rules still close its { $count } open holding.
       *[other] Its exit rules still close its { $count } open holdings.
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] The wallet's sells still close its { $count } open holding.
       *[other] The wallet's sells still close its { $count } open holdings.
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] The wallet's sells and its exit rules still close its { $count } open holding.
       *[other] The wallet's sells and its exit rules still close its { $count } open holdings.
    }

copy-watch-state-catching-up = Wallet watch: Catching up. Checking through { -helius } for this wallet.
copy-watch-state-watching = Wallet watch: Watching. Checking through { -helius } for this wallet.
# $ago is the time since the last watch check.
copy-watch-last-check = Last check { $ago }.
copy-watch-recovery-active = Wallet watch active
copy-watch-recovery-catching-up = Wallet watch is catching up
copy-watch-recovery-still-paused = The copy task is still paused. Resume copying when you are ready.
copy-watch-recovery-title = Restore wallet watch
copy-watch-recovery-processing-failed = Wallet activity could not be processed. Saved progress is preserved. Retry after the problem is resolved.
copy-watch-recovery-provider-failed = { -helius } checks failed. Saved progress is preserved. Retry when the provider is available.
copy-watch-recovery-budget-intro = This wallet has more activity than its current watch can check. Choose how to continue.
copy-watch-approve = Try to catch up using { -helius }
copy-watch-approve-help = Continues from saved progress. May use more { -helius } credits and can still fall behind.
copy-watch-approve-unavailable = { -helius } catch-up is unavailable. Configure an enabled { -helius } RPC endpoint to continue without skipping unchecked activity.
copy-watch-no-provider = No catch-up provider is supported for this watch.
copy-watch-budget-label = Signatures checked per check
# $min and $max are the signature limits per check.
copy-watch-budget-hint = Or skip unchecked activity and resume from now. Choose { $min }–{ $max } signatures per check; a higher limit may use more RPC calls.
copy-watch-ack = I understand missed activity will not be copied.
# $min and $max are the signature limits and $step the step between them.
copy-watch-toast-range = Choose between { $min } and { $max } signatures per poll in { $step }-signature steps
copy-watch-toast-ack = Acknowledge that signatures since the last completed check will be skipped
copy-watch-resumed = Wallet watch resumed from now; copy task remains paused
copy-watch-resume-failed = Wallet watch could not be resumed
copy-watch-retry-started = Wallet watch retry started from saved progress; copy task remains paused
copy-watch-retry-failed = Wallet watch could not be retried
copy-watch-approve-title = Allow { -helius } catch-up for this wallet
copy-watch-approve-message = { -helius } can check successful Solana transactions from saved progress without skipping the unchecked interval. It currently charges 10 credits per 100 full transactions returned, rounded up, with a 10 credit minimum per request. A check can make multiple requests; usage and provider pricing may vary. Copying remains paused until you resume it separately.
copy-watch-approve-confirm = Allow for this wallet
copy-watch-approved = Wallet watch started from saved progress; copy task remains paused
copy-watch-restore-failed = Wallet watch could not be restored

copy-action-pause = Pause
copy-action-resume = Resume
copy-action-resume-copy = Resume copy
copy-action-resume-from-now = Resume from now
copy-action-retry-watch = Retry wallet watch
copy-action-return-paper = Return to Paper
copy-action-edit-rules = Edit rules
copy-action-clone = Clone
copy-action-profile = Wallet profile
copy-resume-live-title = Resume live copying
# $name is the task name.
copy-resume-live-message = “{ $name }” will submit real swaps from your wallet when this wallet trades again.
copy-resume-live-confirm = Resume live
copy-task-resumed = Task resumed
copy-task-paused = Task paused
copy-task-state-failed = Task state could not be changed
# $name is the task name.
copy-return-paper-message = New copies by “{ $name }” will be simulated again, without spending { -sol }.
copy-return-paper-cancel = Keep live
copy-task-returned-paper = Task returned to Paper
copy-mode-change-failed = Execution mode could not be changed
copy-delete-title = Delete copy task
# $name is the task name.
copy-delete-message = Delete “{ $name }”? Its decisions and paper results are removed and the wallet is no longer watched for this task.
copy-delete-confirm = Delete task
copy-delete-cancel = Keep task
copy-task-deleted = Copy task deleted
copy-task-delete-failed = Copy task could not be deleted

## Overview tab (pages/copy/overview.js)

copy-overview-results = Results
# $error is the failure detail.
copy-analytics-load-failed = Analytics could not be loaded: { $error }
copy-analytics-loading = Loading analytics…
copy-exit-bucket =
    { $count ->
        [one] { $count } sell · { $pnl }
       *[other] { $count } sells · { $pnl }
    }
copy-overview-average-win = Average win
# $amount is a signed SOL amount.
copy-overview-average-loss = Average loss { $amount }
copy-overview-profit-factor = Profit factor
copy-overview-profit-factor-note = Gross wins ÷ gross losses
copy-overview-average-hold = Average hold
copy-overview-average-hold-note = Entry to exit
copy-overview-best-round = Best round
# $amount is a signed SOL amount.
copy-overview-worst-round = Worst { $amount }
copy-overview-curve-title = Cumulative P&L
copy-overview-exits-title = Sells by exit
copy-overview-skips-title = Why trades were skipped
copy-book-title-live = Live book
copy-book-title-paper = Paper book
copy-book-all-time = All time
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] buy
       *[other] buys
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] exit by your rules
       *[other] exits by your rules
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] wallet sell
       *[other] wallet sells
    }
copy-book-manual-closes = <strong>{ $count }</strong> closed by hand
copy-book-skipped = <strong>{ $count }</strong> skipped
copy-book-failed = <strong>{ $count }</strong> failed
copy-book-closed = { $count } closed
# $mode is the execution mode label; $total and $remaining are decimal SOL amounts.
copy-book-budget-note = { $mode } spend of { $total } · { $remaining } left
copy-check-passed = passed
copy-check-not-passed = not passed
copy-readiness-title = Before going live
copy-readiness-live-note = This task trades live. Return it to Paper from the header above.
copy-readiness-all-pass = Every check passes.
copy-readiness-needs-review = Arming needs an explicit review of what is not ready.
copy-readiness-arm = Review and arm live

## Rules tab and review (pages/copy/rules.js)

copy-rules-title = Rules in effect
# $pct is a formatted percentage.
copy-rules-size-ratio = { $pct } of the wallet's trade
# $amount is a formatted SOL amount.
copy-rules-size-fixed = { $amount } per copy
copy-rules-target-any = Any size
copy-rules-target-min = At least { $amount }
copy-rules-target-max = At most { $amount }
# $min and $max are decimal SOL amounts.
copy-rules-target-between = { $min } – { $max } { -sol }
# $value is the Trader default in the rule's own wording.
copy-rules-source-override = Task override · Trader { $value }
copy-rules-source-default = Trader default
copy-rules-not-used = Not used: the wallet's sells decide
copy-rules-col-rule = Rule
copy-rules-col-applies = Applies
copy-rules-col-source = Source
# $spent and $remaining are decimal SOL amounts; $mode is the execution mode label.
copy-rules-budget-note = { $spent } spent in { $mode } · { $remaining } left
copy-rules-token-copies =
    { $count ->
        [one] About { $count } full copy of one token
       *[other] About { $count } full copies of one token
    }
copy-rules-sizing = Sizing
copy-rules-copy-size = Copy size
copy-rules-entry-filters = Entry filters
copy-rules-target-size = Wallet trade size
copy-rules-repeat-buys = Repeat buys
copy-rules-repeat-first-only = First buy of each token only
copy-rules-repeat-every = Every buy, up to the per-token cap
copy-rules-filter-pass = Filtering pass
copy-rules-filter-required = Required
copy-rules-filter-not-required = Not required
copy-rules-filter-task-override = Task override
copy-rules-exits = Exits
copy-rules-exits-inactive = Holdings are sold only when the wallet sells; the rules below do not run in this mode.

## Exit rules (pages/copy/policy.js)

copy-rule-status = Status
copy-rule-on = On
copy-rule-off = Off
copy-rule-unit-seconds = seconds
copy-rule-unit-minutes = minutes
copy-rule-stop-loss-threshold = Sells at a loss of
copy-rule-stop-loss-min-hold = Not before holding
copy-rule-no-minimum = No minimum
copy-rule-partial-exits = Partial exits
copy-rule-partial-allowed = Allowed
copy-rule-partial-full-only = Full exit only
copy-rule-partial-size = Partial exit size
copy-rule-trailing-activation = Arms at a gain of
copy-rule-trailing-distance = Sells below the peak by
copy-rule-take-profit-target = Sells at a gain of
copy-rule-time-duration = Checks after holding
copy-rule-time-threshold = Sells while P&L is at or below
copy-preset-inherit = Trader defaults
copy-preset-conservative = Conservative
copy-preset-balanced = Balanced
copy-preset-aggressive = Aggressive
copy-preset-custom = Custom
copy-validate-stop-loss = Stop loss must be above 0% and at most 100%.
copy-validate-partial-size = Partial exit size must be between 0% and 100%.
copy-validate-min-hold = Minimum hold must be a whole number of seconds.
copy-validate-trailing-activation = Trailing activation must be above 0% and at most 100%.
copy-validate-trailing-distance = Trailing distance must be above 0% and at most 100%.
copy-validate-take-profit = Take profit must be above 0%.
copy-validate-time-duration = The time rule needs a duration above zero.
copy-validate-time-threshold = The time rule threshold is a loss: use 0% or a negative number.
copy-warning-mirror = Only the wallet's sells close holdings: no stop loss protects them, and a token the wallet never sells stays held.
copy-warning-no-rules = No exit rule is on and wallet sells are ignored: holdings are never sold.
copy-warning-no-stop-loss = No stop loss applies: a falling token is held until another rule or the wallet sells.
# $hold is a duration and $threshold a signed percentage.
copy-warning-stop-delay = The stop loss waits { $hold } after each buy: a token that falls faster closes well past { $threshold }.
# $target, $slippage and $fee are formatted percentages.
copy-warning-take-profit-cost = Take profit at { $target } does not cover selling ({ $slippage } slippage and a { $fee } swap fee), so it closes rounds at a loss.
copy-warning-trailing-distance = The trailing distance is at least its activation gain, so an armed trail can sell below entry.

## Execution tab (pages/copy/execution.js)

copy-execution-title = Execution quality
# $limit is a formatted duration.
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Any
# $limit is a formatted duration and $count the sample window.
copy-execution-limit-on =
    { $count ->
        [one] Pauses above { $limit } on average over { $count } trade
       *[other] Pauses above { $limit } on average over { $count } trades
    }
copy-execution-limit-off = Kill switch off
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } trade seen as they happened
       *[other] { $count } trades seen as they happened
    }
copy-execution-p95 = p95 arrival
copy-execution-median-slippage = Median slippage
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } measured fill
       *[other] { $count } measured fills
    }
copy-execution-worst-slippage = Worst slippage
# $amount is a signed percentage.
copy-execution-average-slippage = Average { $amount }
copy-execution-delay-title = Detection delay
copy-execution-delay-note = Time from the wallet's block to this bot seeing the trade. Replays after downtime are excluded.
# $limit is a formatted duration.
copy-execution-delay-limit = Bars past the { $limit } arrival limit are amber.
copy-execution-fastest = Fastest
copy-execution-average = Average
copy-execution-slowest = Slowest
copy-execution-fill-title = Fill against the wallet
copy-execution-fill-note = Positive means worse than the wallet: paid more on a buy, received less on a mirrored sell. A paper fill of a token without a pool price is priced at the wallet's own trade, so it measures nothing and is left out.
copy-execution-samples = Samples
copy-execution-median = Median
copy-execution-worst = Worst
copy-execution-decisions = Decisions in range

## Compare view (pages/copy/compare.js)

copy-compare-title = Compare wallets
copy-compare-back = Back to wallet
# $error is the failure detail.
copy-compare-load-failed = Comparison could not be loaded: { $error }
copy-compare-loading = Loading comparison…
copy-compare-empty = No tasks to compare.
copy-compare-curve-title = Cumulative realized P&L
copy-table-wallet = Wallet
copy-table-mode = Mode
copy-table-rounds = Rounds
copy-table-realized = Realized
copy-table-profit-factor = Profit factor
copy-table-average-hold = Avg hold
copy-table-median-slippage = Median slippage

## Charts (pages/copy/charts.js)

# $amount is a signed SOL amount.
copy-chart-curve-label = Cumulative P&L { $amount } { -sol }
copy-chart-compare-label = Cumulative P&L by task
copy-chart-empty-curve = No closed rounds in this range yet.
copy-chart-empty-bars = Nothing recorded in this range.
copy-chart-empty-histogram = No arrival samples in this range.
copy-chart-empty-compare = No closed rounds to compare in this range.
copy-chart-histogram-title = { $count } of { $total }

## Wallet profile (pages/copy/profile.js)

copy-profile-copy = Copy this wallet
copy-profile-copy-other = Copy with other rules
copy-profile-loading = Loading wallet profile…
copy-profile-watch-title = Watch
copy-profile-watched = Watched
copy-profile-watch-resume-hint = Resuming a task watches it again
copy-profile-watch-add-hint = Adding a task starts watching it
copy-profile-stream = Stream
copy-profile-subscribed = Subscribed
copy-profile-not-subscribed = Not subscribed
copy-profile-sources =
    { $count ->
        [one] { $count } source
       *[other] { $count } sources
    }
copy-profile-last-activity = Last activity
copy-profile-last-error = Last error
copy-profile-own-wallet = This is one of your own wallets; copying it is refused.
copy-profile-observed-title = Trades observed
copy-profile-observed-none = No trades from this wallet in this bot yet. A Paper task observes it without spending { -sol }.
copy-profile-swaps-seen = Swaps seen
copy-profile-swaps-seen-note = Distinct wallet swaps across your tasks
copy-profile-buys-sells = Buys / sells
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Tokens traded
copy-profile-first-seen = First seen
copy-profile-last-seen = Last seen
copy-profile-tasks-title = Your tasks on this wallet
copy-table-task = Task

## Arm live dialog (pages/copy/arm_gate.js)

copy-arm-acks-left =
    { $count ->
        [one] { $count } acknowledgement left to tick
       *[other] { $count } acknowledgements left to tick
    }
copy-arm-readiness-title = Readiness from the paper book
copy-arm-exposure-title = Exposure
copy-arm-per-copy = Per copy
# $left and $total are decimal SOL amounts.
copy-arm-budget-left-value = { $left } of { $total } { -sol }
copy-arm-budget-left = Live budget left
copy-arm-budget-left-note = Paper spend is counted separately and does not use it
copy-arm-exits = Exits
# $hold is a duration.
copy-arm-stop-note = Not before a { $hold } hold: a faster fall closes lower
# $tasks lists the other tasks copying the wallet.
copy-arm-shared = This wallet is also copied by { $tasks }: each task copies its trades on its own budget.
copy-arm-unavailable = Live execution is unavailable right now; see the last check.
# $budget and $trade are decimal SOL amounts.
copy-arm-ack-real-native = Real { -sol }: this task can spend up to { $budget } { -sol } from your wallet, at most { $trade } { -sol } per copy.
copy-arm-ack-fees = Live copies pay real network fees and slippage; paper results do not promise live results.
copy-arm-ack-unready = Some readiness checks have not passed. Arm this task anyway.
# $name is the task name.
copy-arm-lead = “{ $name }” will copy this wallet's trades with real swaps from your wallet.
copy-arm-confirmation-missing = The live confirmation could not be loaded
copy-arm-armed = Live copying armed
copy-arm-failed = Live copying could not be armed

## Holdings tab (pages/copy/holdings.js)

copy-holdings-title = Holdings
copy-holdings-view-label = Holdings view
# $count is the number of open holdings.
copy-holdings-view-open = Open ({ $count })
# $count is the number of closed rounds, or an ellipsis while loading.
copy-holdings-view-closed = Closed rounds ({ $count })
copy-holdings-reset = Reset paper book
copy-holdings-live-note = Live copies are real positions.
copy-holdings-open-positions = Open Positions
copy-holdings-token-details = Open token details
# $time is a formatted date and time.
copy-holdings-opened = Opened { $time }
copy-holdings-no-pool-price = No pool price
copy-holdings-close = Close
copy-holdings-write-off = Write off
copy-holdings-activity = Activity
copy-holdings-no-exit-rule = No exit rule
# $level is the exit price relative to entry and $span the time until the rule acts.
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } in { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = Trail arms { $level }
copy-holdings-watch-time = Time ≤ { $level }
copy-holdings-watch-time-until = Time ≤ { $level } in { $span }
copy-holdings-watch-wallet-sells = Wallet sells
copy-holdings-empty = No open paper holdings. Buys copied from the wallet appear here.
copy-holdings-col-token = Token
copy-holdings-col-cost = Cost
copy-holdings-col-entry = Entry
copy-holdings-col-mark = Mark
copy-holdings-col-peak = Peak
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Exit rules
copy-holdings-col-held = Held
copy-holdings-col-actions = Actions
copy-holdings-col-invested = Invested
copy-holdings-col-proceeds = Proceeds
copy-holdings-col-exit = Exit
copy-holdings-col-closed = Closed
copy-holdings-price-note = Prices are { -sol } per token. Entry includes the buy's slippage and fees; the peak and the exit levels are relative to it, so a holding opens with its peak below entry. Hover one for its pool price.
copy-holdings-paused-rules = Paused: no new copies. Your exit rules still close these holdings.
copy-holdings-paused-mirror = Paused: no new copies. The wallet's sells still close these holdings.
copy-holdings-paused-hybrid = Paused: no new copies. The wallet's sells and your exit rules still close these holdings.
# $error is the failure detail.
copy-holdings-closed-load-failed = Closed rounds could not be loaded: { $error }
copy-holdings-closed-loading = Loading closed rounds…
copy-holdings-closed-empty = No closed rounds yet.
copy-holdings-closed-latest = Latest { $shown } of { $total } rounds.
copy-holdings-close-title = Close paper holding
# $token is the token symbol and $price a pool price with its unit.
copy-holdings-close-message = Sell { $token } in the paper book at the pool price ({ $price }) with the task's slippage and fees.
copy-holdings-close-confirm = Close holding
copy-holdings-write-off-title = Write off paper holding
# $token is the token symbol and $cost a SOL amount.
copy-holdings-write-off-message = { $token } has no pool price to sell at. Writing it off closes it at zero and books its { $cost } cost as a loss.
copy-holdings-keep = Keep
copy-holdings-written-off = { $token } written off
copy-holdings-closed = { $token } closed
copy-holdings-written-off-detail = Closed at zero proceeds
# $price is a pool price with its unit.
copy-holdings-sold-at = Sold at { $price }
copy-holdings-close-failed = Holding could not be closed
# $name is the task name.
copy-holdings-reset-message = Start “{ $name }” over: its paper holdings, spend, fills, exits and skips are removed. The rules and the wallet stay.
copy-holdings-reset-cancel = Keep history
copy-holdings-reset-done = Paper book reset
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } decision removed
       *[other] { $count } decisions removed
    }
copy-holdings-reset-failed = Paper book could not be reset

## Activity tab (pages/copy/activity.js). Ids come from CopyOutcome
## (src/trader/copy/types.rs).

copy-activity-title = Activity
copy-activity-filter-label = Activity filter
copy-filter-all = All
copy-outcome-paper-filled = Paper buy
copy-outcome-live-submitted = Live buy submitted
copy-outcome-live-confirmed = Live buy confirmed
copy-outcome-live-failed = Live buy failed
copy-outcome-paper-sell-observed = Paper sell · wallet sold
copy-outcome-live-sell-submitted = Live sell submitted
copy-outcome-live-sell-failed = Live sell failed
copy-outcome-skipped = Skipped
copy-activity-decision = Decision
# $rule is the exit rule label.
copy-activity-paper-exit = Paper exit · { $rule }
# $input, $price and $target are formatted values; $slippage a signed percentage.
copy-activity-filled = { $input } at { $price } · wallet bought { $target }
copy-activity-filled-slippage = { $input } at { $price } · wallet bought { $target } · slippage { $slippage }
copy-activity-filled-unpriced = { $input } at { $price } · wallet bought { $target } · priced at the wallet's trade, no pool price
copy-activity-live-sized = { $sized } · wallet bought { $target }
copy-activity-sell-nothing = Wallet sold { $amount } · nothing held to sell
copy-activity-written-off = Written off at zero: no pool price
# $tokens is a token amount and $proceeds a SOL amount.
copy-activity-sold = { $tokens } tokens for { $proceeds } at { $price }
copy-activity-full-close = Full close
copy-activity-partial-exit = { $pct } exit
# $label is the skip reason and $detail its figures.
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = minimum { $amount }
copy-activity-skip-maximum = maximum { $value }
copy-activity-skip-stale = { $arrival } late, limit { $limit }
copy-activity-skip-latency = { $average } average, limit { $limit }
# $span is the time from the block to detection.
copy-activity-arrival-replayed = Replayed { $span } after the block
copy-activity-arrival-seen = Seen { $span } after the block
copy-activity-link-wallet-tx = Wallet tx
copy-activity-link-own-tx = Your tx
copy-activity-only-token = Only this token
copy-activity-skipped-group = Skipped ×{ $count }
# $since is a formatted date and time.
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } token · since { $since }
       *[other] { $tokens } tokens · since { $since }
    }
copy-activity-mint-filter =
    .placeholder = Token mint
    .aria-label = Filter by token mint
copy-activity-clear = Clear
# $error is the failure detail.
copy-activity-load-failed = Activity could not be loaded: { $error }
copy-activity-loading = Loading activity…
copy-activity-no-match = Nothing matches this filter.
copy-activity-empty = No decisions yet. Fills, exits and skips appear here as the wallet trades.
copy-activity-load-older = Load older
copy-activity-start = Start of history
copy-activity-older-failed = Older activity could not be loaded

## Task editor (pages/copy/editor.js, pages/copy/editor_steps.js)

copy-step-wallet = Wallet
copy-step-sizing = Sizing
copy-step-entry = Entry filters
copy-step-exits = Exits
copy-step-review = Review
# $name is the task name.
copy-editor-title-edit = Edit { $name }
copy-editor-title-clone = Clone { $name }
# $mode is the execution mode label.
copy-editor-sub-edit = { $mode } task · changes apply to its next decisions
copy-editor-sub-clone = Same rules, empty paper book, starts in Paper
copy-editor-save-edit = Save changes
copy-editor-save-clone = Create clone
copy-editor-save-create = Create paper task
# Appended to a task name to label its clone.
copy-editor-clone-suffix = (copy)
copy-editor-discard-edit = Discard changes
copy-editor-discard-create = Discard this task
# $name is the task name.
copy-editor-discard-edit-message = Your changes to “{ $name }” are not saved.
copy-editor-discard-create-message = The wallet and rules entered so far are not saved.
copy-editor-discard-confirm = Discard
copy-editor-keep-editing = Keep editing
copy-editor-toast-updated = Task updated
copy-editor-toast-clone = Clone created
copy-editor-toast-created = Paper task created
copy-unit-native = { -sol }
copy-editor-any = Any
# $tasks lists the tasks already copying the wallet.
copy-editor-duplicate = Already copied by { $tasks }. This task copies the same trades again, with its own rules and budget.
copy-editor-wallet = Wallet
copy-editor-wallet-identity = A task's wallet is its identity. To copy another wallet with these rules, clone the task.
copy-editor-address-label = Wallet address
copy-editor-address-placeholder = Solana wallet address
copy-editor-address-help-clone = Same rules with an empty paper book. Keep this wallet to test other rules on it, or enter another wallet.
copy-editor-address-help-create = The wallet whose buys (and, if you choose, sells) this task copies.
copy-editor-name-label = Name <em>optional</em>
copy-editor-name-placeholder = e.g. Fast rotator
copy-editor-enabled-title = Process the wallet's trades
copy-editor-enabled-help = Off keeps the task paused until you resume it.
copy-editor-note-live = This task is live: changes apply to its next real copies.
copy-editor-note-paper = Tasks run in Paper until you arm them: trades are simulated at the pool price and nothing is spent.
copy-editor-copy-size = Copy size
copy-editor-sizing-fixed = Fixed amount
copy-editor-sizing-ratio = Share of the wallet's trade
copy-editor-amount-fixed = Amount per copy
copy-editor-amount-ratio = Share of each trade
# $minimum is the smallest copy in SOL.
copy-editor-amount-help-fixed = Spent on each copied buy, at least { $minimum }.
copy-editor-amount-help-ratio = Of the wallet's own buy, up to the per-trade cap.
copy-editor-help-trade-cap = No single copy spends more.
copy-editor-help-token-cap = Total spent on one token.
copy-editor-help-budget = Everything this task may spend over its life; Paper and Live each count their own spend.
copy-editor-preview-title = What a copy costs
copy-editor-preview-empty = Enter the sizing to see what a copy costs.
# $target and $copy are SOL amounts.
copy-editor-preview-example = The wallet buys { $target } → you copy <strong>{ $copy }</strong>
# $size is the size of one copy in SOL.
copy-editor-preview-once = One token takes a single copy of { $size }, as each token is bought once
copy-editor-preview-token-cap =
    { $count ->
        [one] One token takes at most { $count } copy of { $size }
       *[other] One token takes at most { $count } copies of { $size }
    }
# $perToken is the per-token clause and $count the number of copies the budget covers.
copy-editor-preview-summary-exact = { $perToken }; the budget covers about { $count } of them. Network and priority fees come on top.
copy-editor-preview-summary-minimum = { $perToken }; the budget covers at least { $count } of them. Network and priority fees come on top.
copy-editor-target-min = Smallest wallet trade copied
copy-editor-target-min-help = Ignore the wallet's smaller buys. Leave empty for no minimum.
copy-editor-target-max = Largest wallet trade copied
copy-editor-target-max-help = Ignore the wallet's larger buys. Leave empty for no maximum.
copy-editor-buy-once-title = Buy each token once
copy-editor-buy-once-help = Copy only the wallet's first buy of a token; later buys of it are skipped.
copy-editor-filter-require = Require
copy-editor-filter-skip = Don't require
copy-editor-filter-help = Require a token to pass your Filtering pipeline before it is copied.
copy-editor-filter-warning = With the default Filtering setup almost every token fails, so a task that requires a pass copies nothing. Require it only when your filters pass the tokens this wallet trades.
copy-editor-exit-both = Both
copy-editor-exit-help-buy-only = Your rules below sell every holding; the wallet's sells are ignored.
copy-editor-exit-help-hybrid = Whichever comes first: the wallet sells, or one of your rules fires.
copy-editor-exit-help-mirror = Holdings are sold only when the wallet sells. Your exit rules do not run.
copy-editor-who-sells = Who sells
copy-editor-preset = Preset
copy-editor-preset-help = A preset fills every rule below; adjust any of them after.
# $mine and $both are the names of the two exit choices that run the rules.
copy-editor-mirror-note = These rules do not run while the wallet's sells decide. They apply if you switch to { $mine } or { $both }.
copy-editor-rule-inherit = Trader default
# $value is the Trader's value in the rule's own wording.
copy-editor-inherit-value = Trader default ({ $value })
# $rule is the exit rule name.
copy-editor-rule-aria = { $rule } setting
copy-editor-rule-empty-uses = Empty uses the Trader default: { $value }
# $summary is the Trader's setting for the rule.
copy-editor-rule-follows = Follows the Trader: { $summary }
copy-editor-rule-follows-plain = Follows the Trader's setting.
copy-editor-rule-off-note = Off for this task, whatever the Trader uses.
copy-editor-unnamed = Unnamed task
# $name is the task name, $mode its execution mode and $status what happens once saved.
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = processes trades once saved
copy-editor-review-paused = saved paused
copy-editor-error-address = Enter a valid Solana wallet address.
copy-editor-error-sizing = Every sizing value must be above zero.
# $minimum is the smallest copy in SOL.
copy-editor-error-min-copy = A copy must be at least { $minimum }: raise the amount per copy.
copy-editor-error-min-cap = A copy must be at least { $minimum }: raise the per-trade cap.
copy-editor-error-trade-cap = The per-trade cap cannot exceed the per-token cap.
copy-editor-error-token-cap = The per-token cap cannot exceed the total budget.
# $min and $max are formatted percentages.
copy-editor-error-slippage = Slippage must be between { $min } and { $max }.
copy-editor-error-target-limits = Wallet trade limits must be zero or more.
copy-editor-error-target-order = The smallest wallet trade cannot exceed the largest.

## Copy notices: toasts, the event log and Telegram (trader/copy/notify.rs).

# $id is the task number, shown when the task has no name.
copy-notice-task-unnamed = Task #{ $id }
# $task is the task name, $title the notice title.
copy-notice-heading = { $task }: { $title }
# $task is the task name, $title and $detail the notice title and detail.
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Paper copy buy
copy-notice-title-paper-sell = Paper copy sell
copy-notice-title-paper-closed = Paper holding closed
# $rule is the exit rule label.
copy-notice-title-paper-exit = Paper exit: { $rule }
copy-notice-title-live-buy-submitted = Live copy buy submitted
copy-notice-title-live-buy-confirmed = Live copy buy confirmed
copy-notice-title-live-buy-failed = Live copy buy failed
copy-notice-title-live-sell-submitted = Live copy sell submitted
copy-notice-title-live-sell-failed = Live copy sell failed
copy-notice-title-auto-paused = Copy task auto-paused
# $amount is a SOL amount.
copy-notice-detail-bought = Bought for { $amount } { -sol }
copy-notice-detail-sold = Sold for { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
# $percent is the share of the holding sold, one decimal.
copy-notice-detail-partial-close = { $percent }% of the holding
copy-notice-detail-full-close = Full close
# $error is the failure text reported by the swap.
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Swap failed
