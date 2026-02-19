# ScreenerBot Documentation

Technical documentation for ScreenerBot — a Solana DeFi trading automation system.

## Structure

```
docs/
├── architecture/          # How systems work (living docs, kept current)
├── development/           # How to build, develop, and contribute
└── investigations/        # Deep technical analyses (dated, historical)
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
| [Webserver Port Conflict](architecture/webserver-port-conflict.md) | Port conflict detection and resolution |
| [Workflow Order](architecture/workflow-order.md) | Service dependency resolution and startup sequencing |

### 🛠 Development

Guides for building, developing, and contributing to ScreenerBot.

| Document | Description |
|----------|-------------|
| [Building](development/building.md) | Cross-platform build guide (macOS, Windows, Linux) |
| [CLI Reference](development/cli-reference.md) | Command-line arguments and options |
| [Assistant Auth API](development/Assistant-auth-api.md) | an LLM provider OAuth integration |
| [Assistant Auth Reference](development/Assistant-auth-reference.md) | Quick reference for Assistant auth flow |
| [Trader Overview](development/trader-overview.md) | Trader module quick summary |
| [Trader Migration](development/trader-migration.md) | Trader module reorganization guide |

### 🔬 Investigations

Historical deep-dive technical analyses. These are **immutable records** — they describe what was found and decided at a specific point in time. They are not updated when code changes.

| Investigation | Date | Description |
|--------------|------|-------------|
| [Memory Optimization](investigations/2026-02-memory/) | Feb 2026 | Root cause analysis of 804MB+ startup RSS, 10-component architecture plan |

## Contributing Documentation

When adding new documentation:

1. **Architecture docs** → `architecture/` — Must describe current system behavior. Update when code changes.
2. **Dev guides** → `development/` — How-to guides for building, testing, contributing.
3. **Investigations** → `investigations/YYYY-MM-topic/` — Create a dated folder with a `README.md` summary.
4. **Update this index** — Add a row to the appropriate table above.
5. Use kebab-case filenames: `my-new-doc.md` (not `MY_NEW_DOC.md`).
6. No sensitive content — no API keys, tokens, passwords, server IPs, or deployment details.
