# ScreenerBot Documentation

Technical documentation for ScreenerBot — a Solana DeFi trading automation system.

## Structure

```
docs/
├── architecture/          # How systems work (living docs, kept current)
├── development/           # How to build, develop, and contribute
└── investigations/        # Deep technical analyses (dated, historical)
    ├── 2025-10-blacklist/
    ├── 2025-10-fixes/
    ├── 2025-10-trader/
    ├── 2025-11-dashboard-performance/
    ├── 2025-11-notifications/
    ├── 2025-11-rpc-metrics/
    ├── 2025-11-timestamps/
    ├── 2025-11-trader-reorganization/
    ├── 2025-11-wallet/
    ├── 2025-12-fixes/
    ├── 2025-12-frontend/
    ├── 2025-ai-backend/
    ├── 2025-jupiter-referral/
    ├── 2025-license-gating/
    ├── 2025-linux-x11/
    ├── 2025-token-details/
    ├── 2026-02-memory/
    ├── 2026-02-onchain-filtering/
    └── 2026-02-token-00-scam/
```

### 📐 Architecture

Living documentation describing how each major system works. These docs are the source of truth for system behavior and should be updated when the code changes.

| Document | Lines | Description |
|----------|-------|-------------|
| [System Overview](architecture/overview.md) | 287 | High-level system map, reading order, module connections |
| **Core Trading** | | |
| [Tokens Module](architecture/tokens.md) | 1936 | Token lifecycle, database schema, caching, market data, security |
| [Pools Module](architecture/pools.md) | 910 | Pool discovery, DEX decoders, price calculation, swap integration |
| [Positions Module](architecture/positions.md) | 849 | Position lifecycle, state machine, verification, PnL tracking |
| [OHLCV Module](architecture/ohlcvs.md) | 861 | Candle data fetching, aggregation, caching, strategy integration |
| [Trader Engine](architecture/trader.md) | 538 | Dual monitors, entry pipeline, priority exits, safety system |
| [Trading Strategies](architecture/strategies.md) | 407 | Condition-based strategy system, rule tree, evaluation engine |
| [Filtering Pipeline](architecture/filtering.md) | 1043 | Token quality control — filter chain, sources, caching, query system |
| **Execution** | | |
| [Swap Routing](architecture/swaps.md) | 494 | Multi-router quote+execute, fallback chain, Jupiter referral fee |
| [Transactions](architecture/transactions.md) | 416 | Transaction monitoring, analyzer pipeline, 10+ DEX detection |
| **Data Sources** | | |
| [RPC Module](architecture/rpc.md) | 610 | Multi-provider Solana RPC, circuit breaker, rate limiting |
| [APIs Module](architecture/apis.md) | 412 | External HTTP clients + LLM providers + SOL price service |
| **User Interface** | | |
| [Webserver](architecture/webserver.md) | 1096 | Axum server lifecycle, middleware gates, embedded assets, headless auth, performance patterns |
| [Telegram](architecture/telegram.md) | 364 | Telegram bot: discovery, notifications, commands, sessions/2FA |
| **System** | | |
| [Config](architecture/config.md) | 482 | TOML configuration, macro system, hot reload, metadata |
| [Services](architecture/services.md) | 457 | Service manager, lifecycle, cached health/metrics, registered services |
| [Wallets](architecture/wallets.md) | 670 | Multi-wallet key storage (wallets.db) + wallet monitoring (wallet.db) |
| [Infrastructure](architecture/infrastructure.md) | 841 | SQLite tuning+maintenance, errors, events, logging, connectivity, actions |

### 🛠 Development

Guides for building, developing, and contributing to ScreenerBot.

| Document | Description |
|----------|-------------|
| [Building](development/building.md) | Cross-platform build guide (macOS, Windows, Linux) |
| [Build Scripts Analysis](development/build-scripts-analysis.md) | Build scripts analysis and documentation |
| [CLI Reference](development/cli-reference.md) | Command-line arguments and options |
| [Assistant Auth API](development/Assistant-auth-api.md) | an LLM provider OAuth integration |
| [Assistant Auth Reference](development/Assistant-auth-reference.md) | Quick reference for Assistant auth flow |
| [Debug Manual Trading](development/debug-manual-trading.md) | Manual trading debug guide |
| [Early CLI Validation](development/early-cli-validation.md) | Early CLI argument validation |
| [Request Manager](development/request-manager-reference.md) | Request manager quick reference |
| [Systematic Completion](development/systematic-completion-plan.md) | Systematic completion plan |
| [Trader Overview](development/trader-overview.md) | Trader module quick summary |
| [Trader Migration](development/trader-migration.md) | Trader module reorganization guide |
| [Verification Checklist](development/verification-checklist.md) | Verification checklist |
| [Documentation Plan](development/documentation-plan.md) | Documentation planning and roadmap |
| [TODOs Archive](development/todos-archive.md) | Archived TODO items and task tracking |

### 🔬 Investigations

Historical deep-dive technical analyses. These are **immutable records** — they describe what was found and decided at a specific point in time. They are not updated when code changes.

| Investigation | Date | Description |
|--------------|------|-------------|
| [Blacklist System](investigations/2025-10-blacklist/) | Oct 2025 | Blacklist implementation, investigation, and simplification |
| [Bug Fixes & Audits](investigations/2025-10-fixes/) | Oct 2025 | P0 fixes, exit monitor, pool decoder, metrics planning (11 docs) |
| [Trader Foundation](investigations/2025-10-trader/) | Oct 2025 | Initial trader planning, improvement roadmap, Phase 2 design |
| [Dashboard Performance](investigations/2025-11-dashboard-performance/) | Nov 2025 | Dashboard loading analysis, frontend performance fixes (6 docs) |
| [Trader Reorganization](investigations/2025-11-trader-reorganization/) | Nov 2025 | Major trader module reorg, retry system, UI/UX standardization (14 docs) |
| [Notifications](investigations/2025-11-notifications/) | Nov 2025 | Toast system implementation, panel improvements |
| [RPC Metrics](investigations/2025-11-rpc-metrics/) | Nov 2025 | RPC metrics fix and investigation (2 docs) |
| [Wallet](investigations/2025-11-wallet/) | Nov 2025 | Wallet improvements and loading investigation |
| [Timestamps](investigations/2025-11-timestamps/) | Oct-Nov 2025 | Timestamp fields, restructuring, timeframe system |
| [Bug Fixes](investigations/2025-12-fixes/) | Dec 2025 | Deep investigation and multiple fix rounds |
| [Frontend](investigations/2025-12-frontend/) | Dec 2025 | Frontend fixes and systematic migration guide |
| [AI Backend](investigations/2025-ai-backend/) | 2025 | AI backend review, critical fixes, chat bug report |
| [Jupiter Referral](investigations/2025-jupiter-referral/) | 2025 | Jupiter referral program research and implementation guide |
| [License Gating](investigations/2025-license-gating/) | 2025 | License-gated initialization architecture design |
| [Linux X11](investigations/2025-linux-x11/) | 2025 | Linux X11 dependency investigation for cross-platform builds |
| [Token Details](investigations/2025-token-details/) | 2025 | Token details dialog improvement plan and gap analysis |
| [Memory Optimization](investigations/2026-02-memory/) | Feb 2026 | Root cause analysis of 804MB+ startup RSS, 10-component architecture plan (56 research docs) |
| [On-Chain Filtering](investigations/2026-02-onchain-filtering/) | Feb 2026 | On-chain scam detection: symbol analysis, authority reputation, risk scoring |
| [Token "00" Scam](investigations/2026-02-token-00-scam/) | Feb 2026 | Investigation of scam tokens with "00" symbol pattern |

## Contributing Documentation

When adding new documentation:

1. **Architecture docs** → `architecture/` — Must describe current system behavior. Update when code changes.
2. **Dev guides** → `development/` — How-to guides for building, testing, contributing.
3. **Investigations** → `investigations/YYYY-MM-topic/` — Create a dated folder with a `README.md` summary.
4. **Update this index** — Add a row to the appropriate table above.
5. Use kebab-case filenames: `my-new-doc.md` (not `MY_NEW_DOC.md`).
6. No sensitive content — no API keys, tokens, passwords, server IPs, or deployment details.
