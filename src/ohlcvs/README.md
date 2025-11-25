# OHLCV Module

Comprehensive OHLCV (Open, High, Low, Close, Volume) data management system for ScreenerBot with multi-timeframe support, intelligent caching, gap-filling, and smart monitoring priorities.

## 📁 Module Structure

```
src/ohlcvs/
├── mod.rs          - Public API and module exports
├── types.rs        - Core data structures and enums
├── database.rs     - SQLite database layer
├── fetcher.rs      - GeckoTerminal API fetching with rate limiting
├── cache.rs        - Three-tier caching system
├── manager.rs      - Multi-pool management and failover
├── monitor.rs      - Background monitoring service
├── aggregator.rs   - Timeframe aggregation logic
├── gaps.rs         - Gap detection and filling
├── priorities.rs   - Smart priority system
└── service.rs      - Main service implementation
```

## 🎯 Features

### Multi-Timeframe Support
- **Supported**: 1m, 5m, 15m, 1h, 4h, 12h, 1d
- **Base Data**: Always fetch 1-minute data, aggregate to higher timeframes
- **Storage**: Raw 1m data in SQLite, aggregated data cached

### Multi-Pool Architecture
```
Token → Multiple Pools → Priority Selection
         ├── Pool A (highest liquidity) ← DEFAULT
         ├── Pool B (backup)
         └── Pool C (tertiary)
```

- Automatic pool discovery
- Failover on pool failures
- Health tracking per pool

### Smart Priority System

| Priority | Base Interval | Use Case |
|----------|--------------|----------|
| **Critical** | 30s | Open positions |
| **High** | 1m | Recently viewed tokens |
| **Medium** | 5m | Watched tokens |
| **Low** | 15m | Historical/inactive |

### Adaptive Throttling

```
Fetch Interval = Base Interval * (1 + hours_inactive/24) * (1 + empty_fetches/10)
Max Interval = Base Interval * 10 (capped)
```

### Three-Tier Caching

1. **Hot Cache** (Memory)
   - Last 24h of 1m data for active tokens
   - LRU eviction, max 100 tokens
   - Sub-50ms access time

2. **Warm Cache** (SQLite)
   - 7 days of all data
   - Indexed for fast queries
   - Automatic aggregation

3. **Cold Storage** (Archived)
   - Aggregated summaries
   - Historical analysis

### Gap Detection & Filling

- Automatic gap detection in data sequences
- Priority-based filling (recent gaps first)
- Batch processing to minimize API calls
- Progress tracking in database

## 🚀 Usage

### Public API

```rust
use screenerbot::ohlcvs::{
    get_ohlcv_data, get_available_pools, get_data_gaps, 
    request_refresh, add_token_monitoring, Priority, Timeframe
};

// Get OHLCV data
let data = get_ohlcv_data(
    "mint_address",
    Timeframe::Minute5,
    None, // pool_address (None = use default)
    100,  // limit
    None, // from_timestamp
    None, // to_timestamp
).await?;

// Add token to monitoring
add_token_monitoring("mint_address", Priority::High).await?;

// Force refresh
request_refresh("mint_address").await?;

// Get available pools
let pools = get_available_pools("mint_address").await?;

// Check for gaps
let gaps = get_data_gaps("mint_address", Timeframe::Minute1).await?;
```

### REST API Endpoints

```bash
# Get OHLCV data
GET /api/ohlcv/{mint}?timeframe=5m&limit=100&from=1234567890&to=1234567999

# Get available pools
GET /api/ohlcv/{mint}/pools

# Get detected gaps
GET /api/ohlcv/{mint}/gaps?timeframe=1m

# Get data status
GET /api/ohlcv/{mint}/status

# Force refresh
POST /api/ohlcv/{mint}/refresh

# Start monitoring
POST /api/ohlcv/{mint}/monitor
{
  "priority": "high"
}

# Stop monitoring
DELETE /api/ohlcv/{mint}/monitor

# Record chart view (updates priority)
POST /api/ohlcv/{mint}/view

# Get system metrics
GET /api/ohlcv/metrics
```

### Response Examples

**OHLCV Data Response:**
```json
{
  "success": true,
  "data": {
    "mint": "token_address",
    "pool_address": "pool_address",
    "timeframe": "5m",
    "count": 100,
    "data": [
      {
        "timestamp": 1234567890,
        "open": 1.23,
        "high": 1.25,
        "low": 1.20,
        "close": 1.24,
        "volume": 10000.0
      }
    ]
  }
}
```

**Pools Response:**
```json
{
  "success": true,
  "data": {
    "mint": "token_address",
    "pools": [
      {
        "address": "pool1",
        "dex": "raydium",
        "liquidity": 50000.0,
        "is_default": true,
        "is_healthy": true,
        "last_successful_fetch": "2025-10-06T12:00:00Z",
        "failure_count": 0
      }
    ],
    "default_pool": "pool1"
  }
}
```

**Metrics Response:**
```json
{
  "success": true,
  "data": {
    "tokens_monitored": 50,
    "pools_tracked": 150,
    "api_calls_per_minute": 15.5,
    "cache_hit_rate_percent": 85.0,
    "average_fetch_latency_ms": 250.0,
    "gaps_detected": 10,
    "gaps_filled": 8,
    "data_points_stored": 500000,
    "database_size_mb": 32.0
  }
}
```

## ⚙️ Configuration

Configuration is in `data/config.toml` under `[ohlcv]`:

```toml
[ohlcv]
enabled = true
max_monitored_tokens = 100
retention_days = 7
api_rate_limit = 30
default_fetch_interval_secs = 900
critical_fetch_interval_secs = 30
high_fetch_interval_secs = 60
medium_fetch_interval_secs = 300
max_empty_fetches = 10
auto_fill_gaps = true
gap_fill_interval_secs = 300
cache_size = 100
cache_retention_hours = 24
cleanup_interval_secs = 3600
pool_failover_enabled = true
max_pool_failures = 5
```

Access config values:

```rust
use screenerbot::config::with_config;

let enabled = with_config(|cfg| cfg.ohlcv.enabled);
let rate_limit = with_config(|cfg| cfg.ohlcv.api_rate_limit);
```

## 🗄️ Database Schema

The module uses `data/ohlcvs.db` with the following tables:

- **ohlcv_pools** - Pool configurations and health
- **ohlcv_1m** - Raw 1-minute data (7-day retention)
- **ohlcv_aggregated** - Cached aggregated timeframes
- **ohlcv_gaps** - Detected gaps and fill status
- **ohlcv_monitor_config** - Token monitoring configurations

## 🔄 Background Services

The monitor runs several background loops:

1. **Main Monitor Loop** (5s interval)
   - Checks each token's fetch schedule
   - Processes priority-based fetching
   - Updates activity tracking

2. **Pool Service Sync Loop** (30s interval)
   - Syncs token list with Pool Service (~500 tokens)
   - Pool Service updates prices every 5-10s (very fast)
   - Auto-upgrades priority for tokens with open positions
   - Removes inactive tokens that are no longer in Pool Service

3. **Gap Fill Loop** (5m interval)
   - Detects gaps in recent data
   - Fills gaps for active tokens
   - Prioritizes critical tokens

4. **Cleanup Loop** (1h interval)
   - Removes data older than retention period
   - Compacts database
   - Maintains storage limits

5. **Cache Maintenance Loop** (10m interval)
   - Evicts expired cache entries
   - Rebalances LRU cache
   - Updates hit rate metrics

## 📊 Performance Characteristics

- **Cache Hit Rate**: > 80% for active tokens
- **API Calls**: < 30/minute (GeckoTerminal limit)
- **Response Time**: < 50ms for cached data, < 500ms for fresh fetches
- **Memory Usage**: ~50MB for 100 monitored tokens
- **Database Size**: ~32MB per 500k data points

## 🔍 Rate Limiting

The fetcher implements strict rate limiting:

- **Limit**: 30 requests/minute (GeckoTerminal)
- **Strategy**: Priority queue with time-based throttling
- **Backoff**: Exponential for 429 responses
- **Batching**: Combine similar requests when possible

## 🎯 Priority Management

Priority is dynamically adjusted based on activity:

```rust
// Activity types that update priority
ActivityType::PositionOpened     → Critical
ActivityType::PositionClosed     → High
ActivityType::ChartViewed        → Medium
ActivityType::TokenViewed        → Medium
ActivityType::DataRequested      → High
```

## 🧪 Testing

```bash
# Run module tests
cargo test --lib ohlcvs

# Test specific component
cargo test --lib ohlcvs::aggregator
cargo test --lib ohlcvs::cache
cargo test --lib ohlcvs::gaps

# Integration test with API
cargo run --bin screenerbot
# Then: curl http://localhost:8080/api/ohlcv/metrics
```

## 🚨 Error Handling

All operations return `OhlcvResult<T>`:

```rust
pub enum OhlcvError {
    DatabaseError(String),
    ApiError(String),
    RateLimitExceeded,
    PoolNotFound(String),
    InvalidTimeframe(String),
    DataGap { start: i64, end: i64 },
    CacheError(String),
    NotFound(String),
}
```

## 📈 Metrics & Monitoring

Monitor system health via:

1. **API Endpoint**: `GET /api/ohlcv/metrics`
2. **Service Health**: `GET /api/services` (includes OHLCV)
3. **Logs**: Tagged with `[OHLCV Monitor]`

Key metrics:
- Tokens monitored
- Pools tracked
- API calls/minute
- Cache hit rate
- Average latency
- Gaps detected/filled
- Database size

## 🔧 Troubleshooting

### High API usage
- Check `api_calls_per_minute` in metrics
- Increase fetch intervals in config
- Reduce `max_monitored_tokens`

### Low cache hit rate
- Increase `cache_size`
- Increase `cache_retention_hours`
- Check for excessive timeframe switching

### Data gaps
- Check pool health: `GET /api/ohlcv/{mint}/pools`
- Enable `auto_fill_gaps` in config
- Manually trigger refresh: `POST /api/ohlcv/{mint}/refresh`

### Pool failures
- Verify pool addresses are correct
- Check `pool_failover_enabled` is true
- Manually reset failures via pool manager

## 📝 Architecture Notes

### Design Decisions

1. **1-minute base data**: Always fetch finest granularity, aggregate up
2. **Single pool per token**: Use highest liquidity, fallback on failure
3. **Priority-based scheduling**: Critical tokens never throttled
4. **Gap tracking**: Explicit database tracking vs implicit detection
5. **Three-tier cache**: Balance memory, speed, and persistence

### Extension Points

- **New DEX**: Add pool discovery for new DEX
- **New timeframe**: Add to `Timeframe` enum
- **Custom aggregation**: Implement in `aggregator.rs`
- **Alternative API**: Swap `fetcher.rs` implementation

## 🚀 Migration from Old System

To migrate to the new OHLCV module:

1. Stop the bot
2. Delete old `data/ohlcvs.db` (if exists)
3. Deploy new code
4. Start bot - fresh database will be created
5. Monitor will bootstrap with open positions

No data migration needed - system fetches fresh data.

## 📚 Related Documentation

- [GeckoTerminal API Docs](https://www.geckoterminal.com/dex-api)
- [Config System README](../config/README.md)
- [Service Manager](../services/README.md)

## 🤝 Contributing

When modifying the OHLCV module:

1. Update tests for changed components
2. Verify rate limiting still works
3. Test gap detection with sample data
4. Check cache hit rate isn't degraded
5. Update this README if adding features

## 📄 License

Part of ScreenerBot - see main LICENSE file.
