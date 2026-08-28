//! Direct pool swaps, decoded and quoted from REAL captured mainnet accounts —
//! with no network, no wallet and no keys.
//!
//! # Why fixtures rather than synthetic state
//!
//! Every unit test in the venue modules builds its own pool struct, which proves
//! the arithmetic but proves nothing about the LAYOUT: an offset that drifts by
//! eight bytes still passes a test whose fixture was constructed field by field.
//! The files under `tests/fixtures/direct_swaps/` are the exact bytes mainnet
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

use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::swaps::direct::venues::clmm_ticks::{
    decode_tick_array, TickArrayBitmap,
};
use screenerbot::chains::solana::swaps::direct::venues::layout::{
    mint_decimals, token_account_amount, u64_at, u8_at,
};
use screenerbot::chains::solana::swaps::direct::venues::meteora_damm::{DammMarket, DammPoolState};
use screenerbot::chains::solana::swaps::direct::venues::orca_whirlpool::{
    candidate_tick_array_starts, decode_tick_array as orca_decode_tick_array, oracle_address,
    tick_array_address as orca_tick_array_address, WhirlpoolMarket, WhirlpoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::pumpfun_amm::{
    FeeTierTable, GlobalConfig, PumpAmmMarket, PumpAmmPoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::raydium_amm_v4::{
    AmmV4Market, AmmV4PoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::raydium_clmm::{
    ClmmFeeConfig, ClmmMarket, ClmmPoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::raydium_cpmm::{
    CpmmFeeConfig, CpmmMarket, CpmmPoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::token2022::transfer_fee_schedule;
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
    fn load(name: &str) -> Self {
        use base64::Engine;
        let path = format!(
            "{}/tests/fixtures/direct_swaps/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("fixture {path} could not be read: {e}"));
        let json: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("fixture {path} is not JSON: {e}"));

        let pool = Pubkey::from_str(json["pool"].as_str().expect("fixture names its pool"))
            .expect("fixture pool is a pubkey");
        let mut accounts = HashMap::new();
        for (address, value) in json["accounts"]
            .as_object()
            .expect("fixture carries an accounts map")
        {
            let data = base64::engine::general_purpose::STANDARD
                .decode(value["data"].as_str().expect("account carries base64 data"))
                .expect("account data is valid base64");
            accounts.insert(
                address.clone(),
                screenerbot::chains::solana::solana_sdk::account::Account {
                    lamports: 0,
                    data,
                    owner: Pubkey::from_str(value["owner"].as_str().expect("account has an owner"))
                        .expect("owner is a pubkey"),
                    executable: false,
                    rent_epoch: 0,
                },
            );
        }
        Self { pool, accounts }
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

fn cpmm_market() -> CpmmMarket {
    let fixture = Fixture::load("raydium_cpmm_pool");
    let state = CpmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured CPMM pool must decode");
    let config = CpmmFeeConfig::decode(fixture.data(&state.amm_config))
        .expect("the captured AmmConfig must decode");
    CpmmMarket::new(
        state,
        config,
        fixture.balance(&state.vault_0),
        fixture.balance(&state.vault_1),
        transfer_fee_schedule(fixture.account(&state.mint_0)),
        transfer_fee_schedule(fixture.account(&state.mint_1)),
    )
}

fn amm_v4_market() -> AmmV4Market {
    let fixture = Fixture::load("raydium_amm_v4_pool");
    let state = AmmV4PoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured AMM v4 pool must decode");
    AmmV4Market::new(
        state,
        fixture.balance(&state.coin_vault),
        fixture.balance(&state.pc_vault),
    )
}

fn clmm_market() -> ClmmMarket {
    use screenerbot::chains::solana::swaps::direct::venues::clmm_ticks::{
        bitmap_extension_address, tick_array_address,
    };

    let fixture = Fixture::load("raydium_clmm_pool");
    let program = fixture.account(&fixture.pool).owner;
    let state = ClmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured CLMM pool must decode");
    let config = ClmmFeeConfig::decode(fixture.data(&state.amm_config))
        .expect("the captured AmmConfig must decode");

    // Same derivation `load()` uses: the pool's own bitmap picks which tick
    // arrays to fetch, both directions, before the extension is known.
    let initial_bitmap = TickArrayBitmap::from_pool_state(fixture.data(&fixture.pool))
        .expect("the captured bitmap decodes");
    let tick_array_addresses: Vec<Pubkey> = [true, false]
        .into_iter()
        .flat_map(|zero_for_one| {
            initial_bitmap.arrays_for_swap(state.tick_current, state.tick_spacing, zero_for_one)
        })
        .map(|start| tick_array_address(&program, &fixture.pool, start))
        .collect();

    let bitmap_extension_address = bitmap_extension_address(&program, &fixture.pool);
    let mut bitmap = initial_bitmap;
    if let Some(extension) = fixture.accounts.get(&bitmap_extension_address.to_string()) {
        bitmap = bitmap.with_extension(&fixture.pool, &extension.data);
    }

    let mut ticks = Vec::new();
    for address in tick_array_addresses {
        if let Some(account) = fixture.accounts.get(&address.to_string()) {
            let decoded = decode_tick_array(&fixture.pool, &account.data)
                .expect("a captured tick array must match the expected layout");
            ticks.extend(decoded);
        }
    }

    ClmmMarket::new(
        state,
        config,
        bitmap,
        bitmap_extension_address,
        fixture.account(&state.mint_0).owner,
        fixture.account(&state.mint_1).owner,
        fixture.balance(&state.vault_0),
        fixture.balance(&state.vault_1),
        transfer_fee_schedule(fixture.account(&state.mint_0)),
        transfer_fee_schedule(fixture.account(&state.mint_1)),
        ticks,
    )
}

fn orca_market() -> WhirlpoolMarket {
    let fixture = Fixture::load("orca_whirlpool_pool");
    let program = fixture.account(&fixture.pool).owner;
    let state = WhirlpoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured Whirlpool state must decode");

    // Same derivation `load()` uses: candidates in both directions, arithmetic
    // only, since a Whirlpool carries no bitmap -- existence is learned from
    // which accounts the fixture actually captured.
    let mut tick_array_starts: Vec<i32> = Vec::new();
    for zero_for_one in [true, false] {
        for start in
            candidate_tick_array_starts(state.tick_current, state.tick_spacing, zero_for_one)
        {
            if !tick_array_starts.contains(&start) {
                tick_array_starts.push(start);
            }
        }
    }

    let mut ticks = Vec::new();
    let mut available_starts: Vec<i32> = Vec::new();
    for start in &tick_array_starts {
        let address = orca_tick_array_address(&program, &fixture.pool, *start);
        if let Some(account) = fixture.accounts.get(&address.to_string()) {
            available_starts.push(*start);
            ticks.extend(
                orca_decode_tick_array(&account.data, *start, state.tick_spacing)
                    .expect("a captured tick array must match the expected layout"),
            );
        }
    }

    let oracle = oracle_address(&program, &fixture.pool);
    // The captured fixture has no oracle account at all -- this pool is
    // classic (non-adaptive-fee), matching the module's own on-chain finding.
    assert!(
        !fixture.accounts.contains_key(&oracle.to_string()),
        "the fixture pool is expected to be a classic Whirlpool with no oracle account; \
         re-check the adaptive-fee refusal test if this fixture is ever refreshed"
    );

    WhirlpoolMarket::new(
        state,
        oracle,
        fixture.account(&state.mint_a).owner,
        fixture.account(&state.mint_b).owner,
        (
            mint_decimals(fixture.data(&state.mint_a)).expect("mint_a decimals"),
            mint_decimals(fixture.data(&state.mint_b)).expect("mint_b decimals"),
        ),
        fixture.balance(&state.vault_a),
        fixture.balance(&state.vault_b),
        transfer_fee_schedule(fixture.account(&state.mint_a)),
        transfer_fee_schedule(fixture.account(&state.mint_b)),
        ticks,
        available_starts,
    )
}

fn damm_market() -> DammMarket {
    let fixture = Fixture::load("meteora_damm_v2_pool");
    let state = DammPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured DAMM v2 pool must decode");
    let mint_a = fixture.account(&state.mint_a);
    let mint_b = fixture.account(&state.mint_b);
    let decimals_a = u8_at(&mint_a.data, 44).expect("mint_a carries a decimals byte");
    let decimals_b = u8_at(&mint_b.data, 44).expect("mint_b carries a decimals byte");

    // The fixture's fee schedule is keyed on the wall clock (activation_type
    // 1, a unix timestamp) rather than the slot -- `load()` reads the same
    // clock live, so re-reading it here rather than freezing a captured value
    // is what keeps this fixture from rotting the day the cliff period ends.
    let current_point = chrono::Utc::now().timestamp().max(0) as u64;

    DammMarket::new(
        state,
        mint_a.owner,
        mint_b.owner,
        decimals_a,
        decimals_b,
        fixture.balance(&state.vault_a),
        fixture.balance(&state.vault_b),
        transfer_fee_schedule(mint_a),
        transfer_fee_schedule(mint_b),
        current_point,
    )
}

/// pump-swap's own programme id, for deriving the `GlobalConfig` and
/// `FeeConfig` PDAs the fixture captured. Kept local to this test rather than
/// imported: the venue does not expose these addresses as `pub`, since
/// production always derives them itself.
fn pump_amm_program_id_for_test() -> Pubkey {
    Pubkey::from_str("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA").unwrap()
}

fn pump_amm_market() -> PumpAmmMarket {
    let fixture = Fixture::load("pumpfun_amm_pool");
    let state = PumpAmmPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured pump-swap pool must decode");

    let program = pump_amm_program_id_for_test();
    let fee_program = Pubkey::from_str("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ").unwrap();
    let (global_config, _) = Pubkey::find_program_address(&[b"global_config"], &program);
    let (fee_config, _) =
        Pubkey::find_program_address(&[b"fee_config", program.as_ref()], &fee_program);

    let global = GlobalConfig::decode(fixture.data(&global_config))
        .expect("the captured GlobalConfig must decode");
    let protocol_fee_recipient = global
        .first_fee_recipient()
        .expect("the captured global config lists a protocol fee recipient");
    let buyback_fee_recipient = global
        .first_buyback_recipient()
        .expect("the captured global config lists a buyback fee recipient");
    let tiers = FeeTierTable::decode(fixture.data(&fee_config));

    let base_mint = fixture.account(&state.base_mint);
    let quote_mint = fixture.account(&state.quote_mint);
    let base_supply = u64_at(&base_mint.data, 36).expect("base mint carries a supply");
    let base_decimals = u8_at(&base_mint.data, 44).expect("base mint carries a decimals byte");
    let quote_decimals = u8_at(&quote_mint.data, 44).expect("quote mint carries a decimals byte");

    PumpAmmMarket::new(
        state,
        protocol_fee_recipient,
        buyback_fee_recipient,
        tiers,
        global.flat_fees(),
        base_mint.owner,
        quote_mint.owner,
        base_decimals,
        quote_decimals,
        base_supply,
        fixture.balance(&state.base_token_account),
        fixture.balance(&state.quote_token_account),
        transfer_fee_schedule(base_mint),
        transfer_fee_schedule(quote_mint),
    )
}

// ============================================================================
// LAYOUT — the fixtures exist to catch an offset that drifted
// ============================================================================

#[test]
fn the_cpmm_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("raydium_cpmm_pool");
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
    let fixture = Fixture::load("raydium_cpmm_pool");
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
    let fixture = Fixture::load("raydium_amm_v4_pool");
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
    let fixture = Fixture::load("raydium_clmm_pool");
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

    let fixture = Fixture::load("raydium_clmm_pool");
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
    let market = clmm_market();
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
    let fixture = Fixture::load("orca_whirlpool_pool");
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
    let fixture = Fixture::load("orca_whirlpool_pool");
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
    let market = orca_market();
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
    let fixture = Fixture::load("meteora_damm_v2_pool");
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
    let market = damm_market();
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
    let fixture = Fixture::load("pumpfun_amm_pool");
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

    let market = pump_amm_market();
    assert!(
        market.market_cap().is_some(),
        "a real pool with real reserves must have a computable market cap"
    );
}

#[test]
fn a_pump_amm_quote_off_real_state_charges_a_fee_and_is_monotonic() {
    let market = pump_amm_market();
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
    let market = cpmm_market();
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
    let market = amm_v4_market();
    let (coin, _) = market.mints();
    let (coin_reserve, _) = market.reserves();

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
    assert!(cpmm_market().quote(&stranger, 5_000_000).is_err());
    assert!(amm_v4_market().quote(&stranger, 5_000_000).is_err());
}

// ============================================================================
// THE PLATFORM FEE — the assertion that catches revenue silently going missing
// ============================================================================

#[test]
fn a_sol_funded_buy_holds_back_the_fee_before_the_pool_sees_it() {
    let _guard = common::config_guard();
    let market = cpmm_market();
    let (mint_0, mint_1) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: mint_0,
        output_mint: mint_1,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, &market).expect("quotes offline");
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
    let market = cpmm_market();
    let (mint_0, mint_1) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: mint_1,
        output_mint: mint_0,
        amount_in: 1_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, &market).expect("quotes offline");
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
    let market = amm_v4_market();
    let (coin, pc) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: coin,
        output_mint: pc,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, &market).expect("quotes");
    let plan = direct::build_plan(&intent, &market, &quote).expect("plans");

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
    let market = amm_v4_market();
    let (coin, pc) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: coin,
        output_mint: pc,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };
    let quote = direct::quote_with_market(&intent, &market).expect("quotes");
    let plan = direct::build_plan(&intent, &market, &quote).expect("plans");

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
    let market = cpmm_market();
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
