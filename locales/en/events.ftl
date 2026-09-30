# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = OHLCV event: { $subtype }
events-filtering-default = Filtering event: { $subtype }
events-trader-default = Trader event: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = Task '{ $name }' completed
events-task-failed = Task '{ $name }' failed
events-task-timed-out = Task '{ $name }' timed out

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = Task completed
events-subtype-task-failed = Task failed
events-subtype-task-timed-out = Task timed out

# Shown when an event carries no display text.
events-message-none = No message

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = Failed to cleanup OHLCV cache
events-ohlcv-gap-cleanup-failed = Failed to cleanup filled gap records
events-ohlcv-gap-fill-failed = Gap fill error for { $mint }
events-ohlcv-backfill-scheduled = Scheduled multi-timeframe backfill for { $mint } via { $pool }
events-ohlcv-fetch-failed = Failed to fetch OHLCV for { $mint } via { $pool }: { $error }
events-ohlcv-gap-detection-failed = Gap detection failed for { $mint } via { $pool }
events-ohlcv-fetch-success = Stored { $count } OHLCV points for { $mint }
events-ohlcv-retention-backfill-failed = Retention backfill failed for { $mint } via { $pool }
events-ohlcv-empty-fetch = Empty OHLCV fetch for { $mint } via { $pool }
events-ohlcv-pool-discovery-failed = Pool discovery failed for { $mint }
events-ohlcv-pool-discovery-success = Discovered pools for { $mint }
events-ohlcv-process-token-error = Error processing { $mint }: { $error }
events-ohlcv-rate-limit-hit = Rate limit triggered while processing { $mint }
events-ohlcv-pool-unavailable = No healthy pools available for { $mint }; deferring
events-ohlcv-token-missing = Token { $mint } was missing during processing
events-monitors-stopped = Automated trading monitors stopped
events-monitors-starting = Automated trading monitors starting up
events-entry-monitor-started = Entry opportunity monitor started
events-exit-monitor-started = Exit/position monitor started
events-trader-service-stopped = Trader service gracefully stopped
events-trader-service-stopping = Trader service shutdown initiated
events-trader-service-started = Trader service fully initialized and running
events-trader-auto-trading-error = Auto trading encountered an error
events-trader-trading-enabled = Trading is enabled and active
events-trader-trading-disabled = Trading is disabled in configuration
events-trader-service-initializing = Trader service initialization beginning
events-connectivity-monitoring-stopped = Connectivity monitoring stopped
events-connectivity-monitoring-started = Connectivity monitoring started (interval={ $seconds }s)
events-connectivity-service-initialized = Connectivity service initialized with { $count } monitors
events-connectivity-critical-unhealthy = { $count } critical endpoint(s) unhealthy - System should pause operations
events-connectivity-endpoint-recovered = Endpoint recovered from { $from } to healthy
