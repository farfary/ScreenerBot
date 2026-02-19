# ScreenerBot Documentation Creation Plan

**Created:** November 13, 2025  
**Purpose:** Step-by-step plan for creating comprehensive, accurate user documentation based on codebase analysis  
**Target:** Website docs at `repos/Website/app/docs/`

---

## Plan Overview

This plan ensures every documentation section is:

1. **Research-driven**: Based on actual codebase investigation
2. **User-focused**: Explains HOW to use features, not code implementation
3. **Accurate**: Reflects real system capabilities
4. **Non-technical**: No algorithm details or proprietary information revealed

---

## Public Documentation Guardrails

Use this checklist before publishing any new doc or updating an existing one:

- ✅ **Describe user-facing behavior** (what the dashboard shows, how to configure options, what outcomes to expect).
- ✅ **Stay conceptual** when explaining architecture (pipelines, major subsystems, supported integrations).
- ✅ **Highlight safety practices** (licensing, wallet management, monitoring steps) without exposing internal tooling.
- ⚠️ **Avoid implementation details** such as loop timings, request counts, dependency graphs, database schemas, or internal service names.
- ⚠️ **Do not share proprietary logic** (filter formulas, strategy algorithms, fallback chains, exact heuristics).
- ⚠️ **Never promise specific performance numbers** (seconds, requests per second, storage sizes); describe behavior qualitatively instead.
- ✅ **Explain configuration effects** (“minimum liquidity blocks low-volume tokens”) instead of how the code enforces them.
- ✅ **Reference official UI paths** (Dashboard → Services) so users know where to act without seeing back-end concepts.

If a topic requires deeper technical disclosure, capture it in internal engineering docs (e.g., `/docs/FLOW.md`) instead of the public site.

---

## Documentation Structure

### Phase 1: Foundation (Getting Started)

### Phase 2: Core Usage (Dashboard & Trading)

### Phase 3: Configuration & Advanced

### Phase 4: Reference & Support

---

## PHASE 1: FOUNDATION - Getting Started

### **Doc 1.1: Introduction**

**File:** `docs/introduction.mdx`  
**Research Steps:**

1. Read `FLOW.md` for system overview
2. Read `.github/Assistant-instructions.md` for high-level description
3. Review `src/run.rs` for startup sequence
4. Check `src/services/` for service list
5. Review `src/license/` for licensing system

**Content to Write:**

- What is ScreenerBot? (2-3 paragraphs)
- Key capabilities (bullet list, no technical details)
- System architecture diagram (high-level: Discovery → Filter → Trade → Monitor)
- Who should use it?
- What you need to get started

**Code Investigation:**

```bash
# Commands to run before writing:
rg "ScreenerBot is" -A 5
rg "System Overview" FLOW.md -A 20
rg "pub struct Service" src/services/mod.rs -A 10
```

---

### **Doc 1.2: License Guide**

**File:** `docs/getting-started/license.mdx`  
**Research Steps:**

1. Read existing `repos/Website/app/blog/_content/nft-license-guide.mdx`
2. Review `src/license/mod.rs` for verification flow
3. Check `src/license/types.rs` for LicenseStatus structure
4. Review website license API at `repos/Website/app/api/license/`
5. Check `repos/Website/prisma/schema.prisma` for License model

**Content to Write:**

- NFT licensing explained (reuse existing blog content)
- How to purchase (link to pricing page)
- License verification process (user perspective)
- Wallet setup requirements
- License expiration & renewal
- Transferring licenses
- Troubleshooting license issues

**Code Investigation:**

```bash
# Commands to run:
rg "verify_license" src/license/mod.rs -A 10
rg "LicenseStatus" src/license/types.rs -A 15
cat repos/Website/app/blog/_content/nft-license-guide.mdx
ls repos/Website/app/api/license/
```

---

### **Doc 1.3: Installation Guide**

**File:** `docs/getting-started/installation.mdx`  
**Research Steps:**

1. Check if release binaries exist or build instructions
2. Review `Cargo.toml` for dependencies
3. Check `electron/package.json` for desktop app configuration
4. Review `DESKTOP_APP_GUIDE.md` if exists
5. Look for any install scripts or setup documentation

**Content to Write:**

- System requirements (OS, RAM, disk space)
- Download instructions (from website)
- Installation steps for each OS
- First launch instructions
- Directory structure created
- Verifying installation

**Code Investigation:**

```bash
# Commands to run:
cat electron/package.json | grep -A 5 "identifier\|productName"
cat Cargo.toml | grep -A 2 "\[package\]"
ls -la | grep -i "install\|setup\|desktop"
cat DESKTOP_APP_GUIDE.md
```

---

### **Doc 1.4: Initial Setup**

**File:** `docs/getting-started/setup.mdx`  
**Research Steps:**

1. Review `data/config.toml` structure
2. Read `src/config/schemas.rs` for all config sections
3. Check `src/rpc.rs` for RPC setup requirements
4. Review `src/utils.rs` for wallet loading (`get_wallet_keypair`)
5. Look at startup sequence in `src/run.rs`

**Content to Write:**

- Wallet setup (creating/importing keypair)
- Config file location (`data/config.toml`)
- Essential configuration (wallet, RPC)
- RPC provider selection (link to blog post)
- First-time configuration checklist
- Security recommendations
- Starting the bot for the first time

**Code Investigation:**

```bash
# Commands to run:
head -50 data/config.toml
rg "main_wallet_private" src/config/schemas.rs -B 2 -A 5
rg "get_wallet_keypair\|get_wallet_address" src/utils.rs -A 10
rg "fn main\|register_all_services" src/run.rs -A 20
```

---

### **Doc 1.5: Dashboard Access**

**File:** `docs/getting-started/dashboard.mdx`  
**Research Steps:**

1. Review `src/webserver/server.rs` for port and startup
2. Check `src/webserver/templates/base.html` for layout
3. Review `src/webserver/templates/pages/*.html` for page list
4. Check `src/webserver/routes/mod.rs` for all routes
5. Review `src/config/schemas.rs` for webserver config

**Content to Write:**

- Starting the bot & accessing dashboard
- Default URL (http://localhost:8080)
- Dashboard navigation overview
- Page descriptions (Home, Positions, Tokens, etc.)
- Understanding the interface layout
- First-time dashboard tour

**Code Investigation:**

```bash
# Commands to run:
rg "bind\|listen\|port" src/webserver/server.rs -A 5
ls src/webserver/templates/pages/
rg "Router::new\|route\(" src/webserver/routes/mod.rs -A 30
rg "webserver" src/config/schemas.rs -A 10
```

---

## PHASE 2: CORE USAGE - Dashboard & Trading

### **Doc 2.1: Home Dashboard**

**File:** `docs/dashboard/home.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/home.html`
2. Review `src/webserver/templates/scripts/pages/home.js`
3. Check `src/webserver/routes/dashboard.rs` for API endpoints
4. Review `src/wallet.rs` for wallet snapshot structure
5. Check `src/positions/metrics.rs` for P&L calculations

**Content to Write:**

- Wallet overview section (balance, tokens, daily change)
- Trader analytics section (P&L, win rate, period filters)
- System status indicators
- Understanding metrics
- Interpreting charts/graphs
- Refreshing data

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/home.html
rg "WalletInfo\|PositionsSummary\|SystemInfo" src/webserver/routes/dashboard.rs -A 10
rg "WalletSnapshot\|get_current_wallet_status" src/wallet.rs -A 10
rg "ProceedsMetrics" src/positions/metrics.rs -A 15
```

---

### **Doc 2.2: Positions Management**

**File:** `docs/dashboard/positions.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/positions.html`
2. Review `src/webserver/templates/scripts/pages/positions.js`
3. Check `src/webserver/routes/positions.rs` for position APIs
4. Review `src/positions/types.rs` for Position structure
5. Review `src/positions/operations.rs` for position operations
6. Check `src/trader/manual/orders.rs` for manual trading

**Content to Write:**

- Viewing open positions (entry price, P&L, hold time)
- Position details & real-time updates
- Manual actions (sell, partial sell)
- Closed positions history
- Understanding P&L calculations
- Position metrics & statistics
- DCA entries visualization
- Partial exits tracking

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/positions.html
rg "Position {" src/positions/types.rs -A 30
rg "manual_buy\|manual_sell\|partial_close" src/positions/operations.rs -A 10
rg "EntryRecord\|ExitRecord" src/positions/types.rs -A 15
rg "GET.*position\|POST.*position" src/webserver/routes/positions.rs
```

---

### **Doc 2.3: Tokens Discovery**

**File:** `docs/dashboard/tokens.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/tokens.html`
2. Review `src/webserver/templates/scripts/pages/tokens.js`
3. Check `src/webserver/routes/tokens.rs` for token APIs
4. Review `src/tokens/types.rs` for Token structure
5. Review `src/tokens/database.rs` for data schema
6. Check `src/tokens/discovery.rs` for discovery sources

**Content to Write:**

- Token discovery system overview
- Viewing discovered tokens
- Market data display (price, volume, liquidity, mcap)
- Security information (rugcheck score, risks)
- Data sources (DexScreener, GeckoTerminal, Rugcheck)
- Token details & metadata
- Pool information
- Blacklist management

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/tokens.html
rg "Token {" src/tokens/types.rs -A 25
rg "DexScreenerData\|GeckoTerminalData\|RugcheckData" src/tokens/types.rs -A 20
rg "TokenBlacklistRecord" src/tokens/database.rs -A 10
rg "discovery" src/tokens/discovery.rs -A 15
```

---

### **Doc 2.4: Filtering System**

**File:** `docs/dashboard/filtering.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/filtering.html`
2. Review `src/webserver/templates/scripts/pages/filtering.js`
3. Check `src/webserver/routes/filtering_api.rs`
4. Review `src/filtering/engine.rs` for filtering logic
5. Review `src/filtering/sources/` for all filter sources
6. Check `src/filtering/types.rs` for FilteringSnapshot

**Content to Write:**

- What is token filtering?
- How filtering works (flow diagram)
- Filter sources (DexScreener, GeckoTerminal, Rugcheck)
- Viewing passed tokens
- Viewing rejected tokens with reasons
- Filter statistics
- Understanding filter priority
- Configuring filters (link to config doc)

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/filtering.html
rg "FilteringSnapshot\|PassedToken\|RejectedToken" src/filtering/types.rs -A 15
ls src/filtering/sources/
rg "compute_snapshot" src/filtering/engine.rs -A 20
rg "GET.*filter\|POST.*filter" src/webserver/routes/filtering_api.rs
```

---

### **Doc 2.5: Trader Control**

**File:** `docs/dashboard/trader.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/trader.html`
2. Review `src/webserver/templates/scripts/pages/trader.js`
3. Check `src/webserver/routes/trader.rs` for trader APIs
4. Review `src/trader/controller.rs` for start/stop logic
5. Review `src/trader/auto/` for entry/exit monitors
6. Check `src/config/schemas.rs` for trader config

**Content to Write:**

- Starting/stopping automated trading
- Dry run vs live mode
- Trader status monitoring
- Entry monitor (how it works, what to watch)
- Exit monitor (conditions, active monitoring)
- Manual trading controls
- Trading statistics
- Safety features

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/trader.html
rg "start_trader\|stop_trader\|is_trader_running" src/trader/controller.rs -A 10
ls src/trader/auto/
rg "entry_monitor\|exit_monitor" src/trader/auto/ -A 15
rg "\[trader\]" src/config/schemas.rs -A 30
```

---

### **Doc 2.6: Strategies**

**File:** `docs/dashboard/strategies.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/strategies.html`
2. Review `src/webserver/templates/scripts/pages/strategies.js`
3. Check `src/webserver/routes/strategies.rs` for strategy APIs
4. Review `src/strategies/types.rs` for Strategy structure
5. Review `src/strategies/engine.rs` for evaluation
6. Check `src/strategies/conditions/` for condition types

**Content to Write:**

- What are strategies?
- Entry vs exit strategies
- Creating strategies
- Strategy conditions overview
- Managing strategies (enable/disable/edit)
- Strategy priority
- Strategy evaluation & testing
- Strategy performance tracking

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/strategies.html
rg "Strategy {" src/strategies/types.rs -A 25
rg "StrategyEngine\|evaluate_strategy" src/strategies/engine.rs -A 15
ls src/strategies/conditions/
rg "GET.*strateg\|POST.*strateg" src/webserver/routes/strategies.rs
```

---

### **Doc 2.7: Wallet Monitoring**

**File:** `docs/dashboard/wallet.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/wallet.html`
2. Review `src/webserver/templates/scripts/pages/wallet.js`
3. Check `src/webserver/routes/wallet.rs` for wallet APIs
4. Review `src/wallet.rs` for wallet monitoring system
5. Check database schema in `src/wallet.rs` (wallet_snapshots, token_balances)

**Content to Write:**

- Current balance overview
- Token holdings list
- Balance history & snapshots
- SOL flow analysis (daily income/expenses)
- Charts & visualizations
- Transaction history
- Export functionality
- Understanding balance changes

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/wallet.html
rg "WalletSnapshot\|TokenBalance" src/wallet.rs -A 15
rg "sol_flow\|dashboard_metrics" src/wallet.rs -A 20
rg "SCHEMA_WALLET" src/wallet.rs -A 30
rg "GET.*wallet\|POST.*wallet" src/webserver/routes/wallet.rs
```

---

### **Doc 2.8: Transactions**

**File:** `docs/dashboard/transactions.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/transactions.html`
2. Review `src/webserver/templates/scripts/pages/transactions.js`
3. Check `src/webserver/routes/transactions.rs` for transaction APIs
4. Review `src/transactions/types.rs` for Transaction structure
5. Review `src/transactions/analyzer/` for analysis logic
6. Check `src/transactions/verifier.rs` for verification

**Content to Write:**

- Real-time transaction monitoring
- Transaction types (swap, transfer, ATA)
- Transaction details & analysis
- Swap detection & P&L
- Router detection (Jupiter, GMGN, DEX)
- Transaction verification
- Understanding transaction status
- Filtering & searching transactions

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/transactions.html
rg "Transaction {" src/transactions/types.rs -A 30
rg "TransactionType\|SwapPnLInfo" src/transactions/types.rs -A 15
ls src/transactions/analyzer/
rg "verify_transaction" src/transactions/verifier.rs -A 10
```

---

### **Doc 2.9: Events Logging**

**File:** `docs/dashboard/events.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/events.html`
2. Review `src/webserver/templates/scripts/pages/events.js`
3. Check `src/webserver/routes/events.rs` for event APIs
4. Review `src/events/types.rs` for Event structure
5. Review `src/events/maintenance.rs` for event recording

**Content to Write:**

- What are events?
- Event categories (Swap, Position, Token, System, etc.)
- Severity levels
- Viewing & filtering events
- Searching events
- Understanding event details
- Using events for debugging
- Event retention

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/events.html
rg "Event {" src/events/types.rs -A 20
rg "EventCategory\|Severity" src/events/types.rs -A 15
rg "record_.*_event" src/events/maintenance.rs
rg "GET.*event\|POST.*event" src/webserver/routes/events.rs
```

---

### **Doc 2.10: Configuration UI**

**File:** `docs/dashboard/configuration.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/config.html`
2. Review `src/webserver/templates/scripts/pages/config.js`
3. Check `src/webserver/routes/config.rs` for config APIs
4. Review `src/config/schemas.rs` for ALL config sections
5. Review `src/config/metadata.rs` for UI metadata
6. Check `src/webserver/templates.rs` for CONFIG_METADATA

**Content to Write:**

- Configuration system overview
- Navigating config sections
- Understanding config structure
- Making changes in the UI
- Saving & persisting changes
- Reloading configuration
- Resetting to defaults
- Export/import configuration
- Validation & error handling

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/config.html
rg "CONFIG_METADATA" src/webserver/templates.rs -A 50
rg "config_struct!" src/config/schemas.rs -A 100
rg "GET.*config\|POST.*config" src/webserver/routes/config.rs
rg "reload_config\|save_config" src/config/utils.rs -A 10
```

---

### **Doc 2.11: Services Management**

**File:** `docs/dashboard/services.mdx`  
**Research Steps:**

1. Review `src/webserver/templates/pages/services.html`
2. Review `src/webserver/templates/scripts/pages/services.js`
3. Check `src/webserver/routes/services.rs` for service APIs
4. Review `src/services/mod.rs` for Service trait
5. Review `src/services/implementations/` for all services
6. Check `src/run.rs` for service registration

**Content to Write:**

- What are services?
- Service architecture overview
- Viewing service status
- Service health indicators
- Service metrics & performance
- Service dependencies
- Starting/stopping services
- Service startup order
- Troubleshooting service issues

**Code Investigation:**

```bash
# Commands to run:
cat src/webserver/templates/pages/services.html
rg "Service trait" src/services/mod.rs -A 30
ls src/services/implementations/
rg "register_all_services" src/run.rs -A 100
rg "GET.*service\|POST.*service" src/webserver/routes/services.rs
```

---

## PHASE 3: CONFIGURATION & ADVANCED

### **Doc 3.1: Trader Configuration**

**File:** `docs/configuration/trader.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - trader section
2. Review `src/trader/config.rs` if exists
3. Check `src/trader/auto/` for how settings are used
4. Review `src/trader/execution/` for trade execution

**Content to Write:**

- All trader config fields explained
- Recommended starting values
- Conservative vs aggressive settings
- DCA configuration details
- Exit strategy settings
- Concurrency & performance settings
- Examples for different trading styles

**Code Investigation:**

```bash
# Commands to run:
rg "\[trader\]" data/config.toml -A 30
rg "pub struct TraderConfig" src/config/schemas.rs -A 50
rg "with_config.*trader" src/trader/ -A 5
```

---

### **Doc 3.2: Position Configuration**

**File:** `docs/configuration/positions.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - positions section
2. Check how settings are used in `src/positions/operations.rs`
3. Review `src/trader/exit/` for exit strategy implementation

**Content to Write:**

- Position config fields explained
- Profit calculations & fees
- Partial exit settings
- Trailing stop configuration
- Cooldown settings
- Examples & recommendations

**Code Investigation:**

```bash
# Commands to run:
rg "\[positions\]" data/config.toml -A 15
rg "pub struct PositionsConfig" src/config/schemas.rs -A 30
rg "with_config.*positions" src/positions/ -A 5
```

---

### **Doc 3.3: Filtering Configuration**

**File:** `docs/configuration/filtering.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - filtering section (all subsections)
2. Review `src/filtering/sources/` to understand each filter
3. Check `src/filtering/engine.rs` for how filters are applied

**Content to Write:**

- Complete filtering config reference
- DexScreener filters (liquidity, volume, price changes, etc.)
- GeckoTerminal filters
- Rugcheck security filters
- Understanding thresholds
- Filter combinations & priority
- Examples for different risk levels

**Code Investigation:**

```bash
# Commands to run:
rg "\[filtering" data/config.toml -A 100
rg "pub struct FilteringConfig" src/config/schemas.rs -A 100
ls src/filtering/sources/
rg "apply_filter\|check_" src/filtering/sources/ -A 10
```

---

### **Doc 3.4: Swap Router Configuration**

**File:** `docs/configuration/swaps.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - swaps section
2. Review `src/swaps/gmgn.rs` for GMGN settings
3. Review `src/swaps/jupiter.rs` for Jupiter settings
4. Check `src/swaps/mod.rs` for router selection logic

**Content to Write:**

- GMGN router settings
- Jupiter router settings
- Slippage configuration
- Swap mode (ExactIn vs ExactOut)
- Router comparison & selection
- Priority fees & compute limits
- Best practices

**Code Investigation:**

```bash
# Commands to run:
rg "\[swaps" data/config.toml -A 50
rg "pub struct SwapsConfig" src/config/schemas.rs -A 60
rg "get_gmgn_quote\|get_jupiter_quote" src/swaps/ -A 15
```

---

### **Doc 3.5: Token Sources Configuration**

**File:** `docs/configuration/token-sources.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - tokens section
2. Review `src/tokens/market/` for market data sources
3. Review `src/tokens/security/` for security sources
4. Review `src/tokens/discovery.rs` for discovery sources
5. Check `src/tokens/updates.rs` for update intervals

**Content to Write:**

- Data source priority
- Enabling/disabling sources
- Rate limits & timeouts
- Discovery sources configuration
- Update interval settings
- Source comparison & reliability

**Code Investigation:**

```bash
# Commands to run:
rg "\[tokens" data/config.toml -A 100
rg "pub struct TokensConfig" src/config/schemas.rs -A 80
ls src/tokens/market/ src/tokens/security/
rg "preferred_market_data_source" src/tokens/ -A 5
```

---

### **Doc 3.6: Pool & Monitoring Configuration**

**File:** `docs/configuration/pools-monitoring.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - pools, monitoring, ohlcv, wallet sections
2. Review `src/pools/service.rs` for pool settings
3. Review `src/connectivity/` for monitoring
4. Review `src/ohlcvs/` for OHLCV settings

**Content to Write:**

- Pool system configuration
- Connectivity monitoring settings
- OHLCV data configuration
- Wallet monitoring settings
- Performance tuning
- Resource management

**Code Investigation:**

```bash
# Commands to run:
rg "\[pools\]|\[monitoring\]|\[ohlcv\]|\[wallet\]" data/config.toml -A 30
rg "pub struct PoolsConfig\|MonitoringConfig\|OhlcvConfig\|WalletConfig" src/config/schemas.rs -A 40
```

---

### **Doc 3.7: RPC Configuration**

**File:** `docs/configuration/rpc.mdx`  
**Research Steps:**

1. Review `src/config/schemas.rs` - rpc section
2. Review `src/rpc.rs` for RPC client implementation
3. Check existing blog post about RPC providers
4. Review rate limiting & fallback logic

**Content to Write:**

- RPC endpoint configuration
- Multiple endpoint support
- Rate limiting
- Fallback strategy
- Choosing RPC providers (link to blog)
- Premium vs free endpoints
- Testing RPC health

**Code Investigation:**

```bash
# Commands to run:
rg "\[rpc\]" data/config.toml -A 10
rg "pub struct RpcConfig" src/config/schemas.rs -A 20
rg "get_rpc_client\|RPC_CLIENT" src/rpc.rs -A 20
cat repos/Website/app/blog/_content/best-rpc-providers.mdx
```

---

### **Doc 3.8: Advanced - Strategy System**

**File:** `docs/advanced/strategies.mdx`  
**Research Steps:**

1. Review `src/strategies/` in depth
2. Review `src/strategies/conditions/` for all condition types
3. Check `src/strategies/db.rs` for persistence
4. Review `src/strategies/engine.rs` for evaluation logic

**Content to Write:**

- Strategy system architecture
- Creating custom strategies
- Condition types reference
- Combining conditions (AND/OR logic)
- Strategy evaluation process
- Testing strategies
- Strategy priority & execution order
- Advanced examples

**Code Investigation:**

```bash
# Commands to run:
rg "Strategy {" src/strategies/types.rs -A 40
ls src/strategies/conditions/
cat src/strategies/conditions/*.rs | rg "pub struct" -A 10
rg "evaluate_strategy" src/strategies/engine.rs -A 30
```

---

### **Doc 3.9: Advanced - OHLCV System**

**File:** `docs/advanced/ohlcv.mdx`  
**Research Steps:**

1. Review `src/ohlcvs/` module structure
2. Review `src/ohlcvs/types.rs` for timeframes
3. Review `src/ohlcvs/monitor.rs` for monitoring
4. Review `src/ohlcvs/fetcher.rs` for data sources

**Content to Write:**

- OHLCV data system overview
- Timeframes available
- Data sources & fetching
- Priority-based monitoring
- Using OHLCV for analysis
- Gap detection & auto-fill
- Cache management
- Integration with strategies

**Code Investigation:**

```bash
# Commands to run:
rg "Timeframe" src/ohlcvs/types.rs -A 10
rg "OhlcvDataPoint" src/ohlcvs/types.rs -A 15
rg "Priority" src/ohlcvs/priorities.rs -A 15
rg "fetch_ohlcv" src/ohlcvs/fetcher.rs -A 20
```

---

### **Doc 3.10: Advanced - Transaction System**

**File:** `docs/advanced/transactions.mdx`  
**Research Steps:**

1. Review `src/transactions/` module overview
2. Review `src/transactions/analyzer/` for analysis
3. Review `src/transactions/websocket.rs` for real-time monitoring
4. Review `src/transactions/verifier.rs` for verification

**Content to Write:**

- Transaction monitoring architecture
- Real-time WebSocket monitoring
- Transaction analysis & classification
- Swap detection algorithms (high-level)
- P&L calculation methodology
- Verification process
- DEX & router detection
- ATA operations tracking

**Code Investigation:**

```bash
# Commands to run:
rg "TransactionType\|SwapPnLInfo" src/transactions/types.rs -A 20
ls src/transactions/analyzer/
rg "analyze_transaction" src/transactions/analyzer/ -A 15
rg "verify_.*_transaction" src/transactions/verifier.rs -A 15
```

---

### **Doc 3.11: Advanced - Events System**

**File:** `docs/advanced/events.mdx`  
**Research Steps:**

1. Review `src/events/` module structure
2. Review `src/events/types.rs` for event types
3. Review `src/events/maintenance.rs` for recording
4. Review `src/events/db.rs` for persistence

**Content to Write:**

- Events system architecture
- Event categories & types
- Event recording (when & why)
- Event structure & metadata
- Querying events programmatically
- Using events for analytics
- Event retention & cleanup
- Performance considerations

**Code Investigation:**

```bash
# Commands to run:
rg "Event {" src/events/types.rs -A 25
rg "EventCategory" src/events/types.rs -A 20
rg "record_.*_event" src/events/maintenance.rs
rg "search_events\|recent_events" src/events/ -A 10
```

---

### **Doc 3.12: Advanced - Service Architecture**

**File:** `docs/advanced/services.mdx`  
**Research Steps:**

1. Review `src/services/mod.rs` for Service trait
2. Review `src/services/implementations/` for all services
3. Review `src/run.rs` for registration & startup
4. Review `src/services/metrics.rs` for metrics collection

**Content to Write:**

- Service architecture overview
- Service lifecycle (init → start → stop)
- Dependency resolution
- Priority-based startup
- Service health monitoring
- Metrics collection (tokio_metrics)
- Creating custom services (advanced)
- Service debugging

**Code Investigation:**

```bash
# Commands to run:
rg "pub trait Service" src/services/mod.rs -A 30
rg "ServiceManager" src/services/mod.rs -A 50
ls src/services/implementations/
rg "register_service" src/run.rs -A 5
```

---

## PHASE 4: REFERENCE & SUPPORT

### **Doc 4.1: Data Sources Reference**

**File:** `docs/reference/data-sources.mdx`  
**Research Steps:**

1. Review existing blog post `data-sources-behind-screenerbot.mdx`
2. Review `src/apis/` for all API clients
3. Check rate limiting in each API client
4. Review data source usage across modules

**Content to Write:**

- Comprehensive data source list
- What each source provides
- Rate limits & quotas
- Data accuracy & reliability
- Source priority & fallback
- API endpoints (no keys)
- Integration details

**Code Investigation:**

```bash
# Commands to run:
cat repos/Website/app/blog/_content/data-sources-behind-screenerbot.mdx
ls src/apis/
rg "rate_limit\|timeout" src/apis/ -A 5
rg "DexScreener\|GeckoTerminal\|Rugcheck\|Jupiter\|GMGN" src/apis/ -A 10
```

---

### **Doc 4.2: Supported DEXs**

**File:** `docs/reference/supported-dexs.mdx`  
**Research Steps:**

1. Review `src/pools/decoders/` for all DEX decoders
2. Review `src/pools/types.rs` for ProgramKind enum
3. Review `src/constants.rs` for DEX program IDs
4. Check `src/pools/calculator.rs` for price calculation

**Content to Write:**

- Complete list of supported DEXs
- DEX program IDs (for reference)
- Pool types supported per DEX
- Pricing accuracy per DEX
- DEX-specific considerations
- Transaction routing through each DEX

**Code Investigation:**

```bash
# Commands to run:
ls src/pools/decoders/
rg "ProgramKind" src/pools/types.rs -A 30
rg "RAYDIUM.*PROGRAM\|ORCA.*PROGRAM\|METEORA.*PROGRAM\|PUMP" src/constants.rs
rg "match program_kind" src/pools/calculator.rs -A 50
```

---

### **Doc 4.3: Troubleshooting Guide**

**File:** `docs/support/troubleshooting.mdx`  
**Research Steps:**

1. Review common errors in `src/errors/`
2. Review connectivity issues in `src/connectivity/`
3. Check validation logic in various modules
4. Review logs for common error patterns
5. Check existing issue reports if available

**Content to Write:**

- Common issues & solutions
- License verification failures
- RPC connection problems
- Trading not working (multiple causes)
- Filtering issues
- Position management errors
- Performance issues
- Dashboard problems
- Configuration errors
- Service failures
- Step-by-step debugging process

**Code Investigation:**

```bash
# Commands to run:
ls src/errors/
rg "ScreenerBotError\|BlockchainError" src/errors/ -A 15
rg "are_critical_endpoints_healthy" src/connectivity/ -A 10
rg "error!\|Err\(" src/ --type rust | head -100
```

---

### **Doc 4.4: Best Practices**

**File:** `docs/support/best-practices.mdx`  
**Research Steps:**

1. Review safety checks in `src/trader/safety/`
2. Review config validation across modules
3. Review recommended settings in config
4. Check security considerations in code

**Content to Write:**

- Configuration best practices
- Trading best practices
- Security recommendations
- Performance optimization
- Risk management
- Monitoring & maintenance
- Backup procedures
- Upgrade considerations

**Code Investigation:**

```bash
# Commands to run:
ls src/trader/safety/
rg "safety_check\|validate\|verify" src/trader/safety/ -A 10
rg "recommended\|default\|min.*max" src/config/schemas.rs
```

---

### **Doc 4.5: FAQ**

**File:** `docs/support/faq.mdx`  
**Research Steps:**

1. Compile questions from all previous docs
2. Review code for common user concerns
3. Check existing support channels if any
4. Consider user workflow questions

**Content to Write:**

- General questions (20+)
- Trading questions (15+)
- Technical questions (15+)
- Configuration questions (10+)
- Troubleshooting questions (10+)
- Each with clear, concise answers

**Code Investigation:**

```bash
# Commands to run:
# Review all previous research notes
# Consider common user journeys
# Identify potential confusion points
```

---

### **Doc 4.6: Glossary**

**File:** `docs/reference/glossary.mdx`  
**Research Steps:**

1. Collect all technical terms from docs
2. Review Solana-specific terminology
3. Review bot-specific terminology
4. Review trading terminology

**Content to Write:**

- Comprehensive term definitions (50+)
- Alphabetically organized
- Cross-references where appropriate
- Links to detailed docs
- Solana-specific terms
- DeFi trading terms
- Bot-specific terms

**Code Investigation:**

```bash
# Commands to run:
rg "ATA|DEX|DCA|FDV|LP|OHLCV|P&L|RPC|SPL" src/ --type rust | head -50
# Review all documentation for technical terms
```

---

## Execution Guidelines

### For Each Documentation Section:

1. **Research Phase** (30-60 min per doc)
   - Run all specified commands
   - Read relevant source files
   - Take notes on actual behavior
   - Identify user-facing features only
   - Note any discrepancies

2. **Writing Phase** (60-90 min per doc)
   - Write in clear, user-friendly language
   - Focus on HOW to use, not HOW it works internally
   - Include step-by-step instructions
   - Add screenshots/diagrams where helpful
   - Provide examples
   - Link to related docs

3. **Validation Phase** (15-30 min per doc)
   - Verify accuracy against code
   - Check for proprietary information (remove if found)
   - Ensure user perspective maintained
   - Test any provided commands
   - Review for clarity

4. **Integration Phase** (15 min per doc)
   - Add to website docs structure
   - Update navigation
   - Add cross-references
   - Test links
   - Update table of contents

---

## Documentation Standards

### Writing Style:

- **User-focused**: Always "you" perspective, never code perspective
- **Clear**: Simple language, avoid jargon, define terms
- **Actionable**: Every doc should enable user action
- **Accurate**: Based on actual code, not assumptions
- **Non-technical**: No code snippets unless absolutely necessary

### Structure:

- **Overview**: What & Why at the top
- **Prerequisites**: What user needs first
- **Steps**: Clear numbered/bulleted instructions
- **Examples**: Real-world use cases
- **Troubleshooting**: Common issues at bottom
- **Related**: Links to other relevant docs

### What to NEVER Include:

- ❌ Source code snippets (except config examples)
- ❌ Algorithm details or proprietary logic
- ❌ Internal implementation details
- ❌ API keys or sensitive data
- ❌ Database schemas (unless user-facing)
- ❌ Technical architecture diagrams (unless high-level)

### What to ALWAYS Include:

- ✅ What the feature does (user perspective)
- ✅ How to use it (step-by-step)
- ✅ Why to use it (benefits)
- ✅ When to use it (use cases)
- ✅ Common issues & solutions
- ✅ Related features & docs

---

## Progress Tracking

### Phase 1: Foundation (5 docs)

- [ ] 1.1 Introduction
- [ ] 1.2 License Guide
- [ ] 1.3 Installation Guide
- [ ] 1.4 Initial Setup
- [ ] 1.5 Dashboard Access

### Phase 2: Core Usage (11 docs)

- [ ] 2.1 Home Dashboard
- [ ] 2.2 Positions Management
- [ ] 2.3 Tokens Discovery
- [ ] 2.4 Filtering System
- [ ] 2.5 Trader Control
- [ ] 2.6 Strategies
- [ ] 2.7 Wallet Monitoring
- [ ] 2.8 Transactions
- [ ] 2.9 Events Logging
- [ ] 2.10 Configuration UI
- [ ] 2.11 Services Management

### Phase 3: Configuration & Advanced (12 docs)

- [ ] 3.1 Trader Configuration
- [ ] 3.2 Position Configuration
- [ ] 3.3 Filtering Configuration
- [ ] 3.4 Swap Router Configuration
- [ ] 3.5 Token Sources Configuration
- [ ] 3.6 Pool & Monitoring Configuration
- [ ] 3.7 RPC Configuration
- [ ] 3.8 Advanced - Strategy System
- [ ] 3.9 Advanced - OHLCV System
- [ ] 3.10 Advanced - Transaction System
- [ ] 3.11 Advanced - Events System
- [ ] 3.12 Advanced - Service Architecture

### Phase 4: Reference & Support (6 docs)

- [ ] 4.1 Data Sources Reference
- [ ] 4.2 Supported DEXs
- [ ] 4.3 Troubleshooting Guide
- [ ] 4.4 Best Practices
- [ ] 4.5 FAQ
- [ ] 4.6 Glossary

**Total: 34 documentation files**

---

## Next Steps

When ready to start a specific documentation section:

1. **Tell me which doc number** (e.g., "Let's do 1.1 Introduction")
2. **I will run the research commands** and analyze the codebase
3. **I will draft the documentation** following the guidelines
4. **You review** and provide feedback
5. **I finalize** and we move to the next doc

This ensures every piece of documentation is:

- ✅ Accurate (based on actual code)
- ✅ User-friendly (no technical jargon)
- ✅ Complete (covers all aspects)
- ✅ Safe (no proprietary information)

---

**Ready to start whenever you are!** 🚀

---

## Completed Documentation

### ✅ Doc 1.1: Introduction (November 13, 2025)

**File Created:** `repos/Website/app/docs/introduction/page.tsx`

**Content Completed:**

- What is ScreenerBot? - Comprehensive overview of the bot's purpose and capabilities
- Core Capabilities - 6 key features: Token Discovery, Multi-DEX Price Analysis, Intelligent Filtering, Automated Trading, Position Management, Safety & Security
- How It Works - 6-step flow diagram: Discovery → Analysis → Filtering → Strategy Evaluation → Execution → Monitoring
- Who Should Use It? - 4 target user profiles with detailed descriptions
- What You Need to Get Started - System requirements and prerequisites
- Important Notes - Critical information about dry-run mode, risk management, continuous operation, and monitoring
- Next Steps - Links to purchase license and download bot

**Research Sources Used:**

- `FLOW.md` - System architecture and component overview
- `.github/Assistant-instructions.md` - Detailed module descriptions and capabilities
- `src/run.rs` - Startup sequence and service registration
- `src/services/implementations/` - Complete list of 18 services
- `src/license/mod.rs` - License verification system

**Key Features Highlighted:**

- 12+ DEX decoder support (Raydium, Orca, Meteora, Pumpfun, etc.)
- Multi-source token discovery (DexScreener, GeckoTerminal, Raydium)
- Advanced filtering with security analysis (Rugcheck integration)
- Strategy-based automated trading with DCA and partial exit support
- Real-time position tracking with comprehensive P&L calculations
- Multi-timeframe OHLCV data for technical analysis

**User-Focused Approach:**

- No code implementation details exposed
- Focus on WHAT the bot does, not HOW it's implemented
- Clear step-by-step workflow explanation
- Practical requirements and prerequisites
- Safety warnings and best practices included

**Status:** Complete and ready for user review ✓

---

### ✅ Doc 1.2: License Guide (November 13, 2025)

**File Created:** `repos/Website/app/docs/getting-started/license/page.tsx`

**Content Completed:**

- Why NFT Licenses? - Benefits comparison with traditional licensing (ownership, transferability, verifiability)
- How It Works - 3-step process: Purchase (with USDC), Automatic Verification (on-chain), Management (wallet apps & blockchain explorers)
- License Metadata - Complete explanation of NFT attributes (Tier, Start Date, Expiry Date, Duration, Issued To, Revoked status)
- Transferring Your License - Step-by-step NFT transfer instructions with post-transfer implications
- Wallet Requirements - Setup requirements and security best practices
- License Expiration & Renewal - What happens when license expires, data preservation, renewal process
- Troubleshooting License Issues - 4 common problems with detailed solutions:
  - "No valid ScreenerBot license found"
  - "Failed to get token accounts"
  - Slow license verification
  - Lost wallet scenario
- Next Steps - Links to pricing and download pages

**Research Sources Used:**

- `repos/Website/app/blog/_content/nft-license-guide.mdx` - Existing blog content for NFT licensing concepts
- `src/license/mod.rs` - Verification flow implementation (verify_license_for_wallet, verify_license_for_wallet_with_endpoints)
- `src/license/types.rs` - LicenseStatus structure and MetadataJson parsing
- `repos/Website/prisma/schema.prisma` - License model with all tracked fields
- `repos/Website/app/api/license/` - API endpoints structure

**Key Information Provided:**

- NFT verification is fully automatic on bot startup
- License metadata is immutable and on-chain
- Supports wallet transfer for reselling/gifting licenses
- Uses USDC on Solana for payments
- No grace period - access ends exactly at expiry timestamp
- Bot queries Solana blockchain directly for verification (no company server dependency)
- RPC endpoint quality affects verification speed
- Data preserved after license expiry (configs, positions, databases)

**User-Focused Approach:**

- Clear comparison of NFT vs traditional licensing benefits
- Step-by-step purchase and transfer processes
- Practical wallet security recommendations
- Detailed troubleshooting for common issues
- External links to blockchain explorers for verification
- No code implementation details exposed

**Status:** Complete and ready for user review ✓

---

### ✅ Doc 1.3: Installation Guide (November 14, 2025)

**File Created:** `repos/Website/app/docs/getting-started/installation/page.tsx`

**Content Completed:**

- System Requirements - Three tiers with detailed specs:
  - Minimum: 4 cores, 4GB RAM, 2GB storage (testing & light trading)
  - Recommended: 8 cores, 8GB RAM, 10GB storage (active trading)
  - Best Performance: 8+ cores, 16GB RAM, 25GB+ storage (heavy trading & long-term data)
  - Storage breakdown: ~500MB app, up to 20GB data folder
- Prerequisites - 5 essential requirements: Valid license NFT, wallet private key, RPC URLs, fast internet, SOL for trading
- Download Section - Platform-specific download links and requirements:
  - macOS: 10.13+, Intel/Apple Silicon, DMG installer
  - Windows: Win10+ 64-bit, MSI/EXE, VC++ redistributables
  - Linux: Ubuntu 20.04+/Debian 11+, AppImage/DEB, x86_64
- Desktop Installation - Detailed step-by-step for each OS:
  - macOS: DMG install, Gatekeeper bypass, security preferences
  - Windows: MSI installer, SmartScreen warnings, directory setup
  - Linux: AppImage (chmod +x) and DEB package methods, dependencies
- Server Deployment - Complete VPS setup guide:
  - VPS provider recommendations
  - Ubuntu/Debian installation commands
  - systemd auto-start configuration
  - SSH tunnel for remote dashboard access
- Directory Structure - Complete file tree with descriptions:
  - data/ folder structure (config.toml, databases, wallet keypair)
  - logs/ with 24h rotation
  - Security warning about mainnet-wallet.json private key
- Verification - 4-step process to confirm successful installation
- Troubleshooting - 4 common installation issues with solutions:
  - macOS Gatekeeper blocking
  - Windows SmartScreen warnings
  - Linux AppImage permissions/FUSE
  - Port 8080 conflicts

**Research Sources Used:**

- `electron/package.json` - App configuration, bundle settings, platform requirements (macOS 10.13+)
- `Cargo.toml` - Package information, version, build profiles
- User specifications for hardware requirements and storage needs

**Key Information Provided:**

- Bundled executables for all major platforms (no compilation needed)
- Specific minimum/recommended/best performance tiers
- Actual storage numbers: 500MB app + up to 20GB data
- Server deployment for 24/7 operation with systemd service
- Complete directory structure showing all created files
- Security emphasis on wallet keypair file protection
- Remote access via SSH tunnel for VPS deployments
- Platform-specific installation nuances and security warnings

**User-Focused Approach:**

- Clear hardware requirements for different trading intensities
- Step-by-step installation for each platform
- Practical server deployment guide with copy-paste commands
- Visual icons and color-coded sections for each OS
- Real-world troubleshooting for common installation blockers
- No technical build/compilation details exposed

**Status:** Complete and ready for user review ✓

---

### ✅ Doc 1.4: Initial Setup Guide (November 14, 2025)

**File Created:** `repos/Website/app/docs/getting-started/setup/page.tsx`

**Content Completed:**

- Setup Overview - 3-step quick reference: Wallet Setup → RPC Configuration → First Launch
- Prerequisites Reminder - 5 checklist items with links to previous guides
- Step 1: Wallet Configuration - Complete private key setup:
  - Getting private key from Phantom Wallet (recovery phrase export)
  - Getting private key from Solflare Wallet (direct export)
  - Using keypair JSON files (array format)
  - Supported formats: Base58 (87-88 chars) and Array (64 numbers)
  - Security warnings about private key protection
- Step 2: RPC Configuration - Comprehensive RPC provider guide:
  - Free RPC providers: Helius (100k/day), QuickNode, Alchemy
  - Premium RPC providers: Helius Pro, QuickNode dedicated, Triton
  - Step-by-step RPC URL acquisition (Helius, QuickNode examples)
  - Link to Best RPC Providers blog post
- Step 3: Configuration File Creation:
  - Platform-specific config.toml locations (macOS, Windows, Linux)
  - Minimal configuration template with placeholders
  - Example configuration with actual values
  - Multiple RPC endpoints for failover (optional)
  - Recommendation: Keep trader.enabled = false initially, start small (0.005 SOL)
- Step 4: First Launch & Verification:
  - Desktop application launch instructions
  - Server deployment commands (nohup, background process)
  - 5-point verification checklist:
    - License verification successful
    - Wallet loaded correctly
    - RPC connection established
    - Dashboard accessible at http://localhost:8080
    - Services running properly
  - Dashboard access instructions
- Troubleshooting Setup Issues - 5 common problems with detailed solutions:
  - License verification failed (wrong wallet, expired license)
  - Invalid private key format (base58 vs array, length issues)
  - RPC connection failed (URL errors, provider downtime)
  - Config file not found (location, filename, extension issues)
  - Dashboard not loading (port conflicts, initialization time)
- Security Best Practices - 5 critical security recommendations:
  - Never share config.toml (contains private key)
  - Set proper file permissions (chmod 600)
  - Encrypt backups
  - Server security (firewall, SSH keys)
  - Start with minimal trading amounts

**Research Sources Used:**

- `data/config.toml` - Actual configuration file structure and all sections
- `src/config/utils.rs` - Wallet keypair loading functions (get_wallet_keypair, base58/array format support)
- `src/run.rs` - Startup sequence, license verification, initialization mode logic
- `src/paths.rs` - Platform-specific directory locations (~/Library/Application Support/ScreenerBot, %LOCALAPPDATA%\ScreenerBot, $XDG_DATA_HOME/ScreenerBot)

**Key Information Provided:**

- Both base58 and array private key formats supported
- Bot validates private key length (must be 64 bytes)
- Multiple RPC endpoints supported with round-robin + failover
- Initialization mode if config.toml doesn't exist (webserver only)
- License verification happens automatically on startup
- Dashboard starts on port 8080 after 15-20 second initialization
- config.toml is the single source of truth for all configuration
- Wallet consistency validation after license verification

**User-Focused Approach:**

- Step-by-step wallet export from popular wallets (Phantom, Solflare)
- Clear distinction between free and premium RPC providers
- Copy-paste configuration templates with placeholders
- Comprehensive troubleshooting for every common setup issue
- Strong emphasis on security throughout (private key protection)
- Recommendation to start conservative (trader disabled, small amounts)
- No algorithm details or proprietary logic exposed

**Status:** Complete and ready for user review ✓

---

### ✅ Doc 1.5: Dashboard Access Guide (January 25, 2025)

**File Created:** `repos/Website/app/docs/getting-started/dashboard/page.tsx`

**Content Completed:**

- Dashboard Access & Navigation - Complete guide to the ScreenerBot control center interface, covering access, navigation, and all 12 dashboard pages
- Quick Access Info - Default URL (http://localhost:8080), startup time (15-20s), remote access via SSH tunnel
- First Launch - 4-step initialization flow:
  - Bot starts (config load, license verify, wallet validate)
  - Services initialize (RPC, Tokens, Pools, Positions, Transactions)
  - Webserver ready (dashboard accessible)
  - Full initialization (all services running, data syncing, ~15-20s total)
- Dashboard Interface Overview - Complete 3-row header layout breakdown:
  - Row 1 Control Bar: Brand logo, bot status card (ACTIVE/STOPPED, P&L), wallet card (SOL balance, daily change, token count, worth), positions card (count, unrealized P&L, RPC health)
  - Quick Actions (right side): Notifications (bell icon, badge count), refresh interval (5s/10s/30s/manual), theme toggle (light/dark), settings button
  - Row 2 Navigation Tabs: Horizontal tabs for major sections, active tab highlighted in blue
  - Row 3 Live Ticker: Scrolling real-time updates (trades, new tokens, system events, price changes)
- Dashboard Pages - All 12 pages with complete descriptions:
  - **Home:** Wallet overview hero, trader analytics with period tabs (today/yesterday/week/month/all-time), open positions summary, primary stats (Net P&L, Win Rate, Profit, Loss), secondary stats (Buys, Sells, Max Drawdown)
  - **Positions:** Open/closed positions management, manual buy/sell actions, DCA entry tracking, partial exit management, real-time P&L calculations
  - **Tokens:** Token discovery from multiple sources, market data (price, volume, liquidity), security scoring (Rugcheck integration), blacklist management
  - **Filtering:** Passed tokens ready for trading, rejected tokens with detailed reasons, filter statistics and performance, real-time filter updates
  - **Trader:** Start/stop automated trading control, entry monitor status, exit monitor activity, trading performance metrics
  - **Strategies:** Strategy creation and editing, condition-based trading logic, strategy priority management, performance tracking
  - **Wallet:** Balance history with snapshots, token holdings breakdown, daily income/expense tracking, CSV export functionality
  - **Transactions:** Live transaction stream, swap detection and analysis, router identification, transaction verification status
  - **Events:** Comprehensive event logging, category and severity filters, event search functionality, debugging and diagnostics
  - **Config:** Web-based configuration editor, all settings in one place, export/import configurations, live reload without restart
  - **Services:** Service status monitoring, health indicators, performance metrics, dependency visualization
  - **Initialization (Special):** First-time setup wizard shown only when config.toml doesn't exist, guides through wallet/RPC/license setup, auto-redirects after completion
- Navigation Tips - 4 key areas:
  - Keyboard Shortcuts: Tab (cycle items), Enter (activate), Esc (close modals)
  - Auto-Refresh: Default 5s, configurable to 10s/30s/manual, force refresh icon
  - Theme Customization: Light/dark toggle, browser storage persistence, syncs across tabs
  - Notifications Panel: Bell icon access, unread badge count, quick alerts, click to jump to details
- Troubleshooting Dashboard Access - 4 common problems with comprehensive solutions:
  - Can't Access Dashboard: Verify bot running, wait 15-20s for init, confirm URL, check port 8080 availability, try different browser
  - Dashboard Loading Slowly: Increase refresh interval to 30s/manual, check RPC endpoint performance, close unused tabs, clear browser cache, check system resources
  - Data Not Updating: Click manual refresh icon, verify auto-refresh enabled, check bot/services health (Services page), check JS console for errors (F12), try hard refresh (Ctrl+F5/Cmd+Shift+R)
  - Some Pages Missing/Broken: Check Services page for running services, restart bot to reinitialize, review Events page for errors, clear browser cache completely, try different device
- Dashboard Best Practices - 5 key recommendations:
  - Keep dashboard open in dedicated window/tab for real-time monitoring
  - Check Services page regularly to ensure critical services are running and healthy
  - Review Events page daily for warnings, errors, or unusual activity
  - Adjust refresh rate: faster (5s) during active trading, slower (30s) or manual during idle
  - Bookmark frequently used pages (Positions, Trader, Filtering) for quick access

**Research Sources Used:**

- `src/webserver/server.rs` - DEFAULT_PORT = 8080, bind to 0.0.0.0:8080, error handling for port conflicts (address in use, permission denied)
- `src/webserver/routes/mod.rs` - All 12 routes confirmed: /, /home, /services, /tokens, /positions, /events, /transactions, /filtering, /wallet, /config, /strategies, /trader, /initialization
- `src/webserver/templates/base.html` - Complete three-row header structure, theme system (dark default), initialization status check with redirect logic
- `src/webserver/templates/pages/*.html` - 12 HTML page files confirming complete dashboard page list
- `src/webserver/templates/pages/home.html` - Wallet hero section structure, trader analytics with period tabs, primary/secondary stats layout, positions grid

**Key Information Provided:**

- Dashboard runs on hardcoded port 8080 (no configuration needed)
- Full initialization takes 15-20 seconds from bot startup
- Three-row header provides comprehensive status at a glance
- 12 main pages cover all bot functionality systematically
- Initialization page is special: only shown when config.toml missing, auto-redirects after setup
- Auto-refresh configurable for bandwidth management: 5s/10s/30s/manual
- Theme toggle with browser localStorage persistence
- Notifications panel with unread badge for important alerts
- Keyboard navigation fully supported (Tab, Enter, Esc)
- Remote access supported via SSH tunnel for VPS deployments
- First launch may have empty data (normal - background sync populates within minutes)

**User-Focused Approach:**

- Clear URL and startup time expectations (http://localhost:8080, 15-20s)
- Visual 4-step initialization flow for first-time users
- Detailed header component breakdown with icons and purpose
- All 12 pages described with purpose, key features, and links to detailed guides
- 4 comprehensive troubleshooting scenarios with step-by-step multi-point solutions
- 5 practical best practices for effective dashboard usage and monitoring
- Navigation tips covering keyboard shortcuts, refresh control, themes, notifications
- Emphasis on Services and Events pages for health monitoring
- No backend implementation details, algorithm logic, or code exposed

**Status:** Complete and ready for user review ✓

---

### ✅ Doc 2.1: Home Dashboard Guide (November 14, 2025)

**File Created:** `repos/Website/app/docs/dashboard/home/page.tsx`

**Content Completed:**

- Home Dashboard Guide - Complete guide to the central command center showing wallet analytics, trader performance, position snapshots, system health, token statistics, and license information
- Dashboard Overview - What You'll See section with 6 key information areas: Wallet Analytics, Trader Analytics, Positions Snapshot, System Metrics, Token Statistics, License Information
- Section 1: Wallet Analytics - Hero card metrics explained:
  - Current Balance: Live SOL balance from blockchain with 4 decimal precision
  - Daily Change: SOL and % change from start_of_day_balance_sol, color-coded green/red
  - Token Holdings: Count of discovered tokens with current prices
  - Tokens Worth: Total value of all token holdings in SOL (4 decimals)
  - Data freshness note: Balance updated every 60s, token prices every 30s
- Section 2: Trader Analytics - Performance tracking with 5 period tabs (Today, Yesterday, Week, Month, All-Time):
  - Primary Stats Grid (2×2): Net P&L SOL (profit minus loss), Win Rate % (profitable trades / total trades \* 100), Total Profit SOL, Total Loss SOL
  - Secondary Stats Grid (3-column): Total Buys, Total Sells, Max Drawdown %
  - Period Filtering: Each period recalculates all stats from filtered trades
- Section 3: Positions Snapshot - Open positions summary:
  - Count: Number of currently open positions
  - Total Invested: Cumulative SOL spent including DCA entries
  - Unrealized P&L: Calculated from current_price vs average_entry_price
  - Unrealized P&L %: (unrealized_pnl / total_invested_sol) \* 100
  - Link to full Positions page for detailed management
- Section 4: System Metrics - Bot health indicators with Chart.js sparklines:
  - Uptime: Human-readable format (e.g., "2d 14h 32m")
  - Memory Usage: Chart showing last 20 data points with gradient fill
  - CPU Usage: Percentage with 20-point history sparkline
  - Charts: Line charts with tension 0.4, borderWidth 2, point radius 0
- Section 5: Token Statistics - Discovery and filtering counters:
  - Total Discovered: All tokens in database from all sources
  - With Prices: Tokens having current SOL price data
  - Passed Filtering: Tokens meeting all filter criteria (ready for trading)
  - Rejected: Tokens failing one or more filters with reasons
  - Discovery Stats: Found Today, This Week, This Month, All Time
- Section 6: License Information - NFT license status card:
  - Status: Valid (green) or Invalid/Expired (red)
  - Tier: License tier level (e.g., "Tier 1", "Tier 2")
  - Expiry Date: Human-readable date (e.g., "Dec 31, 2025")
  - Mint Address: Full Solana NFT mint address
  - Days Remaining: Countdown with color coding (green >30, yellow 15-30, red <15)
- Understanding Key Metrics - 4 detailed metric explanations:
  - Win Rate: Importance of context (50%+ is good baseline, depends on strategy type)
  - Drawdown: Definition, calculation from highest cumulative P&L to lowest, recovery strategies
  - Unrealized P&L: Difference between realized (closed) and unrealized (open) P&L, update frequency
  - Token Discovery Rate: What affects counts (sources enabled, filter strictness, market conditions), interpreting trends
- Using the Home Dashboard Effectively - 5 strategic usage recommendations:
  - Morning Routine: Check wallet change, review P&L periods, verify open positions, assess system health, check token discovery trends
  - Active Trading: Monitor unrealized P&L changes, watch win rate fluctuations, track system metrics during high activity, check discovery rate for new opportunities
  - End of Day: Review All-Time period stats, compare today vs yesterday performance, note positions needing attention, check license expiration proximity
  - Weekly Review: Compare week vs month periods, analyze max drawdown recovery, evaluate token discovery patterns, assess system stability trends
  - Performance Analysis: Use period tabs to identify profitable timeframes, correlate market conditions with win rate, track improvement after strategy adjustments, monitor consistency across periods
- Troubleshooting Common Issues - 4 comprehensive problem scenarios with solutions:
  - Wallet Balance Not Updating: Check RPC connection (Services page), verify wallet address (Config page), restart bot if balance frozen >5 minutes, try alternative RPC endpoint
  - Unrealized P&L Shows "—": Verify Pool Service running (Services page), check token pool existence (Tokens page), confirm price_updater service active, wait 30-60s for next price update cycle
  - System Metrics Not Refreshing: Check if auto-refresh enabled (top-right icon), verify webserver service healthy, inspect browser console for JS errors (F12), try hard refresh (Ctrl+F5)
  - Token Stats Seem Low: Confirm discovery sources enabled (Config → tokens section), check Events page for discovery errors, verify internet connectivity, allow 5-10 minutes for initial sync after bot start
- Best Practices - 5 key recommendations for optimal dashboard usage:
  - Refresh Strategy: Use manual refresh during stable periods, enable 30s auto-refresh when actively monitoring, switch to 5s during critical trade executions
  - Metric Monitoring: Check unrealized P&L at least 2x daily, review win rate weekly (not obsessively), track max drawdown for risk assessment, monitor token discovery rate for market activity gauge
  - Context Awareness: Compare periods to identify trends (not single days), consider market conditions when evaluating performance, correlate P&L with position count/size changes
  - License Management: Set calendar reminder 7 days before expiration, check expiry date weekly if <30 days remaining, prepare renewal to avoid service interruption
  - System Health: Monitor uptime for unexpected restarts, watch memory usage trends (not spikes), check CPU during high transaction periods, investigate sustained >80% resource usage

**Research Sources Used:**

- `src/webserver/templates/pages/home.html` - Complete HTML structure with all 6 major sections: wallet hero card, trader analytics period tabs, positions grid, system metrics with canvas charts, token statistics cards, license info card
- `src/webserver/templates/scripts/pages/home.js` - JavaScript implementation: Chart.js sparkline initialization (createChart with 20 data points, line charts, tension 0.4), fetchData() from /api/dashboard/home, updateUI() dispatcher to 6 section updaters (updateTraderStats with period tabs, updateWalletStats, updatePositionsStats, updateSystemStats, updateTokenStats, updateLicenseInfo), animateValue() for number transitions, color coding logic (profit green, loss red)
- `src/webserver/routes/dashboard.rs` - API structures: HomeDashboardResponse with 7 top-level sections, TraderAnalytics with 5 TradingPeriodStats periods (Today/Yesterday/Week/Month/All-Time containing buys/sells/profit_sol/loss_sol/net_pnl_sol/drawdown_percent/win_rate), WalletAnalytics (current_balance_sol/token_count/tokens_worth_sol/start_of_day_balance_sol/change_sol/change_percent), PositionsSnapshot (open_count/total_invested_sol/unrealized_pnl_sol/unrealized_pnl_percent), SystemMetrics (uptime_seconds/memory_mb/memory_percent/cpu_percent with history arrays), TokenStatistics (total/with_prices/passed/rejected + discovery counts for today/week/month/all_time), LicenseInfo (valid/tier/expiry/mint/days_remaining)
- `src/wallet.rs` - WalletSnapshot structure with fields: timestamp, wallet_address, sol_balance, token_balances Vec<TokenBalance>, total_tokens_worth_sol, start_of_day_balance_sol. Balance history stored in SQLite wallet_snapshots table. Token balances updated every 60 seconds via get_current_wallet_status().
- `src/positions/metrics.rs` - ProceedsMetricsSnapshot calculation with fields: total_proceeds_sol, profit_sol, loss_sol, net_pnl_sol, total_buys, total_sells, win_rate, max_drawdown_percent. Calculated per trading period with timestamp filtering.

**Key Information Provided:**

- Dashboard displays 6 major information sections in organized layout
- Wallet balance updated every 60 seconds from blockchain RPC
- Token prices updated every 30 seconds by Pool Service price_updater
- Trader analytics supports 5 period filters with independent calculations per period
- All periods recalculate stats from filtered closed positions (no aggregation shortcuts)
- Win rate = (profitable_trades / total_trades) \* 100, considers only closed verified positions
- Max drawdown calculated from highest cumulative P&L to lowest point in period
- Unrealized P&L uses current_price from Pool Service, updated every 30s for open positions
- System metrics: Chart.js sparklines show last 20 data points with line charts (tension 0.4)
- Token statistics track discovery from 3 sources (DexScreener, GeckoTerminal, Raydium)
- License info checks NFT status on-chain with days remaining color-coded (green >30, yellow 15-30, red <15)
- Data fetched from /api/dashboard/home endpoint which aggregates from multiple services
- Auto-refresh configurable (5s/10s/30s/manual) via top-right control icon

**User-Focused Approach:**

- Clear breakdown of all 6 dashboard sections with purpose and metrics
- Detailed metric explanations (Win Rate, Drawdown, Unrealized P&L, Discovery Rate)
- 5 strategic usage scenarios (morning routine, active trading, end of day, weekly review, performance analysis)
- 4 comprehensive troubleshooting scenarios with multi-step solutions
- 5 best practices covering refresh strategy, metric monitoring, context awareness, license management, system health
- Period tab behavior explained (filters trades by timestamp range, recalculates all stats)
- Chart.js sparkline visualization details (20 points, gradient fill, borderWidth 2, tension 0.4)
- Color coding conventions (green profit/positive, red loss/negative, yellow warnings)
- No backend algorithm details or proprietary logic exposed

**Status:** Complete and ready for user review ✓

---

### ✅ Doc 2.2: Positions Management (November 14, 2025)

**File Created:** `repos/Website/app/docs/dashboard/positions/page.tsx`

**Content Completed:**

- Complete guide to managing trading positions with DCA support, partial exits, real-time P&L tracking, and direct position actions from the dashboard
- Overview - Key features list: Real-time price updates (30s), automatic P&L calculation (realized and unrealized), DCA with cooldown protection, partial exit capabilities (any percentage), transaction verification with chain validation, historical tracking of all entries/exits
- Two distinct views: Open positions (unrealized P&L, action buttons) and Closed positions (realized P&L, historical data)
- Open Positions View - 11 columns explained in detail:
  - Token (logo, symbol, name from tokens database)
  - Actions (Add button for DCA, Sell button for partial/full exits, disabled during execution)
  - Entry Time (Unix timestamp, sortable, MMM DD YYYY HH:MM format)
  - DCA (count of additional entries after initial, displayed as chip, "—" if none)
  - Avg Entry (SOL) (weighted average across all entries, uses average_entry_price field, 12 decimal precision)
  - Current (SOL) (real-time from price_updater service every 30s, "—" if unavailable)
  - Total Invested (cumulative SOL spent including DCA, total_size_sol field, 4 decimals)
  - Size (current position size %, (remaining_token_amount / token_amount) \* 100, color-coded chip: green 100%, yellow 50-99%, red <50%)
  - Exits (partial exit count, warning chip with count, "—" if none)
  - Unrealized PnL (potential profit/loss in SOL, updated every 30s, green/red color coding)
  - Unrealized % (percentage return, (unrealized_pnl / total_size_sol) \* 100, 2 decimals)
- Closed Positions View - 10 columns for historical data:
  - Token, Exit Time (final close timestamp), DCA (historical count), Avg Entry (SOL), Avg Exit (SOL) (weighted across all exits), Total Invested, Exits (partial exit count before close), Proceeds (total SOL received from all exits), PnL (realized profit/loss), PnL % (final percentage return)
- Position Actions - Add to Position (DCA):
  - Click Add button → dialog with token symbol, wallet balance, quick-select buttons (configured entry sizes), custom SOL input
  - Executes swap → updates total_size_sol, recalculates average_entry_price (weighted), increments dca_count, sets last_dca_time for cooldown, creates EntryRecord with is_dca: true
  - DCA cooldown default 60s (configurable positions.dca_cooldown_secs), prevents rapid accidental entries
- Position Actions - Sell Position (Partial or Full):
  - Click Sell button → dialog with token symbol, holdings, percentage slider (1-100%), quick-select (25%/50%/75%/100%), estimated SOL proceeds
  - If 100%: Full exit, position closes, exit_time and exit_transaction_signature set
  - If <100%: Partial exit, position stays open with updated remaining_token_amount
  - Creates ExitRecord, updates average_exit_price (weighted), increments partial_exit_count, accumulates total_exited_amount
- Understanding Open Position Metrics - Average Entry Price Calculation:
  - Weighted formula: `(entry1_price * entry1_amount + entry2_price * entry2_amount + ...) / (entry1_amount + entry2_amount + ...)`
  - Example with initial entry + DCA entry showing precise calculation
- Understanding Open Position Metrics - Unrealized P&L Calculation:
  - Without exits: `unrealized_pnl_sol = (current_price - average_entry_price) * token_amount`
  - With partial exits: Complex formula including remaining value, cost basis remaining, realized from exits
  - Price update frequency: 30s from price_updater service, shows "—" if pool price unavailable
- Understanding Closed Position Metrics - Realized P&L, Synthetic Exits:
  - Simple formula: `pnl_sol = sol_received - total_size_sol`, `pnl_percent = (pnl_sol / total_size_sol) * 100`
  - With partial exits: Uses total_sol_received_all_exits
  - Synthetic exits: zero balance detected but no exit transaction found, sets synthetic_exit: true, closed_reason: "synthetic_phantom_closure", P&L as 0.0
- Position Lifecycle & States - 6 state flow:
  - Open (initial after entry tx), EntryPending (awaiting verification), EntryVerified (confirmed on-chain), Closing (exit tx submitted), ExitPending (awaiting exit verification), Closed (exit confirmed, finalized)
- Position Lifecycle - Phantom Position Detection:
  - Background worker checks wallet every 60s
  - Zero balance detected → increments phantom_confirmations, sets phantom_first_seen
  - After 3 consecutive confirmations (180s) → triggers synthetic closure
- DCA Strategy - Execution flow:
  - Initial position → add more at different price → bot calculates new weighted average → updates total_size_sol → increments dca_count → creates EntryRecord with is_dca: true
- DCA Safeguards - 4 protections: Cooldown timer (60s), per-position lock (mutex on mint), balance verification, pending state flag
- DCA Best Practices - Wait for price confirmation (2-3 updates), use consistent sizing, monitor total invested vs wallet balance, check DCA count before adding
- Partial Exit Strategy - Execution flow:
  - Select percentage (1-99% partial, 100% full) → bot calculates sell_amount → executes swap → updates position (total_exited_amount, remaining_token_amount, partial_exit_count) → recalculates average_exit_price → creates ExitRecord with is_partial: true → unrealized P&L recalculated immediately
- Average Exit Price Calculation - Weighted formula with detailed example (2-exit scenario)
- Common Partial Exit Strategies - 4 strategies table: Take Profits Ladder, Risk Reduction, Trailing Stop, Time-Based
- P&L Tracking System - Architecture:
  - Background price updater (30s), automatic recalculation (unrealized_pnl), partial exit adjustment (immediate), final close calculation (once, stored permanently)
- P&L Formulas - Complete formulas for open (with/without partial exits) and closed positions
- P&L Edge Cases - 4 scenarios table: Price data unavailable, zero/negative entry price, synthetic exit, price update failure
- Transaction Verification System - 7-step flow:
  - Submission (tx sent, signature captured) → Queue (verification item added) → Worker processing (polls every 10s) → Chain fetch (RPC jsonParsed) → Analysis (parse transfers/deltas/fees) → Validation (±5% tolerance) → Update (position fields in DB and memory) → State transition (verified state)
- Verification Retries - 3-tier table: 1-5 attempts (10s), 6-10 attempts (30s), 11+ attempts (60s), max 20 retries (~15 min)
- Verified vs Unverified Positions - 5-field comparison table
- Database Schema - 6 tables overview:
  - positions (main records, 45+ columns for all metadata/P&L/DCA/partial exit fields)
  - position_entries (historical entry records: position_id, timestamp, amount, price, sol_spent, signature, is_dca, fees_lamports)
  - position_exits (historical exit records: position_id, timestamp, amount, price, sol_received, signature, is_partial, percentage, fees_lamports)
  - position_states (state transition history: position_id, state, changed_at, reason)
  - position_tracking (price update history: position_id, price, price_source, pool_type, tracked_at)
  - token_snapshots (token metadata at open/close for historical analysis)
- Best Practices - Position Sizing:
  - Configure 4-5 preset entry sizes in trader.entry_sizes
  - Never risk >5% of wallet on single position
  - Reserve 2x initial entry for potential DCA
  - Limit concurrent positions via positions.max_open_positions
- Best Practices - Entry Timing:
  - Wait for confirmation (2-3 pool updates, 60-90s)
  - Check pool liquidity (>5 SOL for entries >0.01 SOL)
  - Respect position open cooldown (60s between different tokens)
  - DCA discipline (only if price dropped >15% or strong conviction)
- Best Practices - Exit Timing:
  - Set profit targets (partial exits at predefined levels)
  - Stop-loss discipline (-30% threshold)
  - Time decay (consider exiting stagnant positions >7 days at <5% P&L)
  - Liquidity risk (gradual partial exits for <2 SOL pool liquidity tokens)
- Best Practices - Monitoring:
  - Check current_price_updated timestamp (<2 min old)
  - Unverified positions >5 min indicate RPC issues
  - Phantom confirmations >0: check for manual transfers
  - Verify average_entry_price and remaining_token_amount accuracy for DCA/partial exit positions
- Troubleshooting - 5 comprehensive issues with diagnosis and solutions:
  - Position not appearing after entry
  - Unrealized P&L showing "—"
  - DCA button disabled/grayed out
  - Partial exit amount mismatch
  - Unexpected synthetic exit
  - Verification stuck in pending state
- Performance Tips - Database maintenance, price update optimization, state persistence in localStorage
- API Reference - 3 endpoints documented:
  - GET /api/positions (query params: status, limit, mint)
  - GET /api/positions/:key/details (returns position, executions, transactions, state_history)
  - GET /api/positions/stats (returns total, open, closed, total_invested_sol, total_pnl)
- Related Documentation - Links to 6 related pages (Home Dashboard, Trader Configuration, Strategies, Transactions, Tokens, Events)

**Research Sources Used:**

- `src/webserver/routes/positions.rs` - PositionResponse structure with 29 fields (id, mint, symbol, name, logo_url, entry_price, entry_time, exit_price, exit_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, entry_transaction_signature, exit_transaction_signature, token_amount, effective_entry_price, effective_exit_price, sol_received, profit_target_min, profit_target_max, liquidity_tier, transaction_entry_verified, transaction_exit_verified, entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated, phantom_confirmations, synthetic_exit, closed_reason), PositionDetailResponse with 4 sections (position, executions, transactions, state_history), load_positions_with_filters() async function for status filtering ("open", "closed", "all")
- `src/positions/types.rs` - Position structure with 43 fields including DCA fields (dca_count, average_entry_price, last_dca_time) and Partial Exit fields (remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count), EntryRecord structure (id, position_id, timestamp, amount, price, sol_spent, transaction_signature, is_dca, fees_lamports), ExitRecord structure (id, position_id, timestamp, amount, price, sol_received, transaction_signature, is_partial, percentage, fees_lamports)
- `src/positions/operations.rs` - open_position_impl() with global position permit (acquire_global_position_permit), per-mint lock (acquire_position_lock), duplicate position guards (is_open_position check, DB guard), cooldown checks, DCA swap pending state (register_pending_dca_swap), partial exit pending state (register_pending_partial_exit)
- `src/positions/db.rs` - Database schema with 6 tables: positions (45+ columns), position_entries, position_exits, position_states, position_tracking, token_snapshots, POSITION_SELECT_COLUMNS constant with all field names, save_position() async function, update_position_price_fields() for price updates
- `src/webserver/templates/scripts/pages/positions.js` - DataTable implementation with dynamic columns based on view (open/closed), buildColumns() function returning different column arrays, SUB_TABS constant with open/closed tabs, tokenCell/priceCell/solCell/pnlCell/percentCell/timeCell/dcaCell/partialExitsCell/currentSizeCell render functions, switchView() function updating columns and stateKey, loadPositionsPage() async function fetching /api/positions with status and limit params, row actions handling (data-action="add" or "sell" with data-mint attribute), TradeActionDialog integration for Add/Sell actions
- `src/positions/lib.rs` - calculate_position_pnl() function with (pnl_sol, pnl_percent) return tuple, handles DCA with average_entry_price, partial exits with remaining_token_amount, fallback to effective_entry_price and entry_price, returns (0.0, 0.0) for invalid entry prices
- `src/positions/price_updater.rs` - start_price_updater() background service, updates current_price and unrealized_pnl every 30s from pool data, updates position.unrealized_pnl and position.unrealized_pnl_percent in-memory and DB
- `src/positions/verifier.rs` - Verification queue with retry logic, VerificationItem with is_dca and is_partial_exit flags, PositionTransition enum (DcaVerified, PartialExitVerified), tolerance checks (±5%), amount matching for partial exits
- `src/positions/apply.rs` - apply_transition() function handling DcaVerified and PartialExitVerified transitions, updates position fields, recalculates unrealized P&L after partial exit, saves to DB
- `src/positions/state.rs` - Global POSITIONS RwLock<Vec<Position>>, MINT_TO_POSITION_INDEX HashMap for O(1) lookups, SIG_TO_MINT_INDEX for signature to mint mapping, is_open_position() check, pending state management (PendingDcaSwap, PendingPartialExit)

**Key Information Provided:**

- Positions system supports full lifecycle: open → DCA → partial exits → close with comprehensive tracking
- DCA weighted average calculation recalculates entry price across all entries
- Partial exits track remaining_token_amount, total_exited_amount, weighted average_exit_price
- Real-time P&L updates every 30s from Pool Service price_updater background task
- Unrealized P&L formula adjusted for partial exits (remaining value + realized from exits)
- Transaction verification uses 7-step chain validation with retry logic (max 20 attempts)
- Phantom position detection triggers after 3 consecutive zero-balance confirmations (180s)
- Position states tracked in DB: Open, EntryPending, EntryVerified, Closing, ExitPending, Closed
- DCA has 4 safeguards: cooldown timer, per-position lock, balance verification, pending state
- Partial exit strategies documented: Take Profits Ladder, Risk Reduction, Trailing Stop, Time-Based
- Database has 6 tables with complete entry/exit history tracking
- API endpoints: GET /api/positions (list), GET /api/positions/:key/details (detail), GET /api/positions/stats (summary)
- All calculations use SOL as monetary unit (no USD conversions)
- Color coding: green (profit/positive), red (loss/negative), yellow (warnings)

**User-Focused Approach:**

- Clear explanation of Open vs Closed view differences with specialized columns
- Step-by-step DCA and partial exit workflows with dialog screenshots described
- Detailed metric explanations (average entry/exit price, unrealized/realized P&L, position size %)
- 4 partial exit strategies with use cases and execution details
- Position lifecycle visualized as 6-state flow with triggers and transitions
- Phantom position detection explained with causes and prevention
- 5 comprehensive troubleshooting scenarios with diagnosis steps and multi-point solutions
- Best practices organized by category: sizing, entry timing, exit timing, monitoring
- Database schema overview for advanced users (can query with sqlite3)
- Performance tips for large position counts
- API reference for custom integrations
- No algorithm implementation details or proprietary logic exposed

**Status:** Complete and ready for user review ✓

---

- Daily Change: Absolute and percentage change since 00:00 UTC today (green for profit, red for loss)
- Tokens: SPL token count in wallet (excludes SOL)
- Tokens Worth: Combined value of all token holdings converted to SOL
- Start of Day: Baseline balance at 00:00 UTC for daily change calculation
- Refresh frequency: Updates every dashboard refresh (default 5s), data from wallet snapshots captured every minute
- Section 2: Trader Analytics - Comprehensive performance metrics with 5 time period tabs:
  - Time Periods: Today (00:00 UTC-now), Yesterday (full 24h), This Week (last 7 days), This Month (last 30 days), All Time (complete history)
  - Primary Metrics (Large Cards):
    - Net P&L: Total profit minus loss for period (green profit, red loss)
    - Win Rate: (Winning Trades / Total Trades) × 100, indicates consistency
    - Profit: Total SOL earned from winning trades (gross profit before subtracting losses)
    - Loss: Total SOL lost from losing trades
  - Secondary Metrics (Compact Cards):
    - Buys: Total buy transactions (automated + manual)
    - Sells: Total sell transactions (automated + manual)
    - Max Drawdown: Largest peak-to-trough decline as percentage
  - Statistics Calculation Rules: Based on closed positions only, trade counted at exit time not entry time, DCA entries treated as single position, partial exits create multiple P&L calculations
- Section 3: Positions Snapshot - Real-time open position overview:
  - Count: Number of currently open positions (one per token)
  - Invested: Total SOL invested across all positions (sum of entry costs including DCA)
  - P&L (Unrealized): Profit/loss based on current market prices (not realized until sold, green profit/red loss)
  - P&L % (Unrealized): Percentage return on invested capital, formula: (Unrealized P&L / Total Invested) × 100
  - Updates with latest token prices on every dashboard refresh
- Section 4: System Metrics - Application health monitoring:
  - Uptime: Time since last restart in human-readable format (e.g., "2d 14h 35m")
  - Memory Usage: Absolute MB and percentage with live sparkline chart, thresholds: Normal &lt;500MB, Elevated 500-1000MB, High &gt;1000MB
  - CPU Usage: Percentage with live sparkline chart (20 data points), thresholds: Normal &lt;30%, Active 30-70%, High &gt;70%
  - When to Take Action: Memory constantly increasing (restart), CPU &gt;80% consistently (check rate limits), Uptime &lt;1hr frequently (investigate crashes)
- Section 5: Token Statistics - Discovery and filtering activity:
  - Total in Database: Unique tokens discovered from all sources over time
  - With Prices: Tokens that have current price data available
  - Passed Filters: Tokens eligible for automated trading based on filter criteria
  - Rejected by Filters: Tokens excluded due to filter failures (detailed reasons on Filtering page)
  - Discovery Timeline: Found Today (since 00:00 UTC), Found This Week (last 7 days), Found This Month (last 30 days), Found All Time (same as total)
  - Interpreting Discovery Rates: High rates (&gt;100/day) = active market, low rates = limited sources or quiet market
- Section 6: License Information - NFT license status at bottom of dashboard:
  - License Status: Badge colors - VALID (green, active), CHECKING (yellow, verifying), INVALID/EXPIRED (red, not found/expired)
  - Tier: License level (Basic/Pro/Enterprise) with different feature access
  - Days Remaining: Color-coded warnings - &gt;30 days (green), 7-30 days (yellow, renewal recommended), &lt;7 days (red, expiring soon)
  - NFT Mint Address: Unique Solana address viewable on blockchain explorers, clickable to copy
  - Expiration behavior: Bot stops trading but preserves all data, renewal instructions in License Guide
- Using the Dashboard Effectively - 4 key strategies:
  - Auto-Refresh: Simultaneous updates for all sections, adjust speed based on activity (5s active, 30s/manual monitoring)
  - Period Comparison: Switch timeframes to identify trends, compare today vs yesterday for performance direction
  - Watch Key Indicators: Win rate &gt;50%, Net P&L trend, Unrealized P&L %, System memory
  - Discovery Health Check: Verify "Found Today" &gt;0, check Services page, RPC health, API rate limits
- Troubleshooting - 4 common issues with comprehensive solutions:
  - Data Shows All Zeros: Wait 1-2min for initial collection, verify services running, check wallet config, ensure RPC responding, manual page refresh
  - Data Not Updating: Check auto-refresh enabled, open console for JS errors, verify /api/dashboard/home accessible, clear cache, restart if &gt;5min stale
  - Wrong Trading Statistics: Stats use exit time not entry time, only closed positions count, partial exits = multiple P&L records, UTC timezone for periods, verify on Positions page
  - Charts Not Displaying: Requires Chart.js library, check adblockers, wait for 20 data points minimum, clear cache
- Best Practices - 5 key recommendations:
  - Start Your Day Here: Review overnight performance, wallet changes, system health before detailed pages
  - Set Performance Baselines: Track All Time win rate and P&L, use as baseline to evaluate recent changes
  - Monitor License Expiration: Check Days Remaining regularly, set reminder at &lt;14 days to avoid interruptions
  - Watch System Resources: Investigate if memory &gt;500MB or CPU &gt;70% sustained, affects trading speed
  - Use Period Tabs Strategically: Today for immediate feedback, This Week for patterns, All Time for long-term assessment, focus on weekly/monthly trends not daily obsession

**Research Sources Used:**

- `src/webserver/templates/pages/home.html` - Complete HTML structure with all sections: wallet hero (balance, change, tokens, tokens worth, start of day), trader analytics (period tabs, primary stats 2×2 grid, secondary stats 3-column), positions grid (count, invested, P&L, P&L %), system metrics (uptime, memory with chart, CPU with chart), token statistics (total, priced, passed, rejected, found today/week/month/all-time), license card (status badge, tier, days remaining, mint)
- `src/webserver/templates/scripts/pages/home.js` - Complete JavaScript implementation: Chart.js initialization for memory/CPU sparklines (20 data points, line charts with tension 0.4), fetchData() from /api/dashboard/home, updateUI() with all sections (updateTraderStats, updateWalletStats, updatePositionsStats, updateSystemStats, updateTokenStats, updateLicenseInfo), period tab switching logic, value animations, color coding (profit/loss green/red)
- `src/webserver/routes/dashboard.rs` - API response structures: HomeDashboardResponse with 7 top-level sections, TraderAnalytics with 5 periods (TradingPeriodStats: buys, sells, profit_sol, loss_sol, net_pnl_sol, drawdown_percent, win_rate), WalletAnalytics (current_balance_sol, token_count, tokens_worth_sol, start_of_day_balance_sol, change_sol, change_percent), PositionsSnapshot (open_count, total_invested_sol, unrealized_pnl_sol, unrealized_pnl_percent), SystemMetrics (uptime_seconds, uptime_formatted, memory_mb, memory_percent, cpu_percent, cpu_history, memory_history arrays), TokenStatistics (total_in_database, with_prices, passed_filters, rejected_filters, found_today/week/month/all_time), LicenseInfo (valid, tier, start_ts, expiry_ts, mint, days_remaining)
- `src/webserver/routes/dashboard.rs` (lines 320-450) - get_home_dashboard() implementation: Time period calculations using chrono (today_start at 00:00 UTC, yesterday_start, week_start, month_start, epoch_start), calculate_period_stats() helper iterating closed positions filtered by exit_time, P&L aggregation (profit_sol, loss_sol, total_pnl, winning_trades), win rate calculation, max drawdown tracking
- `src/wallet.rs` - WalletSnapshot structure: sol_balance, token_count, tokens_worth_sol, start_of_day_balance (baseline for daily change), TokenBalance array with mint/decimals/balance/price_sol, get_current_wallet_status() API, snapshots captured every minute in background
- `src/positions/metrics.rs` - ProceedsMetricsSnapshot structure: accepted_quotes, rejected_quotes, accepted_profit_quotes, accepted_loss_quotes, total_shortfall_bps_sum, worst_shortfall_bps, average_shortfall_bps, last_update_unix (atomic counters for thread-safe tracking)

**Key Information Provided:**

- Home dashboard combines 6 major information sections into single view
- Wallet analytics hero card shows current balance with 4 decimal precision, daily changes since 00:00 UTC with absolute and percentage
- Trader analytics supports 5 time periods with instant tab switching, no page reload needed
- Statistics calculated from closed positions only using exit_time for period filtering
- Primary trading metrics (Net P&L, Win Rate, Profit, Loss) displayed prominently in 2×2 grid
- Secondary metrics (Buys, Sells, Max Drawdown) in compact 3-column layout
- Positions snapshot shows real-time unrealized P&L based on current market prices
- System metrics include live sparkline charts with 20 data points for memory and CPU
- Memory thresholds: Normal &lt;500MB, Elevated 500-1000MB, High &gt;1000MB indicating potential issues
- CPU thresholds: Normal &lt;30%, Active 30-70%, High &gt;70% sustained may affect performance
- Token statistics track discovery rates across multiple timeframes (today/week/month/all-time)
- License information with color-coded warnings: &gt;30 days green, 7-30 days yellow, &lt;7 days red
- Dashboard auto-refreshes every 5 seconds by default (configurable)
- All sections update simultaneously on each refresh
- Period comparison enables trend identification and performance evaluation
- UTC timezone used consistently for all time-based calculations
- DCA entries treated as part of single position for statistics
- Partial exits create multiple P&L calculations for same position

**User-Focused Approach:**

- Each metric clearly explained with purpose and calculation method
- Color coding explained: green = profit/positive, red = loss/negative
- Practical thresholds provided for system resource interpretation
- Time period definitions explicit (00:00 UTC today, last 7 days, etc.)
- Statistics calculation rules clarified (exit time, closed positions only, DCA/partial handling)
- Comprehensive troubleshooting for 4 common issues with step-by-step solutions
- Best practices focus on daily workflow and performance monitoring
- Strategic guidance on using period tabs and key indicators to watch
- Discovery health checks to verify system is functioning correctly
- License expiration warnings with proactive renewal recommendations
- No backend implementation details, Chart.js configuration details, or algorithm logic exposed

**Status:** Complete and ready for user review ✓
