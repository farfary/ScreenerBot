# Webserver Module — Architecture

> ScreenerBot Dashboard REST API, Authentication & Asset Serving — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Server Setup](#3-server-setup)
4. [Authentication & Middleware](#4-authentication--middleware)
5. [API Routes](#5-api-routes)
6. [Template & Asset System](#6-template--asset-system)
7. [SSE Streaming](#7-sse-streaming)
8. [Service Lifecycle](#8-service-lifecycle)
9. [Module Connections](#9-module-connections)

---

## 1. Overview

The Webserver module serves the ScreenerBot dashboard — a REST API with embedded HTML/CSS/JS frontend. Built on **Axum** (async Rust web framework), it provides 200+ API endpoints across 28 route modules, authentication (token-based for GUI, session-based for headless), and compile-time embedded assets.

**Key characteristics:**
- Axum framework with Tower middleware stack
- Two auth modes: GUI (random token) and headless (session + password + optional TOTP 2FA)
- All assets embedded at compile time (no external file serving)
- Dynamic port selection (GUI) or configurable port (headless)
- SSE support for real-time streaming
- Initialization gate blocks endpoints until setup complete

**109 files, ~28,766 lines**

---

## 2. File Structure

```
src/webserver/
├── mod.rs               # Module declarations
├── server.rs            # Server lifecycle (startup, shutdown, port binding)
├── middleware.rs         # Security gate, auth gate, init gate, cache control
├── state.rs             # AppState (shared state for handlers)
├── session.rs           # Session management (cookies)
├── totp.rs              # TOTP 2FA
├── templates.rs         # HTML page rendering
├── utils.rs             # Response helpers
├── embeds.rs            # Embedded CSS/JS/fonts/images
├── demo.rs              # Demo mode
├── demo_data.rs         # Demo data generators
├── snapshot/            # Status snapshot collection
├── routes/              # 28 route modules (see API Routes section)
│   ├── api/
│   │   ├── status.rs, tokens/, positions/, dashboard/, events.rs
│   │   ├── wallet.rs, wallets/, config/, services.rs, ohlcv.rs
│   │   ├── actions.rs, header.rs, ui_state.rs, billboard.rs
│   │   ├── connectivity/, features/, initialization/, trading/
│   │   ├── trader/, system/, transactions/, strategies/
│   │   ├── tools/, filtering/, lockscreen/, auth/
│   │   ├── telegram/, ai/, updates.rs
│   │   └── ...
│   └── pages/           # HTML page routes
├── templates/           # HTML template functions
└── assets/              # Frontend JS, CSS, fonts (embedded)
```

---

## 3. Server Setup

### Port Binding

| Mode | Host | Port | Selection |
|------|------|------|-----------|
| GUI (Electron) | `127.0.0.1` only | 49152-65535 | Random, tries 100 ports |
| Headless/CLI | Config (default `127.0.0.1`) | Config (default `8080`) | Fixed |

**Startup signal:** Prints `SCREENERBOT_READY:port:token` to stdout for Electron to parse.

### AppState

```rust
pub struct AppState {
    pub startup_time: DateTime<Utc>,
    pub ai_engine: Option<Arc<AiEngine>>,
}
```

---

## 4. Authentication & Middleware

### Middleware Stack (applied in reverse order)

| Layer | Purpose | Mode |
|-------|---------|------|
| CompressionLayer | gzip compression | Both |
| `security_gate` | X-ScreenerBot-Token header validation | GUI only |
| `auth_gate` | Session cookie validation | Headless only |
| `initialization_gate` | Block endpoints until init complete | Both |
| `cache_control` | No-cache headers (prevents Electron caching) | Both |

### GUI Mode Security

- 64-char random alphanumeric token generated at startup
- Required as `X-ScreenerBot-Token` header on every request
- Only localhost binding — no remote access

### Headless Mode Security

- Password-based login → session cookie
- Optional TOTP 2FA
- Can bind to `0.0.0.0` for remote access
- SSE endpoints (`/stream`) exempt from token requirement

---

## 5. API Routes

### Core Status & System

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/health` | Health check |
| GET | `/api/status` | Full system status |
| GET | `/api/status/services` | Service health |
| GET | `/api/status/metrics` | System metrics |
| GET | `/api/system/bootstrap` | Bootstrap data |
| POST | `/api/system/reboot` | Reboot system |
| GET | `/api/system/paths` | System paths |
| GET | `/api/system/data-stats` | Database stats |

### Tokens (~20 endpoints)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/tokens/list` | List all tokens |
| GET | `/api/tokens/stats` | Token statistics |
| POST | `/api/tokens/filter` | Filter tokens |
| GET | `/api/tokens/search` | Search tokens |
| GET/POST/PATCH/DELETE | `/api/tokens/favorites[/:mint]` | Favorites CRUD |
| GET | `/api/tokens/:mint` | Token detail |
| GET | `/api/tokens/:mint/analysis` | AI analysis |
| GET | `/api/tokens/:mint/ohlcv` | OHLCV candles |
| POST | `/api/tokens/:mint/blacklist` | Blacklist token |
| GET | `/api/tokens/:mint/transactions` | Token transactions |

### Positions

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/positions` | List open positions |
| GET | `/api/positions/stats` | Position stats |
| GET | `/api/positions/:key/details` | Position detail |

### Trader (~20 endpoints)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/trader/status` | Trader status |
| POST | `/api/trader/start` | Start trader |
| POST | `/api/trader/stop` | Stop trader |
| POST | `/api/trader/manual/buy` | Manual buy |
| POST | `/api/trader/manual/sell` | Manual sell |
| POST | `/api/trader/manual/add` | Manual DCA |
| POST | `/api/trader/force-stop` | Emergency stop |
| GET | `/api/trader/monitors/status` | Monitor status |
| GET | `/api/trader/loss-limit/status` | Loss limit status |

### Wallets (~15 endpoints)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/wallets` | List wallets |
| POST | `/api/wallets` | Create wallet |
| POST | `/api/wallets/import` | Import wallet |
| GET | `/api/wallets/summary` | Summary |
| GET | `/api/wallets/main` | Main wallet |
| GET/PUT/DELETE | `/api/wallets/:id` | Wallet CRUD |
| POST | `/api/wallets/:id/set-main` | Set as main |

### Config (~30 endpoints)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/config` | Full config |
| GET | `/api/config/{section}` | Section config (rpc, trader, positions, etc.) |
| PATCH | `/api/config/*` | Update any section |
| POST | `/api/config/export` | Export config |
| POST | `/api/config/import` | Import config |
| POST | `/api/config/reload` | Hot reload |
| POST | `/api/config/reset` | Reset defaults |

### Strategies (~12 endpoints)

| Method | Path | Purpose |
|--------|------|---------|
| GET/POST | `/api/strategies` | List/create |
| GET/PUT/DELETE | `/api/strategies/:id` | CRUD |
| GET | `/api/strategies/:id/performance` | Performance stats |
| GET | `/api/strategies/conditions/schemas` | Condition schemas |
| GET | `/api/strategies/templates` | Templates |

### Tools (~25 endpoints)

ATA scanning/cleanup, token burning, keypair generation, pool search, watched tokens, multi-buy/sell operations, wallet consolidation.

### AI/Assistant (~40 endpoints)

Provider management, chat sessions, automation tasks, tool permissions, Assistant OAuth, instructions management.

### Auth

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/auth/login` | Login |
| POST | `/api/auth/logout` | Logout |
| GET | `/api/auth/status` | Auth status |
| POST | `/api/auth/set-password` | Set password |
| POST | `/api/auth/totp/setup` | Setup 2FA |

### Other Routes

Events (cursor-based pagination), filtering stats, connectivity status, initialization flow, dashboard overview, transactions list, services status, telegram config, updates.

### HTML Pages

```
GET  /                   # Home
GET  /home, /tokens, /positions, /events, /services
GET  /transactions, /filtering, /wallets, /tools
GET  /ai, /config, /strategies, /trader
GET  /initialization, /updates, /about, /login
```

---

## 6. Template & Asset System

### Embedded Assets (`embeds.rs`)

All assets compiled into the binary — no runtime file serving.

| Asset Type | Route | Content |
|-----------|-------|---------|
| Core JS | `/scripts/core/:file` | Framework JavaScript |
| Page JS | `/scripts/pages/*file` | Page-specific scripts |
| UI JS | `/scripts/ui/*file` | UI component scripts |
| Static | `/assets/:file` | Images, misc files |
| Fonts | `/assets/fonts/:file` | Lucide icons, custom fonts |
| Providers | `/assets/providers/:file` | DEX/RPC provider logos |

### Template Injection Points

| Placeholder | Value |
|-------------|-------|
| `{{TITLE}}` | Page title |
| `{{NAV_TABS}}` | Navigation HTML |
| `{{CONTENT}}` | Page body content |
| `{{SECURITY_TOKEN}}` | GUI security token |
| `{{WEBSERVER_PORT}}` | Port number |
| `{{IS_GUI_MODE}}` | Mode flag |
| `{{ASSET_VERSION}}` | Cache busting |

---

## 7. SSE Streaming

Server-Sent Events for real-time data:
- Security gate allows `/stream` endpoints without token header (EventSource API limitation)
- Pattern: `/api/*/stream` endpoints

---

## 8. Service Lifecycle

### Startup

```
start_server(port_override, host_override)
  ├─ Determine port/host
  ├─ Generate security token (GUI)
  ├─ Create AppState
  ├─ Build Axum router (all routes + middleware)
  ├─ Create TCP listener
  ├─ Print SCREENERBOT_READY:port:token
  └─ Serve with graceful shutdown
```

### Shutdown

`shutdown()` triggers `SHUTDOWN_NOTIFY` → server exits gracefully.

---

## 9. Module Connections

The webserver routes delegate to all other modules:

```
webserver/routes/ → tokens, positions, pools, trader, strategies,
                    wallets, config, services, events, actions,
                    ohlcvs, filtering, connectivity, ai, telegram
```

Every route handler calls the corresponding module's public API and returns JSON responses.
