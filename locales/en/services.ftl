# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

# $component is a code identifier and is not translated.
services-health-component-unavailable = { $component } component not available
services-health-unavailable = Health status unavailable
services-health-pools-not-running = Pool service not running
services-health-events-db-uninitialized = Events database not initialized
services-health-sol-price-not-running = SOL price service is not running
services-health-sol-price-stale = SOL price data is stale ({ $seconds }s old)
services-health-sol-price-no-data = No SOL price data available yet
services-health-telegram-discovery = Discovery mode
services-health-telegram-disconnected = Disconnected
services-health-wallet-watch-polling-only = Detection running on polling alone
services-health-assistant-tasks-disabled = Disabled in config
# $endpoints is the debug-formatted list of unhealthy critical endpoints.
services-health-connectivity-critical-unhealthy = Critical endpoints unhealthy: { $endpoints }
# $seconds is the age of the filtering snapshot.
services-health-filtering-snapshot-stale = Filtering snapshot is { $seconds }s old

## Services page (pages/services.js)

# Health status tags from ServiceHealth in src/services/health.rs.
services-status-healthy = Healthy
services-status-starting = Starting
services-status-degraded = Degraded
services-status-unhealthy = Unhealthy
services-status-stopping = Stopping
services-status-disabled = Disabled
services-status-unknown = Unknown

# Service ids from Service::name() in src/services/implementations/*.rs.
services-name-account = Account
services-name-assistant-scheduled-tasks = Assistant scheduled tasks
services-name-ata-cleanup = Token account cleanup
services-name-connectivity = Connectivity
services-name-copy-trading = Copy trading
services-name-events = Events
services-name-filtering = Filtering
services-name-llm-analysis = LLM analysis
services-name-ohlcv = OHLCV
services-name-pool-pricing = Pool pricing
services-name-pools = Pools
services-name-positions = Positions
services-name-referral = Referral
services-name-rpc-stats = RPC stats
services-name-sol-price = { -sol } price
services-name-telegram = { -telegram }
services-name-tokens = Tokens
services-name-trader = Trader
services-name-transactions = Transactions
services-name-update-check = Update check
services-name-wallet = Wallet
services-name-wallet-watch = Wallet watch
services-name-webserver = Web server

services-loading = Loading services...
services-load-failed = Failed to load services
services-load-failed-description = Waiting for the backend to respond. We will retry automatically.
services-refresh-failed = Could not refresh services
services-search-placeholder = Search services...
services-summary-total = Total
services-summary-alerts = Alerts
# $degraded and $unhealthy are formatted counts.
services-summary-alerts-tooltip = { $degraded } degraded / { $unhealthy } unhealthy
services-filter-status = Status
services-filter-all-statuses = All Statuses
services-filter-all-services = All Services
services-filter-enabled-only = Enabled Only
services-filter-disabled-only = Disabled Only
services-col-service = Service
services-col-health = Health
services-col-priority = Priority
services-col-uptime = Uptime
services-col-activity = Activity
services-col-last-cycle = Last Cycle
services-col-avg-cycle = Avg Cycle
services-col-avg-poll = Avg Poll
services-col-cycle-rate = Cycle Rate
services-col-tasks = Tasks
services-col-ops = Ops/sec
services-col-errors = Errors
services-col-dependencies = Dependencies
services-dependencies-none = None
# $percent is a formatted percentage.
services-activity-busy = { $percent } busy
services-activity-polls =
    { $count ->
        [one] { $count } poll
       *[other] { $count } polls
    }
# The durations are formatted values; the message spans several lines.
services-tasks-tooltip =
    { $count ->
        [one] { $count } task
       *[other] { $count } tasks
    }
    Last: { $last }
    Avg: { $avg }
    Poll: { $poll }
    Idle: { $idle }
    Total Polls: { $polls }
services-tasks-none = No instrumented tasks

# Empty table (scripts/pages/services.js)
services-empty = No services running
    .message = Services appear here once the bot has started them.
