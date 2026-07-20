# Webserver Module — Architecture

> Axum dashboard server: embedded HTML/CSS/JS frontend + REST API + middleware gates + headless auth (password + optional TOTP) — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Service Integration & Lifecycle](#3-service-integration--lifecycle)
4. [Server Startup (`start_server`) + Mode Selection](#4-server-startup-start_server--mode-selection)
5. [Router Composition (Pages, Assets, `/api`)](#5-router-composition-pages-assets-api)
6. [Middleware Stack](#6-middleware-stack)
7. [GUI Mode Security Token Model](#7-gui-mode-security-token-model)
8. [Headless Authentication (Password, Sessions, TOTP)](#8-headless-authentication-password-sessions-totp)
9. [Templates & Embedded Assets](#9-templates--embedded-assets)
10. [Status Snapshot + Metrics Caching](#10-status-snapshot--metrics-caching)
11. [Performance Optimizations](#11-performance-optimizations)
12. [Streaming (SSE)](#12-streaming-sse)
13. [Module Connections & Extension Points](#13-module-connections--extension-points)

---

## 1. Overview

The `webserver` module is ScreenerBot's HTTP interface for the dashboard.

It serves:

- **HTML pages** for routes like `/tokens`, `/filtering`, `/services`, etc.
- A **JSON REST API** under `/api/*` consumed by the frontend JavaScript.
- **Static assets** (ESM JavaScript modules, fonts, images) embedded into the binary at compile time.

The webserver supports two distinct security models depending on how the bot is run:

- **GUI mode (Electron)**: local-only server on a random high port + per-request security token header (`X-ScreenerBot-Token`).
- **Headless/CLI mode**: configurable bind host/port (default `127.0.0.1:8080`) + optional password/session authentication (and optional TOTP 2FA).

---

## 2. File Structure

```text
src/webserver/
├── mod.rs                    public exports (`start_server`, `shutdown`, `test_port_binding`)
├── server.rs                 axum server lifecycle + port selection + graceful shutdown
├── middleware.rs             request gates (security/auth/init) + cache-control
├── assets/                   embedded dashboard assets (CSS/JS/fonts/images)
├── routes/                   axum routers and handlers (HTML + `/api/*`)
│   ├── mod.rs                router composition: pages + assets + `nest("/api", api_routes())`
│   ├── actions.rs            `/api/actions/*` + SSE stream
│   ├── ai/                   AI assistant API (`/api/ai/*`)
│   ├── asset_serving.rs      `/scripts/*` + `/assets/*` handlers (all embedded)
│   ├── auth/                 headless auth API (`/api/auth/*`)
│   ├── billboard.rs          dashboard billboard / announcements
│   ├── blacklist.rs          blacklist API (`/api/blacklist*`)
│   ├── config/               config API (`/api/config*`)
│   ├── connectivity.rs       connectivity routes (`/api/connectivity/*`)
│   ├── dashboard/            dashboard API routes (merged under `/api/*`)
│   ├── events.rs             event log API (`/api/events*`)
│   ├── features.rs           feature flags (`/api/features/*`)
│   ├── filtering/            filtering API (`/api/filtering*`)
│   ├── header.rs             header state API (merged under `/api/*`)
│   ├── initialization.rs     initialization routes (`/api/initialization/*`)
│   ├── lockscreen.rs         lockscreen routes (`/api/lockscreen/*`)
│   ├── ohlcv.rs              OHLCV routes (`/api/ohlcv*`)
│   ├── positions/            positions API (`/api/positions*`)
│   ├── services.rs           `/api/services` health + metrics endpoints
│   ├── status.rs             `/api/health`, `/api/status`, metrics snapshots
│   ├── strategies/           strategies API (`/api/strategies/*`)
│   ├── system.rs             system endpoints (`/api/system/*`)
│   ├── telegram.rs           Telegram integration (`/api/telegram/*`)
│   ├── tokens/               tokens API (`/api/tokens*`)
│   ├── tools/                tools API (`/api/tools/*`)
│   ├── trader/               auto trader API (`/api/trader/*`)
│   ├── trading.rs            trading operations (`/api/trading/*`)
│   ├── transactions.rs       transactions API (`/api/transactions/*`)
│   ├── ui_state.rs           UI state sync (merged under `/api/*`)
│   ├── updates.rs            update checker API (`/api/updates*`)
│   ├── wallet.rs             active wallet endpoints (merged under `/api/*`)
│   └── wallets/              wallets API (`/api/wallets/*`)
├── templates/                HTML templates / partials (embedded)
├── templates.rs              HTML rendering + injection (token/port/asset version)
├── embeds.rs                 `include_str!`/`include_bytes!` for all embedded assets
├── utils.rs                  `success_response` / `error_response` helpers
├── state.rs                  `AppState` + global OnceLock accessor
├── session.rs                headless session token store (in-memory)
├── totp.rs                   TOTP utilities (secret generation + verify + QR)
├── snapshot/                 status snapshot collectors + caching
└── demo.rs / demo_data.rs    dashboard demo-mode fake data (screenshots/marketing)
```

Service integration entrypoint (outside this module):

- `src/services/implementations/webserver_service.rs` (`WebserverService`)

---

## 3. Service Integration & Lifecycle

ScreenerBot runs the HTTP server as a `ServiceManager` service.

**File:** `src/services/implementations/webserver_service.rs`

Key behaviors:

- `name() == "webserver"`, `priority() == 30`
- `dependencies() == []` and `is_enabled() == true`
  - This is intentional: the dashboard must be reachable **before** initialization completes so the user can finish setup.

### 3.1 Startup flow

Call graph:

1. `WebserverService::start(...)`
2. `webserver::test_port_binding(port_override, host_override)`
   - **Headless mode**: bind check to fail fast if the address is unavailable.
   - **GUI mode**: skipped (GUI uses dynamic port selection).
3. Spawn background task:
   - `webserver::start_server(port_override, host_override)`

### 3.2 Shutdown flow (current behavior)

ServiceManager shutdown mechanism:

- `ServiceManager::stop_all()` calls `self.shutdown.notify_waiters()` and then awaits each service task `JoinHandle` with a 10s timeout.

Webserver specifics:

- The Axum server's graceful shutdown is driven by an internal notifier:
  - `src/webserver/server.rs` → `static SHUTDOWN_NOTIFY: LazyLock<Arc<Notify>>`
  - `webserver::shutdown()` calls `SHUTDOWN_NOTIFY.notify_one()`

**Important:** `Service::stop()` is optional and defaults to a no-op; `WebserverService` currently uses that default (it does not override `stop()` to call `webserver::shutdown()`).

The Axum server task waits on the webserver-local `SHUTDOWN_NOTIFY` (triggered via `webserver::shutdown()`), and it is not currently wired to the `ServiceManager` shutdown `Notify` passed into `Service::start(...)`. As a result, the server task may not exit during `ServiceManager::stop_all()` and is typically terminated when the process / Tokio runtime shuts down.

---

## 4. Server Startup (`start_server`) + Mode Selection

**File:** `src/webserver/server.rs`

Main entrypoint:

```rust
pub async fn start_server(
    port_override: Option<u16>,
    host_override: Option<String>,
) -> Result<(), String>
```

### 4.1 Headless vs GUI mode decision

Mode is decided by:

- `let is_gui = global::is_gui_mode();`

If `is_gui == true`:

- The server:
  - chooses a random available port in the dynamic range `49152..=65535`
  - binds to `127.0.0.1` only
  - generates a 64-char security token
  - prints `SCREENERBOT_READY:{port}:{token}` to stdout (Electron parses this)
  - enables the `security_gate` middleware (token header validation)

If `is_gui == false` (CLI/headless):

- Effective bind address uses precedence:
  - **CLI overrides** (`--port`, `--host`) if present
  - else config (`cfg.webserver.port`, `cfg.webserver.host`)
  - else defaults (`127.0.0.1:8080`)
- No GUI token is required; optional password/session auth can be enabled by config (see §8).

### 4.2 Dynamic port selection (GUI)

`find_available_port()`:

- Generates 100 random candidates within the dynamic range
- For each candidate:
  - attempts `TcpListener::bind("127.0.0.1:{port}")`
  - if successful, immediately drops the listener and returns the port

### 4.3 App state construction

`start_server` constructs shared state:

- If `cfg.ai.enabled` is true:
  - it attempts `crate::ai::try_get_ai_engine()`
  - stores it in `AppState { ai_engine: Option<Arc<AiEngine>> }`

Then it stores a global copy for snapshot collectors:

- `crate::webserver::state::set_global_app_state(Arc<AppState>)`

### 4.4 Router build + serve

Steps:

1. Build router: `routes::create_router(state)`
2. Apply middleware layers in `build_app(...)`
3. Bind listener: `TcpListener::bind(&addr).await`
4. Serve:
   ```rust
   axum::serve(listener, app)
       .with_graceful_shutdown(async { SHUTDOWN_NOTIFY.notified().await; })
       .await
   ```

---

## 5. Router Composition (Pages, Assets, `/api`)

**File:** `src/webserver/routes/mod.rs`

### 5.1 Top-level router: HTML pages + assets + API nesting

`routes::create_router(state)` creates a router that serves:

- **Full HTML pages** (dashboard shell):
  - `/`, `/home`, `/tokens`, `/positions`, `/filtering`, ...
- **Login page** (headless auth):
  - `GET /login`
- **Embedded JS modules**:
  - `GET /scripts/core/:file`
  - `GET /scripts/pages/*file`
  - `GET /scripts/ui/*file`
- **Embedded static assets**:
  - `GET /assets/:file`
  - `GET /assets/fonts/:file`
  - `GET /assets/providers/:file`
- **API**:
  - `nest("/api", api_routes())`

The router is strongly typed:

- All handlers run under `Router<Arc<AppState>>`
- `create_router` ends with `.with_state(state)`

### 5.2 API router composition

`api_routes()` builds the `/api` subtree by combining feature routers:

- some are `merge(...)`'d (no extra path prefix)
- some are `nest("/prefix", ...)`'d (path-scoped group)

Example structure:

```rust
Router::new()
  .merge(status::routes())              // /api/health, /api/status, ...
  .merge(tokens::routes())              // /api/tokens/...
  .nest("/transactions", transactions::routes()) // /api/transactions/...
  .nest("/strategies", strategies::routes())     // /api/strategies/...
  .nest("/auth", auth::routes())        // /api/auth/...
  // ...
```

The strategies API includes backend-filtered list queries (`GET /api/strategies?type=ENTRY|EXIT`) and a dedicated enabled-state mutation (`PATCH /api/strategies/:id/enabled`). Dashboard controls should use that mutation instead of reconstructing full strategy records in JavaScript.

### 5.3 SPA partial-page endpoint: `/api/pages/:page`

Route:

- `GET /api/pages/:page`

Behavior:

- Returns just the page's HTML content (not the full base template).
- Used by the frontend SPA router to swap page content without a full reload.

This route is explicitly whitelisted by `security_gate` and `initialization_gate` because it is required during initial page load and navigation.

---

## 6. Middleware Stack

**Files:**

- `src/webserver/server.rs` (`build_app`)
- `src/webserver/middleware.rs`

The router is wrapped with multiple middleware layers.

### 6.1 Layer ordering (critical)

In Axum, `.layer(...)` stacks are applied **in reverse order** for request handling (the last added runs first).

Current stack:

```rust
app
  .layer(from_fn(cache_control))        // innermost (runs last on response)
  .layer(from_fn(initialization_gate))
  .layer(from_fn(auth_gate))
  .layer(from_fn(security_gate))
  .layer(CompressionLayer::new());      // outermost (runs first)
```

Effective request flow:

1. `CompressionLayer`
2. `security_gate` (GUI only; no-op in headless)
3. `auth_gate` (headless only; no-op in GUI)
4. `initialization_gate`
5. handler
6. `cache_control` adds response headers
7. response is compressed (if applicable)

### 6.2 `cache_control` (always-on)

Adds cache-control headers with path-specific strategies:

**Static assets** (`/scripts/*`, `/assets/*`, `/fonts/*`):
- `Cache-Control: public, max-age=31536000, immutable`
- Aggressive caching safe due to `{{ASSET_VERSION}}` query param

**API endpoints** (`/api/*`):
- `Cache-Control: no-cache, no-store, must-revalidate`
- `Pragma: no-cache`
- `Expires: 0`
- Prevents caching of dynamic trading data

**HTML pages** (all other paths):
- `Cache-Control: no-cache`
- Allows browser caching with revalidation

See §11.2 for detailed caching strategy rationale.

### 6.3 `initialization_gate` (pre-init API blocking)

Purpose:

- Prevents most `/api/*` endpoints from being used before the bot is initialized.

Rules:

- If `global::is_initialization_complete() == true` or `global::is_preview_mode() == true` → allow
  all requests. Preview is a usable dashboard mode; wallet/trading handlers retain their deeper
  readiness and force-stop guards.
- If not initialized:
  - allow:
    - `/api/initialization/*`
    - `/api/system/bootstrap`
    - `/api/health`
    - `/api/version`
    - `/api/actions*` (actions system works independently)
    - static resources and HTML pages:
      - `/scripts/*`
      - `/styles/*`
      - `/api/pages/*`
      - `/` and all non-`/api/` routes
  - block everything else under `/api/*` with:
    - `503 SERVICE_UNAVAILABLE`
    - `{"error": { "code": "INITIALIZATION_REQUIRED", ... }}`

### 6.4 `security_gate` (GUI-only token enforcement)

Purpose:

- In GUI mode, ensures only the embedded dashboard (which has the token) can call API endpoints.

Rules:

- If not in GUI mode → middleware is a no-op.
- In GUI mode:
  - allow without token:
    - `/`
    - `/api/health`
    - `/assets/*`, `/scripts/*`, `/styles/*`
    - `/api/pages/*`
    - `/api/initialization*`
    - `/api/system/bootstrap*`
    - `/api/actions*`
    - `/api/services*`
    - any path ending with `/stream` (SSE; `EventSource` cannot send custom headers)
    - any non-`/api/` path (HTML pages)
  - otherwise require header:
    - `X-ScreenerBot-Token: <token>`
  - invalid/missing token returns `403 FORBIDDEN` with structured JSON error.

### 6.5 `auth_gate` (headless-only session enforcement)

Purpose:

- In headless mode, optionally require a login session cookie for dashboard access.

Rules:

- If GUI mode → no-op.
- If `cfg.webserver.auth_enabled == false` → no-op.
- Allow without auth:
  - `GET /login`
  - `/api/auth/*`
  - `/scripts/*`, `/styles/*`, `/assets/*`
- For other routes:
  - If cookie `screenerbot_session=<token>` is present and `session::validate_session(token)` is true → allow.
  - Otherwise:
    - `/api/*` → `401 UNAUTHORIZED` JSON error
    - HTML page → `302 FOUND` redirect to `/login`

---

## 7. GUI Mode Security Token Model

GUI mode is designed to be "local-only + token-gated":

- Bind address is always `127.0.0.1`
- Port is randomly chosen (high, ephemeral)
- Each process launch creates a fresh security token
- The token is injected into the HTML and used by frontend JS for API calls

### 7.1 Token generation + readiness handshake

**File:** `src/webserver/server.rs`

In GUI mode `start_server`:

- generates token: `global::generate_security_token()`
- prints: `SCREENERBOT_READY:{port}:{token}` to stdout
  - Electron parses this to know where to connect

### 7.2 Token injection into HTML

**File:** `src/webserver/templates.rs`

`templates::base_template(...)` injects:

- `{{SECURITY_TOKEN}}` (GUI only)
- `{{WEBSERVER_PORT}}`
- `{{IS_GUI_MODE}}`

The frontend uses this data to configure API calls.

### 7.3 API validation header

**File:** `src/webserver/middleware.rs`

- Header name constant:
  - `SECURITY_TOKEN_HEADER: &str = "X-ScreenerBot-Token"`

### 7.4 First-run and restart lifecycle

An absent `config.toml` is the canonical first-run state. `/api/system/bootstrap` reports
`initialization_required = true` and the router shows onboarding before the wallet/RPC wizard.
Choosing "Skip for now" calls `POST /api/initialization/skip`, persists `setup_skipped = true`,
starts the preview tier, and navigates to token discovery without restarting.

Completing wallet/RPC setup is intentionally different: `POST /api/initialization/complete`
validates credentials, merges them into the loaded config, persists `setup_skipped = false`, and
schedules a graceful process restart. The run loop stops every service and releases the process
lock before replacement. This prevents preview-owned wallet, RPC, and service singletons from being
promoted in place.

Restart ownership depends on the host:

- headless Unix replaces the process after shutdown; Windows relaunches after shutdown;
- GUI mode emits `SCREENERBOT_RESTART` and exits with code 75, then Electron starts a new owned
  backend, reads its new dynamic port/security token, and loads the previous dashboard route;
- ordinary browser clients poll `/api/health` and compare `instance_id`, reloading only after a
  different backend process answers. A response from the old process is never accepted as proof of
  restart.

---

## 8. Headless Authentication (Password, Sessions, TOTP)

Headless auth is optional and configured under `cfg.webserver.*`.

**File:** `src/config/schemas/webserver.rs`

Relevant fields:

- `auth_enabled: bool` (default false)
- `auth_password_hash: String` (hidden)
- `auth_password_salt: String` (hidden)
- `auth_session_timeout_secs: u64` (default 86400, 0 = session cookie)
- `auth_totp_enabled: bool`
- `auth_totp_secret: String` (hidden)
- login page customization:
  - `auth_show_logo`, `auth_show_name`, `auth_custom_title`

### 8.1 Password management

**File:** `src/webserver/routes/auth/password_handlers.rs`

Endpoint:

- `POST /api/auth/set-password`

Behavior:

- If a password already exists, `current_password` must be provided and verified.
- Sending `new_password == ""` clears the password and disables auth.
- Otherwise:
  - validates length (4..=128)
  - generates new salt: `secure_storage::generate_password_salt()`
  - hashes: `secure_storage::hash_password(...)`
  - persists to config via `config::update_config_section(..., true)`
  - clears all active sessions: `session::clear_all_sessions()` (forces re-login)

### 8.2 Session tokens (cookie auth)

**File:** `src/webserver/session.rs`

Session storage:

- Global in-memory `HashMap<String, Session>` protected by `RwLock`.

Token generation:

- 64-char alphanumeric random string (`rand::thread_rng()`).

Expiration:

- `expires_at == 0` means the server will not expire the session automatically (this happens when `auth_session_timeout_secs == 0`).
- `validate_session()` removes expired sessions as a side effect.

Cookie attributes:

**File:** `src/webserver/routes/auth/helpers.rs`

- Cookie name: `screenerbot_session`
- Cookie string includes:
  - `Path=/; HttpOnly; SameSite=Strict`
  - optional `Max-Age=<timeout>` if timeout > 0
  - if `timeout == 0`, `Max-Age` is omitted (most browsers treat this as a session cookie)

### 8.3 Login / logout endpoints

**File:** `src/webserver/routes/auth/session_handlers.rs`

Endpoints:

- `GET /api/auth/status`
- `POST /api/auth/login`
- `POST /api/auth/logout`

Login flow:

1. Validate auth enabled + password configured
2. Verify password via `secure_storage::verify_password`
3. If TOTP enabled:
   - if no code provided: returns `{ success: false, requires_totp: true, ... }`
   - else verify with `totp::verify_totp(...)`
4. Create session + set cookie header

Logout:

- revokes session token (if present) and clears cookie with `Max-Age=0`

### 8.4 TOTP (2FA)

**Files:**

- `src/webserver/totp.rs` (core utilities)
- `src/webserver/routes/auth/totp_handlers.rs` (API endpoints)

Utility properties (current implementation):

- Algorithm: SHA1
- Digits: 6
- Step: 30s
- Skew tolerance: ±1 step (±30s)
- QR code is rendered as SVG and returned as a `data:image/svg+xml;base64,...` URL

Endpoints:

- `GET /api/auth/totp/status`
- `POST /api/auth/totp/setup` (requires password; returns secret + QR; does not persist)
- `POST /api/auth/totp/verify-setup` (verifies code and persists secret + enables)
- `POST /api/auth/totp/disable` (requires password)

---

## 9. Templates & Embedded Assets

ScreenerBot's dashboard frontend is embedded in the Rust binary; the webserver does not read files at runtime.

### 9.1 Embedded assets (`embeds.rs`)

**File:** `src/webserver/embeds.rs`

- HTML: `include_str!("templates/base.html")`
- CSS: many `include_str!("templates/styles/...")`
- JS: many `include_str!("assets/...")`
- Fonts/images: `include_bytes!(...)`

These constants are used by:

- `templates.rs` (HTML assembly + CSS injection)
- `routes/asset_serving.rs` (serving JS/assets with content-type)

### 9.2 Base template injection (`templates.rs`)

**File:** `src/webserver/templates.rs`

`base_template(title, active_tab, content)`:

- Starts from `BASE_TEMPLATE` and replaces placeholders:
  - `{{TITLE}}`
  - `{{NAV_TABS}}`
  - `{{CONTENT}}`
  - `{{SECURITY_TOKEN}}` (GUI mode only)
  - `{{WEBSERVER_PORT}}`
  - `{{IS_GUI_MODE}}`
  - `{{ASSET_VERSION}}` (cache busting)
  - `{{NEEDS_INITIALIZATION}}` (prevents dashboard flash)
- Injects overlay screens:
  - splash, onboarding, setup, lockscreen
- Inlines CSS:
  - `/*__INJECTED_STYLES__*/` is replaced with a concatenated CSS bundle
  - includes Lucide icon CSS with rewritten font URLs (`/assets/fonts/...`)

Rationale:

- Inline CSS avoids "Flash of Unstyled Content" in WebView and makes page transitions deterministic.
- Stacked dialog dismissal is an idempotent state transition. The top dialog becomes
  non-interactive before focus restoration, poller disposal, or backend cleanup begins, and its
  close activation cannot leak into an underlying view such as Billboard.

### 9.3 Login template

`login_template(...)` is a minimal page shell:

- no nav tabs
- smaller CSS bundle (`FOUNDATION_STYLES` + `LOGIN_PAGE_STYLES`)
- loads `"/scripts/pages/login.js?v={asset_version}"`

### 9.4 Asset serving routes

**File:** `src/webserver/routes/asset_serving.rs`

Routes map requested filenames to embedded constants and return content with correct types:

- JS:
  - `/scripts/core/:file`
  - `/scripts/pages/*file`
  - `/scripts/ui/*file`
- Assets:
  - `/assets/:file` (logo, chart lib, ...)
  - `/assets/fonts/:file` (Lucide + others)
  - `/assets/providers/:file` (provider logos)

---

## 10. Status Snapshot + Metrics Caching

**Files:**

- `src/webserver/routes/status.rs`
- `src/webserver/snapshot/*`

`GET /api/health` is deliberately small but includes a stable per-process `instance_id` in addition
to status, time, and version. Restarting clients use it as an identity handshake.

High-level design:

- `/api/status` is not a "single module" response.
- It aggregates live data from multiple subsystems (services, wallet monitor, RPC stats, pools, discovery, ...).

### 10.1 Snapshot aggregator

`gather_status_snapshot()`:

- uses `tokio::join!` to collect independent data concurrently:
  - position counts
  - wallet snapshot
  - OHLCV stats
  - pools/discovery summary
  - events/transactions status
  - dexscreener/geckoterminal status (if enabled)
  - system metrics and RPC metrics summary

### 10.2 Caching expensive system metrics

System metrics are cached:

- cache TTL: 5 seconds (`SYSTEM_METRICS_CACHE_SECS`)
- avoids expensive sysinfo calls on every request

Uptime source:

- `AppState::uptime_seconds()` (based on `startup_time`)
- retrieved via `state::get_app_state()` (OnceLock)

---

## 11. Performance Optimizations

The webserver is designed for low-latency responses across all endpoints, with careful attention to caching, concurrency, and database query patterns.

### 11.1 Endpoint Optimization Patterns

#### High-Performance Status Endpoints

**`/api/status/services`** — Service health snapshot
- Direct call to `collect_service_status_snapshot()` (no aggregation)
- **~3ms response time**
- Returns only service manager state (name, status, uptime, error count)

**`/api/status/metrics`** — Cached system metrics
- Direct call to `get_cached_system_metrics()` 
- **~3ms response time**
- **5-second TTL cache** for expensive `sysinfo` calls
- Includes CPU, memory, disk, network, uptime

**`/api/status`** — Full system snapshot
- Aggregated call to `gather_status_snapshot()`
- **~168ms response time**
- **9-way parallel collection** via `tokio::join!`:
  - Position counts
  - Wallet snapshot
  - OHLCV stats
  - Pools/discovery summary
  - Events/transactions status
  - DexScreener status (if enabled)
  - GeckoTerminal status (if enabled)
  - System metrics (cached)
  - RPC metrics summary
- Use this for comprehensive dashboard views; use specialized endpoints for frequent polling

#### Trading Statistics Endpoints

**`/api/dashboard/overview`** — Period trading stats
- SQL aggregation via `get_period_trading_stats(since_timestamp)`
- **~4ms response time**
- Calculates P&L, win rate, trade count directly in SQLite
- **Reference pattern**: Never load all closed positions into memory for stats

**`/api/trader/stats`** — Filtered trader statistics
- SQL time-bounded query via `get_closed_positions_since(DateTime)`
- **~3ms response time**
- Uses indexed `closed_at` column for fast filtering
- Returns only positions matching criteria (e.g., last 24h, last 7d)

#### Position Detail Endpoint

**`/api/positions/{key}`** — Individual position details
- **4 parallel async calls** via `tokio::join!`:
  1. `positions::db::get_position(&key)`
  2. `tokens::db::get_token_info(&mint)`
  3. RPC account fetch (if position open)
  4. Price quote (if position open)
- Parallelizes independent I/O operations
- Minimizes total latency by running queries concurrently

**`/api/positions/{key}/activity`** — All-time activity for the token
- Fetched lazily only while the Position Details Activity tab is open
- Merges every position round, entry/exit records, pending operations, state changes,
  on-chain transaction details and unclaimed wallet-only transactions for the mint
- Returns chronological events with server-derived running cost basis and realized P&L
- The frontend groups the history by trading round, keeps each lifecycle chronological,
  and places wallet-only activity in a separate expandable chapter

#### Transaction Collection

**Transaction collector service** — Background data aggregation
- **5 parallel database queries** via `tokio::join!`:
  1. Recent transactions
  2. Transaction count by type
  3. Failed transaction count
  4. Pending transaction status
  5. Recent error summary
- Reduces serial query overhead in background tasks
- Pattern applicable to any multi-table aggregation

### 11.2 Caching Strategy

Response caching is carefully tuned based on content mutability and client requirements.

#### Static Assets (Immutable)

**Routes**: `/scripts/*`, `/assets/*`, `/fonts/*`

Headers:
```
Cache-Control: public, max-age=31536000, immutable
```

Rationale:
- All assets are embedded at compile time with `{{ASSET_VERSION}}` query param
- Version changes on every build → safe to cache forever
- `immutable` directive tells browsers the content will never change
- Eliminates revalidation requests for embedded JS/CSS/fonts

#### API Endpoints (Dynamic)

**Routes**: `/api/*`

Headers:
```
Cache-Control: no-cache, no-store, must-revalidate
Pragma: no-cache
Expires: 0
```

Rationale:
- Trading data changes frequently (positions, balances, metrics)
- No proxy caching allowed (`no-store`)
- Clients must revalidate on every request (`must-revalidate`)
- Prevents stale data in dashboard views

#### HTML Pages (Conditional)

**Routes**: `/`, `/tokens`, `/positions`, etc.

Headers:
```
Cache-Control: no-cache
```

Rationale:
- Pages must check for updates (initialization state, auth state)
- No `no-store` → browsers can cache but must revalidate
- Allows conditional requests (`If-None-Match`, `If-Modified-Since`)
- Balance between performance and freshness

### 11.3 Database Query Patterns

#### SQL Aggregation (Preferred)

Always compute statistics in SQL when possible:

```rust
// ✅ GOOD: SQL aggregation
let stats = positions::db::get_period_trading_stats(since_timestamp)?;
// Returns: { total_pnl, win_rate, trade_count, ... }
```

```rust
// ❌ BAD: In-memory aggregation
let all_positions = positions::db::get_all_closed_positions()?;
let filtered: Vec<_> = all_positions.iter()
    .filter(|p| p.closed_at >= since_timestamp)
    .collect();
let total_pnl: f64 = filtered.iter().map(|p| p.pnl).sum();
```

Benefits:
- **10-100x faster** for large datasets (1000+ positions)
- Constant memory usage (no Vec allocation)
- Leverages SQLite indexes and query planner
- Network-efficient (single result row vs. thousands)

#### Time-Bounded Queries

Use indexed timestamp columns for historical queries:

```rust
// Query only relevant time range
let positions = positions::db::get_closed_positions_since(
    Utc::now() - Duration::hours(24)
)?;
```

Index design:
```sql
CREATE INDEX idx_positions_closed_at ON positions(closed_at);
```

#### Parallel Query Execution

For independent queries, use `tokio::join!`:

```rust
let (positions, tokens, stats, events) = tokio::join!(
    get_recent_positions(),
    get_token_summary(),
    get_trading_stats(),
    get_recent_events(),
);
```

**Do not use `join!` for:**
- Serial dependencies (query B needs result from query A)
- Shared lock contention (multiple writes to same DB)
- Very fast queries (<1ms) where spawn overhead dominates

### 11.4 Middleware Performance

The middleware stack is ordered for minimal overhead on successful requests:

1. **CompressionLayer** (outermost)
   - Runs last on response path
   - Only compresses if response is large and client accepts encoding

2. **security_gate** (GUI mode)
   - Simple string comparison (`X-ScreenerBot-Token`)
   - Whitelisted paths skip validation entirely
   - ~0.1ms overhead

3. **auth_gate** (headless mode)
   - HashMap lookup in RwLock (`session::validate_session`)
   - Lazy expiration pruning (no background task)
   - ~0.2ms overhead

4. **initialization_gate**
   - Atomic boolean check (`is_initialization_complete()`)
   - Path prefix matching (compile-time constants)
   - ~0.05ms overhead

5. **cache_control**
   - String formatting on response path only
   - No request inspection
   - ~0.01ms overhead

Total middleware overhead: **~0.4ms** per request (typical case)

### 11.5 Concurrency Patterns

#### Snapshot Aggregation

The `gather_status_snapshot()` function demonstrates the canonical pattern:

```rust
let (
    position_snapshot,
    wallet_snapshot,
    ohlcv_snapshot,
    pools_snapshot,
    events_snapshot,
    dexscreener_snapshot,
    geckoterminal_snapshot,
    system_metrics,
    rpc_metrics,
) = tokio::join!(
    collect_position_snapshot(),
    collect_wallet_snapshot(),
    collect_ohlcv_snapshot(),
    collect_pools_snapshot(),
    collect_events_snapshot(),
    collect_dexscreener_snapshot(),
    collect_geckoterminal_snapshot(),
    get_cached_system_metrics(),
    collect_rpc_metrics(),
);
```

Key benefits:
- All collectors run concurrently
- Total latency = max(individual latencies), not sum
- No manual spawn/await coordination
- Errors are collected and handled uniformly

#### Service Status Collection

Services are queried in parallel:

```rust
for service in services {
    tasks.push(tokio::spawn(async move {
        service.get_health_snapshot().await
    }));
}
let results = futures::future::join_all(tasks).await;
```

This pattern is used when:
- Number of parallel tasks is dynamic (not known at compile time)
- Each task is independent and returns the same type
- Order of results matters (preserved by `join_all`)

### 11.6 Common Anti-Patterns to Avoid

**❌ Loading entire tables for filtering:**
```rust
let all = db.get_all_positions()?;
let recent = all.iter().filter(|p| p.closed_at > threshold).collect();
```

**✅ Use SQL WHERE clause:**
```rust
let recent = db.get_closed_positions_since(threshold)?;
```

---

**❌ Serial independent queries:**
```rust
let positions = get_positions().await?;
let tokens = get_tokens().await?;
let stats = get_stats().await?;
```

**✅ Parallel with tokio::join!:**
```rust
let (positions, tokens, stats) = tokio::join!(
    get_positions(),
    get_tokens(),
    get_stats(),
);
```

---

**❌ Recomputing expensive metrics per request:**
```rust
async fn metrics_handler() -> Response {
    let sys = System::new_all(); // expensive!
    let cpu = sys.global_cpu_info().cpu_usage();
    // ...
}
```

**✅ Use cached metrics with TTL:**
```rust
async fn metrics_handler() -> Response {
    let metrics = get_cached_system_metrics(); // 5s TTL
    // ...
}
```

---

**❌ Blocking operations in async handlers:**
```rust
async fn handler() -> Response {
    let data = std::fs::read_to_string("data.json")?; // blocks executor!
    // ...
}
```

**✅ Use async I/O or spawn_blocking:**
```rust
async fn handler() -> Response {
    let data = tokio::fs::read_to_string("data.json").await?;
    // or: tokio::task::spawn_blocking(|| std::fs::read_to_string("data.json")).await??
    // ...
}
```

---

## 12. Streaming (SSE)

The webserver uses Server-Sent Events for real-time UI updates where WebSockets are unnecessary.

### 12.1 Actions stream

**File:** `src/webserver/routes/actions.rs`

Endpoint:

- `GET /api/actions/stream`

Implementation:

- Subscribes to `crate::actions::subscribe()` (Tokio broadcast channel).
- Converts each update to JSON and yields as `axum::response::sse::Event`.
- Uses keep-alive:
  - every 15 seconds sends `"keepalive"`

Security interaction:

- `security_gate` explicitly exempts all paths ending in `/stream` because `EventSource` cannot send custom headers.
- In headless auth mode, `EventSource` does send cookies, so `auth_gate` can still enforce login.

---

## 13. Module Connections & Extension Points

### 13.1 Major dependencies (server-side)

The webserver primarily orchestrates other modules:

- `config` — reads and persists configuration, provides UI metadata
- `global` — GUI/headless mode flags, initialization state, GUI security token
- `services` — health/metrics snapshots and service status API
- `events`, `actions` — event log and action stream for UI
- `tokens`, `filtering`, `pools`, `ohlcvs`, `positions`, `trader`, `transactions` — feature APIs
- `rpc` — RPC stats snapshot (global counters)
- `ai` — mode-independent instruction/chat persistence plus optional AI engines wired into `AppState`

### 13.2 Adding a new endpoint / feature router

Pattern:

1. Create a route module under `src/webserver/routes/<feature>.rs` (or folder):
   - expose `pub fn routes() -> Router<Arc<AppState>>`
2. Register it in `api_routes()` via `merge(...)` or `nest("/prefix", ...)`.
3. If the endpoint must work pre-initialization, ensure it is permitted by:
   - `initialization_gate` (and possibly `security_gate` if GUI mode).
4. If the endpoint is used by the GUI frontend, ensure requests include `X-ScreenerBot-Token` (handled by the frontend request manager).

### 13.3 Where to look when debugging

- Server bind/port issues: `src/webserver/server.rs` + `WebserverService::start` pre-flight logs
- `403 missing/invalid token` (GUI): `middleware::security_gate` whitelist + frontend header injection
- `503 initialization required`: `middleware::initialization_gate`; it blocks only when neither
  `global::is_initialization_complete()` nor `global::is_preview_mode()` is true
- chat session `503 CHAT_DB_NOT_INITIALIZED`: `run/bootstrap.rs` must initialize dashboard
  persistence before the webserver starts in every boot mode
- `302 to /login` or `401 auth required` (headless): `middleware::auth_gate` + `/api/auth/*`
