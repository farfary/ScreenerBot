# Services Module — Architecture

> ScreenerBot service lifecycle management (startup/shutdown ordering, metrics, cached health) — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [Where Services Live (File Structure)](#2-where-services-live-file-structure)
3. [Service Trait](#3-service-trait)
4. [ServiceManager Core](#4-servicemanager-core)
5. [Startup Flow (run.rs + ServiceManager::start_all)](#5-startup-flow-runrs--servicemanagerstart_all)
6. [Startup Order Resolution](#6-startup-order-resolution)
7. [Shutdown Flow (ServiceManager::stop_all)](#7-shutdown-flow-servicemanagerstop_all)
8. [Hot-Start Newly Enabled Services](#8-hot-start-newly-enabled-services)
9. [Health & Metrics (ServiceHealth / ServiceMetrics)](#9-health--metrics-servicehealth--servicemetrics)
10. [Global Access + Cached Snapshots](#10-global-access--cached-snapshots)
11. [Registered Services (Current)](#11-registered-services-current)
12. [Pitfalls / Gotchas](#12-pitfalls--gotchas)

---

## 1. Overview

ScreenerBot runs as a single process with many background tasks: WebSocket watchers, pool fetch loops, token discovery, trading monitors, API stats, etc.

The `services` module provides:
* A **common lifecycle** contract (`Service`) for all background subsystems.
* A single **ServiceManager** that:
  * registers services by name,
  * determines a startup order,
  * starts services and tracks their `JoinHandle`s,
  * signals shutdown to all services,
  * waits for all tasks to exit.
* **Health + metrics** collection designed for the dashboard:
  * per-service `ServiceHealth`,
  * per-service `ServiceMetrics` (task activity via `tokio_metrics::TaskMonitor`),
  * cached snapshots to avoid blocking HTTP handlers.

---

## 2. Where Services Live (File Structure)

### 2.1 Core manager and shared types

```text
src/services/
├── mod.rs            Service trait + ServiceManager + global access helpers
├── health.rs         ServiceHealth enum (healthy/degraded/unhealthy/starting/stopping)
├── metrics.rs        ServiceMetrics + MetricsCollector (TaskMonitor + sysinfo)
└── implementations/  ALL Service trait implementations (23 services)
    ├── mod.rs
    ├── ai_service.rs
    ├── ata_cleanup_service.rs
    ├── connectivity_service.rs
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
    ├── telegram_service.rs
    ├── tokens_service.rs
    ├── trader_service.rs
    ├── transactions_service.rs
    ├── update_check_service.rs
    ├── wallet_service.rs
    └── webserver_service.rs
```

### 2.2 Service architecture pattern

**All** `impl Service for ...` live in `src/services/implementations/`.

Services are thin wrappers (<120 lines) that delegate to domain modules:
* `connectivity_service.rs` delegates to `connectivity/checker.rs`
* `trader_service.rs` delegates to `trader/*`
* `telegram_service.rs` delegates to `telegram/*`
* `filtering_service.rs` delegates to `filtering/background.rs`
* `ai_service.rs` delegates to `ai/background_worker.rs`
* `scheduled_ai_tasks_service.rs` delegates to `ai/scheduled_worker.rs`

The single source of truth for **what is actually registered** is:
* `src/run/services.rs` => `register_all_services(manager: &mut ServiceManager)`

---

## 3. Service Trait

**File:** `src/services/mod.rs`

Every service implements:

```rust
#[async_trait]
pub trait Service: Send + Sync {
    fn name(&self) -> &'static str;

    // Lower starts earlier (and, by convention, stops later)
    fn priority(&self) -> i32 { 100 }

    fn dependencies(&self) -> Vec<&'static str> { vec![] }

    // Most services override this to respect initialization + config toggles.
    fn is_enabled(&self) -> bool { true }

    async fn initialize(&mut self) -> crate::Result<()> { Ok(()) }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>>;

    async fn stop(&mut self) -> crate::Result<()> { Ok(()) }

    async fn health(&self) -> ServiceHealth { ServiceHealth::Healthy }

    async fn metrics(&self) -> ServiceMetrics { ServiceMetrics::default() }
}
```

Key architectural points:
* `start()` returns **a vector of JoinHandles**; ServiceManager owns them and awaits them at shutdown.
* `shutdown: Arc<Notify>` is the primary stop signal for long-running loops.
* `monitor: TaskMonitor` is passed in so the spawned tasks can be wrapped with `monitor.instrument(...)`.

---

## 4. ServiceManager Core

**File:** `src/services/mod.rs`

```rust
pub struct ServiceManager {
    services: HashMap<&'static str, Box<dyn Service>>,
    handles: HashMap<&'static str, Vec<JoinHandle<()>>>,
    shutdown: Arc<Notify>,
    metrics_collector: MetricsCollector,
    task_monitors: HashMap<&'static str, TaskMonitor>,
    cached_health: Arc<RwLock<HashMap<&'static str, ServiceHealth>>>,
    cached_metrics: Arc<RwLock<HashMap<&'static str, ServiceMetrics>>>,
}
```

Important responsibilities:
* **Registration**: `register(Box<dyn Service>)` stores by `service.name()`.
* **Monitoring**: `get_task_monitor(name)` creates/stores a per-service `TaskMonitor`.
* **Running set**: a service is considered “running” if it has an entry in `handles`.

---

## 5. Startup Flow (run.rs + ServiceManager::start_all)

### 5.1 Three boot modes: initialization vs preview vs full

**File:** `src/run/mod.rs`

Startup branches based on whether `config.toml` exists and whether wallet + RPC were configured:

* **No config.toml** => "initialization mode"
  * `global::INITIALIZATION_COMPLETE = false`
  * all services are registered, but most `is_enabled()` return false
  * webserver is always enabled so the user can complete setup (or skip it)

* **Config.toml exists, wallet skipped** => "preview mode"
  * Detected when `gui.dashboard.startup.setup_skipped == true` **or** `wallet_encrypted` is empty.
  * `global::PREVIEW_MODE = true`, `INITIALIZATION_COMPLETE = false`.
  * Wallet init/validation is **skipped**.
  * Only the **discovery tier** runs: `connectivity`, `events`, `tokens`, `filtering`, `webserver`
    (these gate on `global::is_preview_or_full()`). Everything wallet/RPC-bound stays off.
  * Token discovery is API-driven (DexScreener / GeckoTerminal / Rugcheck / Jupiter) so it needs
    neither a wallet nor a Solana RPC. The connectivity tier omits its RPC monitor in preview.
    The user completes setup later via the header banner /
    config tab / setup wizard, which calls `POST /api/initialization/complete` and brings the full
    tier up live (no restart).

* **Config.toml exists, wallet present** => "full mode"
  * config is loaded/validated
  * `global::INITIALIZATION_COMPLETE = true`
  * services can start normally

**Two-tier gating:** discovery-tier services use `is_enabled() = global::is_preview_or_full()`
(`is_initialization_complete() || is_preview_mode()`); all other services keep
`is_enabled() = is_initialization_complete()`. `PREVIEW_MODE` and `INITIALIZATION_COMPLETE`
are mutually exclusive — completing setup clears the former and sets the latter.

`src/run/bootstrap.rs` owns the phases outside ServiceManager:

* dashboard persistence (`actions.db`, `ai.db`, `ai_chat.db`) is initialized in every mode because
  the dashboard routes that read it are mode-independent;
* AI engines/providers may initialize in preview when AI is enabled because they do not require a
  wallet or Solana RPC;
* wallet manager, wallet consistency validation, and strategies initialize only for full mode;
* live setup completion calls the same full-runtime initializer as a normal full boot before it
  exposes `INITIALIZATION_COMPLETE` to services; if preview already lazily initialized the RPC
  singleton for token decimals, completion reloads its provider set from the newly saved config.

### 5.2 register_all_services()

**File:** `src/run/services.rs`

All services (currently used) are registered here via `manager.register(...)`.

### 5.3 Global manager insertion (“take, mutate, put back”)

**Files:** `src/services/mod.rs`, `src/run/mod.rs`

`init_global_service_manager(manager)` stores the ServiceManager in:

```rust
static GLOBAL_SERVICE_MANAGER: LazyLock<Arc<RwLock<Option<ServiceManager>>>> = ...;
```

Then `run.rs` does:
1) get the global `Arc<RwLock<Option<ServiceManager>>>`,
2) `take()` the `ServiceManager` out of the `Option`,
3) call `start_all()` (needs `&mut self`),
4) put it back for webserver access.

This pattern exists so webserver endpoints can read health/metrics through the global reference.

### 5.4 ServiceManager::start_all()

**File:** `src/services/mod.rs`

High-level flow:

```text
start_all()
├─ enabled_services = services.filter(|s| s.is_enabled())
├─ ordered = resolve_startup_order(enabled_services)
└─ for service_name in ordered:
   ├─ monitor = get_task_monitor(service_name)
   ├─ service.initialize().await
   ├─ handles = service.start(shutdown.clone(), monitor.clone()).await
   ├─ handles_map[service_name] = handles
   └─ metrics_collector.start_monitoring(service_name, monitor, shutdown.clone())
```

Extra behavior worth knowing:
* Logs gaps >100ms between services (`"Gap before 'X': Nms"`).
* Uses `startup::mark_service_start(service_name)` and `startup::mark_service_ready(service_name)` to track readiness.

---

## 6. Startup Order Resolution

**File:** `src/services/mod.rs` => `resolve_startup_order()`

The algorithm has three steps:

1) `validate_dependencies(services)`:
   * checks that each declared dependency name exists in the `services` map
   * **logs warnings only** (does not fail)

2) DFS visit with cycle detection:
   * detects circular dependencies via `visiting` set
   * pushes visited names into `ordered`
   * (missing dependency names are still visited and pushed; they will later be ignored at start time)

3) **Sort by priority**:

```rust
ordered.sort_by_key(|name| self.services.get(name).map(|s| s.priority()).unwrap_or(100));
```

Implication:
* Dependencies are effectively enforced by convention: a dependency must have a **lower** priority value than its dependents.

**Enabled filter (important):** the DFS pulls a service's declared dependencies into `ordered`
*even if those dependencies are disabled*. So after `resolve_startup_order()` returns, both
`start_all()` and `start_newly_enabled()` **retain only services whose `is_enabled()` is true**
before the start loop. This is what lets preview mode start `tokens`/`filtering` without
dragging in their disabled `transactions`/`pools` dependencies. The filter is a no-op in full
mode (everything enabled). Dependency declarations therefore remain pure ordering hints — never a
hard requirement that a dep actually run.

---

## 7. Shutdown Flow (ServiceManager::stop_all)

**File:** `src/services/mod.rs`

```text
stop_all()
├─ shutdown.notify_waiters()
├─ ordered = resolve_startup_order(running_services)
├─ ordered.reverse()                       (stop in reverse)
└─ for service_name in ordered:
   ├─ service.stop().await                 (best-effort; warning on error)
   └─ await JoinHandles with timeout=10s each
```

Task waiting behavior:
* Each handle is awaited with `tokio::time::timeout(Duration::from_secs(10), handle)`.
* Panics are logged as warnings.
* Timeouts are logged, but shutdown continues (no single service can block shutdown forever).

---

## 8. Hot-Start Newly Enabled Services

**File:** `src/services/mod.rs` => `start_newly_enabled()`

Purpose:
* After config changes, start services whose `is_enabled()` flipped from false => true.

Behavior:
* idempotent: services already in `handles` are skipped
* partial failures are collected and returned as `ServiceStartupReport`:
  * `attempted`, `started`, `failures`, `already_running`, `total_enabled`, `duration_ms`

---

## 9. Health & Metrics (ServiceHealth / ServiceMetrics)

### 9.1 ServiceHealth

**File:** `src/services/health.rs`

```rust
pub enum ServiceHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Starting,
    Stopping,
}
```

Meaning:
* `Healthy` does not necessarily mean “enabled” (some services report `Healthy` when disabled by config).
* `Starting` is used widely for “not ready yet” (e.g. tokens/transactions/positions readiness flags).

### 9.2 ServiceMetrics

**File:** `src/services/metrics.rs`

`ServiceMetrics` intentionally separates:
* **process-wide** CPU/memory (same for all services; one process),
* **per-service async task activity** (TaskMonitor poll/idle metrics),
* **service-specific operational counters** exposed via `custom_metrics`.

Notable fields:
* `task_count` (instrumented task count)
* `total_polls`, `total_poll_duration_ns`, `total_idle_duration_ns`
* derived cycle metrics (`cycles_per_second`, `avg_cycle_duration_ns`, etc.)
* `uptime_seconds`
* `operations_total`, `errors_total`, `custom_metrics: HashMap<String, f64>`

The `activity_percent()` helper is a key dashboard metric:
* activity = poll_time / (poll_time + idle_time)

### 9.3 MetricsCollector

**File:** `src/services/metrics.rs`

Per service, ServiceManager calls:

```rust
metrics_collector.start_monitoring(service_name, monitor, shutdown.clone()).await;
```

This spawns a background collector task that:
* polls `monitor.cumulative()` every 1s,
* stores cumulative totals + deltas in an internal `HashMap`,
* lets the dashboard compute “how busy a service is” without relying on OS threads.

CPU/memory measurement:
* uses `sysinfo::System::refresh_all()` inside `spawn_blocking` to avoid stalling the async runtime.

---

## 10. Global Access + Cached Snapshots

**File:** `src/services/mod.rs`

The ServiceManager periodically refreshes cached health/metrics:

* `init_global_service_manager(manager)` does an initial `manager.update_cache().await`
* then spawns a background loop:
  * every 5 seconds, calls `update_cache()` with a 3s timeout
  * terminates automatically when global manager is cleared during shutdown

For webserver handlers (hot path), use the cached variants:
* `ServiceManager::get_health_cached()`
* `ServiceManager::get_metrics_cached()`

This avoids blocking HTTP threads on async health checks or sysinfo refresh.

---

## 11. Registered Services (Current)

**File:** `src/run/services.rs` => `register_all_services()`

Registered today (in registration order):

| Service name | Type | Defined in | Notes |
|-------------|------|------------|------|
| `connectivity` | `ConnectivityService` | `src/services/implementations/connectivity_service.rs` | gated by init + `cfg.connectivity.enabled`, delegates to `connectivity/checker.rs` |
| `events` | `EventsService` | `src/services/implementations/events_service.rs` | gated by init + `cfg.events.enabled` |
| `transactions` | `TransactionsService` | `src/services/implementations/transactions_service.rs` | starts global tx manager |
| `sol_price` | `SolPriceService` | `src/services/implementations/sol_price_service.rs` | wraps `apis/sol_price.rs` |
| `pool_discovery` | `PoolDiscoveryService` | `src/services/implementations/pool_discovery_service.rs` | depends on transactions + pools + filtering |
| `pool_fetcher` | `PoolFetcherService` | `src/services/implementations/pool_fetcher_service.rs` | depends on transactions + pools + pool_discovery + filtering |
| `pool_calculator` | `PoolCalculatorService` | `src/services/implementations/pool_calculator_service.rs` | depends on pools + pool_fetcher + filtering |
| `pool_analyzer` | `PoolAnalyzerService` | `src/services/implementations/pool_analyzer_service.rs` | depends on pools + pool_fetcher + filtering |
| `pools` | `PoolsService` | `src/services/implementations/pools_service.rs` | initializes pool components + helper tasks |
| `tokens` | `TokensService` | `src/services/implementations/tokens_service.rs` | delegates to `tokens::service::TokensServiceNew` |
| `filtering` | `FilteringService` | `src/services/implementations/filtering_service.rs` | delegates to `filtering/background.rs` |
| `ohlcv` | `OhlcvService` | `src/services/implementations/ohlcv_service.rs` | starts OHLCV runtime + auto-populate from open positions |
| `positions` | `PositionsService` | `src/services/implementations/positions_service.rs` | starts positions manager + verification worker |
| `wallet` | `WalletService` | `src/services/implementations/wallet_service.rs` | wallet monitoring loops |
| `rpc_stats` | `RpcStatsService` | `src/services/implementations/rpc_stats_service.rs` | auto-save stats DB |
| `ata_cleanup` | `AtaCleanupService` | `src/services/implementations/ata_cleanup_service.rs` | starts ATA cleanup background tool |
| `trader` | `TraderService` | `src/services/implementations/trader_service.rs` | delegates to `trader/*` |
| `webserver` | `WebserverService` | `src/services/implementations/webserver_service.rs` | **always enabled** (pre-init support) |
| `ai` | `AiService` | `src/services/implementations/ai_service.rs` | delegates to `ai/background_worker.rs` |
| `scheduled_ai_tasks` | `ScheduledAiTasksService` | `src/services/implementations/scheduled_ai_tasks_service.rs` | delegates to `ai/scheduled_worker.rs` |
| `telegram` | `TelegramService` | `src/services/implementations/telegram_service.rs` | delegates to `telegram/*` |
| `update_check` | `UpdateCheckService` | `src/services/implementations/update_check_service.rs` | periodic update checks |

---

## 12. Pitfalls / Gotchas

1) **Missing dependencies do not fail startup.**  
   `validate_dependencies()` only logs warnings. The DFS order can include dependency names that are not registered; start_all() will ignore unknown names.

2) **Ordering is finally sorted by priority.**  
   Dependencies only work if dependent services have higher priority values than their dependencies.

3) **SIGHUP is intentionally ignored at the process level.**  
   `run.rs` documents that terminal disconnects (SSH/nohup) must not stop a headless bot; use SIGTERM or Ctrl+C.

4) **Disabled dependencies must be filtered before start.**  
   `resolve_startup_order()` pulls declared deps into the order even when disabled. `start_all()` and `start_newly_enabled()` `retain(|s| s.is_enabled())` afterwards. If you add a new boot mode or partial-startup tier, keep that filter — otherwise a disabled RPC/wallet service can be started transitively via a discovery-tier service's declared deps.

5) **Preview mode gates on `is_preview_or_full()`, not `is_initialization_complete()`.**
   The discovery tier (`connectivity`, `events`, `tokens`, `filtering`, `webserver`) uses `global::is_preview_or_full()`. The `initialization_gate` middleware (`src/webserver/middleware.rs`) also treats `is_preview_mode()` as allowed, so dashboard/token APIs work without a wallet. Wallet/RPC endpoints stay protected by their own `are_core_services_ready()` / FORCE_STOP guards. Do not gate a wallet-dependent service on `is_preview_or_full()` — it must stay on `is_initialization_complete()`. AI execution itself is wallet-independent, but `AiService` and `ScheduledAiTasksService` are full-mode services because their background work depends on positions/tools.

6) **Skipping setup must release the initialization wait loop.**
   A first-run process waits for `is_preview_or_full()`, not only `is_initialization_complete()`. Preview is an operational dashboard mode, not an incomplete setup that should time out later.
