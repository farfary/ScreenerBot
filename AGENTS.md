# AI Agent Instructions for ScreenerBot

> **This file is for AI coding agents** (an LLM provider, Cursor, Windsurf, Cline, etc.) that assist contributors working on ScreenerBot. If you are an AI agent, follow these instructions before taking any action.

## Before You Do Anything

**MANDATORY: Read the docs first.**

Before modifying code, creating files, researching, reviewing pull requests, writing issues, or taking any action in this repository:

1. **Read [`docs/README.md`](docs/README.md)** — Understand the documentation structure and what's available.
2. **Read the relevant architecture doc** in `docs/architecture/` for the system you're working on.
3. **Check `docs/investigations/`** for any past deep-dives related to your task.
4. **Read [`CONTRIBUTING.md`](CONTRIBUTING.md)** for code style and contribution guidelines.

Only after reading the relevant docs should you look at source code files.

## Repository Structure

```
ScreenerBot/
├── src/                    # Rust source code (trading engine)
│   ├── config/             # Macro-driven configuration system
│   ├── pools/              # DEX pool discovery, decoding, pricing
│   ├── tokens/             # Token database, market data, security
│   ├── filtering/          # Token quality filtering pipeline
│   ├── trader/             # Automated + manual trading
│   ├── swaps/              # Multi-DEX swap routing
│   ├── positions/          # Position management, P&L tracking
│   ├── strategies/         # Condition-based trading strategies
│   ├── transactions/       # Real-time transaction monitoring
│   ├── ohlcvs/             # OHLCV candlestick data
│   ├── wallets/            # Wallet balance monitoring
│   ├── ai/                 # LLM-powered analysis + chat
│   ├── telegram/           # Telegram bot integration
│   ├── webserver/          # Axum REST API + embedded dashboard
│   ├── services/           # ServiceManager with dependency resolution
│   ├── apis/               # External API clients
│   ├── rpc/                # Solana RPC with multi-provider support
│   ├── connectivity/       # Endpoint health monitoring
│   ├── events/             # Structured event logging
│   ├── errors/             # Error types
│   └── global.rs           # Global state, startup flags, constants
├── electron/               # Electron shell (desktop app wrapper)
├── crates/                 # Local Rust crate dependencies
├── docs/                   # Technical documentation
│   ├── architecture/       # Living system docs (kept current)
│   ├── development/        # Build guides, CLI reference
│   └── investigations/     # Historical deep-dive analyses
├── assets/                 # Build assets
├── icons/                  # App icons
├── Cargo.toml              # Rust dependencies
├── package.json            # Electron dependencies
├── screenerbot.sh          # Linux/macOS installer script
└── AGENTS.md               # This file
```

## Architecture Quick Reference

| System | Entry Point | Doc |
|--------|------------|-----|
| Configuration | `src/config/` — `config_struct!` macro | [docs/architecture/overview.md](docs/architecture/overview.md) |
| Pool Discovery | `src/pools/discovery.rs` | [docs/architecture/overview.md](docs/architecture/overview.md) |
| Token Filtering | `src/filtering/engine.rs` | [docs/architecture/filtering.md](docs/architecture/filtering.md) |
| Swap Execution | `src/swaps/router.rs` | [docs/architecture/swaps.md](docs/architecture/swaps.md) |
| Trading Engine | `src/trader/` (entry.rs, exit.rs) | [docs/development/trader-overview.md](docs/development/trader-overview.md) |
| Strategies | `src/strategies/engine.rs` | [docs/architecture/strategies.md](docs/architecture/strategies.md) |
| OHLCV Data | `src/ohlcvs/monitor.rs` | [docs/architecture/ohlcv-strategy-integration.md](docs/architecture/ohlcv-strategy-integration.md) |
| Positions | `src/positions/state.rs` | [docs/architecture/positions.md](docs/architecture/positions.md) |
| Service Manager | `src/services/mod.rs` | [docs/architecture/startup-order.md](docs/architecture/startup-order.md) |
| Dashboard | `src/webserver/` | [docs/architecture/overview.md](docs/architecture/overview.md) |

## Coding Conventions

### Rust

- **Config access**: Always use `with_config()` or `get_config_clone()` — never hardcode values.
- **Config sections**: Must use `config_struct!` macro (see `src/config/macros.rs`).
- **Database**: SQLite via rusqlite + r2d2. Use `with_init()` for PRAGMA settings.
- **Error handling**: Use `ScreenerBotError` variants from `src/errors/`.
- **Logging**: Use `error!()`, `warning!()`, `info!()`, `debug!()`, `verbose!()` with `LogTag`.
- **Services**: Implement `Service` trait, register in `src/services/implementations/`.
- **Global state**: Check `src/global.rs` for startup flags before accessing services.

### Dashboard (Embedded HTML/CSS/JS)

- **Templates**: `src/webserver/templates/` — HTML with embedded Rust data.
- **Assets**: Embedded via `include_str!`/`include_bytes!` in `src/webserver/embeds.rs`.
- **New JS/CSS files** require: entry in `embeds.rs` + match arm in `asset_serving.rs` + style array in `templates.rs`.
- **Toggles**: Use `.toggle` class from `form_controls.css` — never create custom toggle CSS.
- **Z-index**: Use CSS variables from `floating.css` (`--z-dropdown`, `--z-dialog`, etc.) — never hardcode.
- **Icon buttons**: Use `.btn-icon` class from `components.css`.

### Git

- Small, focused commits with descriptive messages.
- Run `cargo build` before committing Rust changes.
- Run `cargo clippy` to check for warnings.

## Issue Reporting Standards

When creating issues, include:

1. **Environment**: OS, ScreenerBot version, Rust version.
2. **Steps to reproduce**: Exact sequence of actions.
3. **Expected behavior**: What should happen.
4. **Actual behavior**: What actually happens.
5. **Logs**: Relevant log output (from `~/Library/Application Support/screenerbot/logs/latest.log` on macOS, or `~/.local/share/screenerbot/logs/latest.log` on Linux).
6. **Related docs**: Link to relevant docs in `docs/` if applicable.

## Pull Request Standards

When submitting PRs:

1. **Reference the relevant architecture doc** that describes the system you're changing.
2. **Describe what changed and why** — not just "fixed bug".
3. **If adding a new system**: Create an architecture doc in `docs/architecture/`.
4. **If doing deep analysis**: Create an investigation folder in `docs/investigations/YYYY-MM-topic/`.
5. **Update `docs/README.md`** index if you add new docs.
6. **No sensitive content**: No API keys, tokens, passwords, server IPs, or deployment configs.
7. **Build passes**: `cargo build --release` must succeed.

## Security Rules

- **Never commit**: API keys, private keys, wallet seeds, server IPs, deployment configs, passwords.
- **Public constants are OK**: Jupiter referral account, donation address (these are intentionally public).
- **Scan before commit**: `git grep -i 'api_key\|secret\|password\|private_key\|seed_phrase'`

## Documentation Rules

| Category | Location | When to Update |
|----------|----------|---------------|
| How a system works | `docs/architecture/` | When system behavior changes |
| How to build/develop | `docs/development/` | When build process or tools change |
| Deep analysis/investigation | `docs/investigations/YYYY-MM-topic/` | After completing an investigation |
| This index | `docs/README.md` | When adding any new doc |

### Architecture docs are living documents

If you change how a system works, **update the architecture doc**. Outdated docs are worse than no docs.

### Investigation docs are immutable

Investigation folders are historical records. They describe what was found at that point in time. **Never modify** past investigations — create a new one if doing follow-up work.

### Naming conventions

- Folders: `kebab-case` (e.g., `2026-02-memory`)
- Files: `kebab-case.md` (e.g., `filtering.md`, `cli-reference.md`)
- Investigation folders: `YYYY-MM-topic` format
