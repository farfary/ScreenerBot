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
