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
use serde_json::json;

use crate::i18n::{app_locale_nonblocking, resolve_known_setting};
use crate::i18n::{
    ids, locale_info, resolve_locale, text_direction, LanguageIdentifier, MessageId, UiArg, UiText,
};
use crate::logger::{self, LogTag};

/// Environment variable through which the launching shell passes its interface
/// language, so the error screen matches the language the user already sees.
pub const UI_LOCALE_ENV: &str = "SCREENERBOT_UI_LOCALE";

/// Locale for the finished text sent to the shell: the launcher's language when
/// it names a registered locale, else the dashboard language from a loaded
/// configuration (never waiting on its lock), else the system locale.
pub fn startup_locale() -> LanguageIdentifier {
    let from_env = std::env::var(UI_LOCALE_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| locale_info(value).is_some())
        .and_then(|value| resolve_known_setting(&value));
    from_env
        .or_else(app_locale_nonblocking)
        .unwrap_or_else(|| resolve_locale(crate::i18n::SYSTEM_SETTING, None))
}

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
    pub fn ui_text(&self) -> UiText {
        match self {
            StartupRecovery::ResetWalletData { .. } => {
                UiText::new(ids::STARTUP_RECOVERY_RESET_WALLET)
            }
        }
    }
}

/// A fatal startup failure with everything required to inform and recover the user.
///
/// The texts are catalog references; [`StartupError::emit`] renders them, so the
/// shell receives finished text and never needs message ids.
#[derive(Debug, Clone)]
pub struct StartupError {
    /// Stable machine code for the failure class.
    pub code: StartupErrorCode,
    /// Short human headline (e.g. "Wallet changed").
    pub title: UiText,
    /// What happened, including concrete values (addresses, ports, paths).
    pub detail: UiText,
    /// Plain-language, binary-user-appropriate steps to resolve it.
    pub remedy: UiText,
    /// Absolute path to the current log file, so users can find full logs.
    pub log_path: String,
    /// Optional safe automated fix the GUI can perform on the user's behalf.
    pub recovery: Option<StartupRecovery>,
}

impl StartupError {
    /// Construct a startup error, resolving the current log path automatically.
    pub fn new(code: StartupErrorCode, title: UiText, detail: UiText, remedy: UiText) -> Self {
        let log_path = crate::paths::get_logs_directory()
            .join("latest.log")
            .to_string_lossy()
            .into_owned();

        Self {
            code,
            title,
            detail,
            remedy,
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
            UiArg::Nested(Box::new(UiText::new(
                ids::STARTUP_WALLET_MISMATCH_SYSTEMS_DEFAULT,
            )))
        } else {
            UiArg::Text(affected_systems.join(", "))
        };
        let recovery = StartupRecovery::ResetWalletData {
            current: current.to_owned(),
            stored: stored.to_owned(),
        };

        let detail = UiText::new(ids::STARTUP_WALLET_MISMATCH_DETAIL)
            .arg("current", UiArg::Text(current.to_owned()))
            .arg("stored", UiArg::Text(stored.to_owned()))
            .arg("systems", systems);

        let backups = crate::paths::get_data_directory().join("backups");
        let remedy = UiText::new(ids::STARTUP_WALLET_MISMATCH_REMEDY)
            .arg("action", UiArg::Nested(Box::new(recovery.ui_text())))
            .arg("path", UiArg::Text(format!("{}/", backups.display())));

        Self::new(
            StartupErrorCode::WalletMismatch,
            UiText::new(ids::STARTUP_WALLET_MISMATCH_TITLE),
            detail,
            remedy,
        )
        .with_recovery(recovery)
    }

    /// Build a generic fatal startup error from its detail text.
    pub fn generic(detail: UiText) -> Self {
        Self::new(
            StartupErrorCode::Generic,
            UiText::new(ids::STARTUP_GENERIC_TITLE),
            detail,
            UiText::new(ids::STARTUP_GENERIC_REMEDY),
        )
    }

    /// Generic fatal error whose detail is a technical error value.
    pub fn generic_error(error: impl std::fmt::Display) -> Self {
        Self::failed(ids::STARTUP_GENERIC_DETAIL, error)
    }

    /// Generic fatal error whose detail is the catalog message `id` with the
    /// technical error value as its `error` argument.
    pub fn failed(id: MessageId, error: impl std::fmt::Display) -> Self {
        Self::generic(UiText::new(id).arg("error", UiArg::Text(error.to_string())))
    }

    /// Fatal error for an invalid command-line option; `error` is the technical detail.
    pub fn invalid_option(error: impl std::fmt::Display) -> Self {
        Self::new(
            StartupErrorCode::ConfigInvalid,
            UiText::new(ids::STARTUP_OPTION_INVALID_TITLE),
            UiText::new(ids::STARTUP_GENERIC_DETAIL).arg("error", UiArg::Text(error.to_string())),
            UiText::new(ids::STARTUP_OPTION_INVALID_REMEDY),
        )
    }

    /// One-line English summary suitable for the process-level `Err(String)` return.
    pub fn summary(&self) -> String {
        format!(
            "{}: {}",
            self.title.render_source_plain(),
            self.detail.render_source_plain().replace('\n', " ")
        )
    }

    /// JSON payload consumed by the Electron shell: finished text in `locale`,
    /// plus that locale's code and text direction.
    fn to_json(&self, locale: &LanguageIdentifier) -> String {
        let recovery = self.recovery.as_ref().map(|recovery| {
            let mut value = serde_json::to_value(recovery).unwrap_or_default();
            if let Some(map) = value.as_object_mut() {
                map.insert(
                    "label".to_owned(),
                    json!(recovery.ui_text().render_plain(locale)),
                );
            }
            value
        });
        let mut payload = json!({
            "code": self.code,
            "title": self.title.render_plain(locale),
            "detail": self.detail.render_plain(locale),
            "remedy": self.remedy.render_plain(locale),
            "log_path": self.log_path,
            "locale": locale.to_string(),
            "dir": text_direction(locale).as_str(),
        });
        if let (Some(map), Some(recovery)) = (payload.as_object_mut(), recovery) {
            map.insert("recovery".to_owned(), recovery);
        }
        payload.to_string()
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
        let title = self.title.render_source_plain();
        let detail = self.detail.render_source_plain();
        let remedy = self.remedy.render_source_plain();
        block.push_str(&boxed_line(&title, false));
        block.push_str(&boxed_separator());
        for line in detail.lines() {
            block.push_str(&boxed_line(line, false));
        }
        block.push_str(&boxed_separator());
        block.push_str(&boxed_line("How to fix:", false));
        for line in remedy.lines() {
            block.push_str(&boxed_line(line, false));
        }
        block.push_str(&boxed_separator());
        block.push_str(&boxed_line(&format!("Log file: {}", self.log_path), false));
        block.push_str(&boxed_bottom());
        logger::error(LogTag::System, &block);

        // 2. Machine-readable signal for the Electron shell (stdout, like
        //    SCREENERBOT_READY). Base64 keeps it on a single parseable line.
        let encoded = BASE64.encode(self.to_json(&startup_locale()).as_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that mutate `SCREENERBOT_UI_LOCALE`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn samples() -> Vec<StartupError> {
        vec![
            StartupError::wallet_mismatch("CurrentAddr", "StoredAddr", &[]),
            StartupError::wallet_mismatch("CurrentAddr", "StoredAddr", &["Positions".to_owned()]),
            StartupError::new(
                StartupErrorCode::PortInUse,
                UiText::new(ids::STARTUP_PORT_IN_USE_TITLE),
                UiText::new(ids::STARTUP_PORT_IN_USE_DETAIL)
                    .arg("address", UiArg::Text("127.0.0.1:8080".into())),
                UiText::new(ids::STARTUP_PORT_IN_USE_REMEDY),
            ),
            StartupError::new(
                StartupErrorCode::LockHeld,
                UiText::new(ids::STARTUP_LOCK_HELD_TITLE),
                UiText::new(ids::STARTUP_LOCK_HELD_DETAIL),
                UiText::new(ids::STARTUP_LOCK_HELD_REMEDY),
            ),
            StartupError::new(
                StartupErrorCode::ConfigInvalid,
                UiText::new(ids::STARTUP_CONFIG_INVALID_TITLE),
                UiText::new(ids::STARTUP_CONFIG_PARSE_DETAIL)
                    .arg("detail", UiArg::Text("line 3".into())),
                UiText::new(ids::STARTUP_CONFIG_PARSE_REMEDY),
            ),
            StartupError::new(
                StartupErrorCode::ConfigInvalid,
                UiText::new(ids::STARTUP_CONFIG_INVALID_TITLE),
                UiText::new(ids::STARTUP_CONFIG_LOAD_PARSE_DETAIL)
                    .arg("detail", UiArg::Text("line 3".into())),
                UiText::new(ids::STARTUP_CONFIG_LOAD_PARSE_REMEDY),
            ),
            StartupError::invalid_option("bad port"),
            StartupError::generic_error("boom"),
            StartupError::failed(ids::STARTUP_FAILURE_DIRECTORIES, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_CONFIG_LOAD, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_ACTIONS_INIT, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_ACTIONS_SYNC, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_STRATEGY_INIT, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_ANALYSIS_INIT, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_ASSISTANT_INIT, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_WALLETS_INIT, "denied"),
            StartupError::failed(ids::STARTUP_FAILURE_WALLET_VALIDATION, "denied"),
        ]
    }

    #[test]
    fn every_constructor_renders_english_text() {
        for error in samples() {
            for text in [&error.title, &error.detail, &error.remedy] {
                let rendered = text.render_source_plain();
                assert!(!rendered.trim().is_empty(), "empty text for {}", text.id);
                assert!(!rendered.contains("startup-"), "unresolved id {}", text.id);
            }
            assert!(!error.summary().is_empty());
        }
    }

    #[test]
    fn wallet_mismatch_interpolates_values_and_recovery_label() {
        let error = StartupError::wallet_mismatch("CurrentAddr", "StoredAddr", &[]);
        let detail = error.detail.render_source_plain();
        assert!(detail.contains("Current wallet: CurrentAddr"));
        assert!(detail.contains("Previous wallet: StoredAddr"));
        assert!(detail.contains("Transactions, Positions, Wallet History"));
        let remedy = error.remedy.render_source_plain();
        assert!(remedy.contains("\"Reset wallet data & restart\""));
        assert!(remedy.contains("--clean-wallet-data"));
    }

    #[test]
    fn json_payload_carries_rendered_text_locale_and_direction() {
        let locale: LanguageIdentifier = "en".parse().unwrap();
        for error in samples() {
            let json = error.to_json(&locale);
            assert!(!json.contains("startup-"), "id leaked: {json}");
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["title"], error.title.render_plain(&locale));
            assert_eq!(value["detail"], error.detail.render_plain(&locale));
            assert_eq!(value["remedy"], error.remedy.render_plain(&locale));
            assert_eq!(value["code"], error.code.as_str());
            assert_eq!(value["locale"], "en");
            assert_eq!(value["dir"], "ltr");
            assert!(value["log_path"].is_string());
            assert!(value["title"].is_string() && !value["title"].as_str().unwrap().is_empty());
        }
        let error = StartupError::wallet_mismatch("CurrentAddr", "StoredAddr", &[]);
        let value: serde_json::Value = serde_json::from_str(&error.to_json(&locale)).unwrap();
        assert_eq!(value["recovery"]["action"], "reset_wallet_data");
        assert_eq!(value["recovery"]["current"], "CurrentAddr");
        assert_eq!(value["recovery"]["label"], "Reset wallet data & restart");
    }

    #[test]
    fn startup_locale_honours_only_registered_env_locale() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var(UI_LOCALE_ENV).ok();

        let registered = crate::i18n::source_locale();
        std::env::set_var(UI_LOCALE_ENV, registered);
        assert_eq!(startup_locale().to_string(), registered);

        std::env::set_var(UI_LOCALE_ENV, "zz-not-a-locale");
        let fallback = startup_locale();
        assert_ne!(fallback.to_string(), "zz-not-a-locale");
        assert!(locale_info(&fallback.to_string()).is_some() || fallback.language.as_str() != "zz");

        match previous {
            Some(value) => std::env::set_var(UI_LOCALE_ENV, value),
            None => std::env::remove_var(UI_LOCALE_ENV),
        }
    }
}
