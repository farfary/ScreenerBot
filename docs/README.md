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
    └── 2026-02-memory/
```

### 📐 Architecture

Living documentation describing how each major system works. These docs are the source of truth for system behavior and should be updated when the code changes.

| Document | Description |
|----------|-------------|
| [System Overview](architecture/overview.md) | High-level architecture, data flow, service startup order |
| [Filtering Pipeline](architecture/filtering.md) | Token quality control — criteria, evaluation, snapshots |
| [Swap Routing](architecture/swaps.md) | Trait-based multi-DEX router architecture |
| [Trading Strategies](architecture/strategies.md) | Condition-based strategy system |
| [OHLCV Integration](architecture/ohlcv-strategy-integration.md) | Multi-timeframe data flow for strategy evaluation |
| [Partial Sell & DCA](architecture/partial-sell-dca.md) | Position management: DCA entries, partial exits |
| [Positions & Actions](architecture/positions.md) | Position lifecycle, entry/exit records |
| [Startup Order](architecture/startup-order.md) | Service startup vs. user-facing workflow order |
| [Token Details](architecture/token-details-improvements.md) | Token details dialog improvements |
| [Webserver Port Conflict](architecture/webserver-port-conflict.md) | Port conflict detection and resolution |
| [Port Conflict Implementation](architecture/webserver-port-conflict-implementation.md) | Port conflict implementation details |
| [Port Conflict Solution](architecture/webserver-port-conflict-solution.md) | Port conflict solution design |
| [Port Conflict Index](architecture/webserver-port-conflict-index.md) | Port conflict decision index |
| [Port Conflict Quick Ref](architecture/webserver-port-conflict-quickref.md) | Port conflict quick reference |
| [Port Conflict Summary](architecture/webserver-port-conflict-summary.md) | Port conflict solution summary |
| [Workflow Order](architecture/workflow-order.md) | Service dependency resolution and startup sequencing |

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
| [Memory Optimization](investigations/2026-02-memory/) | Feb 2026 | Root cause analysis of 804MB+ startup RSS, 10-component architecture plan (56 research docs) |

## Contributing Documentation

When adding new documentation:

1. **Architecture docs** → `architecture/` — Must describe current system behavior. Update when code changes.
2. **Dev guides** → `development/` — How-to guides for building, testing, contributing.
3. **Investigations** → `investigations/YYYY-MM-topic/` — Create a dated folder with a `README.md` summary.
4. **Update this index** — Add a row to the appropriate table above.
5. Use kebab-case filenames: `my-new-doc.md` (not `MY_NEW_DOC.md`).
6. No sensitive content — no API keys, tokens, passwords, server IPs, or deployment details.
