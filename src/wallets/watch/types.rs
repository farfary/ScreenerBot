// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Types for the wallet observation service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::i18n::{ids, UiArg, UiText};

/// Why a wallet is under observation. One address can serve more than one source at
/// once (`sources` on `WatchTarget` is a `Vec`).
///
/// `Serialize`/`Deserialize`: persisted as the `watch_targets.sources` JSON column,
/// and surfaced verbatim in the target-list/status API responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchSource {
    /// The bot's own trading wallet. Always watched while `wallet.watch_enabled` is
    /// on; not a row in `watch_targets` and not addressable through the watch API --
    /// it is structural, not something the user adds or removes.
    OwnWallet,
    /// A copy task consumes this subject's activity. The task id lets matching avoid
    /// an event-time database lookup and binds execution to the intended task.
    Copy { task_id: i64 },
    /// Notify-only: format matching activity through Telegram (and, later, the
    /// dashboard). No money moves. `rule_id` is the owning `watch_targets.id` --
    /// there is no separate alert-rule table in this phase, so a target IS the rule.
    Alert { rule_id: i64 },
}

/// A wallet under observation, as stored in `watch_targets` (`wallets.db`).
#[derive(Debug, Clone, Serialize)]
pub struct WatchTarget {
    /// Row id in `watch_targets`. `None` for the synthesized own-wallet target, which
    /// is never persisted.
    pub id: Option<i64>,
    /// Base58 Solana address.
    pub address: String,
    /// User-facing label, if any.
    pub label: Option<String>,
    /// Every reason this address is being watched.
    pub sources: Vec<WatchSource>,
    pub enabled: bool,
    /// Maximum signature pages this target may fetch during one poll. Its
    /// independent default is `poller::DEFAULT_PAGE_BUDGET`.
    pub page_budget: usize,
    /// Explicit consent for the higher-cost Helius fallback. The synthesized
    /// own-wallet target always remains false.
    pub high_activity_approved: bool,
    /// A safety pause that survives process restarts. User pauses have no
    /// automatic cause; budget pauses retain the exact limit that stopped them.
    pub disable_reason: Option<WatchDisableReason>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchDisableReason {
    User,
    Unknown,
    SignatureBudget {
        page_budget: usize,
        signatures_checked: usize,
    },
    HeliusUnavailable,
    ProcessingFailed,
}

impl WatchDisableReason {
    /// Catalog text named `wallets-watch-disabled-<kind>` after the serialized `kind`.
    pub fn ui_text(&self) -> UiText {
        match self {
            Self::User => UiText::new(ids::WALLETS_WATCH_DISABLED_USER),
            Self::SignatureBudget { page_budget, .. } => {
                UiText::new(ids::WALLETS_WATCH_DISABLED_SIGNATURE_BUDGET).arg(
                    "limit",
                    UiArg::Text((page_budget * super::poller::PAGE_SIZE).to_string()),
                )
            }
            Self::Unknown => UiText::new(ids::WALLETS_WATCH_DISABLED_UNKNOWN),
            Self::HeliusUnavailable => UiText::new(ids::WALLETS_WATCH_DISABLED_HELIUS_UNAVAILABLE),
            Self::ProcessingFailed => UiText::new(ids::WALLETS_WATCH_DISABLED_PROCESSING_FAILED),
        }
    }
}

/// Side of a detected swap, subject-relative (did the subject's holding of `mint`
/// grow or shrink).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapSide {
    Buy,
    Sell,
}

/// Direction of a plain (non-swap) transfer, subject-relative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    In,
    Out,
}

/// What a decoded, successful transaction did, from the subject's perspective.
#[derive(Debug, Clone)]
pub enum ActivityKind {
    /// A DEX/aggregator swap with exactly one resolved primary mint, SOL-quoted (see
    /// `crate::chains::solana::wallets::classify::classify_transaction_activity`).
    Swap {
        mint: String,
        side: SwapSide,
        native_amount: f64,
        token_amount: f64,
        venue: Option<String>,
        price_native: Option<f64>,
    },
    /// A plain SPL/SOL transfer -- no DEX program involved.
    Transfer {
        mint: String,
        amount: f64,
        direction: TransferDirection,
    },
    /// Decoded successfully but not a swap or a simple transfer (liquidity ops,
    /// program interactions, an ambiguous/skipped multi-hop or non-SOL-quoted route).
    Other,
}

/// One piece of on-chain activity for a watched subject, published on the shared
/// broadcast channel. `detected_at` / `decoded_at` are the day-one latency
/// instrumentation the plan calls for (§9): the gap between them is the decode
/// budget, independent of how long the notification took to arrive.
#[derive(Debug, Clone)]
pub struct WalletActivity {
    /// Base58 address this activity is about.
    pub subject: String,
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    /// When the triggering notification (WS push, poll page, or gap-fill) reached us.
    pub detected_at: DateTime<Utc>,
    /// When `decode()` finished (fetch + analyze complete).
    pub decoded_at: DateTime<Utc>,
    pub success: bool,
    pub kind: ActivityKind,
    /// Consumers filter locally by why this subject is watched; carrying the source
    /// set avoids an event-time database lookup and keeps alert/copy dispatch out of
    /// the detection hot path.
    pub sources: Vec<WatchSource>,
    /// Replayed by a gap-fill (service start or transport reconnect) rather than
    /// seen live. Its arrival distance measures downtime, not pipeline latency.
    pub backfill: bool,
}

/// One realtime notification from the injected chain runtime's subscription
/// (`crate::wallets::watch::runtime::NotificationStream`). Chain-neutral shape:
/// the adapter converts its own wire event into this before handing it to the
/// shared funnel.
#[derive(Debug, Clone)]
pub struct WatchNotification {
    pub signature: String,
    /// Whether the runtime's notification metadata already indicates failure.
    /// External targets skip known failures; the own wallet retains its history path.
    pub failed: bool,
}

/// One signature returned by a cursor page. The chain adapter preserves the
/// provider's failure metadata so the shared funnel can avoid decoding known
/// failed activity for external watch targets without changing cursor progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePageItem {
    pub signature: String,
    pub failed: bool,
}

/// One successful transaction returned by the high-activity provider path. A
/// missing decoded transaction proves the provider payload had no meaningful
/// effect for this subject, so it still advances the durable cursor.
#[derive(Debug, Clone)]
pub struct SuccessfulTransactionPageItem {
    pub signature: String,
    pub transaction: Result<Option<crate::transactions::types::Transaction>, crate::wallets::Error>,
}

/// An ascending page from the high-activity provider path.
#[derive(Debug, Clone)]
pub struct SuccessfulTransactionsPage {
    pub items: Vec<SuccessfulTransactionPageItem>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchMode {
    Standard,
    HeliusHighActivity,
}

/// Per-target status, surfaced by `/api/wallets/watch/:id/status`.
#[derive(Debug, Clone, Serialize)]
pub struct WatchStatus {
    pub target: WatchTarget,
    /// True when the shared subscription transport is `Connected` and this address is
    /// registered with it.
    pub subscribed: bool,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub last_signature: Option<String>,
    pub last_error: Option<UiText>,
    pub mode: WatchMode,
    pub catching_up: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    /// Recovery methods supported by this chain adapter and their current readiness.
    pub catch_up_options: Vec<WatchCatchUpOption>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchCatchUpOption {
    pub provider: &'static str,
    pub available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{format_en, LanguageIdentifier};
    use crate::wallets::watch::error::WatchRuntimeError;

    /// Every variant, listed through an exhaustive match so a new one fails to
    /// compile until it is added here and to the catalog.
    fn disable_reasons() -> Vec<WatchDisableReason> {
        let listed = |reason: WatchDisableReason| match reason {
            WatchDisableReason::User
            | WatchDisableReason::Unknown
            | WatchDisableReason::SignatureBudget { .. }
            | WatchDisableReason::HeliusUnavailable
            | WatchDisableReason::ProcessingFailed => reason,
        };
        vec![
            listed(WatchDisableReason::User),
            listed(WatchDisableReason::Unknown),
            listed(WatchDisableReason::SignatureBudget {
                page_budget: 8,
                signatures_checked: 800,
            }),
            listed(WatchDisableReason::HeliusUnavailable),
            listed(WatchDisableReason::ProcessingFailed),
        ]
    }

    fn runtime_errors() -> Vec<WatchRuntimeError> {
        let listed = |error: WatchRuntimeError| match error {
            WatchRuntimeError::ProviderUnavailable
            | WatchRuntimeError::ProviderRepeatedFailure
            | WatchRuntimeError::ProcessingRepeatedFailure
            | WatchRuntimeError::PositionUnreadable
            | WatchRuntimeError::ProviderCheckFailed
            | WatchRuntimeError::DecodeFailed
            | WatchRuntimeError::ProcessingFailed
            | WatchRuntimeError::PositionSaveFailed => error,
        };
        [
            WatchRuntimeError::ProviderUnavailable,
            WatchRuntimeError::ProviderRepeatedFailure,
            WatchRuntimeError::ProcessingRepeatedFailure,
            WatchRuntimeError::PositionUnreadable,
            WatchRuntimeError::ProviderCheckFailed,
            WatchRuntimeError::DecodeFailed,
            WatchRuntimeError::ProcessingFailed,
            WatchRuntimeError::PositionSaveFailed,
        ]
        .into_iter()
        .map(listed)
        .collect()
    }

    #[test]
    fn every_disable_reason_has_catalog_text_named_after_its_kind() {
        for reason in disable_reasons() {
            let kind = serde_json::to_value(&reason).unwrap()["kind"]
                .as_str()
                .unwrap()
                .replace('_', "-");
            let text = reason.ui_text();
            assert_eq!(text.id, format!("wallets-watch-disabled-{kind}"));
            assert_ne!(format_en(&text.id, None), text.id, "missing {}", text.id);
        }
    }

    /// Kinds that show a detail line under the watch status. Mirrors
    /// `WATCH_DISABLE_DETAIL_LABELS` (pages/wallets/watched.js), which omits
    /// `unknown` on purpose.
    #[test]
    fn every_disable_reason_but_unknown_has_a_detail_line() {
        for reason in disable_reasons() {
            let kind = serde_json::to_value(&reason).unwrap()["kind"]
                .as_str()
                .unwrap()
                .replace('_', "-");
            let key = format!("wallets-watch-reason-{kind}");
            let present = format_en(&key, None) != key;
            assert_eq!(present, reason != WatchDisableReason::Unknown, "{key}");
        }
    }

    #[test]
    fn every_runtime_error_has_catalog_text() {
        for error in runtime_errors() {
            let text = error.ui_text();
            assert_ne!(format_en(&text.id, None), text.id, "missing {}", text.id);
        }
    }

    #[test]
    fn budget_pause_names_the_signature_limit() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let reason = WatchDisableReason::SignatureBudget {
            page_budget: 50,
            signatures_checked: 5000,
        };
        assert_eq!(
            reason.ui_text().render_plain(&en),
            "Paused: reached the 5000-signature check limit before catching up"
        );
    }
}
