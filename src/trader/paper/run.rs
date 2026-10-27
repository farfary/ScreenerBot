// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper run of the shared paper book: its account, the reset that settles the rounds
//! still open and starts the next run, and the settlement of chosen open rounds.

use chrono::Utc;

use crate::positions::paper::PaperAccount;
use crate::positions::Position;
use crate::trader::types::TradeReason;
use crate::trader::{Error, Result};

/// The configured starting balance of a paper run.
fn starting_balance_native() -> f64 {
    crate::config::with_config(|config| config.trader.paper.starting_balance_sol)
}

/// The account of the current paper run, starting the first run when there is none.
pub async fn paper_account() -> Result<PaperAccount> {
    let book = crate::positions::paper::book().ok_or(Error::PaperBookUnavailable)?;
    let wallet = crate::utils::get_wallet_address().map_err(|_| Error::PaperBookUnavailable)?;
    Ok(book.account(&wallet, starting_balance_native()).await?)
}

/// Resets the paper run: every open paper round closes at its last mark as a paper run
/// reset, then a new run starts with the configured starting balance.
pub async fn reset_paper_run() -> Result<PaperAccount> {
    let book = crate::positions::paper::book().ok_or(Error::PaperBookUnavailable)?;
    let wallet = crate::utils::get_wallet_address().map_err(|_| Error::PaperBookUnavailable)?;
    let chain = crate::chains::active_chain();
    let reason = format!("{:?}", TradeReason::PaperReset);
    let account = crate::global::paper_scope(book.reset_run(
        &wallet,
        starting_balance_native(),
        &reason,
        |mint| crate::tokens::get_cached_decimals(chain, mint),
        Utc::now(),
    ))
    .await?;
    crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
            "Paper run reset: epoch {} starts with {:.4} SOL",
            account.epoch, account.starting_balance_native
        ),
    );
    Ok(account)
}

/// Closes each of `rounds` still open at its last mark as a paper run reset, returning
/// their value to the paper cash; the run goes on. Returns how many rounds closed.
pub async fn settle_rounds(rounds: Vec<Position>) -> Result<usize> {
    if rounds.is_empty() {
        return Ok(0);
    }
    let book = crate::positions::paper::book().ok_or(Error::PaperBookUnavailable)?;
    let wallet = crate::utils::get_wallet_address().map_err(|_| Error::PaperBookUnavailable)?;
    let chain = crate::chains::active_chain();
    let reason = format!("{:?}", TradeReason::PaperReset);
    let closed = crate::global::paper_scope(book.settle_at_mark(
        &wallet,
        &rounds,
        &reason,
        |mint| crate::tokens::get_cached_decimals(chain, mint),
        Utc::now(),
    ))
    .await?;
    crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!("Paper rounds settled at their mark: {closed}"),
    );
    Ok(closed)
}
