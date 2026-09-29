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
copy-pause-helius-unavailable = Paused: Helius wallet checks failed
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
