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
11. [Streaming (SSE)](#11-streaming-sse)
12. [Module Connections & Extension Points](#12-module-connections--extension-points)

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
  .nest("/auth", auth::routes())        // /api/auth/...
  // ...
```

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

Adds headers to prevent aggressive caching (especially in embedded WebView):

- `Cache-Control: no-cache, no-store, must-revalidate, max-age=0`
- `Pragma: no-cache`
- `Expires: 0`

### 6.3 `initialization_gate` (pre-init API blocking)

Purpose:

- Prevents most `/api/*` endpoints from being used before the bot is initialized.

Rules:

- If `global::is_initialization_complete() == true` → allow all requests.
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

## 11. Streaming (SSE)

The webserver uses Server-Sent Events for real-time UI updates where WebSockets are unnecessary.

### 11.1 Actions stream

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

## 12. Module Connections & Extension Points

### 12.1 Major dependencies (server-side)

The webserver primarily orchestrates other modules:

- `config` — reads and persists configuration, provides UI metadata
- `global` — GUI/headless mode flags, initialization state, GUI security token
- `services` — health/metrics snapshots and service status API
- `events`, `actions` — event log and action stream for UI
- `tokens`, `filtering`, `pools`, `ohlcvs`, `positions`, `trader`, `transactions` — feature APIs
- `rpc` — RPC stats snapshot (global counters)
- `ai` — optional AI engine wired into `AppState`

### 12.2 Adding a new endpoint / feature router

Pattern:

1. Create a route module under `src/webserver/routes/<feature>.rs` (or folder):
   - expose `pub fn routes() -> Router<Arc<AppState>>`
2. Register it in `api_routes()` via `merge(...)` or `nest("/prefix", ...)`.
3. If the endpoint must work pre-initialization, ensure it is permitted by:
   - `initialization_gate` (and possibly `security_gate` if GUI mode).
4. If the endpoint is used by the GUI frontend, ensure requests include `X-ScreenerBot-Token` (handled by the frontend request manager).

### 12.3 Where to look when debugging

- Server bind/port issues: `src/webserver/server.rs` + `WebserverService::start` pre-flight logs
- `403 missing/invalid token` (GUI): `middleware::security_gate` whitelist + frontend header injection
- `503 initialization required`: `middleware::initialization_gate` + `global::is_initialization_complete()`
- `302 to /login` or `401 auth required` (headless): `middleware::auth_gate` + `/api/auth/*`
