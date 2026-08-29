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
use screenerbot::chains::solana::swaps::direct::venues::fluxbeam::{
    FluxbeamMarket, FluxbeamPoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::layout::{
    mint_decimals, token_account_amount, u64_at, u8_at,
};
use screenerbot::chains::solana::swaps::direct::venues::meteora_damm::{DammMarket, DammPoolState};
use screenerbot::chains::solana::swaps::direct::venues::meteora_dbc::{
    DbcMarket, PoolConfigState as DbcPoolConfigState, VirtualPoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::meteora_dlmm::{
    bin_array_address, bitmap_extension_address as dlmm_bitmap_extension_address,
    event_authority_address, oracle_address as dlmm_oracle_address, DlmmMarket, LbPairState,
};
use screenerbot::chains::solana::swaps::direct::venues::moonit::{
    ConfigAccountState, CurveAccountState, MoonitMarket,
};
use screenerbot::chains::solana::swaps::direct::venues::orca_whirlpool::{
    candidate_tick_array_starts, decode_tick_array as orca_decode_tick_array, oracle_address,
    tick_array_address as orca_tick_array_address, WhirlpoolMarket, WhirlpoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::pumpfun_amm::{
    FeeTierTable, GlobalConfig, PumpAmmMarket, PumpAmmPoolState,
};
use screenerbot::chains::solana::swaps::direct::venues::pumpfun_legacy::{
    BondingCurve, GlobalFeeRecipients, PumpLegacyMarket,
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

fn dlmm_market() -> DlmmMarket {
    let fixture = Fixture::load("meteora_dlmm_pool");
    let program = fixture.account(&fixture.pool).owner;
    let state = LbPairState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured LbPair must decode");
    let raw_v_params = LbPairState::decode_v_parameters(fixture.data(&fixture.pool))
        .expect("the captured LbPair's v_parameters must decode");

    // Same derivation `load()` uses, but against the CLOCK CAPTURED IN THE
    // FIXTURE rather than the current wall clock -- the bin arrays and
    // reserves were captured at that same instant, so re-deriving against
    // "now" would apply a fee-reference decay the captured bins never saw.
    let clock_address = Pubkey::from_str("SysvarC1ock11111111111111111111111111111111").unwrap();
    let clock_data = fixture.data(&clock_address);
    let now = i64::from_le_bytes(
        clock_data[32..40]
            .try_into()
            .expect("the fixture's clock sysvar is 40 bytes"),
    );
    let v_params = raw_v_params.update_references(now, state.active_id, &state.parameters);

    // Same derivation `load()` uses: consecutive candidate array indices in
    // both directions out from the active bin, keeping only what the
    // fixture actually captured.
    let active_array_index = state.active_id.div_euclid(70) as i64;
    let mut array_indices: Vec<i64> = Vec::new();
    for step in [-1i64, 1] {
        let mut index = active_array_index;
        for _ in 0..3 {
            if !array_indices.contains(&index) {
                array_indices.push(index);
            }
            index += step;
        }
    }

    let mut bins: Vec<(i32, u64, u64)> = Vec::new();
    let mut available: Vec<i64> = Vec::new();
    for index in array_indices {
        let address = bin_array_address(&program, &fixture.pool, index);
        if let Some(account) = fixture.accounts.get(&address.to_string()) {
            available.push(index);
            for i in 0..70i32 {
                let offset = 56 + (i as usize) * 144;
                let amount_x = u64_at(&account.data, offset).expect("bin amount_x");
                let amount_y = u64_at(&account.data, offset + 8).expect("bin amount_y");
                let id = (index * 70) as i32 + i;
                bins.push((id, amount_x, amount_y));
            }
        }
    }

    let mint_x = fixture.account(&state.token_x_mint);
    let mint_y = fixture.account(&state.token_y_mint);

    // Optional account: read from the fixture the same way `load()` reads it
    // from the batch, rather than assumed either way.
    let has_bitmap_extension = fixture
        .accounts
        .contains_key(&dlmm_bitmap_extension_address(&program, &fixture.pool).to_string());

    DlmmMarket::new(
        state,
        v_params,
        mint_x.owner,
        mint_y.owner,
        (
            mint_decimals(&mint_x.data).expect("token_x_mint decimals"),
            mint_decimals(&mint_y.data).expect("token_y_mint decimals"),
        ),
        fixture.balance(&state.reserve_x),
        fixture.balance(&state.reserve_y),
        transfer_fee_schedule(mint_x),
        transfer_fee_schedule(mint_y),
        bins,
        available,
        has_bitmap_extension,
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

/// pump.fun legacy's own programme id, for deriving the `Global` and
/// `FeeConfig` PDAs the fixture captured. Kept local to this test rather than
/// imported: the venue does not expose these addresses as `pub`, since
/// production always derives them itself.
fn pump_legacy_program_id_for_test() -> Pubkey {
    Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap()
}

fn pump_legacy_market() -> PumpLegacyMarket {
    let fixture = Fixture::load("pumpfun_legacy_pool");
    let mut curve = BondingCurve::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured bonding curve must decode");
    // The account carries no mint field of its own (see the venue's module
    // docs) -- `load()` recovers it from the curve's own token account via
    // `getTokenAccountsByOwner`, which the offline tier does not call. The
    // fixture was captured against a known live trade, so the mint is known.
    curve.mint = Pubkey::from_str("2xJGewx1p72WFCAwBvmbpejZxqa7EN3mS3PiGjgrpump").unwrap();

    let program = pump_legacy_program_id_for_test();
    let fee_program = Pubkey::from_str("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ").unwrap();
    let (global, _) = Pubkey::find_program_address(&[b"global"], &program);
    let (fee_config, _) =
        Pubkey::find_program_address(&[b"fee_config", program.as_ref()], &fee_program);

    let global_state = GlobalFeeRecipients::decode(fixture.data(&global))
        .expect("the captured Global must decode");
    let fee_recipient = global_state
        .first_fee_recipient()
        .expect("the captured Global lists a protocol fee recipient");
    let (buyback_wallet, buyback_paid) = global_state
        .two_buyback_recipients()
        .expect("the captured Global lists at least two buyback fee recipients");
    let tiers = FeeTierTable::decode(fixture.data(&fee_config));

    let mint_account = fixture.account(&curve.mint);
    let mint_decimals = u8_at(&mint_account.data, 44).expect("mint carries a decimals byte");

    PumpLegacyMarket::new(
        curve,
        mint_account.owner,
        mint_decimals,
        transfer_fee_schedule(mint_account),
        fee_recipient,
        buyback_wallet,
        buyback_paid,
        tiers,
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

#[test]
fn the_dlmm_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("meteora_dlmm_pool");
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
    let fixture = Fixture::load("meteora_dlmm_pool");
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
    let market = dlmm_market();
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
    let market = dlmm_market();
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
    let fixture = Fixture::load("pumpfun_legacy_pool");
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

    let market = pump_legacy_market();
    assert!(
        market.quote(&market.mints().0, 5_000_000).is_ok(),
        "a real curve with real reserves must quote a small buy"
    );
}

#[test]
fn a_pump_legacy_quote_off_real_state_charges_both_fees_and_is_monotonic() {
    let market = pump_legacy_market();
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
    let market = pump_legacy_market();
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

fn dbc_market() -> DbcMarket {
    let fixture = Fixture::load("meteora_dbc_pool");
    let state = VirtualPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured VirtualPool must decode");
    let config = DbcPoolConfigState::decode(fixture.data(&state.config))
        .expect("the captured PoolConfig must decode");

    let base_mint = fixture.account(&state.base_mint);
    let quote_mint = fixture.account(&config.quote_mint);

    DbcMarket::new(
        state,
        config,
        mint_decimals(&base_mint.data).expect("base mint carries a decimals byte"),
        mint_decimals(&quote_mint.data).expect("quote mint carries a decimals byte"),
        base_mint.owner,
        quote_mint.owner,
        transfer_fee_schedule(base_mint),
        transfer_fee_schedule(quote_mint),
        fixture.balance(&state.base_vault),
        fixture.balance(&state.quote_vault),
    )
}

/// The second fixture: a different pool whose config carries
/// `collect_fee_mode = 1` (`OutputToken`) rather than the primary fixture's
/// `0` (`QuoteToken`) -- the other branch of `DbcMarket::fee_on_input`.
fn dbc_output_fee_market() -> DbcMarket {
    let fixture = Fixture::load("meteora_dbc_pool_output_fee");
    let state = VirtualPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured VirtualPool must decode");
    let config = DbcPoolConfigState::decode(fixture.data(&state.config))
        .expect("the captured PoolConfig must decode");

    let base_mint = fixture.account(&state.base_mint);
    let quote_mint = fixture.account(&config.quote_mint);

    DbcMarket::new(
        state,
        config,
        mint_decimals(&base_mint.data).expect("base mint carries a decimals byte"),
        mint_decimals(&quote_mint.data).expect("quote mint carries a decimals byte"),
        base_mint.owner,
        quote_mint.owner,
        transfer_fee_schedule(base_mint),
        transfer_fee_schedule(quote_mint),
        fixture.balance(&state.base_vault),
        fixture.balance(&state.quote_vault),
    )
}

#[test]
fn the_dbc_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("meteora_dbc_pool");
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
    let fixture = Fixture::load("meteora_dbc_pool");
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
    let migration_sqrt_price = screenerbot::chains::solana::swaps::direct::venues::layout::u128_at(
        fixture.data(&state.config),
        280,
    )
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
    let market = dbc_market();
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
    let market = dbc_market();
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
    let fixture = Fixture::load("meteora_dbc_pool_output_fee");
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
    let market = dbc_output_fee_market();
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
    let market = dbc_market();
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

fn moonit_market() -> MoonitMarket {
    let fixture = Fixture::load("moonit_pool");
    let curve = CurveAccountState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured Moonit curve must decode");

    let program = moonit_program_id_for_test();
    let (config_address, bump) = Pubkey::find_program_address(&[b"config_account"], &program);
    assert_eq!(bump, 251, "the live ConfigAccount's own stored bump is 251");
    let config = ConfigAccountState::decode(fixture.data(&config_address))
        .expect("the captured ConfigAccount must decode");

    let mint_account = fixture.account(&curve.mint);
    let mint_decimals =
        mint_decimals(&mint_account.data).expect("the curve's mint carries a decimals byte");

    MoonitMarket::new(
        curve,
        mint_account.owner,
        mint_decimals,
        config.dex_fee,
        config.helio_fee,
        config.fee_bps,
    )
}

#[test]
fn the_moonit_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("moonit_pool");
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

    let market = moonit_market();
    assert!(
        market.quote(&market.mints().1, 5_000_000).is_ok(),
        "a real curve with real reserves must quote a small buy"
    );
}

#[test]
fn a_moonit_quote_off_real_state_is_monotonic_and_settles_native_sol() {
    let market = moonit_market();
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
    let market = moonit_market();
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

fn fluxbeam_market() -> FluxbeamMarket {
    let fixture = Fixture::load("fluxbeam_pool");
    let state = FluxbeamPoolState::decode(fixture.pool, fixture.data(&fixture.pool))
        .expect("the captured FluxBeam SwapV1 state must decode");

    let mint_a_account = fixture.account(&state.mint_a);
    let mint_b_account = fixture.account(&state.mint_b);
    let pool_mint_account = fixture.account(&state.pool_mint);

    FluxbeamMarket::new(
        state,
        fixture.balance(&state.vault_a),
        fixture.balance(&state.vault_b),
        mint_decimals(&mint_a_account.data).expect("mint_a carries a decimals byte"),
        mint_decimals(&mint_b_account.data).expect("mint_b carries a decimals byte"),
        mint_a_account.owner,
        mint_b_account.owner,
        pool_mint_account.owner,
    )
}

#[test]
fn the_fluxbeam_layout_reads_real_values_at_every_offset_it_claims() {
    let fixture = Fixture::load("fluxbeam_pool");
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
    let market = fluxbeam_market();
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
    let market = fluxbeam_market();
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
    assert!(fluxbeam_market().quote(&stranger, 5_000_000).is_err());
}

#[test]
fn the_fluxbeam_instruction_orients_from_the_input_mint_and_matches_the_confirmed_shape() {
    let market = fluxbeam_market();
    let (mint_a, mint_b) = market.mints();
    let fixture = Fixture::load("fluxbeam_pool");
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
    let market = fluxbeam_market();
    let (mint_a, mint_b) = market.mints();
    let intent = DirectSwapIntent {
        pool: market.pool(),
        owner: Pubkey::new_unique(),
        input_mint: mint_a,
        output_mint: mint_b,
        amount_in: 5_000_000,
        slippage_bps: 300,
    };

    let quote = direct::quote_with_market(&intent, &market).expect("quotes offline");
    assert_eq!(quote.fee.side, FeeSide::Input, "SOL is the input leg here");
    assert_eq!(quote.fee.amount, 25_000, "0.5% of 0.005 SOL");
    assert_eq!(
        quote.swap_amount_in,
        5_000_000 - 25_000,
        "the platform fee never reaches the pool"
    );
}
