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
events-position-entry-not-landed = The buy of { $symbol } did not land on chain; its position was removed
events-position-fill-after-force-close = A trade of { $symbol } landed on chain after its position was force closed; it was booked and the position restated
events-position-swap-unbooked = A swap of { $symbol } confirmed on chain is not in its position yet; it is verified again until it is booked

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = Swap
events-category-transaction = Transaction
events-category-pool = Pool
events-category-position = Position
events-category-token = Token
events-category-wallet = Wallet
events-category-trader = Trader
events-category-entry = Entry
events-category-system = System
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = Security
events-category-connectivity = Connectivity
events-category-filtering = Filtering
events-category-scheduled-task = Scheduled task
events-category-learner = Learner
events-category-other = Other

events-loading = Loading events...
events-load-failed = Failed to load events
events-load-failed-description = Waiting for the backend to respond. We will retry automatically.
events-load-error = Could not load events
events-search-placeholder = Search events...
events-summary-total = Total
events-filter-category = Category
events-filter-all-categories = All Categories
events-filter-all-severities = All Severities
events-col-time = Time
events-col-category = Category
events-col-type = Type
events-col-severity = Severity
events-col-message = Message
events-col-token = Token
events-col-details = Details
# $count is the number of payload entries not shown in the preview.
events-payload-more = +{ $count } more

## Event details dialog (ui/events_dialog.js)

events-dialog-title = Event details
events-dialog-close =
    .aria-label = Close dialog
events-dialog-payload = Payload
events-dialog-copy = Copy Details
events-dialog-copy-title =
    .title = Copy all event details
events-dialog-copy-done = Copied!
events-dialog-copy-failed = Failed
events-dialog-not-available = N/A
# $category is the category label; shown when an event has no message.
events-dialog-category-event = { $category } event
events-dialog-field-id = Event ID
events-dialog-field-severity = Severity
events-dialog-field-category = Category
events-dialog-field-subtype = Subtype
events-dialog-field-mint = Token Mint
events-dialog-field-reference = Reference
events-dialog-field-time = Event Time
events-dialog-field-age = Age
events-dialog-field-created = Created
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = EVENT DETAILS
events-dialog-export-message = MESSAGE
events-dialog-export-payload = PAYLOAD
events-dialog-export-line = { $label }: { $value }
