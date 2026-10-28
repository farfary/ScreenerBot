// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trade and close reasons: the persisted reason ids, their display text, and the group
//! key a stored `closed_reason` is counted under.

use crate::i18n::{ids, MessageId, UiArg, UiText};
use crate::positions::{FORCE_CLOSED_PREFIX, PENDING_VERIFICATION_SUFFIX};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeReason {
    // Entry reasons
    StrategySignal,
    ManualEntry,
    ForceBuy,
    CopyBuy,
    DCAScheduled,

    // Exit reasons
    TakeProfit,
    StopLoss,
    TrailingStop,
    BreakEvenStop,
    TimeOverride,
    TimeStagnation,
    StrategyExit,
    LlmAnalysisExit, // Model-scored exit recommendation
    ManualExit,
    RiskManagement,
    Blacklisted,
    ForceSell,
    CopySell,
    /// A paper round closed at its mark when a new paper run starts.
    PaperReset,
}

/// `closed_reason` values written outside the trade-reason path
/// (`positions/apply.rs`, `positions/ledger/sync.rs`) and their labels.
const WRITTEN_REASON_LABELS: [(&str, MessageId); 4] = [
    (
        crate::positions::ledger::CLOSED_EXTERNALLY,
        ids::TRADE_REASON_CLOSED_EXTERNALLY,
    ),
    ("wallet_history", ids::TRADE_REASON_WALLET_HISTORY),
    ("exit_retry_pending", ids::TRADE_REASON_EXIT_RETRY_PENDING),
    (
        "synthetic_exit_permanent_failure",
        ids::TRADE_REASON_SYNTHETIC_EXIT_PERMANENT_FAILURE,
    ),
];

impl TradeReason {
    /// Every reason; the `Debug` name of each is its persisted id.
    pub const ALL: [TradeReason; 19] = [
        TradeReason::StrategySignal,
        TradeReason::ManualEntry,
        TradeReason::ForceBuy,
        TradeReason::CopyBuy,
        TradeReason::DCAScheduled,
        TradeReason::TakeProfit,
        TradeReason::StopLoss,
        TradeReason::TrailingStop,
        TradeReason::BreakEvenStop,
        TradeReason::TimeOverride,
        TradeReason::TimeStagnation,
        TradeReason::StrategyExit,
        TradeReason::LlmAnalysisExit,
        TradeReason::ManualExit,
        TradeReason::RiskManagement,
        TradeReason::Blacklisted,
        TradeReason::ForceSell,
        TradeReason::CopySell,
        TradeReason::PaperReset,
    ];
}

/// Display text of a trade reason. The match is exhaustive, so a new variant
/// fails to compile until it is labelled here and in `TRADE_REASON_LABELS`
/// (ui/trade_reason.js).
pub fn trade_reason_text(reason: &TradeReason) -> UiText {
    UiText::new(match reason {
        TradeReason::StrategySignal => ids::TRADE_REASON_STRATEGY_SIGNAL,
        TradeReason::ManualEntry => ids::TRADE_REASON_MANUAL_ENTRY,
        TradeReason::ForceBuy => ids::TRADE_REASON_FORCE_BUY,
        TradeReason::CopyBuy => ids::TRADE_REASON_COPY_BUY,
        TradeReason::DCAScheduled => ids::TRADE_REASON_DCA_SCHEDULED,
        TradeReason::TakeProfit => ids::TRADE_REASON_TAKE_PROFIT,
        TradeReason::StopLoss => ids::TRADE_REASON_STOP_LOSS,
        TradeReason::TrailingStop => ids::TRADE_REASON_TRAILING_STOP,
        TradeReason::BreakEvenStop => ids::TRADE_REASON_BREAK_EVEN_STOP,
        TradeReason::TimeOverride => ids::TRADE_REASON_TIME_OVERRIDE,
        TradeReason::TimeStagnation => ids::TRADE_REASON_TIME_STAGNATION,
        TradeReason::StrategyExit => ids::TRADE_REASON_STRATEGY_EXIT,
        TradeReason::LlmAnalysisExit => ids::TRADE_REASON_LLM_ANALYSIS_EXIT,
        TradeReason::ManualExit => ids::TRADE_REASON_MANUAL_EXIT,
        TradeReason::RiskManagement => ids::TRADE_REASON_RISK_MANAGEMENT,
        TradeReason::Blacklisted => ids::TRADE_REASON_BLACKLISTED,
        TradeReason::ForceSell => ids::TRADE_REASON_FORCE_SELL,
        TradeReason::CopySell => ids::TRADE_REASON_COPY_SELL,
        TradeReason::PaperReset => ids::TRADE_REASON_PAPER_RESET,
    })
}

/// Display text of a stored `positions.closed_reason`. Mirrors `closeReasonText`
/// (ui/trade_reason.js): the pending-verification suffix and the force-close
/// prefix wrap the base reason, and a value with no label is shown as stored.
pub fn closed_reason_text(stored: &str) -> UiText {
    if let Some(base) = stored.strip_suffix(PENDING_VERIFICATION_SUFFIX) {
        return UiText::new(ids::TRADE_REASON_PENDING_VERIFICATION)
            .arg("reason", UiArg::Nested(Box::new(closed_reason_text(base))));
    }
    if let Some(note) = stored.strip_prefix(FORCE_CLOSED_PREFIX) {
        return UiText::new(ids::TRADE_REASON_FORCE_CLOSED)
            .arg("note", UiArg::Text(note.trim().to_owned()));
    }
    if let Some(reason) = TradeReason::ALL
        .iter()
        .find(|reason| format!("{reason:?}") == stored)
    {
        return trade_reason_text(reason);
    }
    if let Some((_, id)) = WRITTEN_REASON_LABELS.iter().find(|(id, _)| *id == stored) {
        return UiText::new(*id);
    }
    UiText::new(ids::TRADE_REASON_STORED).arg("reason", UiArg::Text(stored.to_owned()))
}

/// Group key of a round closed with no recorded reason.
pub const UNKNOWN_REASON_KEY: &str = "unknown";

/// Group key of every force close: the `force_closed: <note>` prefix and the older
/// `force_closed_ghost` value alike.
pub const FORCE_CLOSED_KEY: &str = "force_closed";

/// The group a stored `closed_reason` is counted under. A missing or empty reason is
/// [`UNKNOWN_REASON_KEY`], the pending-verification suffix is dropped so a close awaiting
/// verification counts with its reason, and every force close, whatever its note, is
/// [`FORCE_CLOSED_KEY`]. Any other value is its own group, as stored.
pub fn closed_reason_key(stored: Option<&str>) -> String {
    let Some(stored) = stored.filter(|stored| !stored.trim().is_empty()) else {
        return UNKNOWN_REASON_KEY.to_owned();
    };
    let base = stored
        .strip_suffix(PENDING_VERIFICATION_SUFFIX)
        .unwrap_or(stored);
    if base.starts_with(FORCE_CLOSED_KEY) {
        FORCE_CLOSED_KEY.to_owned()
    } else {
        base.to_owned()
    }
}

/// Display text of a group key from [`closed_reason_key`].
pub fn closed_reason_group_text(key: &str) -> UiText {
    match key {
        UNKNOWN_REASON_KEY => UiText::new(ids::TRADE_REASON_UNKNOWN),
        FORCE_CLOSED_KEY => UiText::new(ids::TRADE_REASON_FORCE_CLOSED_ANY),
        stored => closed_reason_text(stored),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The number of `TradeReason` variants.
    const TRADE_REASON_COUNT: usize = 19;

    /// The declaration position of each reason. Exhaustive, so a new variant fails
    /// to compile until it is numbered here, and the test below then fails until
    /// `TradeReason::ALL` lists it.
    fn ordinal(reason: &TradeReason) -> usize {
        match reason {
            TradeReason::StrategySignal => 0,
            TradeReason::ManualEntry => 1,
            TradeReason::ForceBuy => 2,
            TradeReason::CopyBuy => 3,
            TradeReason::DCAScheduled => 4,
            TradeReason::TakeProfit => 5,
            TradeReason::StopLoss => 6,
            TradeReason::TrailingStop => 7,
            TradeReason::BreakEvenStop => 8,
            TradeReason::TimeOverride => 9,
            TradeReason::TimeStagnation => 10,
            TradeReason::StrategyExit => 11,
            TradeReason::LlmAnalysisExit => 12,
            TradeReason::ManualExit => 13,
            TradeReason::RiskManagement => 14,
            TradeReason::Blacklisted => 15,
            TradeReason::ForceSell => 16,
            TradeReason::CopySell => 17,
            TradeReason::PaperReset => 18,
        }
    }

    #[test]
    fn the_reason_list_holds_every_reason_once() {
        assert_eq!(TradeReason::ALL.len(), TRADE_REASON_COUNT);
        let mut ordinals: Vec<usize> = TradeReason::ALL.iter().map(ordinal).collect();
        ordinals.sort_unstable();
        assert_eq!(ordinals, (0..TRADE_REASON_COUNT).collect::<Vec<_>>());
    }

    /// `StopLoss` -> `stop-loss`, `DCAScheduled` -> `dca-scheduled`.
    fn kebab(id: &str) -> String {
        let chars: Vec<char> = id.chars().collect();
        let mut out = String::new();
        for (i, c) in chars.iter().enumerate() {
            let starts_word = i > 0
                && c.is_uppercase()
                && (chars[i - 1].is_lowercase()
                    || (chars[i - 1].is_uppercase()
                        && chars.get(i + 1).is_some_and(|n| n.is_lowercase())));
            if starts_word {
                out.push('-');
            }
            out.extend(c.to_lowercase());
        }
        out
    }

    #[test]
    fn trade_reason_labels_exist_in_the_catalog() {
        for reason in &TradeReason::ALL {
            let id = format!("{reason:?}");
            let suffix = kebab(&id);
            let key = trade_reason_text(reason).id.into_owned();
            assert_eq!(
                key,
                format!("trade-reason-{suffix}"),
                "key does not follow the id {id}"
            );
            assert_ne!(crate::i18n::format_en(&key, None), key, "missing {key}");
        }
    }

    /// Every entry of `TRADE_REASON_LABELS` (ui/trade_reason.js) resolves to the
    /// same message here, and Rust knows no reason the dashboard lacks.
    #[test]
    fn closed_reason_labels_match_the_dashboard_map() {
        let js = include_str!("../webserver/templates/scripts/ui/trade_reason.js");
        let map = js
            .split("TRADE_REASON_LABELS = Object.freeze({")
            .nth(1)
            .and_then(|rest| rest.split("});").next())
            .expect("TRADE_REASON_LABELS block");
        let entries: Vec<(&str, &str)> = map
            .lines()
            .filter_map(|line| {
                let (name, key) = line.trim().trim_end_matches(',').split_once(": ")?;
                Some((name, key.trim_matches('"')))
            })
            .collect();
        assert_eq!(
            entries.len(),
            TradeReason::ALL.len() + WRITTEN_REASON_LABELS.len()
        );
        for (name, key) in entries {
            assert_eq!(closed_reason_text(name).id, key, "{name}");
        }
    }

    #[test]
    fn closed_reason_wrappers_and_unknown_values() {
        let en = |text: UiText| text.render_source_plain();
        assert_eq!(en(closed_reason_text("StopLoss")), "Stop Loss");
        assert_eq!(
            en(closed_reason_text("TakeProfit_pending_verification")),
            "Take Profit (pending verification)"
        );
        assert_eq!(
            en(closed_reason_text("force_closed: stuck swap ")),
            "Force closed: stuck swap"
        );
        assert_eq!(en(closed_reason_text("exit")), "exit");
    }

    #[test]
    fn the_break_even_and_stagnation_reasons_read_by_their_stored_ids() {
        let en = |text: UiText| text.render_source_plain();
        assert_eq!(format!("{:?}", TradeReason::BreakEvenStop), "BreakEvenStop");
        assert_eq!(
            format!("{:?}", TradeReason::TimeStagnation),
            "TimeStagnation"
        );
        assert_eq!(en(closed_reason_text("BreakEvenStop")), "Break-Even Stop");
        assert_eq!(
            en(closed_reason_text("TimeStagnation")),
            "Flat Position Exit"
        );
    }

    #[test]
    fn close_reason_wrapper_messages_exist_in_the_catalog() {
        for key in [
            "trade-reason-pending-verification",
            "trade-reason-force-closed",
        ] {
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
        }
    }

    #[test]
    fn closed_reason_key_strips_the_pending_suffix() {
        assert_eq!(
            closed_reason_key(Some("TakeProfit_pending_verification")),
            "TakeProfit"
        );
        assert_eq!(closed_reason_key(Some("TakeProfit")), "TakeProfit");
        assert_eq!(
            closed_reason_key(Some("closed_externally")),
            "closed_externally"
        );
    }

    #[test]
    fn closed_reason_key_groups_every_force_close() {
        for stored in [
            "force_closed: Ghost position, no tokens in the wallet",
            "force_closed: stuck swap ",
            "force_closed_ghost",
            "force_closed: note_pending_verification",
        ] {
            assert_eq!(
                closed_reason_key(Some(stored)),
                FORCE_CLOSED_KEY,
                "{stored}"
            );
        }
    }

    #[test]
    fn closed_reason_key_names_a_missing_reason_unknown() {
        assert_eq!(closed_reason_key(None), UNKNOWN_REASON_KEY);
        assert_eq!(closed_reason_key(Some("")), UNKNOWN_REASON_KEY);
        assert_eq!(closed_reason_key(Some("  ")), UNKNOWN_REASON_KEY);
    }

    /// The `closed_reason` values a live book holds, with the missing reason, each resolve
    /// to a catalog label of their own group, never to the stored text.
    #[test]
    fn group_text_of_every_live_reason_has_a_catalog_id() {
        let stored = [
            Some("ManualExit"),
            None,
            Some(crate::positions::ledger::CLOSED_EXTERNALLY),
            Some("Blacklisted"),
            Some("TakeProfit"),
            Some("TakeProfit_pending_verification"),
            Some("RiskManagement"),
            Some("StrategySignal"),
            Some("TimeOverride"),
            Some("force_closed: Ghost position, no tokens in the wallet"),
            Some("force_closed_ghost"),
        ];
        for reason in stored {
            let text = closed_reason_group_text(&closed_reason_key(reason));
            let id = text.id.to_string();
            assert_ne!(id, ids::TRADE_REASON_STORED.to_string(), "{reason:?}");
            assert_ne!(crate::i18n::format_en(&id, None), id, "missing {id}");
        }
        assert_eq!(
            closed_reason_group_text(UNKNOWN_REASON_KEY).render_source_plain(),
            "Not recorded"
        );
        assert_eq!(
            closed_reason_group_text(FORCE_CLOSED_KEY).render_source_plain(),
            "Force closed"
        );
    }
}
