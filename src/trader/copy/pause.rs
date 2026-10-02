// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Catalog text for why a copy task is paused.

use super::CopyPauseReason;
use crate::i18n::{ids, UiArg, UiText};

/// Seconds as the pause wording shows them: one decimal below ten seconds,
/// whole seconds from there on.
fn seconds(ms: u64) -> String {
    if ms < 10_000 {
        let tenths = (ms + 50) / 100;
        format!("{}.{}", tenths / 10, tenths % 10)
    } else {
        ((ms + 500) / 1000).to_string()
    }
}

impl CopyPauseReason {
    /// Catalog text named `copy-pause-<kind>` after the serialized `kind`.
    pub fn ui_text(&self) -> UiText {
        match self {
            Self::User => UiText::new(ids::COPY_PAUSE_USER),
            Self::LatencyKillSwitch {
                average_ms,
                threshold_ms,
            } => UiText::new(ids::COPY_PAUSE_LATENCY_KILL_SWITCH)
                .arg("average", UiArg::Text(seconds(*average_ms)))
                .arg("threshold", UiArg::Text(seconds(*threshold_ms))),
            Self::WatchDetached => UiText::new(ids::COPY_PAUSE_WATCH_DETACHED),
            Self::WatchBudgetExceeded { page_budget, .. } => {
                UiText::new(ids::COPY_PAUSE_WATCH_BUDGET_EXCEEDED).arg(
                    "limit",
                    UiArg::Text((page_budget * crate::wallets::watch::PAGE_SIZE).to_string()),
                )
            }
            Self::HeliusUnavailable => UiText::new(ids::COPY_PAUSE_HELIUS_UNAVAILABLE),
            Self::WatchProcessingFailed => UiText::new(ids::COPY_PAUSE_WATCH_PROCESSING_FAILED),
        }
    }
}

/// Text for a paused task: its reason, or the plain paused wording when none
/// was recorded.
pub fn pause_text(reason: Option<&CopyPauseReason>) -> UiText {
    reason.map_or_else(
        || UiText::new(ids::COPY_PAUSE_UNSPECIFIED),
        CopyPauseReason::ui_text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{format_en, LanguageIdentifier};

    /// Every variant, listed through an exhaustive match so a new one fails to
    /// compile until it is added here, to the catalog and to `PAUSE_SHORT_LABELS`
    /// (pages/copy/format.js).
    fn all() -> Vec<CopyPauseReason> {
        let listed = |reason: CopyPauseReason| match reason {
            CopyPauseReason::User
            | CopyPauseReason::LatencyKillSwitch { .. }
            | CopyPauseReason::WatchDetached
            | CopyPauseReason::WatchBudgetExceeded { .. }
            | CopyPauseReason::HeliusUnavailable
            | CopyPauseReason::WatchProcessingFailed => reason,
        };
        vec![
            listed(CopyPauseReason::User),
            listed(CopyPauseReason::LatencyKillSwitch {
                average_ms: 4200,
                threshold_ms: 3000,
            }),
            listed(CopyPauseReason::WatchDetached),
            listed(CopyPauseReason::WatchBudgetExceeded {
                page_budget: 8,
                signatures_checked: 800,
            }),
            listed(CopyPauseReason::HeliusUnavailable),
            listed(CopyPauseReason::WatchProcessingFailed),
        ]
    }

    #[test]
    fn every_pause_reason_has_catalog_text_named_after_its_kind() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        for reason in all() {
            let kind = serde_json::to_value(&reason).unwrap()["kind"]
                .as_str()
                .unwrap()
                .replace('_', "-");
            let text = reason.ui_text();
            assert_eq!(text.id, format!("copy-pause-{kind}"));
            assert_ne!(format_en(&text.id, None), text.id, "missing {}", text.id);
            assert!(!text.render_plain(&en).is_empty());
        }
    }

    #[test]
    fn pause_wording_keeps_the_limits() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let budget = CopyPauseReason::WatchBudgetExceeded {
            page_budget: 8,
            signatures_checked: 800,
        };
        assert_eq!(
            budget.ui_text().render_plain(&en),
            "Paused: this wallet reached its 800-signature watch check limit before catching up"
        );
        let latency = CopyPauseReason::LatencyKillSwitch {
            average_ms: 4200,
            threshold_ms: 12_400,
        };
        assert_eq!(
            latency.ui_text().render_plain(&en),
            "Auto-paused: trades arrived 4.2s late on average (limit 12s)"
        );
        assert_eq!(pause_text(None).render_plain(&en), "Paused");
    }
}
