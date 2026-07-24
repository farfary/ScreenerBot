//! Pure: position profit & loss — the single most consequential arithmetic in the bot.
//!
//! Every downstream decision reads this number. The emergency risk limit force-sells at
//! 90% down; the loss limiter pauses entries off realized P&L; the dashboard, the
//! Telegram notifications and the trading stats all render it. A P&L that is wrong in
//! the pessimistic direction liquidates healthy positions, and one wrong in the
//! optimistic direction hides losses until the wallet is empty.
//!
//! Four historical bugs are pinned down here, each of which produced a plausible-looking
//! but wrong number:
//!   * partial-exit proceeds omitted, so a position that sold half at 2x read as a loss;
//!   * cost basis taken from the FIRST buy, so a DCA'd position looked profitable as
//!     soon as it recovered that first buy;
//!   * `exit_price` treated as "closed", so a position whose close FAILED had its P&L
//!     computed as if it held nothing;
//!   * proceeds overwritten instead of accumulated on the final close.
//!
//! Decimals are pre-seeded into the token cache so the token-amount branches run with
//! no DB and no RPC.

mod common;

use common::{seed_decimals, test_position, TEST_MINT};
use screenerbot::positions::{
    calculate_position_pnl, calculate_position_pnl_safe, calculate_position_total_fees,
    calculate_split_pnl, Position,
};

const DECIMALS: u8 = 9;
const UNIT: f64 = 1_000_000_000.0; // 10^DECIMALS

/// Raw on-chain units for a UI token amount at [`DECIMALS`].
fn units(ui_amount: f64) -> u64 {
    (ui_amount * UNIT) as u64
}

/// 1 SOL spent at 0.01 SOL/token = 100 tokens held, nothing sold, no fees recorded.
///
/// Every scenario below starts here and changes exactly one thing, so a failing
/// assertion names the field that broke it.
fn open_position() -> Position {
    seed_decimals(TEST_MINT, DECIMALS);
    let mut position = test_position(0.01, 1.0);
    position.token_amount = Some(units(100.0));
    position.remaining_token_amount = Some(units(100.0));
    position
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{what}: expected {expected}, got {actual}"
    );
}

// ==================== OPEN POSITIONS ====================

#[tokio::test]
async fn an_open_position_is_valued_at_the_current_price() {
    let position = open_position();
    // 100 tokens at 0.02 = 2 SOL against a 1 SOL cost.
    let (pnl, pct) = calculate_position_pnl(&position, Some(0.02)).await;
    assert_close(pnl, 1.0, "pnl");
    assert_close(pct, 100.0, "pnl percent");
}

#[tokio::test]
async fn an_open_position_at_the_entry_price_is_flat() {
    let position = open_position();
    let (pnl, pct) = calculate_position_pnl(&position, Some(0.01)).await;
    assert_close(pnl, 0.0, "pnl");
    assert_close(pct, 0.0, "pnl percent");
}

#[tokio::test]
async fn an_open_position_that_halved_reports_a_fifty_percent_loss() {
    let position = open_position();
    let (pnl, pct) = calculate_position_pnl(&position, Some(0.005)).await;
    assert_close(pnl, -0.5, "pnl");
    assert_close(pct, -50.0, "pnl percent");
}

#[tokio::test]
async fn partial_exit_proceeds_count_towards_an_open_position() {
    // The regression: valuing only the REMAINING tokens against the FULL entry cost.
    // Here half the position was sold at 2x, taking 1 SOL off the table, so the
    // position is already break-even in cash and 100% up in total. Omitting
    // `sol_received` reported roughly a 50% LOSS — and the exit monitor acts on that.
    let mut position = open_position();
    position.remaining_token_amount = Some(units(50.0));
    position.total_exited_amount = units(50.0);
    position.sol_received = Some(1.0);
    position.partial_exit_count = 1;

    let (pnl, pct) = calculate_position_pnl(&position, Some(0.02)).await;
    assert_close(pnl, 1.0, "pnl must include realized proceeds");
    assert_close(pct, 100.0, "pnl percent");
}

#[tokio::test]
async fn the_cost_basis_of_a_dcad_position_includes_every_add() {
    // `total_size_sol` is the cumulative cost; `entry_size_sol` is only the first buy.
    // Using the latter made an averaged-down position look profitable the moment it
    // recovered its FIRST buy, while the position as a whole was still deep underwater.
    let mut position = open_position();
    position.entry_size_sol = 1.0; // first buy
    position.total_size_sol = 2.0; // plus one DCA of 1 SOL
    position.remaining_token_amount = Some(units(200.0));
    position.token_amount = Some(units(100.0)); // never grows on a DCA
    position.average_entry_price = 0.01;
    position.dca_count = 1;

    // 200 tokens at 0.01 = 2 SOL, exactly the 2 SOL invested.
    let (pnl, pct) = calculate_position_pnl(&position, Some(0.01)).await;
    assert_close(pnl, 0.0, "a DCA'd position at its average entry is flat");
    assert_close(pct, 0.0, "pnl percent");
}

#[tokio::test]
async fn holdings_come_from_the_remaining_amount_not_the_entry_amount() {
    // `token_amount` is the ENTRY buy and never grows on a DCA nor shrinks on a partial
    // exit. Valuing it would price tokens the wallet no longer holds.
    let mut position = open_position();
    position.token_amount = Some(units(100.0));
    position.remaining_token_amount = Some(units(10.0)); // 90 sold elsewhere
    position.sol_received = Some(0.0);

    let (pnl, _) = calculate_position_pnl(&position, Some(0.01)).await;
    assert_close(pnl, -0.9, "only the remaining 10 tokens may be valued");
}

#[tokio::test]
async fn fees_are_charged_once_for_the_buy_and_once_for_the_estimated_sell() {
    // An OPEN position has not paid its exit fee yet, so the estimate mirrors the
    // entry fee. Charging more (e.g. adding the profit buffer) inflates losses on
    // small trades, which is what the emergency exit measures.
    let mut position = open_position();
    position.entry_fee_lamports = Some(5_000_000); // 0.005 SOL

    let (pnl, _) = calculate_position_pnl(&position, Some(0.01)).await;
    assert_close(pnl, -0.01, "buy fee plus an equal estimated sell fee");
}

#[tokio::test]
async fn an_open_position_without_a_token_amount_falls_back_to_the_price_ratio() {
    // A position whose entry has not verified yet has no token amount. It must still
    // produce a sane percentage rather than zero.
    let mut position = open_position();
    position.token_amount = None;
    position.remaining_token_amount = None;

    let (pnl, pct) = calculate_position_pnl(&position, Some(0.02)).await;
    assert_close(pct, 100.0, "pnl percent from the price ratio");
    assert_close(pnl, 1.0, "pnl scaled by the entry size");
}

// ==================== FAILED / IN-FLIGHT CLOSES ====================

#[tokio::test]
async fn a_failed_close_is_still_an_open_position() {
    // `close_position_direct` stamps `exit_price` BEFORE the swap, and a failed close
    // clears the SIGNATURE, not the price. Keying "closed" off `exit_price` put a live
    // position into the closed branch, where its P&L was computed as
    // proceeds-minus-cost while IGNORING every token it still holds — a near-total loss
    // on a healthy position, and `check_risk_limits` force-exits at 90%.
    let mut position = open_position();
    position.exit_price = Some(0.005); // stamped, then the close failed
    position.exit_time = None; // never actually closed
    position.exit_transaction_signature = None; // cleared for retry
    position.sol_received = None;

    let (pnl, pct) = calculate_position_pnl(&position, Some(0.02)).await;
    assert_close(pnl, 1.0, "the still-held tokens must be valued");
    assert_close(pct, 100.0, "pnl percent");
}

#[tokio::test]
async fn a_close_in_flight_is_estimated_at_the_current_price() {
    // A submitted-but-unverified exit: the tokens are still ours until it lands, so the
    // position is valued live rather than frozen.
    let mut position = open_position();
    position.exit_transaction_signature = Some("pending-exit".to_owned());
    position.transaction_exit_verified = false;

    let (pnl, pct) = calculate_position_pnl(&position, Some(0.02)).await;
    assert_close(pnl, 1.0, "pnl");
    assert_close(pct, 100.0, "pnl percent");
}

#[tokio::test]
async fn a_close_in_flight_after_partial_exits_counts_the_realized_proceeds() {
    let mut position = open_position();
    position.exit_transaction_signature = Some("pending-exit".to_owned());
    position.transaction_exit_verified = false;
    position.remaining_token_amount = Some(units(50.0));
    position.total_exited_amount = units(50.0);
    position.sol_received = Some(1.0);

    let (pnl, _) = calculate_position_pnl(&position, Some(0.02)).await;
    assert_close(pnl, 1.0, "realized proceeds are part of the estimate");
}

#[tokio::test]
async fn a_close_in_flight_uses_the_cumulative_cost_basis() {
    let mut position = open_position();
    position.exit_transaction_signature = Some("pending-exit".to_owned());
    position.transaction_exit_verified = false;
    position.entry_size_sol = 1.0;
    position.total_size_sol = 2.0;
    position.remaining_token_amount = Some(units(200.0));

    let (pnl, _) = calculate_position_pnl(&position, Some(0.01)).await;
    assert_close(pnl, 0.0, "the DCA add is part of what must be earned back");
}

// ==================== CLOSED POSITIONS ====================

/// A closed position: entered for `invested` SOL, exited for `received` SOL.
fn closed_position(invested: f64, received: f64) -> Position {
    let mut position = open_position();
    position.entry_size_sol = invested;
    position.total_size_sol = invested;
    position.exit_price = Some(0.02);
    position.effective_exit_price = Some(0.02);
    position.exit_time = Some(chrono::Utc::now());
    position.exit_transaction_signature = Some("exit-sig".to_owned());
    position.transaction_exit_verified = true;
    position.sol_received = Some(received);
    position.remaining_token_amount = Some(0);
    position.total_exited_amount = units(100.0);
    position
}

#[tokio::test]
async fn a_closed_position_is_proceeds_minus_cost() {
    let position = closed_position(1.0, 2.5);
    let (pnl, pct) = calculate_position_pnl(&position, None).await;
    assert_close(pnl, 1.5, "pnl");
    assert_close(pct, 150.0, "pnl percent");
}

#[tokio::test]
async fn a_closed_position_needs_no_current_price() {
    // Realized P&L is history. Passing a live price must not change it, or a closed
    // position's recorded result would drift with the market forever.
    let position = closed_position(1.0, 2.5);
    let (with_price, _) = calculate_position_pnl(&position, Some(0.5)).await;
    let (without_price, _) = calculate_position_pnl(&position, None).await;
    assert_close(with_price, without_price, "a closed P&L is fixed");
}

#[tokio::test]
async fn closed_proceeds_accumulate_across_every_exit() {
    // `sol_received` is the position's TOTAL proceeds. Overwriting it with just the
    // final close discarded every partial exit's profit: a position that took 80% off
    // the table before the token died was recorded as a total loss.
    let mut position = closed_position(1.0, 0.0);
    position.sol_received = Some(0.8 + 0.05); // partial exits plus a dying final close
    let (pnl, _) = calculate_position_pnl(&position, None).await;
    assert_close(pnl, -0.15, "earlier proceeds stay on the books");
}

#[tokio::test]
async fn a_closed_dcad_position_must_earn_back_every_add() {
    // Cost basis is `total_size_sol`, not the first buy.
    let mut position = closed_position(1.0, 1.5);
    position.entry_size_sol = 1.0;
    position.total_size_sol = 2.0;

    let (pnl, pct) = calculate_position_pnl(&position, None).await;
    assert_close(pnl, -0.5, "recovering only the first buy is still a loss");
    assert_close(pct, -25.0, "pnl percent against the full basis");
}

#[tokio::test]
async fn a_closed_position_charges_both_real_fees() {
    let mut position = closed_position(1.0, 2.0);
    position.entry_fee_lamports = Some(5_000_000); // 0.005
    position.exit_fee_lamports = Some(7_000_000); // 0.007

    let (pnl, _) = calculate_position_pnl(&position, None).await;
    assert_close(pnl, 1.0 - 0.012, "both actual fees are deducted");
}

#[tokio::test]
async fn a_closed_position_with_no_cost_basis_does_not_divide_by_zero() {
    // Guards against an infinite percentage reaching the dashboard and the stats.
    let mut position = closed_position(0.0, 0.5);
    position.entry_size_sol = 0.0;
    position.total_size_sol = 0.0;

    let (pnl, pct) = calculate_position_pnl(&position, None).await;
    assert_close(pnl, 0.5, "pnl");
    assert!(pct.is_finite(), "percentage must stay finite, got {pct}");
}

// ==================== INVALID INPUTS ====================

#[tokio::test]
async fn an_unusable_entry_price_returns_a_neutral_pnl() {
    // Neutral, not a fabricated loss: a bad entry price must never look like a 100%
    // drawdown to the emergency exit.
    for bad in [0.0, -1.0, f64::NAN] {
        let mut position = open_position();
        position.average_entry_price = bad;
        position.effective_entry_price = Some(bad);
        position.entry_price = bad;

        let (pnl, pct) = calculate_position_pnl(&position, Some(0.02)).await;
        assert_eq!((pnl, pct), (0.0, 0.0), "entry price {bad}");
    }
}

#[tokio::test]
async fn an_unusable_current_price_returns_a_neutral_pnl() {
    for bad in [0.0, -1.0, f64::NAN] {
        let position = open_position();
        let (pnl, pct) = calculate_position_pnl(&position, Some(bad)).await;
        assert_eq!((pnl, pct), (0.0, 0.0), "current price {bad}");
    }
}

#[tokio::test]
async fn an_open_position_with_no_price_at_all_is_neutral() {
    let position = open_position();
    assert_eq!(calculate_position_pnl(&position, None).await, (0.0, 0.0));
}

#[tokio::test]
async fn the_safe_wrapper_separates_a_failure_from_a_break_even() {
    // `(0.0, 0.0)` is ambiguous — it is both "flat" and "could not compute". The safe
    // wrapper exists so a caller can tell them apart.
    let position = open_position();
    assert_eq!(
        calculate_position_pnl_safe(&position, Some(0.01)).await,
        Some((0.0, 0.0)),
        "a genuine break-even is Some"
    );

    let mut broken = open_position();
    broken.average_entry_price = 0.0;
    broken.effective_entry_price = Some(0.0);
    broken.entry_price = 0.0;
    assert_eq!(
        calculate_position_pnl_safe(&broken, Some(0.01)).await,
        None,
        "an uncomputable P&L is None"
    );

    assert_eq!(
        calculate_position_pnl_safe(&position, Some(-1.0)).await,
        None,
        "an unusable current price is None"
    );
}

// ==================== FEES ====================

#[tokio::test]
async fn total_fees_sum_both_legs_and_treat_missing_fees_as_zero() {
    let mut position = open_position();
    assert_close(calculate_position_total_fees(&position), 0.0, "no fees yet");

    position.entry_fee_lamports = Some(5_000_000);
    assert_close(
        calculate_position_total_fees(&position),
        0.005,
        "entry fee only",
    );

    position.exit_fee_lamports = Some(7_000_000);
    assert_close(calculate_position_total_fees(&position), 0.012, "both legs");
}

// ==================== SPLIT (REALIZED vs UNREALIZED) ====================

#[tokio::test]
async fn the_split_reports_realized_and_unrealized_separately() {
    // Half sold at 2x (1 SOL back on a 0.5 SOL basis = +0.5 realized), half still held
    // at 2x (1 SOL of value on a 0.5 SOL basis = +0.5 unrealized).
    let mut position = open_position();
    position.remaining_token_amount = Some(units(50.0));
    position.total_exited_amount = units(50.0);
    position.average_exit_price = Some(0.02);
    position.sol_received = Some(1.0);
    position.partial_exit_count = 1;

    let (realized, unrealized, total, total_pct) = calculate_split_pnl(&position, Some(0.02)).await;
    assert_close(realized, 0.5, "realized");
    assert_close(unrealized, 0.5, "unrealized");
    assert_close(total, 1.0, "total");
    assert_close(total_pct, 100.0, "total percent");
}

#[tokio::test]
async fn the_split_prices_a_dcad_position_against_the_tokens_it_actually_acquired() {
    // The denominator for "what share of the position is this" must be the tokens ever
    // ACQUIRED — `remaining + total_exited`. Using `token_amount` (the FIRST buy, which
    // never grows on a DCA) makes the shares of a DCA'd position exceed 1.0, so the
    // basis attributed to each leg is inflated and both halves of the split come out
    // wrong. Here: 2 SOL bought 200 tokens; 100 were sold for 2 SOL and 100 are still
    // held at 0.02 — each leg carries exactly half the 2 SOL basis.
    let mut position = open_position();
    position.entry_size_sol = 1.0;
    position.total_size_sol = 2.0;
    position.token_amount = Some(units(100.0)); // entry buy only
    position.remaining_token_amount = Some(units(100.0));
    position.total_exited_amount = units(100.0);
    position.average_exit_price = Some(0.02);
    position.sol_received = Some(2.0);
    position.partial_exit_count = 1;
    position.dca_count = 1;

    let (realized, unrealized, total, _) = calculate_split_pnl(&position, Some(0.02)).await;
    assert_close(realized, 1.0, "realized: 2 SOL back on a 1 SOL half-basis");
    assert_close(
        unrealized,
        1.0,
        "unrealized: 2 SOL of value on a 1 SOL half-basis",
    );
    assert_close(total, 2.0, "total");
}

#[tokio::test]
async fn a_position_with_no_exits_has_no_realized_pnl() {
    let position = open_position();
    let (realized, unrealized, _, _) = calculate_split_pnl(&position, Some(0.02)).await;
    assert_close(realized, 0.0, "nothing has been sold");
    assert_close(unrealized, 1.0, "the whole gain is unrealized");
}

#[tokio::test]
async fn the_split_is_neutral_when_the_entry_price_is_unusable() {
    let mut position = open_position();
    position.average_entry_price = 0.0;
    position.effective_entry_price = Some(0.0);
    position.entry_price = 0.0;

    assert_eq!(
        calculate_split_pnl(&position, Some(0.02)).await,
        (0.0, 0.0, 0.0, 0.0)
    );
}
