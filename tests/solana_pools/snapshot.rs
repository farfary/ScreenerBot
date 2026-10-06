// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Recorded Solana pool prices.
//!
//! Every `price-snapshot` case is decoded from its recorded account bytes and the resulting price
//! is compared byte-for-byte with `tests/fixtures/pools/solana/prices-snapshot.json`. Any change
//! to decoder math, account selection, orientation or decimals handling shows up as a differing
//! entry. The snapshot holds one entry per case, in venue then case-name order.
//!
//! # Coverage
//!
//! Eleven cases cover ten of the eleven `ProgramKind`s with a decoder: Meteora DAMM v2, Meteora
//! DBC (plain and with an output fee), Meteora DLMM, Moonit, Orca Whirlpool, pump.fun AMM,
//! pump.fun legacy, Raydium AMM v4, Raydium CLMM and Raydium CPMM. `ProgramKind::Unknown` has no
//! decoder and no pool, so it has no case.
//!
//! **Uncovered: `ProgramKind::FluxbeamAmm`.** The only recorded fluxbeam pool
//! (`7uajENggf2MaiZ5XGff91uoVsch1y5QN3bqjisv7eP6V`, a `direct-swap` case) has an empty token vault
//! on mainnet and its fee account is closed, so its decoder returns no price. The test asserts
//! the covered kinds exactly, so adding a fluxbeam case is a deliberate change to
//! [`COVERED_KINDS`].
//!
//! # Inputs
//!
//! A `price-snapshot` case holds the pool's account bundle, the mint decimals the pricing path
//! reads from the token cache, and the target and quote mints. The bundle is the set the pool
//! analyzer registers for the venue (`PoolAnalyzer::reserve_account_set`) less the WSOL mint,
//! because the fetcher prices from exactly that set; see `fetched_account_set`. The target mint
//! is the pool's non-WSOL mint, so a SOL/USDC pool prices USDC in SOL. Floats are written by
//! `serde_json` (shortest round-trip), so equal text means equal bits; the decoders use only IEEE
//! arithmetic and `powi`.
//!
//! # Offline guarantee
//!
//! Decimals are seeded from the case and every account a decoder reads is in the case, so the
//! comparison touches no network and no database.

use crate::common;
use crate::common::solana_pools::{
    cases_for_purpose, decoder_accounts, mint_of, program_kind, pubkey, SolanaCase,
    TOKEN_ACCOUNT_BASE_LEN,
};
use screenerbot::chains::solana::constants::SOL_MINT;
use screenerbot::chains::solana::pools::analyzer::PoolAnalyzer;
use screenerbot::chains::solana::pools::decoders::decode_pool;
use screenerbot::chains::solana::pools::fetcher::AccountData;
use screenerbot::chains::solana::pools::types::ProgramKind;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

const PRICE_PURPOSE: &str = "price-snapshot";
const SWAP_PURPOSE: &str = "direct-swap";

/// Protocol slugs of the program kinds the price cases cover; see the module doc for the
/// uncovered kind.
const COVERED_KINDS: [&str; 10] = [
    "meteora_damm_v2",
    "meteora_dbc",
    "meteora_dlmm",
    "moonit_amm",
    "orca_whirlpool",
    "pumpfun_amm",
    "pumpfun_legacy",
    "raydium_clmm",
    "raydium_cpmm",
    "raydium_legacy_amm",
];

/// Every `PriceResult` field except `timestamp`, in declaration order.
#[derive(Serialize, Deserialize)]
struct RecordedPrice {
    mint: String,
    price_usd: f64,
    price_native: f64,
    confidence: f32,
    source_pool: Option<String>,
    pool_address: String,
    slot: u64,
    native_reserves: f64,
    token_reserves: f64,
}

#[derive(Serialize, Deserialize)]
struct PriceSnapshot {
    prices: Vec<RecordedPrice>,
}

fn snapshot_path() -> PathBuf {
    common::pool_cases::fixtures_dir()
        .join("solana")
        .join("prices-snapshot.json")
}

fn pretty<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("serialize snapshot");
    text.push('\n');
    text
}

/// Decode one case exactly as the pool calculator dispatches it: the program kind comes from the
/// pool account's owner, the base mint is the target and the quote mint is WSOL.
fn decode_case(case: &SolanaCase) -> Option<RecordedPrice> {
    decode_accounts(case, &decoder_accounts(case))
}

/// Decode `accounts` as the bundle of `case`'s pool, with the case's decimals seeded.
fn decode_accounts(
    case: &SolanaCase,
    accounts: &HashMap<String, AccountData>,
) -> Option<RecordedPrice> {
    for (mint, decimals) in &case.decimals {
        common::seed_decimals(mint, *decimals);
    }
    let kind = program_kind(case);
    decode_pool(kind, accounts, &case.target_mint, SOL_MINT).map(|price| RecordedPrice {
        mint: price.mint,
        price_usd: price.price_usd,
        price_native: price.price_native,
        confidence: price.confidence,
        source_pool: price.source_pool,
        pool_address: price.pool_address,
        slot: price.slot,
        native_reserves: price.native_reserves,
        token_reserves: price.token_reserves,
    })
}

/// Decode every price case in order; a case without a price fails with its venue and pool.
fn decode_all(cases: &[(String, SolanaCase)]) -> Vec<RecordedPrice> {
    cases
        .iter()
        .map(|(name, case)| {
            decode_case(case).unwrap_or_else(|| {
                panic!(
                    "{} case {name} pool {} no longer decodes to a price",
                    case.venue, case.pool
                )
            })
        })
        .collect()
}

/// The price cases of one program kind.
fn price_cases_of(kind: ProgramKind) -> Vec<(String, SolanaCase)> {
    cases_for_purpose(PRICE_PURPOSE)
        .into_iter()
        .filter(|(_, case)| case.venue == kind.protocol_slug())
        .collect()
}

#[test]
fn solana_pool_prices_match_the_recorded_snapshot() {
    common::install_chain_runtimes();

    let cases = cases_for_purpose(PRICE_PURPOSE);
    let kinds: BTreeSet<&str> = cases.iter().map(|(_, case)| case.venue.as_str()).collect();
    assert_eq!(
        kinds,
        BTreeSet::from(COVERED_KINDS),
        "the price cases cover a different set of program kinds"
    );
    let actual = PriceSnapshot {
        prices: decode_all(&cases),
    };

    let expected_text = std::fs::read_to_string(snapshot_path()).expect("read recorded prices");
    let expected: PriceSnapshot =
        serde_json::from_str(&expected_text).expect("parse recorded prices");
    if pretty(&actual) != expected_text {
        if expected.prices.len() != actual.prices.len() {
            eprintln!(
                "DIFF entry count: recorded {}, decoded {}",
                expected.prices.len(),
                actual.prices.len()
            );
        }
        for (index, got) in actual.prices.iter().enumerate() {
            let got = serde_json::to_value(got).expect("serialize decoded price");
            let want = expected
                .prices
                .get(index)
                .map(|price| serde_json::to_value(price).expect("serialize recorded price"));
            if want.as_ref() != Some(&got) {
                eprintln!(
                    "DIFF case={} pool={} expected={want:?} actual={got}",
                    cases[index].0, cases[index].1.pool
                );
            }
        }
        panic!("pool prices differ from the recorded snapshot");
    }
}

/// The account bundle the pool fetcher prices a pool from: the set the pool analyzer
/// registers, less the WSOL mint, which the fetcher never requests. Several decoders find
/// their pool by scanning for the program owner, so a price case holds this bundle and
/// nothing else (a config or tick-array account with the same owner would make that scan
/// ambiguous).
fn fetched_account_set(
    kind: ProgramKind,
    pool: &str,
    pool_data: &[u8],
    target_mint: &str,
) -> Vec<String> {
    PoolAnalyzer::reserve_account_set(
        kind,
        &pubkey(pool),
        pool_data,
        &pubkey(target_mint),
        &pubkey(SOL_MINT),
    )
    .unwrap_or_else(|| panic!("pool {pool}: the analyzer derives no account set"))
    .into_iter()
    .map(|account| account.to_string())
    .filter(|account| account != SOL_MINT)
    .collect()
}

#[test]
fn price_cases_hold_exactly_the_analyzer_account_set() {
    let mut mismatches = Vec::new();
    for (name, case) in cases_for_purpose(PRICE_PURPOSE) {
        let kind = program_kind(&case);
        let pool = &case.accounts[&case.pool];
        let derived: BTreeSet<String> =
            fetched_account_set(kind, &case.pool, &pool.data, &case.target_mint)
                .into_iter()
                .collect();
        let recorded: BTreeSet<String> = case.accounts.keys().cloned().collect();
        if derived != recorded {
            mismatches.push(format!(
                "{} case {name} pool {}: derived {derived:?}, recorded {recorded:?}",
                case.venue, case.pool
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "the analyzer derives a different account set than the recorded bundle:\n{}",
        mismatches.join("\n")
    );
}

/// The venues whose pool layout the swap engine and the pricing path share: the vaults
/// the analyzer derives from a pool are exactly the token accounts its direct-swap
/// case recorded for the swap.
#[test]
fn shared_layout_vaults_match_the_direct_swap_cases() {
    for (name, case) in cases_for_purpose(SWAP_PURPOSE) {
        let mut mints = BTreeSet::new();
        let mut swap_vaults = BTreeSet::new();
        for (address, account) in &case.accounts {
            if let Some(mint) = mint_of(address, account) {
                if mint != *address {
                    swap_vaults.insert(address.clone());
                }
                mints.insert(mint);
            }
        }
        assert!(
            case.target_mint != SOL_MINT && mints.contains(&case.target_mint),
            "{} case {name}: target {} is not a non-WSOL mint of the pool, found {mints:?}",
            case.venue,
            case.target_mint
        );
        let kind = program_kind(&case);
        if !matches!(
            kind,
            ProgramKind::RaydiumLegacyAmm | ProgramKind::MeteoraDbc
        ) {
            continue;
        }
        let derived: BTreeSet<String> = fetched_account_set(
            kind,
            &case.pool,
            &case.accounts[&case.pool].data,
            &case.target_mint,
        )
        .into_iter()
        .filter(|account| *account != case.pool && *account != case.target_mint)
        .collect();
        assert_eq!(
            derived, swap_vaults,
            "{} case {name}: the analyzer's vaults differ from the vaults the swap reads",
            case.venue
        );
    }
}

#[test]
fn a_dbc_price_does_not_depend_on_account_order_or_unrelated_accounts() {
    let cases = price_cases_of(ProgramKind::MeteoraDbc);
    assert!(!cases.is_empty(), "the price cases carry a DBC pool");
    for (name, case) in &cases {
        let expected = serde_json::to_value(decode_case(case).expect("recorded pool prices"))
            .expect("serialize price");
        let mut accounts = decoder_accounts(case);
        // A second token account of the base mint, and a second account of the DBC
        // program that is not a pool, sit in the bundle beside the real ones.
        let pool = accounts[&case.pool].clone();
        let base_vault = accounts
            .values()
            .find(|account| account.pubkey != pool.pubkey && account.data.len() >= 72)
            .expect("bundle carries a vault")
            .clone();
        let decoy_vault = Pubkey::new_unique();
        accounts.insert(
            decoy_vault.to_string(),
            AccountData {
                pubkey: decoy_vault,
                ..base_vault
            },
        );
        let decoy_program_account = Pubkey::new_unique();
        let mut not_a_pool = pool.data.clone();
        not_a_pool[0] ^= 1;
        accounts.insert(
            decoy_program_account.to_string(),
            AccountData {
                pubkey: decoy_program_account,
                data: not_a_pool,
                ..pool
            },
        );
        let mut entries: Vec<(String, AccountData)> = accounts.into_iter().collect();
        for rotation in 0..entries.len() {
            entries.rotate_left(1);
            let reordered: HashMap<String, AccountData> = entries.iter().cloned().collect();
            let got = serde_json::to_value(
                decode_accounts(case, &reordered).expect("the pool still prices"),
            )
            .expect("serialize price");
            assert_eq!(
                got, expected,
                "case {name} pool {} prices differently at rotation {rotation}",
                case.pool
            );
        }
    }
}

#[test]
fn a_migrated_dbc_pool_yields_no_price() {
    for (name, case) in price_cases_of(ProgramKind::MeteoraDbc) {
        let mut accounts = decoder_accounts(&case);
        let pool = accounts
            .get_mut(&case.pool)
            .expect("bundle carries the pool");
        pool.data[305] = 1;
        assert!(
            decode_accounts(&case, &accounts).is_none(),
            "migrated pool {} of case {name} must not price",
            case.pool
        );
    }
}

#[test]
fn an_amm_v4_vault_holding_the_wrong_mint_yields_no_price() {
    let cases = price_cases_of(ProgramKind::RaydiumLegacyAmm);
    assert!(!cases.is_empty(), "the price cases carry an AMM v4 pool");
    for (name, case) in cases {
        let mut accounts = decoder_accounts(&case);
        let vaults: Vec<String> = accounts
            .iter()
            .filter(|(_, account)| account.data.len() == TOKEN_ACCOUNT_BASE_LEN)
            .map(|(address, _)| address.clone())
            .collect();
        assert_eq!(
            vaults.len(),
            2,
            "pool {} of case {name} bundles two vaults",
            case.pool
        );
        // Each vault keeps its address but carries the other side's bytes.
        let first = accounts[&vaults[0]].data.clone();
        let second = accounts[&vaults[1]].data.clone();
        accounts.get_mut(&vaults[0]).unwrap().data = second;
        accounts.get_mut(&vaults[1]).unwrap().data = first;
        assert!(
            decode_accounts(&case, &accounts).is_none(),
            "pool {} of case {name} must not price from vaults holding the other mint",
            case.pool
        );
    }
}
