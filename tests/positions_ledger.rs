//! Pure: the wallet-history ledger reducer — how observed balance deltas become
//! position rounds.
//!
//! This is the accounting a new user's whole Positions view is built from, so the tests
//! that matter most are the ones pinning down what the reducer REFUSES to claim. A
//! fabricated cost basis here would render a confident, permanently wrong P&L on money
//! the bot never traded:
//!   * an airdrop has no cost, and treating that as a zero basis reads as infinite gain;
//!   * a USDC-quoted buy has a real basis we cannot express in SOL, and converting it at
//!     today's rate invents a number that was never true;
//!   * a token -> token swap has no SOL leg at all;
//!   * one SOL leg cannot be split across two positions bought in the same transaction.
//!
//! Each of those must clear `basis_complete` rather than guess. The round-boundary tests
//! pin the other half of the contract: a mint's balance returning to zero CLOSES a round
//! for good, and buying it again opens a genuinely new one rather than resurrecting the
//! old cost basis.
//!
//! No database, no network, no clock: `SubjectAssetDelta` values are built inline.

use screenerbot::constants::{SOL_MINT, USDC_MINT};
use screenerbot::positions::ledger::{
    reconcile_with_wallet, reduce_rounds, LedgerEventKind, LedgerRound, QuoteAsset, WalletHolding,
};
use screenerbot::transactions::deltas::{DeltaKind, SubjectAssetDelta, NATIVE_SOL_SENTINEL};

const WALLET: &str = "6uodGCMLfDLfeXkyW71WpUDK1BEG19bU2EV51x2dcGMv";
const MINT_A: &str = "MintA111111111111111111111111111111111111";
const MINT_B: &str = "MintB111111111111111111111111111111111111";
const TOKEN_DECIMALS: u8 = 6;
const LAMPORTS: f64 = 1_000_000_000.0;

/// One token delta. `before`/`after` are the chain's own raw balances, which the
/// reducer treats as authoritative.
fn token(
    signature: &str,
    slot: u64,
    mint: &str,
    delta: i128,
    before: u128,
    after: u128,
    kind: DeltaKind,
) -> SubjectAssetDelta {
    SubjectAssetDelta {
        wallet_address: WALLET.to_string(),
        signature: signature.to_string(),
        mint: mint.to_string(),
        slot: Some(slot),
        block_time: Some(slot as i64 * 100),
        tx_index: 0,
        delta_raw: delta,
        before_raw: Some(before),
        after_raw: Some(after),
        decimals: TOKEN_DECIMALS,
        kind,
        venue: matches!(kind, DeltaKind::Trade).then(|| "raydium".to_string()),
        fee_lamports: Some(5_000),
        success: true,
    }
}

/// The native SOL consideration leg of a trade, in whole SOL.
fn native(signature: &str, slot: u64, sol: f64) -> SubjectAssetDelta {
    SubjectAssetDelta {
        wallet_address: WALLET.to_string(),
        signature: signature.to_string(),
        mint: NATIVE_SOL_SENTINEL.to_string(),
        slot: Some(slot),
        block_time: Some(slot as i64 * 100),
        tx_index: 0,
        delta_raw: (sol * LAMPORTS) as i128,
        before_raw: None,
        after_raw: None,
        decimals: 9,
        kind: DeltaKind::Trade,
        venue: Some("raydium".to_string()),
        fee_lamports: Some(5_000),
        success: true,
    }
}

/// A quote leg denominated in some other reference asset (wSOL, USDC).
fn quote_leg(
    signature: &str,
    slot: u64,
    mint: &str,
    amount: f64,
    decimals: u8,
) -> SubjectAssetDelta {
    let mut delta = native(signature, slot, 0.0);
    delta.mint = mint.to_string();
    delta.decimals = decimals;
    delta.delta_raw = (amount * 10f64.powi(decimals as i32)) as i128;
    delta
}

/// Buy `tokens` whole units of `mint` for `sol` SOL, from a zero balance.
fn buy(signature: &str, slot: u64, mint: &str, tokens: f64, sol: f64) -> Vec<SubjectAssetDelta> {
    let raw = (tokens * 10f64.powi(TOKEN_DECIMALS as i32)) as u128;
    vec![
        token(signature, slot, mint, raw as i128, 0, raw, DeltaKind::Trade),
        native(signature, slot, -sol),
    ]
}

fn round_for<'a>(rounds: &'a [LedgerRound], mint: &str) -> &'a LedgerRound {
    rounds
        .iter()
        .find(|r| r.mint == mint)
        .unwrap_or_else(|| panic!("no round for {mint}"))
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

// ==================== round boundaries ====================

#[test]
fn a_traded_acquisition_from_zero_opens_an_open_round() {
    let rounds = reduce_rounds(&buy("sig1", 10, MINT_A, 1_000.0, 2.0));

    assert_eq!(rounds.len(), 1);
    let round = &rounds[0];
    assert!(round.is_open);
    assert_eq!(round.round_key, format!("sig1:{MINT_A}"));
    assert_eq!(round.entry_count, 1);
    assert_eq!(round.events[0].kind, LedgerEventKind::Entry);
    assert!(close(round.invested_sol, 2.0));
    assert!(round.basis_complete);
    assert!(round.history_complete);
}

#[test]
fn a_second_buy_is_an_add_within_the_same_round() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        1_000_000_000,
        1_000_000_000,
        2_000_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, -3.0));

    let rounds = reduce_rounds(&deltas);

    assert_eq!(rounds.len(), 1, "a DCA must not open a second round");
    let round = &rounds[0];
    assert_eq!(round.entry_count, 2);
    assert_eq!(round.events[1].kind, LedgerEventKind::Add);
    assert!(close(round.invested_sol, 5.0));
}

#[test]
fn a_partial_sell_keeps_the_round_open_and_is_recorded_separately() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -250_000_000,
        1_000_000_000,
        750_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, 1.0));

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert!(round.is_open);
    assert_eq!(round.exit_count, 1);
    assert_eq!(round.events[1].kind, LedgerEventKind::PartialExit);
    assert_eq!(round.balance_raw, 750_000_000);
    assert!(close(round.realized_proceeds_sol, 1.0));
}

#[test]
fn selling_the_whole_balance_closes_the_round() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -1_000_000_000,
        1_000_000_000,
        0,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, 3.0));

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert!(!round.is_open);
    assert_eq!(round.closed_at, Some(1_100));
    assert_eq!(round.events[1].kind, LedgerEventKind::Exit);
    assert_eq!(round.exit_signature.as_deref(), Some("sig2"));
    assert_eq!(round.realized_pnl_sol.map(|v| (v * 1e6).round()), Some(1e6));
}

#[test]
fn buying_again_after_a_close_starts_a_new_round_with_its_own_basis() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -1_000_000_000,
        1_000_000_000,
        0,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, 3.0));
    deltas.extend(buy("sig3", 12, MINT_A, 500.0, 4.0));

    let rounds = reduce_rounds(&deltas);

    assert_eq!(rounds.len(), 2, "a re-buy is a new round, not a reopen");
    let reopened = rounds.iter().find(|r| r.is_open).expect("second round");
    assert_eq!(reopened.round_key, format!("sig3:{MINT_A}"));
    assert!(
        close(reopened.invested_sol, 4.0),
        "the new round must not inherit the old cost basis"
    );
    assert_eq!(reopened.realized_proceeds_sol, 0.0);
}

// ==================== token -> token ====================

#[test]
fn a_token_to_token_swap_closes_one_round_and_opens_another() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    // One transaction: all of A out, B in. No SOL leg at all.
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -1_000_000_000,
        1_000_000_000,
        0,
        DeltaKind::Trade,
    ));
    deltas.push(token(
        "sig2",
        11,
        MINT_B,
        500_000_000,
        0,
        500_000_000,
        DeltaKind::Trade,
    ));

    let rounds = reduce_rounds(&deltas);

    assert_eq!(rounds.len(), 2);
    let a = round_for(&rounds, MINT_A);
    let b = round_for(&rounds, MINT_B);
    assert!(!a.is_open, "the sold mint's round closes");
    assert!(b.is_open, "the bought mint's round opens");
    assert_eq!(b.round_key, format!("sig2:{MINT_B}"));
}

#[test]
fn a_token_to_token_swap_yields_no_sol_basis_or_proceeds() {
    let deltas = vec![
        token(
            "sig1",
            10,
            MINT_A,
            -1_000_000_000,
            1_000_000_000,
            0,
            DeltaKind::Trade,
        ),
        token(
            "sig1",
            10,
            MINT_B,
            500_000_000,
            0,
            500_000_000,
            DeltaKind::Trade,
        ),
    ];

    let rounds = reduce_rounds(&deltas);
    let b = round_for(&rounds, MINT_B);

    assert!(
        !b.basis_complete,
        "no SOL leg means no basis — it must never be invented"
    );
    assert_eq!(b.realized_pnl_sol, None);
    assert_eq!(b.invested_sol, 0.0);
}

// ==================== what the reducer refuses to claim ====================

#[test]
fn an_airdrop_never_receives_a_cost_basis() {
    let deltas = vec![token(
        "sig1",
        10,
        MINT_A,
        1_000_000_000,
        0,
        1_000_000_000,
        DeltaKind::Transfer,
    )];

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert_eq!(round.events[0].kind, LedgerEventKind::Receive);
    assert_eq!(round.entry_count, 0);
    assert!(
        !round.basis_complete,
        "a zero basis on free tokens reads as infinite gain"
    );
    assert_eq!(round.realized_pnl_sol, None);
}

#[test]
fn a_usd_quoted_buy_is_recorded_but_never_converted_into_a_sol_basis() {
    let deltas = vec![
        token(
            "sig1",
            10,
            MINT_A,
            1_000_000_000,
            0,
            1_000_000_000,
            DeltaKind::Trade,
        ),
        quote_leg("sig1", 10, USDC_MINT, -250.0, 6),
    ];

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    let quote = round.events[0]
        .quote
        .expect("the USD leg is still recorded");
    assert_eq!(quote.asset, QuoteAsset::Usd);
    assert!(close(quote.amount, 250.0));
    assert!(
        !round.basis_complete,
        "USD is never back-converted to SOL at today's rate"
    );
    assert_eq!(round.invested_sol, 0.0);
    assert_eq!(round.average_entry_price_sol, None);
}

#[test]
fn a_round_bought_with_two_different_quote_assets_has_no_basis() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        1_000_000_000,
        1_000_000_000,
        2_000_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(quote_leg("sig2", 11, USDC_MINT, -300.0, 6));

    let rounds = reduce_rounds(&deltas);

    assert!(
        !rounds[0].basis_complete,
        "SOL and USD legs cannot be added into one basis"
    );
}

#[test]
fn one_sol_leg_is_not_split_across_two_positions_bought_together() {
    let deltas = vec![
        token(
            "sig1",
            10,
            MINT_A,
            1_000_000_000,
            0,
            1_000_000_000,
            DeltaKind::Trade,
        ),
        token(
            "sig1",
            10,
            MINT_B,
            500_000_000,
            0,
            500_000_000,
            DeltaKind::Trade,
        ),
        native("sig1", 10, -6.0),
    ];

    let rounds = reduce_rounds(&deltas);

    assert_eq!(rounds.len(), 2);
    for round in &rounds {
        assert!(
            !round.basis_complete,
            "attributing the whole 6 SOL to each would overstate both bases"
        );
        assert_eq!(round.invested_sol, 0.0);
    }
}

#[test]
fn a_round_already_open_before_our_history_is_marked_genesis() {
    // The first thing we ever see for this mint is a sale out of a balance we never
    // watched being acquired.
    let deltas = vec![
        token(
            "sig1",
            10,
            MINT_A,
            -400_000_000,
            1_000_000_000,
            600_000_000,
            DeltaKind::Trade,
        ),
        native("sig1", 10, 1.0),
    ];

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert!(round.is_genesis());
    assert_eq!(round.round_key, format!("genesis:{MINT_A}"));
    assert!(!round.basis_complete);
    assert!(!round.history_complete);
    assert_eq!(round.realized_pnl_sol, None);
    assert_eq!(round.balance_raw, 600_000_000);
}

#[test]
fn a_gap_in_observed_history_clears_the_completeness_flags() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    // The chain says we held 4_000 before this sale; we only ever saw 1_000 arrive.
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -1_000_000_000,
        4_000_000_000,
        3_000_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, 1.0));

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert!(!round.history_complete);
    assert!(!round.basis_complete);
    assert_eq!(round.realized_pnl_sol, None);
}

// ==================== basis arithmetic ====================

#[test]
fn cost_basis_is_released_pro_rata_across_successive_partial_exits() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 1.0);
    // Sell 250 of 1000 => a quarter of the basis leaves.
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -250_000_000,
        1_000_000_000,
        750_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, 0.5));
    // Sell 250 of the remaining 750 => a third of what is left.
    deltas.push(token(
        "sig3",
        12,
        MINT_A,
        -250_000_000,
        750_000_000,
        500_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig3", 12, 0.5));

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert!(close(round.realized_cost_sol, 0.5), "0.25 + 0.25 of basis");
    assert!(close(round.remaining_basis_sol, 0.5));
    assert!(close(round.realized_proceeds_sol, 1.0));
}

#[test]
fn average_entry_price_is_weighted_across_every_priced_acquisition() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 1.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        3_000_000_000,
        1_000_000_000,
        4_000_000_000,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, -9.0));

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    // 10 SOL for 4_000 tokens.
    assert!(close(round.average_entry_price_sol.unwrap(), 0.0025));
    assert!(close(round.events[0].price_sol.unwrap(), 0.001));
    assert!(close(round.events[1].price_sol.unwrap(), 0.003));
}

#[test]
fn realized_pnl_is_withheld_when_a_disposal_had_no_observable_proceeds() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    // Tokens sent to another wallet: a disposal, but not a sale.
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -1_000_000_000,
        1_000_000_000,
        0,
        DeltaKind::Transfer,
    ));

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert_eq!(round.events[1].kind, LedgerEventKind::Send);
    assert_eq!(round.exit_count, 0);
    assert_eq!(
        round.realized_pnl_sol, None,
        "sending tokens out is not proceeds"
    );
    assert!(!round.is_open);
}

#[test]
fn a_wsol_leg_is_priced_as_sol() {
    let deltas = vec![
        token(
            "sig1",
            10,
            MINT_A,
            1_000_000_000,
            0,
            1_000_000_000,
            DeltaKind::Trade,
        ),
        quote_leg("sig1", 10, SOL_MINT, -2.0, 9),
    ];

    let rounds = reduce_rounds(&deltas);
    let round = &rounds[0];

    assert_eq!(round.events[0].quote.unwrap().asset, QuoteAsset::Sol);
    assert!(close(round.invested_sol, 2.0));
    assert!(round.basis_complete);
}

#[test]
fn native_sol_wins_over_a_wsol_leg_in_the_same_transaction() {
    // A route that momentarily wraps SOL must still be read once, from the native leg.
    let deltas = vec![
        token(
            "sig1",
            10,
            MINT_A,
            1_000_000_000,
            0,
            1_000_000_000,
            DeltaKind::Trade,
        ),
        native("sig1", 10, -2.0),
        quote_leg("sig1", 10, SOL_MINT, -2.0, 9),
    ];

    let rounds = reduce_rounds(&deltas);

    assert!(
        close(rounds[0].invested_sol, 2.0),
        "the leg is counted once"
    );
}

// ==================== hygiene ====================

#[test]
fn reference_assets_never_become_positions() {
    let deltas = vec![
        native("sig1", 10, -2.0),
        quote_leg("sig1", 10, SOL_MINT, 2.0, 9),
        quote_leg("sig2", 11, USDC_MINT, 250.0, 6),
    ];

    assert!(
        reduce_rounds(&deltas).is_empty(),
        "SOL, wSOL and stablecoins denominate positions, they are not positions"
    );
}

#[test]
fn failed_transactions_contribute_nothing() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    for delta in deltas.iter_mut() {
        delta.success = false;
    }

    assert!(reduce_rounds(&deltas).is_empty());
}

#[test]
fn reduction_is_independent_of_the_order_the_deltas_arrive_in() {
    let mut deltas = buy("sig1", 10, MINT_A, 1_000.0, 2.0);
    deltas.push(token(
        "sig2",
        11,
        MINT_A,
        -1_000_000_000,
        1_000_000_000,
        0,
        DeltaKind::Trade,
    ));
    deltas.push(native("sig2", 11, 3.0));

    let forward = reduce_rounds(&deltas);
    deltas.reverse();
    let reversed = reduce_rounds(&deltas);

    assert_eq!(
        forward, reversed,
        "chain order is decided by slot, not input"
    );
}

// ==================== reconciliation against the wallet ====================

#[test]
fn reconciliation_defers_to_the_wallet_and_marks_the_round_incomplete() {
    let mut rounds = reduce_rounds(&buy("sig1", 10, MINT_A, 1_000.0, 2.0));
    assert!(rounds[0].basis_complete);

    reconcile_with_wallet(
        &mut rounds,
        &[WalletHolding {
            mint: MINT_A.to_string(),
            amount_raw: 400_000_000,
            decimals: TOKEN_DECIMALS,
        }],
    );

    let round = round_for(&rounds, MINT_A);
    assert_eq!(round.balance_raw, 400_000_000, "on-chain truth wins");
    assert!(!round.history_complete);
    assert!(!round.basis_complete);
    assert_eq!(round.realized_pnl_sol, None);
}

#[test]
fn reconciliation_closes_an_open_round_the_wallet_no_longer_holds() {
    let mut rounds = reduce_rounds(&buy("sig1", 10, MINT_A, 1_000.0, 2.0));

    reconcile_with_wallet(&mut rounds, &[]);

    let round = round_for(&rounds, MINT_A);
    assert!(!round.is_open);
    assert_eq!(round.balance_raw, 0);
    assert_eq!(
        round.closed_at, None,
        "we never saw it close, so no timestamp may be invented"
    );
    assert!(!round.history_complete);
}

#[test]
fn reconciliation_surfaces_a_holding_no_transaction_ever_explained() {
    let mut rounds = reduce_rounds(&buy("sig1", 10, MINT_A, 1_000.0, 2.0));

    reconcile_with_wallet(
        &mut rounds,
        &[
            WalletHolding {
                mint: MINT_A.to_string(),
                amount_raw: 1_000_000_000,
                decimals: TOKEN_DECIMALS,
            },
            WalletHolding {
                mint: MINT_B.to_string(),
                amount_raw: 7_500_000,
                decimals: TOKEN_DECIMALS,
            },
        ],
    );

    assert_eq!(rounds.len(), 2, "everything held must appear as a position");
    let b = round_for(&rounds, MINT_B);
    assert!(b.is_open);
    assert!(b.is_genesis());
    assert_eq!(b.balance_raw, 7_500_000);
    assert!(!b.basis_complete);
    assert!(b.events.is_empty());

    let a = round_for(&rounds, MINT_A);
    assert!(
        a.basis_complete,
        "a round that agrees with the wallet keeps its basis"
    );
}

#[test]
fn two_gaps_in_one_mint_produce_two_distinct_round_keys() {
    // History with two holes: each time our first sight of the mint is a sale off a
    // balance we never watched arrive, so each is its own genesis round. A round key is
    // a UNIQUE index on `positions` — two bare `genesis:<mint>` keys would either fail
    // to insert the second row or collapse both holdings onto one.
    let deltas = vec![
        token(
            "gap-one",
            10,
            MINT_A,
            -1_000_000,
            1_000_000,
            0,
            DeltaKind::Trade,
        ),
        native("gap-one", 10, 2.0),
        token(
            "gap-two",
            20,
            MINT_A,
            -3_000_000,
            3_000_000,
            0,
            DeltaKind::Trade,
        ),
        native("gap-two", 20, 4.0),
    ];

    let rounds = reduce_rounds(&deltas);
    assert_eq!(rounds.len(), 2);
    assert!(rounds.iter().all(LedgerRound::is_genesis));
    assert_ne!(
        rounds[0].round_key, rounds[1].round_key,
        "each gap needs its own identity"
    );
    assert!(rounds
        .iter()
        .any(|round| round.round_key == format!("genesis:{MINT_A}")));
}

#[test]
fn a_held_mint_never_reuses_a_closed_genesis_key() {
    // The wallet still holds a mint whose only history is a genesis round that already
    // closed. The synthetic round for the holding must not claim that round's key.
    let deltas = vec![
        token(
            "gap-one",
            10,
            MINT_A,
            -1_000_000,
            1_000_000,
            0,
            DeltaKind::Trade,
        ),
        native("gap-one", 10, 2.0),
    ];

    let mut rounds = reduce_rounds(&deltas);
    reconcile_with_wallet(
        &mut rounds,
        &[WalletHolding {
            mint: MINT_A.to_string(),
            amount_raw: 5_000_000,
            decimals: TOKEN_DECIMALS,
        }],
    );

    assert_eq!(rounds.len(), 2);
    assert_ne!(rounds[0].round_key, rounds[1].round_key);
    let held = rounds.iter().find(|round| round.is_open).expect("held");
    assert_eq!(held.balance_raw, 5_000_000);
}
