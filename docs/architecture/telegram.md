# Telegram Module — Architecture

> ScreenerBot Telegram Bot Integration — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Bot Lifecycle](#4-bot-lifecycle)
5. [Commands](#5-commands)
6. [Notifications](#6-notifications)
7. [Session & Auth](#7-session--auth)
8. [Module Connections](#8-module-connections)

---

## 1. Overview

The Telegram module provides bot-based remote control and notifications via Teloxide. Supports two modes: Discovery (waiting for user to message bot) and Connected (active command handling + notifications). Includes 2FA/TOTP authentication.

**Key characteristics:**
- Teloxide framework (Rust Telegram bot library)
- Discovery mode for initial setup (no manual chat_id needed)
- 10 notification types covering all trading events
- Command handling for remote trading control
- Session management with optional TOTP 2FA
- Async notification queue (mpsc channel, capacity 100)
- Inline keyboards with pagination

**16 files, ~6,035 lines**

---

## 2. File Structure

```
src/telegram/
├── mod.rs              # Public API & re-exports
├── service.rs          # TelegramService (Service trait)
├── bot.rs              # Bot instance & lifecycle
├── notifier.rs         # Message sending & formatting
├── polling.rs          # Update polling loop
├── discovery.rs        # Chat ID discovery
├── session.rs          # Session & auth management
├── types.rs            # Core types
├── formatters.rs       # HTML message formatting
├── keyboards.rs        # Inline/reply keyboard builders
├── pagination.rs       # Token list pagination
└── commands/           # Command handlers
    ├── mod.rs           # Router & dispatcher
    ├── trading.rs       # Start/stop/pause/resume
    ├── status.rs        # Status/balance/positions
    ├── menu.rs          # Interactive menus
    └── callbacks.rs     # Button click handlers
```

---

## 3. Core Types

### BotState

```rust
pub enum BotState {
    Disconnected,     // No token configured
    Discovery,        // Waiting for user to message bot
    Connected,        // Chat ID known, active
}
```

### Notification

```rust
pub enum Notification {
    TradeAlert { token_symbol, token_mint, trade_type, amount_sol },
    PositionOpened { token_symbol, token_mint, amount_sol, entry_price, ai_reasoning },
    PositionClosed { token_symbol, token_mint, pnl_sol, pnl_percent, exit_reason, duration },
    PartialExit { token_symbol, token_mint, exit_percent, pnl_sol, remaining_percent },
    DcaExecuted { token_symbol, token_mint, dca_amount_sol, total_invested_sol, dca_count },
    SystemError { message, severity },
    DailySummary { date, total_trades, winning, losing, total_pnl_sol, open_positions },
    BotCommand { command, response },
    BotStarted { version, mode },
    BotStopped,
    NewTokensFound { session_id, new_count },
}
```

---

## 4. Bot Lifecycle

### Initialization

```
TelegramService::initialize()
├─ Check config: telegram.enabled?
├─ Validate bot_token via bot.get_me()
├─ Determine state:
│  ├─ No token → Disconnected
│  ├─ Token + no chat_id → Discovery
│  └─ Token + chat_id → Connected
└─ Create notification queue (mpsc, cap=100)
```

### Start

```
TelegramService::start()
├─ Spawn notification sender task
├─ If Discovery:
│  └─ Spawn discovery polling task
├─ If Connected:
│  └─ Spawn command polling task
└─ Return task handles
```

---

## 5. Commands

### Trading Commands

| Command | Purpose |
|---------|---------|
| `/start` | Initialize bot interaction |
| `/stop` | Stop trading |
| `/pause` | Pause new entries |
| `/resume` | Resume trading |
| `/force_stop` | Emergency: exit all positions |

### Status Commands

| Command | Purpose |
|---------|---------|
| `/status` | Trading status overview |
| `/balance` | Wallet SOL + token balances |
| `/positions` | Open positions list |
| `/stats` | Trading statistics |

### Menu Commands

| Command | Purpose |
|---------|---------|
| `/menu` | Main menu with inline buttons |
| `/help` | Command reference |

### Dynamic Commands

| Pattern | Purpose |
|---------|---------|
| `/token_XXXXX` | Token detail view |
| Callback buttons | Pagination, close position, details |

---

## 6. Notifications

### Queue Architecture

```
Services → queue_notification() → mpsc::channel(100) → Sender Task → Teloxide → Telegram API
```

- `queue_notification()` is non-blocking (try_send)
- Sender task batches and rate-limits outgoing messages
- HTML formatting with ParseMode::Html

### Message Formatting

Messages use HTML with:
- Bold headers for notification type
- Emoji indicators (📈 buy, 📉 sell, ⚠️ error)
- Inline token links to Solscan
- P&L percentages with color coding

---

## 7. Session & Auth

```rust
pub struct TelegramSessionManager {
    sessions: HashMap<ChatId, SessionState>,
}

pub enum SessionState {
    Unauthenticated,
    AwaitingTotp,
    Authenticated { expires_at: DateTime<Utc> },
}
```

- Optional TOTP 2FA via `webserver::totp` module
- Session timeout configurable
- Per-chat authentication state
- Commands blocked until authenticated (when 2FA enabled)

---

## 8. Module Connections

```
telegram/
├── config/       ← Bot token, chat_id, notification settings
├── trader/       ← Start/stop/pause commands
├── positions/    ← Position data for /positions
├── wallets/      ← Balance data for /balance
├── tokens/       ← Token data for /token_X
├── webserver/    ← TOTP 2FA integration
└── events/       ← Record telegram events
```

| Caller | Usage |
|--------|-------|
| positions/service | `queue_notification(PositionOpened/Closed)` |
| trader | `queue_notification(BotStarted/Stopped)` |
| pool_analyzer | `queue_notification(NewTokensFound)` |
| error handlers | `queue_notification(SystemError)` |
