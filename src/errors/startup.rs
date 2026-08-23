//! Fatal startup errors — structured, user-actionable failures that prevent boot.
//!
//! A `StartupError` is raised when the application cannot start the dashboard at
//! all (wallet mismatch, port already in use, another instance holding the lock,
//! unreadable config, etc.). Unlike ad-hoc string errors, it carries everything a
//! user needs to recover and is surfaced identically across every run context:
//!
//! - **Headless / terminal / log file:** [`StartupError::emit`] prints a boxed,
//!   professional block (no developer-only `cargo` instructions) to both the
//!   terminal and the rotating log file.
//! - **GUI / Electron:** the same call prints a single machine-readable line,
//!   `SCREENERBOT_ERROR:<base64-json>`, to stdout. The Electron shell parses it
//!   and renders a proper error screen with the title, detail, remedy, and — when
//!   a safe automated fix exists — a one-click recovery button.
//!
//! Remedy text is always written for users running the **compiled binary**, never
//! for a source checkout. When a failure is safely recoverable (e.g. a wallet
//! mismatch that only needs the previous wallet's local history cleared), the
//! error also carries a [`StartupRecovery`] action the GUI can perform on the
//! user's behalf — always backing up before deleting anything.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;

use crate::logger::{self, LogTag};

/// Stable machine-readable identifier for a class of fatal startup failure.
///
/// Serialized as `snake_case` and consumed by the Electron shell to pick the
/// right icon/affordances. Add new variants here rather than overloading
/// [`StartupErrorCode::Generic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupErrorCode {
    /// The configured wallet differs from the wallet recorded in local history.
    WalletMismatch,
    /// The webserver port is already in use by another process.
    PortInUse,
    /// Another ScreenerBot instance is already running (process lock held).
    LockHeld,
    /// `config.toml` exists but could not be read or parsed.
    ConfigInvalid,
    /// Any other fatal startup failure without a dedicated remedy.
    Generic,
}

impl StartupErrorCode {
    /// Lowercase stable string form (matches the serialized representation).
    pub fn as_str(self) -> &'static str {
        match self {
            StartupErrorCode::WalletMismatch => "wallet_mismatch",
            StartupErrorCode::PortInUse => "port_in_use",
            StartupErrorCode::LockHeld => "lock_held",
            StartupErrorCode::ConfigInvalid => "config_invalid",
            StartupErrorCode::Generic => "generic",
        }
    }
}

/// A safe, automated recovery the GUI can offer for a recoverable startup error.
///
/// Each variant maps to a concrete action the binary performs (always with a
/// backup first). The Electron shell relaunches the binary with the matching
/// CLI flag rather than touching user data itself.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum StartupRecovery {
    /// Back up then clear the previous wallet's local history (transactions,
    /// positions, wallet) so the current wallet can start cleanly. Triggered via
    /// the `--clean-wallet-data` flag.
    ResetWalletData { current: String, stored: String },
}

impl StartupRecovery {
    /// Short imperative label for the GUI action button.
    pub fn button_label(&self) -> &'static str {
        match self {
            StartupRecovery::ResetWalletData { .. } => "Reset wallet data & restart",
        }
    }
}

/// A fatal startup failure with everything required to inform and recover the user.
#[derive(Debug, Clone, Serialize)]
pub struct StartupError {
    /// Stable machine code for the failure class.
    pub code: StartupErrorCode,
    /// Short human headline (e.g. "Wallet changed").
    pub title: String,
    /// What happened, including concrete values (addresses, ports, paths).
    pub detail: String,
    /// Plain-language, binary-user-appropriate steps to resolve it.
    pub remedy: String,
    /// Absolute path to the current log file, so users can find full logs.
    pub log_path: String,
    /// Optional safe automated fix the GUI can perform on the user's behalf.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<StartupRecovery>,
}

impl StartupError {
    /// Construct a startup error, resolving the current log path automatically.
    pub fn new(
        code: StartupErrorCode,
        title: impl Into<String>,
        detail: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        let log_path = crate::paths::get_logs_directory()
            .join("latest.log")
            .to_string_lossy()
            .into_owned();

        Self {
            code,
            title: title.into(),
            detail: detail.into(),
            remedy: remedy.into(),
            log_path,
            recovery: None,
        }
    }

    /// Attach a safe automated recovery action (used by the GUI).
    pub fn with_recovery(mut self, recovery: StartupRecovery) -> Self {
        self.recovery = Some(recovery);
        self
    }

    /// Build the wallet-mismatch error with correct, mode-agnostic remedy text
    /// and a `ResetWalletData` recovery action.
    pub fn wallet_mismatch(current: &str, stored: &str, affected_systems: &[String]) -> Self {
        let systems = if affected_systems.is_empty() {
            "Transactions, Positions, Wallet History".to_owned()
        } else {
            affected_systems.join(", ")
        };

        let detail = format!(
            "The wallet in your configuration does not match the wallet recorded in this \
             computer's local history.\n\nCurrent wallet: {current}\nPrevious wallet: {stored}\n\nAffected local data: {systems}\n\nThis usually happens after importing a different private key or restoring a \
             different configuration. Trading, positions, and history belong to the previous \
             wallet and must be cleared before the new wallet can start safely."
        );

        let data_dir = crate::paths::get_data_directory();
        let remedy = format!(
            "Clear the previous wallet's local history to continue (your databases are backed \
             up automatically first):\n\n  - In the app: choose \"Reset wallet data & restart\" below.\n  - From a terminal: run  screenerbot --clean-wallet-data\n\nNo on-chain funds are affected; only this computer's local trade/position history \
             is reset. Backups are written under:\n  {}/backups/",
            data_dir.display()
        );

        Self::new(
            StartupErrorCode::WalletMismatch,
            "Wallet changed",
            detail,
            remedy,
        )
        .with_recovery(StartupRecovery::ResetWalletData {
            current: current.to_owned(),
            stored: stored.to_owned(),
        })
    }

    /// Classify an arbitrary bubbled-up error message into the most specific
    /// startup error possible, upgrading common fatal causes (another instance
    /// running, port already in use, unreadable config) to dedicated codes with
    /// binary-user-appropriate remedies. Falls back to [`StartupError::generic`].
    pub fn classify(message: impl Into<String>) -> Self {
        let message = message.into();
        let lower = message.to_lowercase();

        if lower.contains("already running") || lower.contains("process lock") {
            return Self::new(
                StartupErrorCode::LockHeld,
                "ScreenerBot is already running",
                "Another copy of ScreenerBot is already running on this computer, so a second \
                 one cannot start.",
                "Switch to the window that's already open. If you don't see one, quit any \
                 background ScreenerBot process and try again. If the problem persists after a \
                 reboot, the lock file may be stale and can be removed from the data folder \
                 (.screenerbot.lock).",
            );
        }

        if lower.contains("address already in use") || lower.contains("addrinuse") {
            return Self::new(
                StartupErrorCode::PortInUse,
                "Network port is busy",
                message.clone(),
                "Another program is using the port ScreenerBot needs. Close that program, or \
                 change the webserver port in Settings, then start ScreenerBot again.",
            );
        }

        let config_parse_error = lower.contains("failed to load config")
            || (lower.contains("config") && lower.contains("parse"));
        if config_parse_error {
            return Self::new(
                StartupErrorCode::ConfigInvalid,
                "Configuration could not be read",
                message.clone(),
                "Your configuration file could not be read. Restore a backup from the data \
                 folder, or reset configuration to defaults and set up your wallet and RPC \
                 again.",
            );
        }

        Self::generic(message)
    }

    /// Build a generic fatal startup error from an arbitrary message.
    pub fn generic(message: impl Into<String>) -> Self {
        let message = message.into();
        Self::new(
            StartupErrorCode::Generic,
            "ScreenerBot could not start",
            message,
            "Check the log file for details, then restart the app. If the problem persists, \
             contact support at t.me/screenerbotio_support.",
        )
    }

    /// One-line summary suitable for the process-level `Err(String)` return.
    pub fn summary(&self) -> String {
        format!("{}: {}", self.title, self.detail.replace('\n', " "))
    }

    /// JSON payload consumed by the Electron shell.
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                "{{\"code\":\"{}\",\"title\":\"{}\"}}",
                self.code.as_str(),
                self.title
            )
        })
    }

    /// Surface the error on every channel: a boxed block to the terminal + log
    /// file, and the `SCREENERBOT_ERROR:<base64-json>` signal to stdout for the
    /// GUI. Call exactly once, at the process boundary, after all other logging.
    pub fn emit(&self) {
        // 1. Human-readable boxed block (terminal + rotating log file).
        let mut block = String::new();
        block.push('\n');
        block.push_str(&boxed_top());
        block.push_str(&boxed_line("STARTUP FAILED", true));
        block.push_str(&boxed_line(&self.title, false));
        block.push_str(&boxed_separator());
        for line in self.detail.lines() {
            block.push_str(&boxed_line(line, false));
        }
        block.push_str(&boxed_separator());
        block.push_str(&boxed_line("How to fix:", false));
        for line in self.remedy.lines() {
            block.push_str(&boxed_line(line, false));
        }
        block.push_str(&boxed_separator());
        block.push_str(&boxed_line(&format!("Log file: {}", self.log_path), false));
        block.push_str(&boxed_bottom());
        logger::error(LogTag::System, &block);

        // 2. Machine-readable signal for the Electron shell (stdout, like
        //    SCREENERBOT_READY). Base64 keeps it on a single parseable line.
        let encoded = BASE64.encode(self.to_json().as_bytes());
        println!("SCREENERBOT_ERROR:{encoded}");
        // Ensure the signal is flushed before the process exits.
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl std::error::Error for StartupError {}

impl From<StartupError> for String {
    fn from(e: StartupError) -> Self {
        e.summary()
    }
}

impl From<String> for StartupError {
    fn from(message: String) -> Self {
        StartupError::classify(message)
    }
}

impl From<&str> for StartupError {
    fn from(message: &str) -> Self {
        StartupError::classify(message.to_owned())
    }
}

// =============================================================================
// Box-drawing helpers for the terminal/log block
// =============================================================================

const BOX_WIDTH: usize = 78;

fn boxed_line(text: &str, heading: bool) -> String {
    // Truncate overly long lines so the box stays aligned in narrow terminals.
    let inner = BOX_WIDTH - 4;
    let content: String = if text.chars().count() > inner {
        let mut s: String = text.chars().take(inner - 1).collect();
        s.push('…');
        s
    } else {
        text.to_owned()
    };
    let pad = inner - content.chars().count();
    if heading {
        format!("  | {}{} |\n", content.to_uppercase(), " ".repeat(pad))
    } else {
        format!("  | {}{} |\n", content, " ".repeat(pad))
    }
}

fn boxed_top() -> String {
    format!("  +{}+\n", "-".repeat(BOX_WIDTH - 2))
}

fn boxed_separator() -> String {
    format!("  |{}|\n", "-".repeat(BOX_WIDTH - 2))
}

fn boxed_bottom() -> String {
    format!("  +{}+\n", "-".repeat(BOX_WIDTH - 2))
}
