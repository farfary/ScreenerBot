# Services Module — Architecture

> ScreenerBot Service Manager & Lifecycle — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Service Trait](#3-service-trait)
4. [ServiceManager](#4-servicemanager)
5. [Startup Order](#5-startup-order)
6. [Shutdown Sequence](#6-shutdown-sequence)
7. [Registered Services](#7-registered-services)
8. [Health & Metrics](#8-health--metrics)
9. [Module Connections](#9-module-connections)

---

## 1. Overview

The Services module manages lifecycle for all 20 background services: registration, dependency-aware startup, health monitoring, metrics collection, and graceful shutdown. Uses a Service trait with topological dependency resolution.

**Key characteristics:**
- `Service` trait with async lifecycle methods
- Topological sort for dependency-aware startup
- 10-second per-service shutdown timeout
- Global `GLOBAL_SERVICE_MANAGER` for webserver access
- Hot reload: `start_newly_enabled()` for config changes
- TaskMonitor per service for observability

**23 files, ~3,807 lines**

---

## 2. File Structure

```
src/services/
├── mod.rs              # ServiceManager (891 lines)
├── health.rs           # Health monitoring
├── metrics.rs          # Metrics collection
└── implementations/    # 20 service implementations
    ├── ai_service.rs
    ├── ata_cleanup_service.rs
    ├── events_service.rs
    ├── filtering_service.rs
    ├── ohlcv_service.rs
    ├── pool_analyzer_service.rs
    ├── pool_calculator_service.rs
    ├── pool_discovery_service.rs
    ├── pool_fetcher_service.rs
    ├── pools_service.rs
    ├── positions_service.rs
    ├── rpc_stats_service.rs
    ├── scheduled_ai_tasks_service.rs
    ├── sol_price_service.rs
    ├── tokens_service.rs
    ├── transactions_service.rs
    ├── update_check_service.rs
    ├── wallet_service.rs
    └── webserver_service.rs
```

---

## 3. Service Trait

```rust
#[async_trait]
pub trait Service: Send + Sync {
    fn name(&self) -> &'static str;
    fn priority(&self) -> i32 { 100 }              // Lower = starts earlier
    fn dependencies(&self) -> Vec<&'static str> { vec![] }
    fn is_enabled(&self) -> bool { true }
    async fn initialize(&mut self) -> Result<(), String>;
    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn health(&self) -> ServiceHealth;
    async fn metrics(&self) -> ServiceMetrics;
}
```

---

## 4. ServiceManager

```rust
pub struct ServiceManager {
    services: HashMap<&'static str, Box<dyn Service>>,
    handles: HashMap<&'static str, Vec<JoinHandle<()>>>,
    shutdown: Arc<Notify>,
    metrics_collector: MetricsCollector,
    task_monitors: HashMap<&'static str, TaskMonitor>,
    cached_health: Arc<RwLock<...>>,
    cached_metrics: Arc<RwLock<...>>,
}
```

**Global access:** `LazyLock<Arc<RwLock<Option<ServiceManager>>>>`

### Key Methods

| Method | Purpose |
|--------|---------|
| `register(service)` | Add service to registry |
| `start_all()` | Resolve deps → init → start (topological order) |
| `stop_all()` | Shutdown in reverse order |
| `start_newly_enabled()` | Hot-start after config change |
| `get_health_snapshot()` | All services' health |
| `get_metrics_snapshot()` | All services' metrics |

---

## 5. Startup Order

**Algorithm:** Topological sort with cycle detection

1. Build dependency graph from `service.dependencies()`
2. Validate all dependencies exist
3. Detect circular dependencies → error
4. DFS traversal produces topological order
5. Within same dependency level, sort by `priority()`
6. Initialize each service sequentially
7. Start each service sequentially (spawns async tasks)

**Timing:** Logs gaps >100ms between starts. Tracks total startup duration.

---

## 6. Shutdown Sequence

```
stop_all()
├─ shutdown.notify_waiters()        # Signal all services
├─ Reverse startup order            # LIFO (last started → first stopped)
├─ For each service:
│  ├─ service.stop()                # Graceful stop
│  ├─ await handles (10s timeout)   # Wait for tasks
│  └─ Log timeout/panic if needed
└─ Return shutdown report
```

**10-second timeout** per service. Services that exceed are logged as warnings but don't block others.

---

## 7. Registered Services

| Service | Priority | Dependencies | Purpose |
|---------|----------|-------------|---------|
| `tokens` | 10 | — | Token registry, loading, stale filter |
| `pools` | 15 | tokens | Pool management |
| `wallet` | 20 | — | Balance monitoring |
| `positions` | 25 | tokens, pools | Position tracking |
| `sol_price` | 30 | — | SOL/USD price |
| `events` | 35 | — | Event recording |
| `filtering` | 40 | tokens | Token filter engine |
| `ohlcv` | 45 | tokens, pools | OHLCV candle data |
| `pool_discovery` | 50 | pools | New pool detection |
| `pool_fetcher` | 55 | pools | Pool data refresh |
| `pool_analyzer` | 60 | pools | Pool analysis |
| `pool_calculator` | 65 | pools | Pool calculations |
| `transactions` | 70 | — | Transaction monitoring |
| `rpc_stats` | 75 | — | RPC metrics |
| `connectivity` | 80 | — | Endpoint health |
| `ata_cleanup` | 85 | wallet | ATA garbage collection |
| `telegram` | 50 | — | Telegram bot |
| `ai` | 90 | — | AI engine |
| `scheduled_ai` | 95 | ai | Scheduled AI tasks |
| `webserver` | 100 | — | Dashboard HTTP server |
| `update_check` | 100 | — | Version checking |

---

## 8. Health & Metrics

### ServiceHealth

```rust
pub struct ServiceHealth {
    pub name: String,
    pub status: HealthStatus,          // Healthy, Degraded, Unhealthy
    pub last_check: DateTime<Utc>,
    pub details: Option<String>,
}
```

### ServiceMetrics

```rust
pub struct ServiceMetrics {
    pub name: String,
    pub task_count: usize,
    pub uptime_seconds: u64,
    pub custom: HashMap<String, Value>,
}
```

---

## 9. Module Connections

```
services/
├── config/        ← is_enabled() checks
├── logger/        ← Startup/shutdown logging
├── events/        ← Service events recording
└── all modules    ← Each service wraps a module's background work
```

| Caller | Usage |
|--------|-------|
| main.rs | `start_all()` at startup, `stop_all()` at shutdown |
| webserver | Health/metrics API endpoints |
| config reload | `start_newly_enabled()` for dynamic services |
