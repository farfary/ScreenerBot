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
