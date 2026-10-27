// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading mode serialization, the catalog keys `TRADING_MODE_LABELS`
//! (ui/trading_mode.js) names, and the live-arming gate.

use super::*;
use crate::i18n::format_en;

const MODES: [TradingMode; 2] = [TradingMode::Paper, TradingMode::Live];

#[test]
fn trading_mode_serializes_as_copy_mode_did() {
    for (mode, id) in [(TradingMode::Paper, "paper"), (TradingMode::Live, "live")] {
        assert_eq!(serde_json::to_value(mode).unwrap(), serde_json::json!(id));
        assert_eq!(
            serde_json::from_value::<TradingMode>(serde_json::json!(id)).unwrap(),
            mode
        );
        assert_eq!(mode.as_str(), id);
        assert_eq!(TradingMode::parse(id), Some(mode));
    }
    assert_eq!(TradingMode::parse("Paper"), None);
    assert!(serde_json::from_value::<TradingMode>(serde_json::json!("Live")).is_err());
}

#[test]
fn every_trading_mode_has_a_label() {
    for mode in MODES {
        let key = format!("common-mode-{}", mode.as_str());
        assert_ne!(format_en(&key, None), key, "missing catalog text {key}");
    }
}

#[test]
fn live_needs_its_own_phrase_and_paper_never_does() {
    for phrase in [LIVE_ARM_CONFIRMATION, STRATEGY_LIVE_CONFIRMATION] {
        assert_eq!(
            confirm_live(TradingMode::Paper, TradingMode::Live, None, phrase),
            Err(PaperModeError::LiveConfirmationRequired)
        );
        assert_eq!(
            confirm_live(TradingMode::Paper, TradingMode::Live, Some(""), phrase),
            Err(PaperModeError::LiveConfirmationRequired)
        );
        assert_eq!(
            confirm_live(TradingMode::Paper, TradingMode::Live, Some(phrase), phrase),
            Ok(TradingMode::Live)
        );
        for current in MODES {
            assert_eq!(
                confirm_live(current, TradingMode::Paper, None, phrase),
                Ok(TradingMode::Paper)
            );
        }
        assert_eq!(
            confirm_live(TradingMode::Live, TradingMode::Live, None, phrase),
            Ok(TradingMode::Live)
        );
    }
    assert_eq!(
        confirm_live(
            TradingMode::Paper,
            TradingMode::Live,
            Some(LIVE_ARM_CONFIRMATION),
            STRATEGY_LIVE_CONFIRMATION
        ),
        Err(PaperModeError::LiveConfirmationRequired)
    );
}

#[test]
fn each_mode_books_on_its_own_book() {
    assert_eq!(TradingMode::Live.book(), crate::positions::Book::Live);
    assert_eq!(TradingMode::Paper.book(), crate::positions::Book::Paper);
}
