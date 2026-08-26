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
//! @336/@368), store them base64 under `accounts`, and re-run. Balances move, so
//! assertions here are on RELATIONSHIPS — fee rates, orientation, monotonicity —
//! not on a specific output amount that would rot with the next trade.

mod common;

use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::swaps::direct::venues::layout::token_account_amount;
use screenerbot::chains::solana::swaps::direct::venues::raydium_amm_v4::{
    AmmV4Market, AmmV4PoolState,
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
