//! Telegram Module for ScreenerBot
//!
//! A comprehensive, standalone Telegram integration module providing:
//! - Bot management and lifecycle
//! - Notification sending
//! - Command handling with authentication
//! - Chat discovery for initial setup
//! - Session management with 2FA support
//!
//! # Architecture
//!
//! ```text
//! telegram/
//! ├── mod.rs           # This file - public API
//! ├── types.rs         # Core types (Notification, Session, etc.)
//! ├── bot.rs           # Bot instance management
//! ├── service.rs       # ServiceManager integration
//! │
//! ├── notifier.rs      # Message sending
//! ├── session.rs       # Session & auth management
//! ├── discovery.rs     # Chat ID discovery
//! ├── polling.rs       # Update polling
//! │
//! ├── keyboards.rs     # Inline keyboards
//! ├── formatters.rs    # HTML message formatters
//! │
//! └── commands/        # Command handlers
//!     ├── mod.rs       # Command router
//!     ├── trading.rs   # Trading controls
//!     ├── status.rs    # Status commands
//!     ├── menu.rs      # Interactive menus
//!     └── callbacks.rs # Button click handlers
//! ```
//!
//! Note: 2FA/TOTP functionality uses webserver::totp module (shared lockscreen 2FA)
//!
//! # Usage
//!
//! ## Sending Notifications
//!
//! ```rust,ignore
//! use crate::telegram::{send_notification, queue_notification, Notification};
//!
//! // Async send (blocks until sent)
//! send_notification(Notification::position_opened(...)).await;
//!
//! // Non-blocking queue (for sync contexts)
//! queue_notification(Notification::system_error(...));
//! ```
//!
//! ## Discovery Mode
//!
//! ```rust,ignore
//! use crate::telegram::discovery;
//!
//! // Start discovery (waits for user to message the bot)
//! discovery::start_discovery().await?;
//!
//! // Get discovered chats
//! let chats = discovery::get_discovered_chats().await;
//!
//! // Select a chat and save to config
//! discovery::select_discovered_chat(chat_id).await?;
//!
//! // Stop discovery
//! discovery::stop_discovery().await;
//! ```
//!
//! ## Service Integration
//!
//! The module integrates with ServiceManager automatically.
//! Use `service::TelegramService` for the Service trait implementation.

// ============================================================================
// SUBMODULES
// ============================================================================

pub mod bot;
pub mod commands;
pub mod discovery;
pub mod formatters;
pub mod keyboards;
pub mod notifier;
pub mod polling;
pub mod service;
pub mod session;
pub mod types;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

// Core types
pub use types::{
    BotState, DiscoveredChat, ErrorSeverity, Notification, NotificationType, SessionState,
    TelegramSession,
};

// Session management
pub use session::{get_session_manager, TelegramSessionManager};

// Notification sending
pub use notifier::{
    init_notifier, is_enabled as is_notifier_enabled, queue_notification, send_notification,
    send_test_message, TelegramNotifier,
};

// Discovery
pub use discovery::{
    clear_discovered_chats, get_discovered_chats, is_discovery_running, select_discovered_chat,
    start_discovery, stop_discovery,
};

// Service
pub use service::{get_bot_state, get_service, is_ready, start_discovery_mode, stop_discovery_mode, TelegramService};

// Formatters (commonly used)
pub use formatters::{format_duration, format_pnl, format_price, format_sol, html_escape};
