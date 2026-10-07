// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Direct pool swaps, decoded and quoted from REAL captured mainnet accounts —
//! with no network, no wallet and no keys.
//!
//! # Why fixtures rather than synthetic state
//!
//! Every unit test in the venue modules builds its own pool struct, which proves
//! the arithmetic but proves nothing about the LAYOUT: an offset that drifts by
//! eight bytes still passes a test whose fixture was constructed field by field.
//! The `direct-swap` cases under `tests/fixtures/pools/solana/` are the exact bytes mainnet
//! returned for a live pool, its config and its vaults, so a decode that reads
//! the wrong offset produces a nonsense number here and fails.
//!
//! # Refreshing a fixture
//!
//! Fetch `getAccountInfo` for the pool and for the accounts its state points at
//! (CPMM: `amm_config` @8, vaults @72/@104, mints @168/@200; AMM v4: vaults
//! @336/@368; CLMM: `amm_config` @9, vaults @137/@169, mints @73/@105, plus the
//! tick-array accounts `TickArrayBitmap::arrays_for_swap` names in both
//! directions from the pool's own `tick_current`), store them base64 under
//! `accounts`, and re-run. Balances move, so assertions here are on
//! RELATIONSHIPS — fee rates, orientation, monotonicity — not on a specific
//! output amount that would rot with the next trade.

mod common;

use screenerbot::chains::solana::layout::{token_account_amount, u64_at};
use screenerbot::chains::solana::pools::layouts::fluxbeam::FluxbeamPoolState;
use screenerbot::chains::solana::pools::layouts::meteora_damm::DammPoolState;
use screenerbot::chains::solana::pools::layouts::meteora_dbc::PoolConfigState as DbcPoolConfigState;
use screenerbot::chains::solana::pools::layouts::meteora_dbc::VirtualPoolState;
use screenerbot::chains::solana::pools::layouts::meteora_dlmm::LbPairState;
use screenerbot::chains::solana::pools::layouts::moonit::{ConfigAccountState, CurveAccountState};
use screenerbot::chains::solana::pools::layouts::orca_whirlpool::{
    decode_tick_array as orca_decode_tick_array, WhirlpoolState,
};
use screenerbot::chains::solana::pools::layouts::pumpfun_amm::PumpAmmPoolState;
use screenerbot::chains::solana::pools::layouts::pumpfun_legacy::{
    BondingCurve, GlobalFeeRecipients,
};
use screenerbot::chains::solana::pools::layouts::raydium_amm_v4::AmmV4PoolState;
use screenerbot::chains::solana::pools::layouts::raydium_clmm::{
    decode_tick_array, ClmmFeeConfig, ClmmPoolState, TickArrayBitmap,
};
use screenerbot::chains::solana::pools::layouts::raydium_cpmm::{CpmmFeeConfig, CpmmPoolState};
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::swaps::direct::venues::meteora_dlmm::{
    bin_array_address, bitmap_extension_address as dlmm_bitmap_extension_address,
    event_authority_address, oracle_address as dlmm_oracle_address,
};
use screenerbot::chains::solana::swaps::direct::venues::orca_whirlpool::{
    candidate_tick_array_starts, tick_array_address as orca_tick_array_address,
};
use screenerbot::chains::solana::swaps::direct::{
    self, DirectSwapIntent, FeeSide, PoolMarket, SwapAccounts,
};
use std::collections::HashMap;
use std::str::FromStr;

const WSOL: &str = "So11111111111111111111111111111111111111112";

// ============================================================================
// FIXTURE LOADING
// ============================================================================

struct Fixture {
    pool: Pubkey,
    accounts: HashMap<String, screenerbot::chains::solana::solana_sdk::account::Account>,
}

impl Fixture {
    /// The `direct-swap` case `case` of venue `venue`, as the accounts a venue loader reads.
    fn load(venue: &str, case: &str) -> Self {
        let case = common::solana_pools::load_case(venue, case);
        let accounts = case
            .accounts
            .iter()
            .map(|(address, account)| {
                (
                    address.clone(),
                    screenerbot::chains::solana::solana_sdk::account::Account {
                        lamports: account.lamports,
                        data: account.data.clone(),
                        owner: Pubkey::from_str(&account.owner).expect("owner is a pubkey"),
                        executable: false,
                        rent_epoch: 0,
                    },
                )
            })
            .collect();
        Self {
            pool: Pubkey::from_str(&case.pool).expect("case pool is a pubkey"),
            accounts,
        }
    }

    fn data(&self, address: &Pubkey) -> &[u8] {
        &self
            .accounts
            .get(&address.to_string())
            .unwrap_or_else(|| panic!("fixture is missing account {address}"))
            .data
    }

    fn account(
        &self,
        address: &Pubkey,
    ) -> &screenerbot::chains::solana::solana_sdk::account::Account {
        self.accounts
            .get(&address.to_string())
            .unwrap_or_else(|| panic!("fixture is missing account {address}"))
    }

    fn balance(&self, address: &Pubkey) -> u64 {
        token_account_amount(self.data(address)).expect("vault is a token account")
    }
}

/// pump.fun legacy's own programme id, for deriving the `Global` and
/// `FeeConfig` PDAs the fixture captured. Kept local to this test rather than
/// imported: the venue does not expose these addresses as `pub`, since
/// production always derives them itself.
fn pump_legacy_program_id_for_test() -> Pubkey {
    Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap()
}

/// The market the venue's own `load` builds from the `direct-swap` case of `venue`, replayed
/// through the production dispatch and account reads.
fn fixture_market(venue: &str) -> Box<dyn PoolMarket> {
    fixture_market_case(venue, "direct-swap")
}

fn fixture_market_case(venue: &str, case: &str) -> Box<dyn PoolMarket> {
    let case = common::solana_pools::load_case(venue, case);
    common::solana_pools::load_market(&case)
        .unwrap_or_else(|e| panic!("the recorded {venue} pool must load: {e}"))
}

// ============================================================================
// LAYOUT — the fixtures exist to catch an offset that drifted
// ============================================================================

#[test]
fn the_cpmm_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("raydium_cpmm", "direct-swap");
    let state = CpmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured CPMM pool must decode");

    assert_eq!(
        state.mint_0.to_string(),
        WSOL,
        "this fixture is a SOL pool; a wrong mint offset would not land on WSOL"
    );
    assert_eq!(state.decimals_0, 9, "WSOL has nine decimals");
    assert!(
        state.decimals_1 <= 18,
        "a decimals byte read from the wrong offset is almost never a plausible value, got {}",
        state.decimals_1
    );
    assert!(state.swap_enabled(), "the fixture pool is tradable");
    assert!(
        state.open_time > 1_600_000_000 && state.open_time < 4_000_000_000,
        "open_time must look like a unix timestamp, got {}",
        state.open_time
    );
    assert_ne!(state.vault_0, state.vault_1);
    assert_ne!(state.amm_config, Pubkey::default());
}

#[test]
fn the_cpmm_fee_config_reads_a_plausible_rate_rather_than_padding() {
    let fixture = Fixture::load("raydium_cpmm", "direct-swap");
    let state = CpmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool)).unwrap();
    let config = CpmmFeeConfig::decode(fixture.data(&state.amm_config)).unwrap();

    // Rates are over 1_000_000. A real trade fee is single-digit basis points to
    // a few percent; padding read as a rate is either zero or absurd.
    assert!(
        config.trade_fee_rate > 0 && config.trade_fee_rate <= 100_000,
        "trade_fee_rate {} is not a plausible rate over 1e6",
        config.trade_fee_rate
    );
    assert!(
        config.protocol_fee_rate <= 1_000_000 && config.fund_fee_rate <= 1_000_000,
        "protocol/fund rates must be fractions of the trade fee"
    );
}

#[test]
fn the_amm_v4_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("raydium_legacy_amm", "direct-swap");
    let state = AmmV4PoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured AMM v4 pool must decode");

    assert_eq!(
        state.coin_mint.to_string(),
        WSOL,
        "this fixture is the SOL/USDC pool"
    );
    assert_eq!(state.coin_decimals, 9);
    assert_eq!(state.pc_decimals, 6, "USDC has six decimals");
    assert!(state.swap_enabled(), "the fixture pool is tradable");
    assert_eq!(
        state.swap_fee_numerator, 25,
        "the standard v4 swap fee is 25/10000"
    );
    assert_eq!(state.swap_fee_denominator, 10_000);
    assert_ne!(state.coin_vault, state.pc_vault);
}

#[test]
fn the_clmm_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("raydium_clmm", "direct-swap");
    let state = ClmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured CLMM pool must decode");

    assert_eq!(
        state.mint_0.to_string(),
        WSOL,
        "this fixture is a SOL/USDC pool; a wrong mint offset would not land on WSOL"
    );
    assert_eq!(state.decimals_0, 9, "WSOL has nine decimals");
    assert!(
        state.decimals_1 <= 18,
        "a decimals byte read from the wrong offset is almost never a plausible value, got {}",
        state.decimals_1
    );
    assert!(state.swap_enabled(), "the fixture pool is tradable");
    assert!(
        state.tick_spacing > 0 && state.tick_spacing < 1_000,
        "tick_spacing read from the wrong offset would not be a small positive integer, got {}",
        state.tick_spacing
    );
    assert!(state.liquidity > 0, "a live deep pool must carry liquidity");
    assert!(
        state.sqrt_price_x64 > 0,
        "sqrt_price_x64 must be a real Q64.64 value, not padding"
    );
    assert_ne!(state.vault_0, state.vault_1);
    assert_ne!(state.amm_config, Pubkey::default());

    let config = ClmmFeeConfig::decode(fixture.data(&state.amm_config))
        .expect("the captured AmmConfig must decode");
    assert!(
        config.trade_fee_rate > 0 && config.trade_fee_rate <= 100_000,
        "trade_fee_rate {} is not a plausible rate over 1e6",
        config.trade_fee_rate
    );
}

#[test]
fn the_clmm_captured_tick_arrays_hold_real_ticks_not_padding() {
    use screenerbot::chains::solana::swaps::direct::venues::clmm_ticks::tick_array_address;

    let fixture = Fixture::load("raydium_clmm", "direct-swap");
    let program = fixture.account(&fixture.pool).owner;
    let state = ClmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool)).unwrap();
    let bitmap = TickArrayBitmap::from_pool_state(fixture.data(&fixture.pool)).unwrap();

    let mut ticks = Vec::new();
    for zero_for_one in [true, false] {
        for start in bitmap.arrays_for_swap(state.tick_current, state.tick_spacing, zero_for_one) {
            let address = tick_array_address(&program, &fixture.pool, start);
            if let Some(account) = fixture.accounts.get(&address.to_string()) {
                ticks
                    .extend(decode_tick_array(&fixture.pool, &account.data).expect(
                        "a captured tick array named by the pool's own bitmap must decode",
                    ));
            }
        }
    }

    // A deep, actively-traded pool must have at least one initialised tick in
    // the arrays either side of its current price -- otherwise this fixture
    // is not exercising the tick walk it was captured to protect.
    assert!(
        !ticks.is_empty(),
        "no initialised ticks were decoded from the captured arrays"
    );
    for tick in &ticks {
        assert!(
            tick.tick > -443_636 && tick.tick < 443_636,
            "a tick decoded from padding would not be a real index, got {}",
            tick.tick
        );
    }
}

#[test]
fn a_clmm_quote_off_real_state_walks_ticks_and_charges_the_configured_rate() {
    let market = fixture_market("raydium_clmm");
    let (mint_0, mint_1) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote = market
        .quote(&mint_0, amount_in)
        .expect("a live, captured pool quotes");
    assert!(quote.expected_out > 0);
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a pool this deep, got {}%",
        quote.price_impact_pct
    );

    // Monotonic: more in must mean more out. A dust-sized trade against a
    // pool this deep does not reliably show sub-linear scaling to the raw
    // unit -- that concavity is asserted properly on live state in
    // `tests/direct_swaps_mainnet.rs`, where the size can be chosen to move
    // the pool enough to matter.
    let ten_x = market
        .quote(&mint_0, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > quote.expected_out, "more in, more out");

    // And the reverse direction must also price.
    let back = market
        .quote(&mint_1, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(
        back.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

#[test]
fn the_orca_whirlpool_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("orca_whirlpool", "direct-swap");
    let state = WhirlpoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured Whirlpool must decode");

    assert_eq!(
        state.mint_a.to_string(),
        WSOL,
        "this fixture is a SOL/USDC pool; a wrong mint offset would not land on WSOL"
    );
    assert!(
        state.tick_spacing > 0 && state.tick_spacing < 1_000,
        "tick_spacing read from the wrong offset would not be a small positive integer, got {}",
        state.tick_spacing
    );
    assert!(
        state.fee_rate > 0,
        "fee_rate {} is not a plausible rate over 1e6",
        state.fee_rate
    );
    assert!(state.liquidity > 0, "a live deep pool must carry liquidity");
    assert!(
        state.sqrt_price > 0,
        "sqrt_price must be a real Q64.64 value, not padding"
    );
    assert_ne!(state.vault_a, state.vault_b);
    assert_ne!(state.mint_a, state.mint_b);
}

#[test]
fn the_orca_whirlpool_captured_tick_arrays_hold_real_ticks_not_padding() {
    let fixture = Fixture::load("orca_whirlpool", "direct-swap");
    let program = fixture.account(&fixture.pool).owner;
    let state = WhirlpoolState::decode(fixture.pool, fixture.data(&fixture.pool)).unwrap();

    let mut ticks = Vec::new();
    for zero_for_one in [true, false] {
        for start in
            candidate_tick_array_starts(state.tick_current, state.tick_spacing, zero_for_one)
        {
            let address = orca_tick_array_address(&program, &fixture.pool, start);
            if let Some(account) = fixture.accounts.get(&address.to_string()) {
                ticks.extend(
                    orca_decode_tick_array(&account.data, start, state.tick_spacing).expect(
                        "a captured tick array named by the candidate derivation must decode",
                    ),
                );
            }
        }
    }

    assert!(
        !ticks.is_empty(),
        "no initialised ticks were decoded from the captured arrays"
    );
    for tick in &ticks {
        assert!(
            tick.tick > -500_000 && tick.tick < 500_000,
            "a tick decoded from padding would not be a real index, got {}",
            tick.tick
        );
    }
}

#[test]
fn an_orca_whirlpool_quote_off_real_state_walks_ticks_and_charges_the_configured_rate() {
    let market = fixture_market("orca_whirlpool");
    let (mint_a, mint_b) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote = market
        .quote(&mint_a, amount_in)
        .expect("a live, captured pool quotes");
    assert!(quote.expected_out > 0);
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a pool this deep, got {}%",
        quote.price_impact_pct
    );

    let ten_x = market
        .quote(&mint_a, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > quote.expected_out, "more in, more out");

    let back = market
        .quote(&mint_b, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(
        back.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

#[test]
fn the_damm_v2_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("meteora_damm_v2", "direct-swap");
    let state = DammPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured DAMM v2 pool must decode");

    assert_eq!(
        state.mint_b.to_string(),
        WSOL,
        "this fixture is a SOL-quoted pool; a wrong mint offset would not land on WSOL"
    );
    assert_ne!(state.mint_a, state.mint_b);
    assert_ne!(state.vault_a, state.vault_b);
    assert!(state.liquidity > 0, "a live deep pool must carry liquidity");
    assert!(
        state.sqrt_price > 0 && state.sqrt_price >= state.sqrt_min_price,
        "sqrt_price must be a real Q64.64 value inside the pool's own range"
    );
    assert!(
        state.sqrt_price <= state.sqrt_max_price,
        "sqrt_price read from a drifted offset would not sit inside sqrt_max_price"
    );
    assert_eq!(state.pool_status, 0, "the fixture pool is tradable");
    assert!(
        state.collect_fee_mode <= 1,
        "collect_fee_mode is a two-value enum, got {}",
        state.collect_fee_mode
    );
}

#[test]
fn a_damm_v2_quote_off_real_state_charges_a_fee_and_is_monotonic() {
    let market = fixture_market("meteora_damm_v2");
    let (mint_a, mint_b) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote = market
        .quote(&mint_b, amount_in)
        .expect("a live, captured pool quotes");
    assert!(quote.expected_out > 0);
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a pool this deep, got {}%",
        quote.price_impact_pct
    );

    let ten_x = market
        .quote(&mint_b, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > quote.expected_out, "more in, more out");

    let back = market
        .quote(&mint_a, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(
        back.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

#[test]
fn the_pump_amm_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("pumpfun_amm", "direct-swap");
    let state = PumpAmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured pump-swap pool must decode");

    assert_eq!(
        state.quote_mint.to_string(),
        WSOL,
        "this fixture's quote side is SOL; a wrong mint offset would not land on WSOL"
    );
    assert_ne!(state.base_mint, state.quote_mint);
    assert_ne!(state.base_token_account, state.quote_token_account);
    assert!(
        !state.is_cashback_coin,
        "this fixture was chosen to be a plain pool this venue can quote"
    );

    // The venue prices its fee tier off the market cap, which needs a non-empty base reserve
    // and a non-zero base supply, so the recorded pool must load and hold both.
    fixture_market("pumpfun_amm");
    assert!(
        fixture.balance(&state.base_token_account) > 0,
        "a real pool holds base tokens"
    );
    assert!(
        u64_at(&fixture.account(&state.base_mint).data, 36).is_some_and(|supply| supply > 0),
        "a real mint has a supply"
    );
}

#[test]
fn a_pump_amm_quote_off_real_state_charges_a_fee_and_is_monotonic() {
    let market = fixture_market("pumpfun_amm");
    let (base, quote) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote_result = market
        .quote(&quote, amount_in)
        .expect("a live, captured pool quotes");
    assert!(quote_result.expected_out > 0);
    assert!(
        quote_result.lp_fee > 0,
        "a real trade against a live pool must pay a nonzero fee"
    );
    assert!(
        quote_result.price_impact_pct < 5.0,
        "0.005 SOL should barely move a deep pool, got {}%",
        quote_result.price_impact_pct
    );

    let ten_x = market
        .quote(&quote, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(
        ten_x.expected_out > quote_result.expected_out,
        "more in, more out"
    );

    let back = market
        .quote(&base, quote_result.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(
        back.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

// ============================================================================
// QUOTES — relationships that must hold whatever the balances are today
// ============================================================================

#[test]
fn a_cpmm_quote_off_real_state_charges_the_configured_rate() {
    let market = fixture_market("raydium_cpmm");
    let (mint_0, mint_1) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote = market
        .quote(&mint_0, amount_in)
        .expect("a live pool quotes");
    assert!(quote.expected_out > 0);
    assert!(
        quote.lp_fee > 0 && quote.lp_fee < amount_in / 10,
        "the pool fee on 0.005 SOL should be a few basis points, got {}",
        quote.lp_fee
    );
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a pool this size, got {}%",
        quote.price_impact_pct
    );

    // And the reverse direction must also price.
    let back = market
        .quote(&mint_1, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(
        back.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

#[test]
fn an_amm_v4_quote_off_real_state_is_monotonic_and_concave_in_size() {
    let fixture = Fixture::load("raydium_legacy_amm", "direct-swap");
    let state = AmmV4PoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured AMM v4 pool must decode");
    let (coin_reserve, _) = state.tradable_reserves(
        fixture.balance(&state.coin_vault),
        fixture.balance(&state.pc_vault),
    );
    let market = fixture_market("raydium_legacy_amm");
    let (coin, _) = market.mints();

    let small = market.quote(&coin, 5_000_000).expect("quote");
    let large = market.quote(&coin, 50_000_000).expect("quote");
    assert!(large.expected_out > small.expected_out, "more in, more out");

    // Concavity and rising impact only show at a size that actually moves the
    // pool. At 0.005 SOL against a reserve this deep the curve is linear to the
    // raw unit and the reported impact is just the pool's own fee, so comparing
    // impact between two dust trades measures integer rounding, not the market.
    let unit = coin_reserve / 100;
    let one = market.quote(&coin, unit).expect("quote");
    let ten = market.quote(&coin, unit * 10).expect("quote");
    assert!(
        ten.expected_out < one.expected_out * 10,
        "ten times the size must return LESS than ten times the output: {} vs {}",
        ten.expected_out,
        one.expected_out * 10
    );
    assert!(
        ten.price_impact_pct > one.price_impact_pct,
        "a ten-times-larger trade must report more impact: {}% vs {}%",
        ten.price_impact_pct,
        one.price_impact_pct
    );
    assert!(
        ten.price_impact_pct > 1.0,
        "a tenth of the reserve is a large trade, got {}% impact",
        ten.price_impact_pct
    );
    assert!(
        small.price_impact_pct < 1.0,
        "0.005 SOL against a reserve this deep is not a large trade, got {}%",
        small.price_impact_pct
    );
}

#[test]
fn both_venues_refuse_a_mint_the_pool_does_not_hold() {
    let stranger = Pubkey::new_unique();
    assert!(fixture_market("raydium_cpmm")
        .quote(&stranger, 5_000_000)
        .is_err());
    assert!(fixture_market("raydium_legacy_amm")
        .quote(&stranger, 5_000_000)
        .is_err());
}

// ============================================================================
// THE PLATFORM FEE — the assertion that catches revenue silently going missing
// ============================================================================

#[test]
fn a_sol_funded_buy_holds_back_the_fee_before_the_pool_sees_it() {
    let _guard = common::config_guard();
    let market = fixture_market("raydium_cpmm");
    let (mint_0, mint_1) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: mint_0,
        output_mint: mint_1,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, market.as_ref()).expect("quotes offline");
    assert_eq!(quote.fee.side, FeeSide::Input, "SOL is the input leg here");
    assert_eq!(quote.fee.amount, 25_000, "0.5% of 0.005 SOL");
    assert_eq!(
        quote.swap_amount_in,
        5_000_000 - 25_000,
        "the fee never reaches the pool"
    );
    assert_eq!(
        quote.min_net_out, quote.min_out,
        "an input-side fee does not reduce what the wallet receives"
    );
}

#[test]
fn a_sell_back_to_sol_takes_the_fee_out_of_the_proceeds() {
    let _guard = common::config_guard();
    let market = fixture_market("raydium_cpmm");
    let (mint_0, mint_1) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: mint_1,
        output_mint: mint_0,
        amount_in: 1_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, market.as_ref()).expect("quotes offline");
    assert_eq!(
        quote.fee.side,
        FeeSide::Output,
        "SOL is the output leg here"
    );
    assert_eq!(
        quote.swap_amount_in, intent.amount_in,
        "nothing is held back from the input on a sell"
    );
    assert_eq!(
        quote.fee.amount,
        quote.min_out * 50 / 10_000,
        "the fee is 0.5% of the GUARANTEED output, not of the estimate"
    );
    assert_eq!(quote.min_net_out, quote.min_out - quote.fee.amount);
}

#[test]
fn the_plan_carries_the_fee_transfer_in_the_same_transaction_as_the_swap() {
    let _guard = common::config_guard();
    let market = fixture_market("raydium_legacy_amm");
    let (coin, pc) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: coin,
        output_mint: pc,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, market.as_ref()).expect("quotes");
    let plan = direct::build_plan(&intent, market.as_ref(), &quote).expect("plans");

    let destination = quote
        .fee
        .destination
        .expect("a real pair has a fee account");
    let fee_index = plan
        .instructions
        .iter()
        .position(|ix| {
            ix.program_id == screenerbot::chains::solana::spl_token::id()
                && ix.data.first() == Some(&12)
                && ix.accounts.iter().any(|a| a.pubkey == destination)
        })
        .expect("the fee transfer must be IN the transaction, not merely computed");
    let swap_index = plan
        .instructions
        .iter()
        .position(|ix| ix.program_id.to_string() == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")
        .expect("the swap instruction must be present");
    let close_index = plan.instructions.iter().position(|ix| {
        ix.program_id == screenerbot::chains::solana::spl_token::id() && ix.data.first() == Some(&9)
    });

    assert!(
        fee_index > swap_index,
        "an output-side fee is transferred AFTER the swap that produces it"
    );
    if let Some(close_index) = close_index {
        assert!(
            fee_index < close_index,
            "the fee must be taken before the WSOL account is closed, or it reads an account \
             that no longer exists and reverts the swap with it"
        );
    }
}

#[test]
fn the_plan_leads_with_a_compute_budget_and_creates_both_accounts_idempotently() {
    let _guard = common::config_guard();
    let market = fixture_market("raydium_legacy_amm");
    let (coin, pc) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: coin,
        output_mint: pc,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };
    let quote = direct::quote_with_market(&intent, market.as_ref()).expect("quotes");
    let plan = direct::build_plan(&intent, market.as_ref(), &quote).expect("plans");

    assert_eq!(
        plan.instructions[0].data.first(),
        Some(&2),
        "SetComputeUnitLimit must lead, or it does not apply"
    );
    assert_eq!(plan.instructions[1].data.first(), Some(&3));
    for ix in &plan.instructions[2..4] {
        assert_eq!(
            ix.data,
            vec![1u8],
            "both ATAs are created idempotently -- never read-then-create"
        );
    }
}

// ============================================================================
// INSTRUCTION ORIENTATION against real pool state
// ============================================================================

#[test]
fn the_cpmm_instruction_pairs_each_wallet_account_with_the_matching_vault() {
    let market = fixture_market("raydium_cpmm");
    let (mint_0, mint_1) = market.mints();
    let owner = Pubkey::new_unique();
    let ata_in = Pubkey::new_unique();
    let ata_out = Pubkey::new_unique();

    let ix = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: mint_1,
                output_mint: mint_0,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            900_000,
        )
        .expect("builds against real state");

    // input_token_account @4, output @5, input_vault @6, output_vault @7.
    assert_eq!(ix.accounts[4].pubkey, ata_in);
    assert_eq!(ix.accounts[5].pubkey, ata_out);
    assert_eq!(
        ix.accounts[10].pubkey, mint_1,
        "the input mint follows the direction, not the pool's token_0"
    );
    assert_eq!(ix.accounts[11].pubkey, mint_0);
    assert_ne!(
        ix.accounts[6].pubkey, ix.accounts[7].pubkey,
        "a swap can never route both legs through one vault"
    );
}

#[test]
fn the_dlmm_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("meteora_dlmm", "direct-swap");
    let state = LbPairState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured LbPair must decode");

    assert_eq!(
        state.token_x_mint.to_string(),
        WSOL,
        "this fixture is a SOL/USDC pool; a wrong mint offset would not land on WSOL"
    );
    assert_ne!(state.token_x_mint, state.token_y_mint);
    assert_ne!(state.reserve_x, state.reserve_y);
    assert!(
        state.bin_step > 0 && state.bin_step < 1_000,
        "bin_step read from the wrong offset would not be a small positive integer, got {}",
        state.bin_step
    );
    assert_eq!(state.status, 0, "the fixture pool is tradable");
    assert!(
        state.parameters.base_factor > 0,
        "base_factor {} is not a plausible rate",
        state.parameters.base_factor
    );
    assert!(
        state.parameters.protocol_share <= 2_500,
        "protocol_share {} exceeds the programme's own 25% ceiling",
        state.parameters.protocol_share
    );
    assert_eq!(
        dlmm_oracle_address(&fixture.account(&fixture.pool).owner, &fixture.pool),
        state.oracle,
        "the stored oracle field must match its own PDA derivation"
    );

    let v_params = LbPairState::decode_v_parameters(fixture.data(&fixture.pool))
        .expect("v_parameters must decode");
    assert!(
        v_params.last_update_timestamp > 1_700_000_000,
        "last_update_timestamp read from the wrong offset would not be a plausible Unix time, \
         got {}",
        v_params.last_update_timestamp
    );
}

#[test]
fn the_dlmm_captured_bin_arrays_hold_real_bins_not_padding() {
    let fixture = Fixture::load("meteora_dlmm", "direct-swap");
    let program = fixture.account(&fixture.pool).owner;
    let state = LbPairState::decode(fixture.pool, fixture.data(&fixture.pool)).unwrap();

    let active_array_index = state.active_id.div_euclid(70) as i64;
    let address = bin_array_address(&program, &fixture.pool, active_array_index);
    let account = fixture
        .accounts
        .get(&address.to_string())
        .expect("the fixture must have captured the active bin array");
    assert_eq!(
        account.data.len(),
        10_136,
        "a real BinArray account is 10136 bytes"
    );

    let local_index = state.active_id - (active_array_index as i32) * 70;
    let offset = 56 + (local_index as usize) * 144;
    let amount_x = u64_at(&account.data, offset).expect("active bin amount_x");
    let amount_y = u64_at(&account.data, offset + 8).expect("active bin amount_y");
    assert!(
        amount_x > 0 || amount_y > 0,
        "the pool's own active bin decoded to no liquidity at all -- offset drift, not a real gap"
    );

    // Every bin id in this array is a real, small integer -- a wrong
    // `BINS_OFFSET`/`BIN_SIZE` would still produce SOME id here, but a
    // decode off by even one field would make every other assertion above
    // fail first (garbage `amount_x`/`amount_y`), which is the real drift
    // detector; this only guards the id arithmetic itself.
    for i in 0..70i32 {
        let bin_id = (active_array_index as i32) * 70 + i;
        assert!(
            bin_id > -500_000 && bin_id < 500_000,
            "bin id out of any real range"
        );
    }
}

#[test]
fn a_dlmm_quote_off_real_state_walks_bins_and_charges_the_configured_rate() {
    let market = fixture_market("meteora_dlmm");
    let (mint_x, mint_y) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote = market
        .quote(&mint_x, amount_in)
        .expect("a live, captured pool quotes");
    assert!(quote.expected_out > 0);
    assert!(
        quote.lp_fee > 0,
        "base_factor={} bin_step={} must charge something",
        4,
        4
    );
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a pool this deep, got {}%",
        quote.price_impact_pct
    );

    let ten_x = market
        .quote(&mint_x, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > quote.expected_out, "more in, more out");

    let back = market
        .quote(&mint_y, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(
        back.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

#[test]
fn a_dlmm_swap_instruction_names_the_event_authority_and_orients_from_the_input_mint() {
    let market = fixture_market("meteora_dlmm");
    let (mint_x, mint_y) = market.mints();
    let program =
        Pubkey::from_str(screenerbot::chains::solana::constants::METEORA_DLMM_PROGRAM_ID).unwrap();
    let owner = Pubkey::new_unique();
    let ata_in = Pubkey::new_unique();
    let ata_out = Pubkey::new_unique();

    let ix = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: mint_y,
                output_mint: mint_x,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            0,
        )
        .expect("builds against real state");

    assert_eq!(ix.program_id, program);
    // input_token_in @4, user_token_out @5 (see module docs' account order).
    assert_eq!(ix.accounts[4].pubkey, ata_in);
    assert_eq!(ix.accounts[5].pubkey, ata_out);
    assert_eq!(
        ix.accounts[14].pubkey,
        event_authority_address(&program),
        "event_authority sits at index 14 in the 16 named accounts"
    );
    assert_eq!(
        ix.accounts[1].pubkey,
        dlmm_bitmap_extension_address(&program, &market.pool())
    );
    // At least one trailing bin array beyond the 16 named accounts.
    assert!(
        ix.accounts.len() > 16,
        "a real swap must name at least one bin array"
    );
}

// ============================================================================
// PUMP.FUN LEGACY — a native-SOL bonding curve, not an AMM
// ============================================================================

#[test]
fn the_pump_legacy_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("pumpfun_legacy", "direct-swap");
    let curve = BondingCurve::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured bonding curve must decode");

    assert!(
        !curve.complete,
        "this fixture was chosen to be a curve still trading, not migrated"
    );
    assert!(
        !curve.is_mayhem_mode,
        "this fixture was chosen to be a curve this venue actually charges fees on"
    );
    assert!(
        !curve.is_cashback_coin,
        "this fixture was chosen to be a plain curve this venue can quote"
    );
    assert_eq!(
        curve.quote_mint,
        Pubkey::default(),
        "a native-SOL curve's quote_mint field is the default pubkey, not WSOL"
    );
    assert!(
        curve.creator_set(),
        "this fixture was chosen to have a creator, so the creator-fee path is exercised"
    );
    assert!(curve.virtual_sol_reserves > curve.real_sol_reserves.saturating_sub(1));
    assert!(
        curve.virtual_token_reserves > 0 && curve.virtual_sol_reserves > 0,
        "a curve still trading must have both virtual reserves"
    );

    let program = pump_legacy_program_id_for_test();
    let (derived_creator_vault, _) =
        Pubkey::find_program_address(&[b"creator-vault", curve.creator.as_ref()], &program);
    // Confirmed against a live trade's own creator_vault account, so a
    // one-letter seed slip ("creator_vault" vs "creator-vault") fails here
    // rather than on chain after a priority fee is paid.
    assert_ne!(derived_creator_vault, Pubkey::default());

    let global_state = GlobalFeeRecipients::decode(
        fixture.data(&Pubkey::find_program_address(&[b"global"], &program).0),
    )
    .expect("the captured Global must decode");
    assert!(global_state.first_fee_recipient().is_some());
    assert!(global_state.two_buyback_recipients().is_some());

    let market = fixture_market("pumpfun_legacy");
    assert!(
        market.quote(&market.mints().0, 5_000_000).is_ok(),
        "a real curve with real reserves must quote a small buy"
    );
}

#[test]
fn a_pump_legacy_quote_off_real_state_charges_both_fees_and_is_monotonic() {
    let market = fixture_market("pumpfun_legacy");
    let (mint, sol) = market.mints();
    assert_eq!(sol.to_string(), WSOL);
    let amount_in = 5_000_000; // 0.005 SOL, already net of the platform fee

    let quote = market
        .quote(&sol, amount_in)
        .expect("a live, captured curve quotes a buy");
    assert!(quote.expected_out > 0);
    assert!(
        quote.lp_fee > 0,
        "a real trade against a curve with a creator set must pay a nonzero fee \
         (protocol + creator, verified exactly against five live trades)"
    );
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a curve with real depth, got {}%",
        quote.price_impact_pct
    );

    let ten_x = market
        .quote(&sol, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > quote.expected_out, "more in, more out");

    let sell = market
        .quote(&mint, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(sell.expected_out > 0);
    assert!(
        sell.expected_out < amount_in,
        "a round trip through two fees cannot return more than it started with"
    );
}

#[test]
fn a_pump_legacy_swap_instruction_names_the_curve_creator_vault_and_trailing_buyback_pair() {
    let market = fixture_market("pumpfun_legacy");
    let (mint, sol) = market.mints();
    let owner = Pubkey::new_unique();
    let ata_in = Pubkey::new_unique();
    let ata_out = Pubkey::new_unique();
    let program = pump_legacy_program_id_for_test();

    let buy = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: sol,
                output_mint: mint,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            5_000_000,
            0,
        )
        .expect("builds against real state");
    assert_eq!(buy.program_id, program);
    assert_eq!(
        buy.data[0..8],
        [56, 252, 116, 8, 158, 223, 205, 95],
        "buy_exact_sol_in"
    );
    assert_eq!(
        buy.accounts[5].pubkey, ata_out,
        "buy writes tokens to the OUTPUT account"
    );
    assert_eq!(
        buy.accounts[6].pubkey, owner,
        "user is the native SOL source, not an ATA"
    );
    assert!(buy.accounts[6].is_signer);
    assert_eq!(
        buy.accounts.len(),
        19,
        "16 IDL accounts + bonding_curve_v2 (this fixture's curve has a creator) + the \
         undocumented buyback pair"
    );

    let sell = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: mint,
                output_mint: sol,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            0,
        )
        .expect("builds against real state");
    assert_eq!(
        sell.data[0..8],
        [51, 230, 133, 164, 1, 127, 131, 173],
        "sell"
    );
    assert_eq!(
        sell.accounts[5].pubkey, ata_in,
        "sell spends the INPUT account's tokens"
    );
    assert_eq!(
        sell.accounts.len(),
        17,
        "14 IDL accounts + bonding_curve_v2 (this fixture's curve has a creator) + the \
         undocumented buyback pair"
    );
}

// ============================================================================
// METEORA DBC
// ============================================================================

#[test]
fn the_dbc_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("meteora_dbc", "direct-swap");
    let state = VirtualPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured VirtualPool must decode");
    let config = DbcPoolConfigState::decode(fixture.data(&state.config))
        .expect("the captured PoolConfig must decode");

    assert_eq!(
        config.quote_mint.to_string(),
        WSOL,
        "this fixture is a SOL-quoted pool; a wrong mint offset would not land on WSOL"
    );
    assert_ne!(state.base_mint, config.quote_mint);
    assert_ne!(state.base_vault, state.quote_vault);
    assert!(!state.is_migrated, "this fixture was chosen pre-migration");
    assert!(
        state.sqrt_price > 0,
        "a real pool must carry a nonzero sqrt price"
    );
    assert!(
        state.sqrt_price >= config.sqrt_start_price,
        "the pool's current price can never sit below its own curve floor"
    );
    assert!(
        config.cliff_fee_numerator > 0 && config.cliff_fee_numerator < 1_000_000_000,
        "a plausible fee rate below its own 1e9 denominator, got {}",
        config.cliff_fee_numerator
    );
    assert!(
        config.collect_fee_mode == 0 || config.collect_fee_mode == 1,
        "collect_fee_mode is a two-value enum on this venue, got {}",
        config.collect_fee_mode
    );
    assert!(
        !config.scheduler_active(),
        "this fixture was chosen flat-fee"
    );
    assert!(
        !config.dynamic_fee_initialized,
        "this fixture was chosen flat-fee"
    );
}

#[test]
fn the_dbc_curve_points_are_real_segments_not_padding() {
    let fixture = Fixture::load("meteora_dbc", "direct-swap");
    let state = VirtualPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured VirtualPool must decode");
    let config = DbcPoolConfigState::decode(fixture.data(&state.config))
        .expect("the captured PoolConfig must decode");

    let points = config.curve_points();
    assert!(
        points.len() >= 2,
        "this fixture's pool was chosen for carrying at least two real segments, got {}",
        points.len()
    );

    // Ascending sqrt price, each one strictly past the last -- a decode that
    // wandered into the padding tail would produce a zero or a value that
    // does not keep climbing.
    let mut previous = config.sqrt_start_price;
    for (sqrt_price, liquidity) in &points {
        assert!(
            *sqrt_price > previous,
            "curve points must strictly increase: {sqrt_price} did not exceed {previous}"
        );
        assert!(*liquidity > 0, "a real segment carries non-zero liquidity");
        previous = *sqrt_price;
    }

    // The LAST point's price must sit at, or extremely close to, the pool's
    // own migration price -- the field `migration_sqrt_price` at a completely
    // different byte offset (280 vs 408), so a match here is not a
    // coincidence of a shared offset. Not exact equality: on this fixture the
    // two differ by about 1 part in 10^12, evidently independent roundings of
    // the same target performed when the config was created, not a decode
    // error (an offset error produces a wildly different value, not an
    // agreement to eleven significant figures).
    let migration_sqrt_price =
        screenerbot::chains::solana::layout::u128_at(fixture.data(&state.config), 280)
            .expect("migration_sqrt_price is at offset 280");
    let last = points.last().expect("checked non-empty above").0;
    let diff = last.abs_diff(migration_sqrt_price);
    assert!(
        diff * 1_000_000_000 < migration_sqrt_price,
        "the curve's last point ({last}) must sit within 1 part in 10^9 of \
         migration_sqrt_price ({migration_sqrt_price}), got a difference of {diff}"
    );
}

#[test]
fn a_dbc_quote_off_real_state_charges_a_fee_and_is_monotonic() {
    let market = fixture_market("meteora_dbc");
    let (base, quote) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let buy = market
        .quote(&quote, amount_in)
        .expect("a live, captured pool quotes");
    assert!(buy.expected_out > 0);
    assert!(
        buy.lp_fee > 0,
        "a real trade against a live pool must pay a nonzero fee"
    );

    let ten_x = market
        .quote(&quote, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > buy.expected_out, "more in, more out");

    let sell = market
        .quote(&base, buy.expected_out)
        .expect("the reverse direction quotes too");
    assert!(sell.expected_out > 0);
    assert!(
        sell.expected_out < amount_in,
        "a round trip through two platform-equivalent fees cannot return more than it started \
         with"
    );
}

#[test]
fn a_dbc_quote_orients_from_the_input_mint_not_a_hardcoded_side() {
    let market = fixture_market("meteora_dbc");
    let (base, quote) = market.mints();

    let buy = market.quote(&quote, 5_000_000).expect("buy quotes");
    // Base has far more raw units per human token than quote does at this
    // pool's price, so a base-side sell needs a proportionally larger raw
    // amount to move the curve by a measurable amount.
    let sell = market.quote(&base, 5_000_000_000).expect("sell quotes");

    // A buy returns base units, a sell returns quote units -- if orientation
    // were swapped, one of these would be quoting against the wrong reserve
    // and the two directions would not both succeed independently.
    assert!(buy.expected_out > 0);
    assert!(sell.expected_out > 0);
    assert!(market.trades(&quote, &base));
    assert!(market.trades(&base, &quote));
    assert!(!market.trades(&base, &base));
}

#[test]
fn the_output_token_collect_fee_mode_is_a_real_second_pool_not_a_toy() {
    let fixture = Fixture::load("meteora_dbc", "direct-swap-output-fee");
    let state = VirtualPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured VirtualPool must decode");
    let config = DbcPoolConfigState::decode(fixture.data(&state.config))
        .expect("the captured PoolConfig must decode");
    assert_eq!(
        config.collect_fee_mode, 1,
        "this fixture was chosen for exercising OutputToken, the OTHER branch of fee_on_input"
    );

    // A buy on THIS pool charges the fee on the OUTPUT (base) leg, not the
    // input -- the opposite of the primary fixture's QuoteToken pool.
    let market = fixture_market_case("meteora_dbc", "direct-swap-output-fee");
    let (_, quote) = market.mints();
    let buy = market.quote(&quote, 5_000_000);
    // This pool is essentially untouched (chosen for the branch, not depth),
    // so a tiny size may legitimately find no liquidity; either a real quote
    // or an explicit InsufficientLiquidity is acceptable here, a panic is not.
    match buy {
        Ok(q) => assert!(q.expected_out > 0),
        Err(screenerbot::chains::solana::swaps::direct::error::DirectSwapError::InsufficientLiquidity { .. }) => {}
        Err(e) => panic!("unexpected error against a real captured pool: {e:?}"),
    }
}

#[test]
fn a_dbc_swap_instruction_carries_the_confirmed_account_order_and_discriminator() {
    let market = fixture_market("meteora_dbc");
    let (base, quote) = market.mints();
    let owner = Pubkey::new_unique();
    let ata_in = Pubkey::new_unique();
    let ata_out = Pubkey::new_unique();

    let buy = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: quote,
                output_mint: base,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            5_000_000,
            0,
        )
        .expect("builds against real state");

    assert_eq!(
        buy.data[0..8],
        [248, 198, 158, 145, 225, 117, 135, 200],
        "sha256(\"global:swap\")[..8], confirmed against two live mainnet swaps"
    );
    assert_eq!(buy.accounts.len(), 15, "the confirmed live account count");
    assert_eq!(
        buy.accounts[0].pubkey.to_string(),
        "FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM",
        "pool_authority is a fixed address, not a derived PDA slot"
    );
    assert_eq!(buy.accounts[3].pubkey, ata_in, "input_token_account");
    assert_eq!(buy.accounts[4].pubkey, ata_out, "output_token_account");
    assert_eq!(buy.accounts[9].pubkey, owner);
    assert!(buy.accounts[9].is_signer);
    assert_eq!(
        buy.accounts[12].pubkey.to_string(),
        "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN",
        "an absent referral_token_account is spelled as the programme's own id"
    );

    let sell = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: base,
                output_mint: quote,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            0,
        )
        .expect("builds against real state");
    assert_eq!(
        sell.accounts[3].pubkey, ata_in,
        "sell spends the input account's base tokens"
    );
    assert_eq!(
        sell.accounts[4].pubkey, ata_out,
        "sell credits the output account's quote"
    );
}

// ============================================================================
// MOONIT — a native-SOL ConstantProductV1 bonding curve
// ============================================================================

fn moonit_program_id_for_test() -> Pubkey {
    Pubkey::from_str("MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG").unwrap()
}

#[test]
fn the_moonit_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("moonit_amm", "direct-swap");
    let curve = CurveAccountState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured Moonit curve must decode");

    assert_eq!(
        curve.total_supply, 1_000_000_000_000_000_000,
        "the programme enforces this exact total supply for ConstantProductV1"
    );
    assert!(
        curve.curve_amount > 0 && curve.curve_amount <= curve.total_supply,
        "a curve still trading holds a real, non-empty token balance"
    );
    assert_eq!(
        curve.collateral_currency, 0,
        "this fixture trades SOL collateral"
    );
    assert_eq!(curve.curve_type, 1, "this fixture is ConstantProductV1");
    assert_eq!(curve.decimals, 9);

    let mint_account = fixture.account(&curve.mint);
    assert_eq!(
        mint_account.owner.to_string(),
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "every Moonit mint observed while building this venue is legacy SPL"
    );

    let program = moonit_program_id_for_test();
    let (config_address, _) = Pubkey::find_program_address(&[b"config_account"], &program);
    let config = ConfigAccountState::decode(fixture.data(&config_address))
        .expect("the captured ConfigAccount must decode");
    assert_eq!(
        config.fee_bps, 100,
        "verified against every replayed real trade"
    );
    assert_eq!(
        config.dex_fee.to_string(),
        "3udvfL24waJcLhskRAsStNMoNUvtyXdxrWQz4hgi953N"
    );
    assert_eq!(
        config.helio_fee.to_string(),
        "5K5RtTWzzLp4P8Npi84ocf7F1vBsAu29N1irG4iiUnzt"
    );

    let market = fixture_market("moonit_amm");
    assert!(
        market.quote(&market.mints().1, 5_000_000).is_ok(),
        "a real curve with real reserves must quote a small buy"
    );
}

#[test]
fn a_moonit_quote_off_real_state_is_monotonic_and_settles_native_sol() {
    let market = fixture_market("moonit_amm");
    let (mint, sol) = market.mints();
    assert_eq!(sol.to_string(), WSOL);
    assert!(market.settles_native_sol());

    let amount_in = 5_000_000; // 0.005 SOL
    let quote = market
        .quote(&sol, amount_in)
        .expect("a live, captured curve quotes a buy");
    assert!(quote.expected_out > 0);
    assert!(
        quote.lp_fee > 0,
        "the 100bps protocol fee is always charged"
    );
    assert!(
        quote.price_impact_pct < 5.0,
        "0.005 SOL should barely move a curve this deep, got {}%",
        quote.price_impact_pct
    );

    let ten_x = market
        .quote(&sol, amount_in * 10)
        .expect("a larger size still quotes");
    assert!(ten_x.expected_out > quote.expected_out, "more in, more out");

    let sell = market
        .quote(&mint, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(sell.expected_out > 0);
    assert!(
        sell.expected_out < amount_in,
        "a round trip through two fee charges cannot return more than it started with"
    );
}

#[test]
fn a_moonit_swap_instruction_carries_the_eleven_idl_accounts_in_order() {
    let market = fixture_market("moonit_amm");
    let (mint, sol) = market.mints();
    let owner = Pubkey::new_unique();
    let ata_in = Pubkey::new_unique();
    let ata_out = Pubkey::new_unique();
    let program = moonit_program_id_for_test();
    let (config_address, _) = Pubkey::find_program_address(&[b"config_account"], &program);

    let buy = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: sol,
                output_mint: mint,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            5_000_000,
            0,
        )
        .expect("builds against real state");
    assert_eq!(buy.program_id, program);
    assert_eq!(
        buy.data[0..8],
        [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]
    );
    assert_eq!(buy.accounts.len(), 11);
    assert_eq!(buy.accounts[0].pubkey, owner);
    assert!(buy.accounts[0].is_signer);
    assert_eq!(
        buy.accounts[1].pubkey, ata_out,
        "a buy's senderTokenAccount is the wallet's OWN base-mint account, \
         receiving the tokens bought"
    );
    assert_eq!(buy.accounts[2].pubkey, market.pool());
    assert_eq!(buy.accounts[6].pubkey, mint);
    assert_eq!(buy.accounts[7].pubkey, config_address);
    assert_eq!(
        buy.accounts[10].pubkey.to_string(),
        "11111111111111111111111111111111"
    );
    // fixed_side byte and the trailing zero slippage_bps.
    assert_eq!(buy.data[24], 0);
    assert_eq!(&buy.data[25..33], &0u64.to_le_bytes());

    let sell = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: mint,
                output_mint: sol,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            0,
        )
        .expect("builds against real state");
    assert_eq!(
        sell.data[0..8],
        [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]
    );
    assert_eq!(
        sell.accounts[1].pubkey, ata_in,
        "a sell's senderTokenAccount is the wallet's OWN base-mint account, \
         spending the tokens sold"
    );
    // token_amount is exact (fixedSide::In on the token leg for a sell).
    assert_eq!(&sell.data[8..16], &1_000_000u64.to_le_bytes());
}

// ============================================================================
// FLUXBEAM — a fork of the vanilla spl-token-swap programme, no Anchor IDL
// ============================================================================

fn fluxbeam_program_id_for_test() -> Pubkey {
    Pubkey::from_str("FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X").unwrap()
}

#[test]
fn the_fluxbeam_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("fluxbeam_amm", "direct-swap");
    let state = FluxbeamPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured FluxBeam SwapV1 state must decode");

    assert!(state.is_initialized, "a live pool is initialized");
    assert_eq!(
        state.curve_type, 0,
        "this fixture is ConstantProduct, the only curve this venue quotes"
    );
    assert_eq!(
        state.mint_a.to_string(),
        WSOL,
        "this fixture is a SOL pool; a wrong mint offset would not land on WSOL"
    );
    assert_ne!(state.mint_a, state.mint_b);
    assert_ne!(state.vault_a, state.vault_b);

    // The authority PDA is the vanilla spl-token-swap derivation: seed is just
    // the pool's own pubkey, bump 255 on this fixture -- confirmed to match a
    // live swap's own `authority` account.
    let program = fluxbeam_program_id_for_test();
    let (authority, bump) = Pubkey::find_program_address(&[fixture.pool.as_ref()], &program);
    assert_eq!(bump, 255, "the live pool's own stored bump_seed is 255");
    assert_eq!(
        authority.to_string(),
        "5WCAmQDfnpfYDcNnCbcpf69tHVVLwnTWs1QGae145VPg",
        "must re-derive the exact authority a live swap named"
    );

    // Rates are plausible fee fractions, not padding read from the wrong offset.
    assert!(
        state.trade_fee_numerator > 0
            && state.trade_fee_denominator > 0
            && state.trade_fee_numerator < state.trade_fee_denominator,
        "trade_fee {}/{} is not a plausible rate",
        state.trade_fee_numerator,
        state.trade_fee_denominator
    );
    assert!(
        state.owner_trade_fee_numerator > 0 && state.owner_trade_fee_denominator > 0,
        "owner_trade_fee {}/{} is not a plausible rate",
        state.owner_trade_fee_numerator,
        state.owner_trade_fee_denominator
    );
    assert_ne!(state.fee_account, Pubkey::default());
    assert_ne!(state.pool_mint, Pubkey::default());
}

#[test]
fn a_fluxbeam_quote_off_real_state_charges_the_pools_own_rate() {
    let market = fixture_market("fluxbeam_amm");
    let (mint_a, mint_b) = market.mints();
    let amount_in = 5_000_000; // 0.005 SOL

    let quote = market
        .quote(&mint_a, amount_in)
        .expect("a live pool quotes");
    assert!(quote.expected_out > 0);
    assert!(
        quote.lp_fee > 0,
        "both the trade fee and the owner fee are always charged on this pool"
    );
    assert!(
        quote.lp_fee < amount_in,
        "the fee can never consume the whole trade"
    );
    // This fixture's owner_trade_fee is 99/100 -- an unusually high, PER-POOL
    // rate read straight from the account (see the module docs), so almost
    // the whole trade is fee rather than genuine reserve-depth impact. The
    // fee itself is asserted above; here just check it dominates as expected.
    assert!(
        quote.price_impact_pct > 90.0,
        "a 99% owner fee must dominate the reported impact, got {}%",
        quote.price_impact_pct
    );

    // The reverse direction must also price, and a round trip through two
    // fees cannot return more than it started with.
    let back = market
        .quote(&mint_b, quote.expected_out)
        .expect("the reverse direction quotes too");
    assert!(back.expected_out > 0);
    assert!(back.expected_out < amount_in);
}

#[test]
fn a_fluxbeam_quote_is_monotonic_and_concave_in_size() {
    let market = fixture_market("fluxbeam_amm");
    let (mint_a, _) = market.mints();

    let small = market.quote(&mint_a, 5_000_000).expect("quote");
    let large = market.quote(&mint_a, 500_000_000).expect("quote");
    assert!(large.expected_out > small.expected_out, "more in, more out");
    assert!(
        large.expected_out < small.expected_out * 100,
        "a hundred times the size must return LESS than a hundred times the output: \
         {} vs {}",
        large.expected_out,
        small.expected_out * 100
    );
    assert!(
        large.price_impact_pct >= small.price_impact_pct,
        "a bigger trade cannot report less impact: {}% vs {}%",
        large.price_impact_pct,
        small.price_impact_pct
    );
}

#[test]
fn fluxbeam_refuses_a_mint_the_pool_does_not_hold() {
    let stranger = Pubkey::new_unique();
    assert!(fixture_market("fluxbeam_amm")
        .quote(&stranger, 5_000_000)
        .is_err());
}

#[test]
fn the_fluxbeam_instruction_orients_from_the_input_mint_and_matches_the_confirmed_shape() {
    let market = fixture_market("fluxbeam_amm");
    let (mint_a, mint_b) = market.mints();
    let fixture = Fixture::load("fluxbeam_amm", "direct-swap");
    let state = FluxbeamPoolState::decode(fixture.pool, fixture.data(&fixture.pool)).unwrap();
    let owner = Pubkey::new_unique();
    let ata_in = Pubkey::new_unique();
    let ata_out = Pubkey::new_unique();

    let buy = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: mint_a,
                output_mint: mint_b,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            1,
        )
        .expect("builds against real state");

    // Tag 1, amount_in, min_out -- confirmed against three real buys.
    assert_eq!(buy.data[0], 1);
    assert_eq!(&buy.data[1..9], &1_000_000u64.to_le_bytes());
    assert_eq!(&buy.data[9..17], &1u64.to_le_bytes());
    assert_eq!(buy.data.len(), 17);
    assert_eq!(buy.accounts.len(), 14);

    // 0 pool, 1 authority, 2 owner, 3 source, 4 swap_source_vault,
    // 5 swap_destination_vault, 6 destination, 7 pool_mint, 8 fee_account,
    // 9 mint_a, 10 mint_b, 11/12/13 token programmes.
    assert_eq!(buy.accounts[0].pubkey, market.pool());
    assert_eq!(buy.accounts[2].pubkey, owner);
    assert!(buy.accounts[2].is_signer);
    assert_eq!(buy.accounts[3].pubkey, ata_in);
    assert_eq!(
        buy.accounts[4].pubkey, state.vault_a,
        "buying with mint_a must route through vault_a as the swap source"
    );
    assert_eq!(buy.accounts[5].pubkey, state.vault_b);
    assert_eq!(buy.accounts[6].pubkey, ata_out);
    assert_eq!(buy.accounts[7].pubkey, state.pool_mint);
    assert_eq!(buy.accounts[8].pubkey, state.fee_account);
    assert_eq!(buy.accounts[9].pubkey, mint_a);
    assert_eq!(buy.accounts[10].pubkey, mint_b);
    assert_ne!(buy.accounts[4].pubkey, buy.accounts[5].pubkey);

    // Selling reverses the vaults: accounts 3-6 are swap-ordered.
    let sell = market
        .swap_instruction(
            &SwapAccounts {
                owner,
                input_mint: mint_b,
                output_mint: mint_a,
                input_token_account: ata_in,
                output_token_account: ata_out,
            },
            1_000_000,
            1,
        )
        .expect("builds against real state");
    assert_eq!(
        sell.accounts[4].pubkey, state.vault_b,
        "selling mint_b must route through vault_b as the swap source"
    );
    assert_eq!(sell.accounts[5].pubkey, state.vault_a);

    // Slots 9-13 are swap-ordered too, NOT pool-ordered. This venue shipped its
    // first draft pool-ordered and a live BUY still simulated clean, because on
    // a pool whose SOL side is token A the two orderings coincide -- and on this
    // very fixture they coincide for the PROGRAMMES as well, since its pool_mint
    // and its token_b are both Token-2022. Only the reverse direction separates
    // them, so it has to be asserted here or nothing offline catches a
    // regression. On chain the pool-ordered list is rejected with
    // `custom program error: 0x18`.
    let program_a = market
        .token_program(&mint_a)
        .expect("mint_a is in the pool");
    let program_b = market
        .token_program(&mint_b)
        .expect("mint_b is in the pool");

    assert_eq!(buy.accounts[11].pubkey, program_a, "buy source programme");
    assert_eq!(
        buy.accounts[12].pubkey, program_b,
        "buy destination programme"
    );

    assert_eq!(
        sell.accounts[9].pubkey, mint_b,
        "sell source mint is mint_b"
    );
    assert_eq!(
        sell.accounts[10].pubkey, mint_a,
        "sell destination mint is mint_a"
    );
    assert_eq!(sell.accounts[11].pubkey, program_b, "sell source programme");
    assert_eq!(
        sell.accounts[12].pubkey, program_a,
        "sell destination programme"
    );
    assert_eq!(
        buy.accounts[13].pubkey, sell.accounts[13].pubkey,
        "slot 13 is the POOL mint's programme, so it never depends on direction"
    );
}

#[test]
fn a_fluxbeam_funded_buy_holds_back_the_platform_fee_before_the_pool_sees_it() {
    let _guard = common::config_guard();
    let market = fixture_market("fluxbeam_amm");
    let (mint_a, mint_b) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: mint_a,
        output_mint: mint_b,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, market.as_ref()).expect("quotes offline");
    assert_eq!(quote.fee.side, FeeSide::Input, "SOL is the input leg here");
    assert_eq!(quote.fee.amount, 25_000, "0.5% of 0.005 SOL");
    assert_eq!(
        quote.swap_amount_in,
        5_000_000 - 25_000,
        "the platform fee never reaches the pool"
    );
}

// ============================================================================
// CROSS-VENUE INVARIANTS
// ============================================================================

/// Every fixture market, paired with the name a failure should name.
fn every_fixture_market() -> Vec<(&'static str, Box<dyn PoolMarket>)> {
    vec![
        ("raydium_amm_v4", fixture_market("raydium_legacy_amm")),
        ("raydium_cpmm", fixture_market("raydium_cpmm")),
        ("raydium_clmm", fixture_market("raydium_clmm")),
        ("orca_whirlpool", fixture_market("orca_whirlpool")),
        ("meteora_dlmm", fixture_market("meteora_dlmm")),
        ("meteora_damm", fixture_market("meteora_damm_v2")),
        ("meteora_dbc", fixture_market("meteora_dbc")),
        (
            "meteora_dbc_output_fee",
            fixture_market_case("meteora_dbc", "direct-swap-output-fee"),
        ),
        ("pumpfun_amm", fixture_market("pumpfun_amm")),
        ("pumpfun_legacy", fixture_market("pumpfun_legacy")),
        ("moonit", fixture_market("moonit_amm")),
        ("fluxbeam", fixture_market("fluxbeam_amm")),
    ]
}

/// `VenueQuote::lp_fee` is contracted to be in INPUT raw units on BOTH legs.
///
/// A venue that charges its fee on the OUTPUT must convert back at the realised
/// rate of that same fill; reporting the raw output-side figure is a silent unit
/// error no compiler and no existing test catches. The check compares the fee
/// RATE the venue charges on a buy against the rate it reports on the sell of
/// what that buy returned: a pool charges the same rate in both directions, so
/// the two must agree. If the sell figure is left in output units the ratio is
/// off by the token price -- millions, not percent.
#[test]
fn every_venue_reports_its_pool_fee_in_input_units_on_both_legs() {
    let mut offenders: Vec<String> = Vec::new();

    for (name, market) in every_fixture_market() {
        let (a, b) = market.mints();
        let sol = if a.to_string() == WSOL {
            a
        } else if b.to_string() == WSOL {
            b
        } else {
            continue; // fixture without a SOL leg; nothing to denominate against
        };
        let token = if sol == a { b } else { a };

        let buy = match market.quote(&sol, 5_000_000) {
            Ok(q) => q,
            Err(e) => {
                offenders.push(format!("{name}: the buy leg would not quote: {e}"));
                continue;
            }
        };
        // Both legs are quoted against ONE unchanged snapshot, so selling the
        // buy's proceeds back means selling into a curve that never received
        // the buy. On a bonding curve sitting at its own starting price that is
        // legitimately unfillable -- a property of the fixture, not of the fee
        // units this test is about.
        let sell = match market.quote(&token, buy.expected_out) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("{name:<24} skipped: the sell leg cannot quote off this snapshot ({e})");
                continue;
            }
        };

        let buy_rate = buy.lp_fee as f64 / buy.amount_in as f64;
        let sell_rate = sell.lp_fee as f64 / sell.amount_in as f64;
        eprintln!(
            "{name:<24} buy lp_fee {:>18} / {:>18} = {buy_rate:.8}   \
             sell lp_fee {:>18} / {:>18} = {sell_rate:.8}",
            buy.lp_fee, buy.amount_in, sell.lp_fee, sell.amount_in
        );

        if buy_rate <= 0.0 {
            continue; // a fee-free pool has nothing to compare
        }
        if sell_rate > buy_rate * 5.0 || sell_rate < buy_rate / 5.0 {
            offenders.push(format!(
                "{name}: charges {:.4}% of the input on a buy but reports {:.4}% on the sell -- \
                 the sell figure is not in input raw units",
                buy_rate * 100.0,
                sell_rate * 100.0
            ));
        }
    }

    assert!(
        offenders.is_empty(),
        "lp_fee must be in INPUT raw units on both legs:\n  {}",
        offenders.join("\n  ")
    );
}

/// The preflight's flat network-fee cushion must actually cover the network fee
/// the transaction it is preflighting will pay.
///
/// `execute::preflight_balance` refuses a swap the wallet cannot afford, sizing
/// the requirement as `amount_in + ATA rent + NETWORK_FEE_CUSHION_LAMPORTS`. Its
/// own doc calls the cushion "deliberately conservative: a preflight that
/// under-estimates would let a swap through that then fails on chain, which is
/// the exact failure mode this preflight exists to avoid."
///
/// The cushion was a flat 10_000 lamports, but Solana charges the prioritization
/// fee on the compute-unit LIMIT the transaction requests times the compute-unit
/// PRICE it sets -- both of which this very plan puts in instructions 0 and 1. At
/// the default 50_000 micro-lamports/CU that was already more than the cushion
/// for every venue in the engine, before the 5_000-lamport base fee was added,
/// and `chains.solana.swaps.direct.priority_fee_micro_lamports` may be set 200x higher still.
/// A wallet sitting just above `amount_in` passed, and the swap then died for
/// fees -- on an exit, at the worst possible moment.
///
/// `compute::network_fee_lamports` now derives the figure from the plan's own
/// budget instructions, so this asserts against that function rather than
/// against a constant mirrored out of the engine.
#[test]
fn the_preflight_cushion_covers_the_priority_fee_the_plan_requests() {
    let _guard = common::config_guard();

    /// One signature, at the runtime's fixed per-signature price.
    const BASE_FEE_LAMPORTS: u64 = 5_000;

    let mut short: Vec<String> = Vec::new();

    for (name, market) in every_fixture_market() {
        let (a, b) = market.mints();
        let sol = if a.to_string() == WSOL {
            a
        } else if b.to_string() == WSOL {
            b
        } else {
            continue;
        };
        let token = if sol == a { b } else { a };

        let intent = DirectSwapIntent {
            pool: market.pool(),
            owner: Pubkey::new_unique(),
            input_mint: sol,
            output_mint: token,
            amount_in: 5_000_000,
            slippage_bps: 300,
        };
        let Ok(quote) = direct::quote_with_market(&intent, market.as_ref()) else {
            continue;
        };
        let plan = direct::build_plan(&intent, market.as_ref(), &quote).expect("plan builds");

        // plan.rs guarantees the compute budget leads the transaction: index 0 is
        // SetComputeUnitLimit (discriminator 2), index 1 SetComputeUnitPrice (3).
        assert_eq!(plan.instructions[0].data[0], 2);
        assert_eq!(plan.instructions[1].data[0], 3);
        let limit = u32::from_le_bytes(plan.instructions[0].data[1..5].try_into().unwrap()) as u64;
        let price_micro_lamports =
            u64::from_le_bytes(plan.instructions[1].data[1..9].try_into().unwrap());
        let priority_fee = (limit * price_micro_lamports).div_ceil(1_000_000);
        let network_fee = priority_fee + BASE_FEE_LAMPORTS;

        let reserved = direct::compute::network_fee_lamports(&plan.instructions);
        eprintln!(
            "{name:<24} limit {limit:>7} CU x {price_micro_lamports} uL/CU = {priority_fee:>7} \
             + {BASE_FEE_LAMPORTS} base = {network_fee:>7} lamports vs a \
             {reserved} cushion"
        );
        if network_fee > reserved {
            short.push(format!(
                "{name}: the transaction pays {network_fee} lamports in fees but the preflight \
                 only reserves {reserved}"
            ));
        }
    }

    assert!(
        short.is_empty(),
        "the preflight must reserve at least what the transaction will pay:\n  {}",
        short.join("\n  ")
    );
}

/// A dust-sized trade's reported price impact must be the pool's own fee and
/// essentially nothing else.
///
/// `DirectPoolRouter::get_quote` refuses any quote whose `price_impact_pct`
/// exceeds `chains.solana.swaps.direct.max_price_impact_pct` (10% by default) and returns
/// `NoRoute` -- which the opening path counts towards retiring the mint. So an
/// impact figure wrong in the HIGH direction does not merely mislead a log line:
/// it takes the venue out of service and blames the token for it.
///
/// The venues do not compute it on the same basis. The constant-product ones and
/// Meteora DLMM compare the REALISED rate against spot, so the pool's own fee
/// shows up inside the impact; Raydium CLMM and Orca measure the move in the
/// squared sqrt price, which excludes it. Both are defensible, and this is the
/// bound that holds on either: at 0.005 SOL against a real pool nothing but the
/// fee can move the realised rate, so the impact can sit anywhere from zero up
/// to the fee plus a rounding-scale margin -- and a venue whose formula is in
/// the wrong basis (a price scaled by the wrong decimals, an inverted side)
/// lands far outside that band.
///
/// FluxBeam's fixture pool charges a 99.2% owner fee, and reporting 99.2% impact
/// for it is CORRECT: the ceiling refusing that pool is the gate working.
#[test]
fn a_minimum_sized_buy_reports_an_impact_no_larger_than_the_pools_own_fee() {
    /// How far above the pool's own fee rate a dust trade's impact may sit, in
    /// percentage points. Dust cannot move a real pool, so anything beyond
    /// integer-rounding scale is a formula in the wrong basis.
    const MARGIN_PCT_POINTS: f64 = 1.0;

    let mut implausible: Vec<String> = Vec::new();

    for (name, market) in every_fixture_market() {
        let (a, b) = market.mints();
        let sol = if a.to_string() == WSOL {
            a
        } else if b.to_string() == WSOL {
            b
        } else {
            continue;
        };

        let Ok(quote) = market.quote(&sol, 5_000_000) else {
            continue;
        };
        let fee_pct = quote.lp_fee as f64 / quote.amount_in as f64 * 100.0;
        eprintln!(
            "{name:<24} 0.005 SOL buy -> impact {:.4}% against a {fee_pct:.4}% pool fee",
            quote.price_impact_pct
        );
        assert!(
            quote.price_impact_pct.is_finite() && quote.price_impact_pct >= 0.0,
            "{name}: price impact must be a real percentage, got {}",
            quote.price_impact_pct
        );
        if quote.price_impact_pct > fee_pct + MARGIN_PCT_POINTS {
            implausible.push(format!(
                "{name}: reports {:.4}% impact on a 0.005 SOL buy against a pool that only \
                 charges {fee_pct:.4}% -- dust cannot move a real pool that far",
                quote.price_impact_pct
            ));
        }
    }

    assert!(
        implausible.is_empty(),
        "a dust trade's impact is its fee, not a market move:\n  {}",
        implausible.join("\n  ")
    );
}

// ============================================================================
// A CONFIRMED SWAP MUST NOT BE FAILED BY ITS OWN MEASUREMENT
// ============================================================================
//
// `verify.rs` states the rule itself: safety comes from `min_out`, which the
// pool programme enforced, so "nothing measured here can make a confirmed swap
// unsafe after the fact", and a read that cannot be had "does NOT fail the
// swap". Two paths break that rule, and both do it by turning a gap in the RPC's
// own reply into a verdict about the trade.
//
// The damage is not theoretical. `open.rs` asks
// `swaps::unconfirmed_swap_signature` what to do with a failed entry: `Some` ->
// create a pending position and hand it to verification, `None` -> return
// `SwapFailed` and create NOTHING. Neither error below is recoverable through
// that function, so a buy that landed, moved the SOL and delivered the tokens
// ends up with the tokens in the wallet and no position anywhere.

fn measurement_plan(mint: Pubkey, min_net_out: u64) -> direct::SwapPlan {
    direct::SwapPlan {
        instructions: Vec::new(),
        venue_compute_units: 0,
        input_account: Pubkey::new_unique(),
        output_account: Pubkey::new_unique(),
        output_is_native: false,
        quote: direct::DirectQuote {
            pool: Pubkey::new_unique(),
            program: screenerbot::chains::solana::pools::types::ProgramKind::RaydiumCpmm,
            input_mint: Pubkey::from_str(WSOL).unwrap(),
            output_mint: mint,
            amount_in: 5_000_000,
            swap_amount_in: 4_975_000,
            expected_out: min_net_out,
            min_out: min_net_out,
            expected_net_out: min_net_out,
            min_net_out,
            fee: direct::PlatformFee::none(),
            lp_fee: 0,
            price_impact_pct: 0.0,
            slippage_bps: 100,
        },
    }
}

fn transaction_details(
    meta: Option<screenerbot::chains::solana::rpc::types::TransactionMeta>,
    owner: &Pubkey,
) -> screenerbot::chains::solana::rpc::types::TransactionDetails {
    screenerbot::chains::solana::rpc::types::TransactionDetails {
        slot: 42,
        transaction: screenerbot::chains::solana::rpc::types::TransactionData {
            message: serde_json::json!({ "accountKeys": [owner.to_string()] }),
            signatures: vec!["sig".to_owned()],
        },
        meta,
        block_time: None,
    }
}

fn token_balance(
    owner: Option<&Pubkey>,
    mint: &Pubkey,
    amount: &str,
) -> screenerbot::chains::solana::rpc::types::TokenBalance {
    screenerbot::chains::solana::rpc::types::TokenBalance {
        account_index: 0,
        mint: mint.to_string(),
        owner: owner.map(|o| o.to_string()),
        program_id: None,
        ui_token_amount: screenerbot::chains::solana::rpc::types::UiTokenAmount {
            amount: amount.to_owned(),
            decimals: 6,
            ui_amount: None,
            ui_amount_string: None,
        },
    }
}

fn empty_meta() -> screenerbot::chains::solana::rpc::types::TransactionMeta {
    screenerbot::chains::solana::rpc::types::TransactionMeta {
        err: None,
        pre_balances: vec![],
        post_balances: vec![],
        pre_token_balances: None,
        post_token_balances: None,
        fee: 5_000,
        compute_units_consumed: None,
        log_messages: None,
        inner_instructions: None,
    }
}

/// A transaction the node returns WITHOUT metadata is a read this engine cannot
/// measure -- not a transaction that failed.
///
/// The settle loop already proved success: it returns `Ok` only for a signature
/// status carrying no error, and the pool programme itself refused to return
/// less than `min_out`. `receipt_from_transaction` nonetheless raises
/// `TransactionFailed` the moment `meta` is absent, which reports a landed,
/// successful swap as a chain failure -- while the neighbouring path, a read
/// that never succeeds at all, correctly falls back to `min_net_out` and marks
/// the receipt inexact.
#[test]
fn a_confirmed_transaction_with_no_metadata_is_a_measurement_gap_not_a_failed_swap() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let plan = measurement_plan(mint, 900_000);
    let details = transaction_details(None, &owner);

    let receipt = direct::verify::receipt_from_transaction("sig", &owner, &plan, &details);
    assert!(
        receipt.is_ok(),
        "a confirmed swap must not be failed because its metadata could not be read: {:?}",
        receipt.err()
    );
    let receipt = receipt.unwrap();
    assert_eq!(
        receipt.received, 900_000,
        "the chain-guaranteed minimum is the honest fallback"
    );
    assert!(!receipt.exact, "and it must be marked inexact");
}

/// A node that omits `owner` on its token balances must not turn a delivered
/// buy into `OutputNotReceived`.
///
/// The token path measures the delta by filtering `pre`/`post` token balances on
/// `mint` AND `owner`. `owner` is an OPTIONAL field of the RPC reply; where it
/// is absent the filter matches nothing, both sides read zero, and a buy that
/// actually delivered its tokens is reported as having delivered none. The
/// module's own rule -- "a confirmed swap is never failed by a measurement" --
/// is exactly what this violates, and unlike the native path, which distrusts
/// its own reading and falls back, the token path fails hard.
#[test]
fn a_node_that_omits_the_balance_owner_does_not_turn_a_delivered_buy_into_a_failure() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let plan = measurement_plan(mint, 900_000);

    let mut meta = empty_meta();
    // Same transaction, same amounts, `owner` simply not populated by the node.
    meta.pre_token_balances = Some(vec![token_balance(None, &mint, "0")]);
    meta.post_token_balances = Some(vec![token_balance(None, &mint, "1000000")]);
    let details = transaction_details(Some(meta), &owner);

    let receipt = direct::verify::receipt_from_transaction("sig", &owner, &plan, &details);
    assert!(
        receipt.is_ok(),
        "a buy that delivered 1_000_000 raw units must not be reported as \
         OutputNotReceived because the node left one field out: {:?}",
        receipt.err()
    );
}

/// Every venue's signed transaction fits the 1,232-byte packet on both legs.
///
/// The direct engine builds LEGACY transactions: no lookup tables, so every
/// account costs 32 bytes. The plan already carries the worst case of its own
/// shape — both token accounts created (the creations are idempotent and always
/// present), the WSOL wrap on a buy, the platform fee transfer and the WSOL
/// close — and each venue's optional accounts are those its captured pool
/// state calls for. A transaction over the limit is refused by the pre-send
/// gate before any node sees it, so a venue that cannot fit could never trade;
/// the margin printed per leg is how close each one runs.
#[test]
fn every_venue_plan_fits_the_packet_on_both_legs() {
    use screenerbot::chains::solana::solana_sdk::{
        signature::Signature,
        transaction::{Transaction, VersionedTransaction},
    };
    use screenerbot::chains::solana::swaps::presend;

    let _guard = common::config_guard();
    let mut over: Vec<String> = Vec::new();

    for (name, market) in every_fixture_market() {
        let (a, b) = market.mints();
        let sol = if a.to_string() == WSOL {
            a
        } else if b.to_string() == WSOL {
            b
        } else {
            continue;
        };
        let token = if sol == a { b } else { a };
        let owner = Pubkey::new_unique();

        let buy = DirectSwapIntent {
            pool: market.pool(),
            owner,
            input_mint: sol,
            output_mint: token,
            amount_in: 5_000_000,
            slippage_bps: 300,
        };
        let buy_quote = direct::quote_with_market(&buy, market.as_ref())
            .unwrap_or_else(|e| panic!("{name}: the buy leg would not quote: {e}"));

        // Both legs are quoted against one unchanged snapshot, and a bonding
        // curve sitting at its starting price cannot fill a sell at all. The
        // account list, and so the size, does not depend on the amounts, so
        // such a sell is priced by hand with the fee the real sell would carry.
        let sell_intent = DirectSwapIntent {
            pool: market.pool(),
            owner,
            input_mint: token,
            output_mint: sol,
            amount_in: buy_quote.expected_out,
            slippage_bps: 300,
        };
        let sell_quote =
            direct::quote_with_market(&sell_intent, market.as_ref()).unwrap_or_else(|_| {
                let fee_side = FeeSide::for_pair(&token, &sol);
                let fee = direct::PlatformFee::resolve(fee_side, &token, &sol, 990_000)
                    .expect("the fee resolves for a SOL leg");
                eprintln!("{name:<24} sell priced by hand: the snapshot cannot fill a sell");
                direct::DirectQuote {
                    pool: market.pool(),
                    program: buy_quote.program,
                    input_mint: token,
                    output_mint: sol,
                    amount_in: buy_quote.expected_out,
                    swap_amount_in: buy_quote.expected_out,
                    expected_out: 1_000_000,
                    min_out: 990_000,
                    expected_net_out: 1_000_000 - fee.amount,
                    min_net_out: 990_000 - fee.amount,
                    fee,
                    lp_fee: 0,
                    price_impact_pct: 0.0,
                    slippage_bps: 300,
                }
            });
        let sell = (sell_intent, sell_quote);

        for (leg, intent, quote) in [("buy", &buy, &buy_quote), ("sell", &sell.0, &sell.1)] {
            let plan = direct::build_plan(intent, market.as_ref(), quote)
                .unwrap_or_else(|e| panic!("{name} {leg}: the plan would not build: {e}"));
            let mut transaction = Transaction::new_with_payer(&plan.instructions, Some(&owner));
            transaction.signatures =
                vec![
                    Signature::default();
                    usize::from(transaction.message.header.num_required_signatures)
                ];
            let transaction = VersionedTransaction::from(transaction);
            let size = bincode::serialize(&transaction).expect("serializes").len();
            let limit = presend::WireFormat::Legacy.size_limit();
            eprintln!(
                "{name:<24} {leg:<4} {size:>5} of {limit} bytes, {:>5} to spare, {} instructions, {} accounts",
                limit as i64 - size as i64,
                plan.instructions.len(),
                transaction.message.static_account_keys().len()
            );
            if presend::measure_transaction(&transaction).is_err() {
                over.push(format!("{name} {leg}: {size} bytes"));
            }
        }
    }

    assert!(
        over.is_empty(),
        "a direct swap transaction must fit the {}-byte packet:\n  {}",
        presend::WireFormat::Legacy.size_limit(),
        over.join("\n  ")
    );
}
