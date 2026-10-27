// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Arming a strategy live or returning it to paper: the confirmation phrase, the live
//! trading gate, the stored mode and the evaluation cache, in that order.

use crate::logger::{self, LogTag};
use crate::strategies::{db::set_strategy_mode, types::Strategy, Error};
use crate::trader::paper::{confirm_live, PaperModeError, TradingMode, STRATEGY_LIVE_CONFIRMATION};
use crate::trader::LiveBlock;

/// The mode `strategy` moves to when `requested` with `confirmation`, while `block` is
/// why live trading is unavailable, if it is. Arming live needs the exact phrase and an
/// open live gate; returning to paper, or keeping a live strategy live, is always allowed.
fn decide_mode(
    current: TradingMode,
    requested: TradingMode,
    confirmation: Option<&str>,
    block: Option<LiveBlock>,
) -> Result<TradingMode, Error> {
    let mode = confirm_live(current, requested, confirmation, STRATEGY_LIVE_CONFIRMATION).map_err(
        |error| match error {
            PaperModeError::LiveConfirmationRequired => Error::LiveConfirmationRequired,
        },
    )?;
    if mode == TradingMode::Live && current != TradingMode::Live {
        if let Some(reason) = block {
            return Err(Error::LiveUnavailable { reason });
        }
    }
    Ok(mode)
}

/// Moves `strategy` to the `requested` mode and returns it. Open positions keep the book
/// they were opened on; only new entries follow the mode. The evaluation cache is cleared
/// so the next cycle reads the new mode.
pub async fn set_mode(
    strategy: &Strategy,
    requested: TradingMode,
    confirmation: Option<&str>,
) -> crate::Result<TradingMode> {
    let mode = decide_mode(
        strategy.mode,
        requested,
        confirmation,
        crate::trader::live_block(),
    )?;
    set_strategy_mode(&strategy.id, mode)?;
    if let Err(error) = super::clear_evaluation_cache().await {
        logger::warning(
            LogTag::System,
            &format!(
                "Strategy {} mode set to {}, evaluation cache not cleared: {error}",
                strategy.id,
                mode.as_str()
            ),
        );
    }
    Ok(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHRASE: Option<&str> = Some(STRATEGY_LIVE_CONFIRMATION);

    #[test]
    fn a_paper_strategy_arms_live_only_with_the_phrase_and_an_open_live_gate() {
        assert_eq!(
            decide_mode(TradingMode::Paper, TradingMode::Live, PHRASE, None).unwrap(),
            TradingMode::Live
        );
        assert!(matches!(
            decide_mode(TradingMode::Paper, TradingMode::Live, Some("arm"), None),
            Err(Error::LiveConfirmationRequired)
        ));
        for block in [LiveBlock::SetupIncomplete, LiveBlock::ForceStop] {
            assert!(
                matches!(
                    decide_mode(TradingMode::Paper, TradingMode::Live, PHRASE, Some(block)),
                    Err(Error::LiveUnavailable { reason }) if reason == block
                ),
                "{block:?} let a paper strategy arm live"
            );
        }
        assert!(
            matches!(
                decide_mode(
                    TradingMode::Paper,
                    TradingMode::Live,
                    None,
                    Some(LiveBlock::ForceStop)
                ),
                Err(Error::LiveConfirmationRequired)
            ),
            "the phrase is checked before the gate"
        );
    }

    #[test]
    fn returning_to_paper_and_staying_live_pass_a_closed_live_gate() {
        for block in [
            None,
            Some(LiveBlock::SetupIncomplete),
            Some(LiveBlock::ForceStop),
        ] {
            assert_eq!(
                decide_mode(TradingMode::Live, TradingMode::Paper, None, block).unwrap(),
                TradingMode::Paper
            );
            assert_eq!(
                decide_mode(TradingMode::Paper, TradingMode::Paper, None, block).unwrap(),
                TradingMode::Paper
            );
            assert_eq!(
                decide_mode(TradingMode::Live, TradingMode::Live, None, block).unwrap(),
                TradingMode::Live
            );
        }
    }
}
