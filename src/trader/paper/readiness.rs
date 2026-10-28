// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-readiness evidence from a paper book. Every check carries catalog text
//! for its title and its detail; the figures are arguments. Each paper book
//! composes the shared checks with its own.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::i18n::{ids, MessageId, UiArg, UiText};

#[derive(Debug, Serialize)]
pub struct ReadinessCheck {
    pub id: &'static str,
    pub text: UiText,
    pub passed: bool,
    pub detail: UiText,
}

/// Advisory evidence from a paper book before arming live. Arming still needs
/// the explicit confirmation; this is what the confirmation is about.
#[derive(Debug, Serialize)]
pub struct Readiness {
    pub ready: bool,
    pub checks: Vec<ReadinessCheck>,
}

impl Readiness {
    /// Ready only when every check passes.
    pub fn from_checks(checks: Vec<ReadinessCheck>) -> Self {
        Self {
            ready: checks.iter().all(|check| check.passed),
            checks,
        }
    }
}

/// The catalog text of a check whose wording names the book it reads.
#[derive(Debug, Clone, Copy)]
pub struct CheckText {
    pub title: MessageId,
    pub passed: MessageId,
    pub failed: MessageId,
}

/// A signed SOL figure as the dashboard writes one, with a true minus sign.
pub fn signed_native(value: f64) -> String {
    let sign = if value > 0.0 {
        "+"
    } else if value < 0.0 {
        "\u{2212}"
    } else {
        ""
    };
    format!("{sign}{:.4}", value.abs())
}

/// A count argument for a plural-aware catalog message.
pub fn count(value: usize) -> UiArg {
    UiArg::Count(i64::try_from(value).unwrap_or(i64::MAX))
}

/// Enough closed paper rounds of the current run to judge the book.
pub fn history(closed_rounds: usize, needed: usize) -> ReadinessCheck {
    history_check(
        closed_rounds,
        needed,
        CheckText {
            title: ids::PAPER_READINESS_HISTORY,
            passed: ids::PAPER_READINESS_HISTORY_MET,
            failed: ids::PAPER_READINESS_HISTORY_SHORT,
        },
        None,
    )
}

/// Enough closed paper rounds entered since the rules changed at `changed` to judge the
/// book; `met` and `short` are the book's wording, which names the change time.
pub fn history_since(
    closed_rounds: usize,
    needed: usize,
    changed: DateTime<Utc>,
    met: MessageId,
    short: MessageId,
) -> ReadinessCheck {
    history_check(
        closed_rounds,
        needed,
        CheckText {
            title: ids::PAPER_READINESS_HISTORY,
            passed: met,
            failed: short,
        },
        Some(UiArg::Time(changed.timestamp_millis())),
    )
}

fn history_check(
    closed_rounds: usize,
    needed: usize,
    text: CheckText,
    changed: Option<UiArg>,
) -> ReadinessCheck {
    let passed = closed_rounds >= needed;
    let detail = UiText::new(if passed { text.passed } else { text.failed })
        .arg("count", count(closed_rounds))
        .arg("needed", count(needed));
    ReadinessCheck {
        id: "history",
        text: UiText::new(text.title),
        passed,
        detail: match changed {
            Some(changed) => detail.arg("changed", changed),
            None => detail,
        },
    }
}

/// Positive realized paper P&L over the closed rounds.
pub fn profit(realized_native: f64, rounds: usize, wins: usize) -> ReadinessCheck {
    ReadinessCheck {
        id: "profit",
        text: UiText::new(ids::PAPER_READINESS_PROFIT),
        passed: realized_native > 0.0,
        detail: UiText::new(ids::PAPER_READINESS_PROFIT_DETAIL)
            .arg("realized", UiArg::Text(signed_native(realized_native)))
            .arg("count", count(rounds))
            .arg("wins", count(wins)),
    }
}

/// Every open paper position has a pool mark.
pub fn priced(unpriced: usize, text: CheckText) -> ReadinessCheck {
    ReadinessCheck {
        id: "priced",
        text: UiText::new(text.title),
        passed: unpriced == 0,
        detail: if unpriced == 0 {
            UiText::new(text.passed)
        } else {
            UiText::new(text.failed).arg("count", count(unpriced))
        },
    }
}

/// Consecutive paper exits without a fill at which a book's exits stop reading as
/// filling: a run of rejected exits means the paper results hold rounds live could not
/// have left.
pub const FAILED_EXIT_LIMIT: u32 = 3;

/// The latest paper exits found a fill: fewer than [`FAILED_EXIT_LIMIT`] in a row
/// filled at no rung of the slippage ladder.
pub fn exits_filled(failed_in_a_row: u32) -> ReadinessCheck {
    let passed = failed_in_a_row < FAILED_EXIT_LIMIT;
    ReadinessCheck {
        id: "exits",
        text: UiText::new(ids::PAPER_READINESS_EXITS),
        passed,
        detail: if passed {
            UiText::new(ids::PAPER_READINESS_EXITS_OK)
        } else {
            UiText::new(ids::PAPER_READINESS_EXITS_FAILING).arg(
                "count",
                count(usize::try_from(failed_in_a_row).unwrap_or(usize::MAX)),
            )
        },
    }
}

/// The runtime could execute live trades right now; `block` is the wording of
/// why it cannot, and `ok` the wording when nothing blocks it.
pub fn runtime(block: Option<UiText>, title: MessageId, ok: MessageId) -> ReadinessCheck {
    ReadinessCheck {
        id: "runtime",
        text: UiText::new(title),
        passed: block.is_none(),
        detail: block.unwrap_or_else(|| UiText::new(ok)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::LanguageIdentifier;

    #[test]
    fn shared_checks_keep_their_figures_and_plurals() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let profit = profit(-0.12345, 1, 0);
        assert!(!profit.passed);
        assert_eq!(
            profit.detail.render_plain(&en),
            "\u{2212}0.1235 SOL realized over 1 round, 0 won"
        );
        let short = history(2, 5);
        assert!(!short.passed);
        assert_eq!(short.detail.render_plain(&en), "2 of 5 closed paper rounds");
        let met = history(1, 1);
        assert!(met.passed);
        assert_eq!(
            met.detail.render_plain(&en),
            "1 closed paper round, 1 needed"
        );
    }

    #[test]
    fn readiness_is_ready_only_when_every_check_passes() {
        let text = CheckText {
            title: ids::PAPER_READINESS_HISTORY,
            passed: ids::PAPER_READINESS_HISTORY,
            failed: ids::PAPER_READINESS_HISTORY,
        };
        assert!(Readiness::from_checks(vec![history(3, 3), priced(0, text)]).ready);
        assert!(!Readiness::from_checks(vec![history(3, 3), priced(1, text)]).ready);
        assert!(
            !Readiness::from_checks(vec![runtime(
                Some(UiText::new(ids::PAPER_READINESS_HISTORY)),
                ids::PAPER_READINESS_HISTORY,
                ids::PAPER_READINESS_HISTORY,
            )])
            .ready
        );
        assert!(Readiness::from_checks(Vec::new()).ready);
    }

    #[test]
    fn exits_stop_reading_as_filling_at_the_failure_limit() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        assert!(exits_filled(0).passed);
        assert!(exits_filled(FAILED_EXIT_LIMIT - 1).passed);
        let failing = exits_filled(FAILED_EXIT_LIMIT);
        assert!(!failing.passed);
        assert_eq!(
            failing.detail.render_plain(&en),
            format!("Paper exits in a row without a fill: {FAILED_EXIT_LIMIT}")
        );
    }
}
