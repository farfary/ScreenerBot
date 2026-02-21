# Telegram Module — Architecture

> ScreenerBot Telegram bot integration (notifications, discovery, commands, sessions/2FA) — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Global Singletons & Shared State](#4-global-singletons--shared-state)
5. [Service Integration (TelegramService)](#5-service-integration-telegramservice)
6. [Chat Discovery Flow](#6-chat-discovery-flow)
7. [Command Polling & Routing](#7-command-polling--routing)
8. [Session + Authentication (TOTP)](#8-session--authentication-totp)
9. [Notification Pipeline](#9-notification-pipeline)
10. [Pagination for \"New Tokens Found\"](#10-pagination-for-new-tokens-found)
11. [Module Connections](#11-module-connections)

---

## 1. Overview

The `telegram` module provides a standalone Telegram integration based on **teloxide**:

* **Notifications** for trading lifecycle events (position opened/closed, errors, new tokens, etc.)
* **Remote control** via commands and inline keyboards (pause/resume, force-stop, token explorer)
* **Chat discovery** (no manual chat_id copy/paste required)
* **Session state + optional 2FA** using the same TOTP secret as the dashboard lockscreen (`webserver::totp`)

The Telegram system is designed to be safe-by-default:
* It can be fully disabled via config.
* If token/chat_id is not configured, it does not start command polling or notification sending.

---

## 2. File Structure

```text
src/telegram/
├── bot.rs                 Bot instance helpers (used by notifier/polling/discovery)
├── mod.rs                 Public API and re-exports (architecture comment block)
├── types.rs               NotificationType, BotState, SessionState, TelegramSession, DiscoveredChat
├── service.rs              Service trait implementation + global TELEGRAM_SERVICE
├── notifier.rs             TelegramNotifier + global queue helpers + preference filtering
├── polling.rs              Main update polling loop (getUpdates + offset tracking)
├── discovery.rs            Chat discovery polling (captures incoming messages to discover chat IDs)
├── session.rs              TelegramSessionManager singleton (sessions + discovery state)
├── pagination.rs           PaginationManager singleton (DashMap + 15m TTL sessions)
├── formatters.rs           HTML formatting helpers
├── keyboards.rs            Inline keyboard builders (callback data formats)
└── commands/
    ├── mod.rs              Command router + auth gate
    ├── trading.rs          /start /stop /pause /resume /force_stop /login
    ├── status.rs           /status /balance /positions /stats
    ├── menu.rs             /menu and navigation helpers
    └── callbacks.rs        CallbackQuery router for inline keyboard buttons
```

---

## 3. Core Types

**File:** `src/telegram/types.rs`

### 3.1 NotificationType + Notification

Notifications are domain-level events that may be sent to Telegram:

```rust
pub enum NotificationType {
    TradeAlert { token_symbol, token_mint, trade_type, amount_sol, wallet },
    PositionOpened { token_symbol, token_mint, amount_sol, entry_price, ai_reasoning },
    PositionClosed { token_symbol, token_mint, pnl_sol, pnl_percent, exit_reason, entry_price, exit_price, invested, received, duration_secs, ai_reasoning },
    PartialExit { token_symbol, token_mint, exit_percent, pnl_sol, remaining_percent },
    DcaExecuted { token_symbol, token_mint, dca_amount_sol, total_invested_sol, dca_count },
    SystemError { message, severity },
    DailySummary { date, total_trades, winning_trades, losing_trades, total_pnl_sol, open_positions },
    BotCommand { command, response },
    BotStarted { version, mode },
    BotStopped { reason },
    NewTokensFound { session_id, new_count },
}

pub enum ErrorSeverity { Info, Warning, Error, Critical }

pub struct Notification {
    pub notification_type: NotificationType,
    pub timestamp: DateTime<Utc>,
}
```

### 3.2 BotState

```rust
pub enum BotState {
    Disconnected,  // not configured
    Discovery,     // bot token exists, waiting to discover chat_id
    Connected,     // configured and operational
}
```

### 3.3 SessionState + TelegramSession

The Telegram module tracks sessions per user and supports optional 2FA:

* `SessionState::Active` => authenticated
* `SessionState::AwaitingTotp` => user must send a 6-digit code
* `SessionState::Locked { until }` => lockout after repeated failures
* `SessionState::Expired` => timed out due to inactivity

---

## 4. Global Singletons & Shared State

The Telegram subsystem uses several global singletons (intentionally) so any module can safely enqueue notifications or query state.

### 4.1 TelegramService global instance

**File:** `src/telegram/service.rs`

```rust
static TELEGRAM_SERVICE: LazyLock<RwLock<TelegramService>> =
    LazyLock::new(|| RwLock::new(TelegramService::new()));
```

Public access helpers:
* `get_service()` / `get_service_mut()`
* `is_ready()`
* `get_bot_state()`
* `start_discovery_mode()` / `stop_discovery_mode()`

### 4.2 Session manager singleton

**File:** `src/telegram/session.rs`

```rust
static SESSION_MANAGER: LazyLock<TelegramSessionManager> =
    LazyLock::new(TelegramSessionManager::new);
```

This manager owns:
* `sessions: HashMap<i64, TelegramSession>` (keyed by user_id)
* `discovered_chats: Vec<DiscoveredChat>`
* `discovery_active: AtomicBool`

### 4.3 Pagination manager singleton

**File:** `src/telegram/pagination.rs`

```rust
pub static PAGINATION_MANAGER: LazyLock<PaginationManager> =
    LazyLock::new(PaginationManager::new);
```

`PaginationManager` stores short-lived sessions in a `DashMap`:
* TTL: 15 minutes (`SESSION_TTL`)
* default page size: 10 items

### 4.4 Notifier + queue singletons

**File:** `src/telegram/notifier.rs`

Global state:
* `NOTIFIER: LazyLock<RwLock<Option<TelegramNotifier>>>`
* `NOTIFICATION_QUEUE: LazyLock<RwLock<Option<mpsc::Sender<Notification>>>>`

These allow:
* `queue_notification(Notification)` from sync contexts (uses `try_send`, drops if full)
* `send_notification(Notification)` from async contexts (respects user preferences)

---

## 5. Service Integration (TelegramService)

**File:** `src/telegram/service.rs`

The Telegram module integrates with ServiceManager via `TelegramService`:

* `name() = "telegram"`
* `priority() = 50`
* `dependencies() = []`

### 5.1 initialize()

Initialization reads telegram config:

* Disabled => sets state `Disconnected` and returns Ok.
* Enabled but `bot_token` empty => state `Disconnected`.
* Enabled + token:
  * `chat_id` empty => state `Discovery`
  * `chat_id` present => state `Connected` and tries `notifier::init_notifier()`

### 5.2 start()

Start behavior:
* If disabled or `bot_token` empty => returns no handles.
* Always creates an mpsc notification queue:
  * `(tx, rx) = mpsc::channel::<Notification>(100)`
  * `notifier::set_notification_queue(tx)`
* If `chat_id` is configured:
  * spawns a notification worker that `rx.recv()` and calls `send_notification(notification).await`
  * optionally starts command polling if `config.commands_enabled`
  * sends a startup notification `Notification::bot_started(...)`

### 5.3 stop()

Stop behavior:
* if notifier enabled, sends a shutdown notification
* calls `self.shutdown.notify_waiters()` so polling workers exit
* ServiceManager awaits handles with timeouts (see services architecture)

---

## 6. Chat Discovery Flow

Discovery exists to avoid copying chat IDs manually.

### 6.1 DiscoveryService

**File:** `src/telegram/discovery.rs`

`DiscoveryService` long-polls Telegram updates (`getUpdates`) and treats any received message as a discovered chat:
* validates bot token via `bot.get_me()`
* tracks update offsets via `last_update_offset: AtomicI64`
* stores discovered chats via `TelegramSessionManager::add_discovered_chat(...)`
* sends an acknowledgement message to the discovered chat with the chat_id value

### 6.2 Selecting a discovered chat

`DiscoveryService::select_chat(chat_id)`:
* finds the chat in session manager
* writes `cfg.telegram.chat_id = chat_id.to_string()` via `update_config_section(...)`

Once `chat_id` is configured:
* TelegramService can transition from `Discovery` to `Connected` (see `stop_discovery_mode()` and `initialize()` logic)

---

## 7. Command Polling & Routing

### 7.1 Polling loop

**File:** `src/telegram/polling.rs`

* Uses teloxide `get_updates().timeout(30)` for long polling.
* Tracks offset via `static LAST_UPDATE_ID: AtomicI32`:
  * each processed update sets `LAST_UPDATE_ID = update_id + 1`

### 7.2 Dispatch

Updates handled:
* `UpdateKind::Message(message)`
* `UpdateKind::CallbackQuery(query)`

Message handling highlights:
* If `session_manager.is_discovery_active()` => handled by discovery handler and skipped from command routing.
* Otherwise:
  * `get_or_create_session(user_id, chat_id, username, first_name)`
  * If session is awaiting TOTP => `handle_auth_attempt(...)`
  * Else => `handle_command(...)`

CallbackQuery handling:
* Always answers callback first to remove the Telegram loading indicator.
* Routes by `data.split(':')` patterns (pagination, menu navigation, confirmations, token actions).

---

## 8. Session + Authentication (TOTP)

**Files:** `src/telegram/session.rs`, `src/telegram/commands/mod.rs`, `src/telegram/commands/trading.rs`

### 8.1 When auth is required

`commands::handle_command()` defines a set of **sensitive commands** (positions/balance/trading controls, token explorer) that require auth.

### 8.2 /login flow

* `/login`:
  * if `webserver.auth_totp_secret` is empty => session is auto-activated (2FA not configured)
  * else transitions session to `AwaitingTotp` and prompts for 6-digit code

### 8.3 TOTP verification

`TelegramSessionManager::verify_totp(user_id, code)`:
* reads `webserver.auth_totp_secret` from config
* verifies via `webserver::totp::verify_totp(secret, code)`
* on success => `SessionState::Active`
* on repeated failures:
  * increments `failed_attempts`
  * locks session for `telegram.lockout_minutes` after `telegram.max_failed_attempts`

### 8.4 Session timeout

`commands::check_auth()` enforces session timeout via:
* `telegram.session_timeout_minutes`
* transitions to `Expired` when idle too long
* may auto-reactivate when commands do not require 2FA or when TOTP secret is empty

---

## 9. Notification Pipeline

### 9.1 Queueing

**File:** `src/telegram/notifier.rs`

`queue_notification(notification)`:
* uses `try_send` on the global `NOTIFICATION_QUEUE`
* drops when the channel is full (logs a warning)

This is safe to call from synchronous contexts and from many services.

### 9.2 Sending

`send_notification(notification)`:
1) checks preferences via `should_send_notification()`
2) reads `bot_token` and `chat_id` from config (no locks held across await)
3) requires global `NOTIFIER` to be initialized (quick check)
4) creates a **temporary TelegramNotifier** and sends the message

Why temporary notifiers:
* avoids holding global locks across `.await`
* keeps notifier construction simple and stateless

### 9.3 Preferences filtering

`should_send_notification()` gates per notification type using config flags like:
* `notify_trade_alerts` + `trade_alert_min_sol`
* `notify_position_opened`, `notify_position_closed`
* `notify_partial_exit`, `notify_dca_executed`
* `notify_system_errors`
* `notify_daily_summary`
* `notify_filtering_alerts`
* `notify_on_startup`, `notify_on_shutdown`

---

## 10. Pagination for "New Tokens Found"

This feature connects **filtering** -> **telegram** -> **inline keyboard pagination**.

Flow:

1) Filtering refresh finds new passed tokens.
2) FilteringService creates a pagination session:
   * `session_id = PAGINATION_MANAGER.create_session(items, None)`
3) FilteringService enqueues `NotificationType::NewTokensFound { session_id, new_count }`.
4) TelegramNotifier detects this notification type and sends page 0:
   * formatted via `formatters::format_tokens_page(...)`
   * keyboard via `keyboards::pagination_keyboard(session_id, page, total_pages)`
5) User clicks pagination buttons:
   * callback query data `page:<session_id>:<page>`
   * handler edits the original message with the new page (`edit_message_text`)

Sessions expire after 15 minutes (`SESSION_TTL`).

---

## 11. Module Connections

```text
telegram/
├── config/            bot token, chat_id, notification prefs, auth settings
├── services/          TelegramService is registered by ServiceManager (run.rs)
├── webserver/totp     shared TOTP secret + verify_totp()
├── trader/            /pause /resume /force_stop + config toggles
├── positions/         /positions list + inline actions
├── wallet/ + utils    /balance
├── filtering/         NewTokensFound notifications + PassedToken paging
└── sol_price          used for USD conversions in status/balance rendering
```
