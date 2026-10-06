// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Recorded Solana pool prices.
//!
//! One live mainnet pool per decoder family is decoded from recorded account bytes and the
//! resulting price is compared byte-for-byte with a recorded file. Any change to decoder
//! math, account selection, orientation or decimals handling shows up as a differing entry.
//!
//! # Coverage
//!
//! Eleven entries cover ten of the eleven `ProgramKind`s with a decoder: Meteora DAMM v2,
//! Meteora DBC (plain and with an output fee), Meteora DLMM, Moonit, Orca Whirlpool,
//! pump.fun AMM, pump.fun legacy, Raydium AMM v4, Raydium CLMM and Raydium CPMM.
//! `ProgramKind::Unknown` has no decoder and no pool, so it has no entry.
//!
//! **Uncovered: `ProgramKind::FluxbeamAmm`.** The only recorded fluxbeam pool
//! (`7uajENggf2MaiZ5XGff91uoVsch1y5QN3bqjisv7eP6V`, from `fluxbeam_pool.json`) has an empty
//! token vault on mainnet and its fee account is closed, so its decoder returns no price.
//! The test asserts the covered kinds exactly, so adding a fluxbeam entry is a deliberate
//! change to [`COVERED_KINDS`].
//!
//! # Inputs (`tests/fixtures/pools/`)
//!
//! * `solana-price-inputs.json` — per entry: the program kind slug, the pool, the target
//!   and quote mints, their decimals, and the account bundle the decoder receives as
//!   `{owner, lamports, slot, data_base64}`. The bundle is the set the pool analyzer
//!   registers for the venue (`PoolAnalyzer::reserve_account_set`) less the WSOL mint,
//!   because the fetcher prices from exactly that set; see `fetched_account_set`.
//! * `solana-prices.json` — the recording target and, per entry in input order, every
//!   `PriceResult` field except `timestamp`.
//!
//! The pool addresses are the public chain accounts already used by
//! `tests/fixtures/direct_swaps/`. The target mint is the pool's non-WSOL mint, so a SOL/USDC
//! pool prices USDC in SOL. Floats are written by `serde_json` (shortest round-trip), so equal
//! text means equal bits; the decoders use only IEEE arithmetic and `powi`, which are
//! deterministic on one target. A run on another target fails with both targets named.
//!
//! # Offline guarantee
//!
//! Decimals are seeded from the recorded mint accounts and every account a decoder reads is
//! in the input file, so the comparison touches no network and no database.
//!
//! # Recording
//!
//! The ignored recorder takes each pool from its direct-swap fixture, derives the fetcher's
//! account bundle from the live pool data, fetches the bundle and both mints from the
//! default public RPC (no keys) through `get_rpc_client()` in batches of at most 50, decodes
//! every entry and writes both files only when every entry yields a finite, positive price:
//!
//! ```text
//! SCREENERBOT_RECORD_POOL_PRICE_FIXTURE=1 cargo test --test pool_price_snapshot \
//!     -- --ignored record_solana_price_inputs --nocapture
//! ```

mod common;

use base64::Engine;
use screenerbot::chains::solana::constants::SOL_MINT;
use screenerbot::chains::solana::pools::analyzer::PoolAnalyzer;
use screenerbot::chains::solana::pools::decoders::decode_pool;
use screenerbot::chains::solana::pools::fetcher::AccountData;
use screenerbot::chains::solana::pools::types::ProgramKind;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

/// The direct-swap fixture that names each recorded pool, with the pool's non-WSOL mint,
/// in recording order. A pool's accounts can refer to further mints (an LP mint), so the
/// target is named here and checked against the fixture.
const ENTRIES: [(&str, &str); 11] = [
    (
        "meteora_damm_v2_pool.json",
        "5zcqoA4zmeLnLdc6GonaFLzxFRna6kQX9F1cLNvZWFd4",
    ),
    (
        "meteora_dbc_pool.json",
        "9rcxe6nSq9GT56KyGV8QHhBYKgjNaGmW2JyDDfsZBAGS",
    ),
    (
        "meteora_dbc_pool_output_fee.json",
        "9b41s5Uvz8cBXR4zwUPENx3HCThJc8Z1DAjRPzfnEb2U",
    ),
    (
        "meteora_dlmm_pool.json",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ),
    (
        "moonit_pool.json",
        "DqExwRSP2TYfNu5J4PhxNy6JV8PfCz9sjd8jPEoAmoon",
    ),
    (
        "orca_whirlpool_pool.json",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ),
    (
        "pumpfun_amm_pool.json",
        "5UUH9RTDiSpq6HKS6bp4NdU9PNJpXRXuiw6ShBTBhgH2",
    ),
    (
        "pumpfun_legacy_pool.json",
        "2xJGewx1p72WFCAwBvmbpejZxqa7EN3mS3PiGjgrpump",
    ),
    (
        "raydium_amm_v4_pool.json",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ),
    (
        "raydium_clmm_pool.json",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ),
    (
        "raydium_cpmm_pool.json",
        "Dz9mQ9NzkBcCsuGPFJ3r1bS4wgqKMHBPiVuniW8Mbonk",
    ),
];

/// Protocol slugs of the program kinds the inputs cover; see the module doc for the
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

/// `get_multiple_accounts` batch ceiling.
const MAX_ACCOUNTS_PER_CALL: usize = 50;

/// SPL mint layout: `decimals` is the byte after the 36-byte mint authority option and the
/// 8-byte supply. Token-2022 keeps the same base layout.
const MINT_DECIMALS_OFFSET: usize = 44;
const MINT_BASE_LEN: usize = 82;
const TOKEN_ACCOUNT_BASE_LEN: usize = 165;
/// Token-2022 `AccountType` byte that follows the base layout when extensions exist.
const ACCOUNT_TYPE_MINT: u8 = 1;
const ACCOUNT_TYPE_TOKEN_ACCOUNT: u8 = 2;

const INPUTS_FILE: &str = "solana-price-inputs.json";
const PRICES_FILE: &str = "solana-prices.json";

#[derive(Serialize, Deserialize)]
struct RecordedAccount {
    owner: String,
    lamports: u64,
    slot: u64,
    data_base64: String,
}

#[derive(Serialize, Deserialize)]
struct PriceInput {
    kind: String,
    pool: String,
    target_mint: String,
    quote_mint: String,
    decimals: BTreeMap<String, u8>,
    accounts: BTreeMap<String, RecordedAccount>,
}

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
    target: String,
    prices: Vec<RecordedPrice>,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("pools")
}

fn swap_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("direct_swaps")
}

fn read_file(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn pretty<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("serialize fixture");
    text.push('\n');
    text
}

fn current_target() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

fn pubkey(address: &str) -> Pubkey {
    Pubkey::from_str(address).unwrap_or_else(|e| panic!("invalid pubkey {address}: {e}"))
}

/// The account map a decoder receives, built from the recorded bytes.
fn account_map(input: &PriceInput) -> HashMap<String, AccountData> {
    input
        .accounts
        .iter()
        .map(|(address, recorded)| {
            let data = base64::engine::general_purpose::STANDARD
                .decode(&recorded.data_base64)
                .unwrap_or_else(|e| panic!("account {address} carries invalid base64: {e}"));
            let account = AccountData {
                pubkey: pubkey(address),
                data,
                slot: recorded.slot,
                fetched_at: Instant::now(),
                lamports: recorded.lamports,
                owner: pubkey(&recorded.owner),
            };
            (address.clone(), account)
        })
        .collect()
}

/// Decode one input exactly as the pool calculator dispatches it: the program kind comes
/// from the pool account's owner, the base mint is the target and the quote mint is WSOL.
fn decode_input(input: &PriceInput) -> Option<RecordedPrice> {
    decode_accounts(input, &account_map(input))
}

/// Decode `accounts` as the bundle of `input`'s pool, with `input`'s decimals seeded.
fn decode_accounts(
    input: &PriceInput,
    accounts: &HashMap<String, AccountData>,
) -> Option<RecordedPrice> {
    for (mint, decimals) in &input.decimals {
        common::seed_decimals(mint, *decimals);
    }
    let pool = accounts
        .get(&input.pool)
        .unwrap_or_else(|| panic!("input for {} lacks its pool account", input.pool));
    let kind = ProgramKind::classify(&pool.owner);
    assert_eq!(
        kind.protocol_slug(),
        input.kind,
        "pool {} classifies as a different program kind",
        input.pool
    );
    decode_pool(kind, accounts, &input.target_mint, SOL_MINT).map(|price| RecordedPrice {
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

/// Decode every input in order; an entry without a price fails with its kind and pool.
fn decode_all(inputs: &[PriceInput]) -> Vec<RecordedPrice> {
    inputs
        .iter()
        .map(|input| {
            decode_input(input).unwrap_or_else(|| {
                panic!(
                    "{} pool {} no longer decodes to a price",
                    input.kind, input.pool
                )
            })
        })
        .collect()
}

fn parse_inputs(text: &str) -> Vec<PriceInput> {
    serde_json::from_str(text).expect("parse price inputs")
}

// ============================================================================
// SNAPSHOT
// ============================================================================

#[test]
fn solana_pool_prices_match_the_recorded_snapshot() {
    common::install_chain_runtimes();

    let inputs = parse_inputs(&read_file(fixture_dir().join(INPUTS_FILE)));
    let kinds: BTreeSet<&str> = inputs.iter().map(|input| input.kind.as_str()).collect();
    assert_eq!(
        kinds,
        BTreeSet::from(COVERED_KINDS),
        "the recorded inputs cover a different set of program kinds"
    );
    let actual = PriceSnapshot {
        target: current_target(),
        prices: decode_all(&inputs),
    };
    let actual_text = pretty(&actual);

    let expected_text = read_file(fixture_dir().join(PRICES_FILE));
    if actual_text != expected_text {
        let expected: PriceSnapshot =
            serde_json::from_str(&expected_text).expect("parse recorded prices");
        if expected.target != actual.target {
            eprintln!(
                "DIFF target: recorded on {}, running on {}",
                expected.target, actual.target
            );
        }
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
                    "DIFF kind={} pool={} expected={want:?} actual={got}",
                    inputs[index].kind, inputs[index].pool
                );
            }
        }
        panic!("pool prices differ from the recorded snapshot");
    }
}

#[test]
fn recorded_inputs_hold_exactly_the_analyzer_account_set() {
    let inputs = parse_inputs(&read_file(fixture_dir().join(INPUTS_FILE)));
    let mut mismatches = Vec::new();
    for input in &inputs {
        let accounts = account_map(input);
        let pool = &accounts[&input.pool];
        let kind = ProgramKind::classify(&pool.owner);
        let derived: BTreeSet<String> =
            fetched_account_set(kind, &input.pool, &pool.data, &input.target_mint)
                .into_iter()
                .collect();
        let recorded: BTreeSet<String> = input.accounts.keys().cloned().collect();
        if derived != recorded {
            mismatches.push(format!(
                "{} pool {}: derived {derived:?}, recorded {recorded:?}",
                input.kind, input.pool
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "the analyzer derives a different account set than the recorded bundle:\n{}",
        mismatches.join("\n")
    );
}

/// The recorded inputs of one program kind.
fn inputs_of(kind: ProgramKind) -> Vec<PriceInput> {
    parse_inputs(&read_file(fixture_dir().join(INPUTS_FILE)))
        .into_iter()
        .filter(|input| input.kind == kind.protocol_slug())
        .collect()
}

/// The venues whose pool layout the swap engine and the pricing path share: the vaults
/// the analyzer derives from a pool are exactly the token accounts its direct-swap
/// fixture recorded for the swap.
#[test]
fn shared_layout_vaults_match_the_direct_swap_fixtures() {
    for (file, target_mint) in ENTRIES {
        let (pool, swap_vaults) = swap_fixture_pool(file, target_mint);
        let json: serde_json::Value =
            serde_json::from_str(&read_file(swap_fixture_dir().join(file))).expect("parse fixture");
        let pool_account = &json["accounts"][&pool];
        let kind = ProgramKind::classify(&pubkey(
            pool_account["owner"].as_str().expect("pool has an owner"),
        ));
        if !matches!(
            kind,
            ProgramKind::RaydiumLegacyAmm | ProgramKind::MeteoraDbc
        ) {
            continue;
        }
        let pool_data = base64::engine::general_purpose::STANDARD
            .decode(pool_account["data"].as_str().expect("pool carries data"))
            .expect("pool data is valid base64");
        let derived: BTreeSet<String> = fetched_account_set(kind, &pool, &pool_data, target_mint)
            .into_iter()
            .filter(|account| *account != pool && account != target_mint)
            .collect();
        let swap_vaults: BTreeSet<String> = swap_vaults.into_iter().collect();
        assert_eq!(
            derived, swap_vaults,
            "{file}: the analyzer's vaults differ from the vaults the swap reads"
        );
    }
}

#[test]
fn a_dbc_price_does_not_depend_on_account_order_or_unrelated_accounts() {
    let inputs = inputs_of(ProgramKind::MeteoraDbc);
    assert!(!inputs.is_empty(), "the recorded inputs carry a DBC pool");
    for input in &inputs {
        let expected = serde_json::to_value(decode_input(input).expect("recorded pool prices"))
            .expect("serialize price");
        let mut accounts = account_map(input);
        // A second token account of the base mint, and a second account of the DBC
        // program that is not a pool, sit in the bundle beside the real ones.
        let pool = accounts[&input.pool].clone();
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
                decode_accounts(input, &reordered).expect("the pool still prices"),
            )
            .expect("serialize price");
            assert_eq!(
                got, expected,
                "pool {} prices differently at rotation {rotation}",
                input.pool
            );
        }
    }
}

#[test]
fn a_migrated_dbc_pool_yields_no_price() {
    for input in inputs_of(ProgramKind::MeteoraDbc) {
        let mut accounts = account_map(&input);
        let pool = accounts
            .get_mut(&input.pool)
            .expect("bundle carries the pool");
        pool.data[305] = 1;
        assert!(
            decode_accounts(&input, &accounts).is_none(),
            "migrated pool {} must not price",
            input.pool
        );
    }
}

#[test]
fn an_amm_v4_vault_holding_the_wrong_mint_yields_no_price() {
    let inputs = inputs_of(ProgramKind::RaydiumLegacyAmm);
    assert!(
        !inputs.is_empty(),
        "the recorded inputs carry an AMM v4 pool"
    );
    for input in inputs {
        let mut accounts = account_map(&input);
        let vaults: Vec<String> = accounts
            .iter()
            .filter(|(_, account)| account.data.len() == TOKEN_ACCOUNT_BASE_LEN)
            .map(|(address, _)| address.clone())
            .collect();
        assert_eq!(vaults.len(), 2, "pool {} bundles two vaults", input.pool);
        // Each vault keeps its address but carries the other side's bytes.
        let first = accounts[&vaults[0]].data.clone();
        let second = accounts[&vaults[1]].data.clone();
        accounts.get_mut(&vaults[0]).unwrap().data = second;
        accounts.get_mut(&vaults[1]).unwrap().data = first;
        assert!(
            decode_accounts(&input, &accounts).is_none(),
            "pool {} must not price from vaults holding the other mint",
            input.pool
        );
    }
}

// ============================================================================
// RECORDER
// ============================================================================

/// The account bundle the pool fetcher prices a pool from: the set the pool analyzer
/// registers, less the WSOL mint, which the fetcher never requests. Several decoders find
/// their pool by scanning for the program owner, so the recorded map holds this bundle and
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

fn is_token_program(owner: &str) -> bool {
    owner == screenerbot::chains::solana::spl_token::id().to_string()
        || owner == screenerbot::chains::solana::spl_token_2022::id().to_string()
}

/// The mint an SPL account refers to: itself for a mint, its `mint` field for a token
/// account. `None` for any other account.
fn mint_of(address: &str, owner: &str, data: &[u8]) -> Option<String> {
    if !is_token_program(owner) {
        return None;
    }
    let account_type = data.get(TOKEN_ACCOUNT_BASE_LEN).copied();
    if data.len() == MINT_BASE_LEN || account_type == Some(ACCOUNT_TYPE_MINT) {
        return Some(address.to_owned());
    }
    if data.len() == TOKEN_ACCOUNT_BASE_LEN || account_type == Some(ACCOUNT_TYPE_TOKEN_ACCOUNT) {
        let mint: [u8; 32] = data[..32].try_into().expect("token account holds a mint");
        return Some(Pubkey::new_from_array(mint).to_string());
    }
    None
}

/// The pool of one direct-swap fixture and the token accounts the fixture lists. The
/// target mint must be one of the mints the fixture's accounts refer to.
fn swap_fixture_pool(file: &str, target_mint: &str) -> (String, Vec<String>) {
    let json: serde_json::Value = serde_json::from_str(&read_file(swap_fixture_dir().join(file)))
        .unwrap_or_else(|e| panic!("parse {file}: {e}"));
    let pool = json["pool"]
        .as_str()
        .expect("fixture names its pool")
        .to_owned();
    let accounts = json["accounts"]
        .as_object()
        .expect("fixture carries accounts");
    let mut mints = BTreeSet::new();
    let mut token_accounts = Vec::new();
    for (address, value) in accounts {
        let owner = value["owner"].as_str().expect("account has an owner");
        let data = base64::engine::general_purpose::STANDARD
            .decode(value["data"].as_str().expect("account carries data"))
            .expect("account data is valid base64");
        if let Some(mint) = mint_of(address, owner, &data) {
            if mint != *address {
                token_accounts.push(address.clone());
            }
            mints.insert(mint);
        }
    }
    assert!(
        target_mint != SOL_MINT && mints.contains(target_mint),
        "{file}: target {target_mint} is not a non-WSOL mint of the pool, found {mints:?}"
    );
    (pool, token_accounts)
}

async fn fetch_accounts(addresses: &[String]) -> (u64, BTreeMap<String, RecordedAccount>) {
    use screenerbot::chains::solana::rpc::{get_rpc_client, RpcClientMethods};

    let rpc = get_rpc_client();
    let slot = rpc.get_slot().await.expect("read the current slot");
    let keys: Vec<Pubkey> = addresses.iter().map(|a| pubkey(a)).collect();
    let mut recorded = BTreeMap::new();
    for (chunk_keys, chunk_addresses) in keys
        .chunks(MAX_ACCOUNTS_PER_CALL)
        .zip(addresses.chunks(MAX_ACCOUNTS_PER_CALL))
    {
        let accounts = rpc
            .get_multiple_accounts(chunk_keys)
            .await
            .expect("fetch pool accounts");
        assert_eq!(accounts.len(), chunk_keys.len(), "one result per account");
        for (address, account) in chunk_addresses.iter().zip(accounts) {
            let account = account.unwrap_or_else(|| panic!("account {address} does not exist"));
            recorded.insert(
                address.clone(),
                RecordedAccount {
                    owner: account.owner.to_string(),
                    lamports: account.lamports,
                    slot,
                    data_base64: base64::engine::general_purpose::STANDARD.encode(&account.data),
                },
            );
        }
    }
    (slot, recorded)
}

fn recorded_decimals(accounts: &BTreeMap<String, RecordedAccount>, mint: &str) -> u8 {
    let account = accounts
        .get(mint)
        .unwrap_or_else(|| panic!("mint account {mint} was not recorded"));
    let data = base64::engine::general_purpose::STANDARD
        .decode(&account.data_base64)
        .expect("recorded data is valid base64");
    *data
        .get(MINT_DECIMALS_OFFSET)
        .unwrap_or_else(|| panic!("mint account {mint} is shorter than the mint layout"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn record_solana_price_inputs() {
    if std::env::var("SCREENERBOT_RECORD_POOL_PRICE_FIXTURE").as_deref() != Ok("1") {
        eprintln!("SKIP recorder: SCREENERBOT_RECORD_POOL_PRICE_FIXTURE is not 1");
        return;
    }
    let _data_dir = common::isolated_env();
    common::install_chain_runtimes();

    let mut inputs = Vec::with_capacity(ENTRIES.len());
    for (file, target_mint) in ENTRIES {
        let target_mint = target_mint.to_owned();
        let (pool, _) = swap_fixture_pool(file, &target_mint);
        let (_, pool_only) = fetch_accounts(std::slice::from_ref(&pool)).await;
        let pool_account = &pool_only[&pool];
        let kind = ProgramKind::classify(&pubkey(&pool_account.owner));
        let pool_data = base64::engine::general_purpose::STANDARD
            .decode(&pool_account.data_base64)
            .expect("recorded data is valid base64");
        let set = fetched_account_set(kind, &pool, &pool_data, &target_mint);

        // One snapshot of the bundle plus the mints its decimals come from.
        let mut addresses = set.clone();
        for mint in [&target_mint, SOL_MINT] {
            if !addresses.iter().any(|a| a == mint) {
                addresses.push(mint.to_owned());
            }
        }
        let (slot, mut fetched) = fetch_accounts(&addresses).await;
        let decimals = BTreeMap::from([
            (
                target_mint.clone(),
                recorded_decimals(&fetched, &target_mint),
            ),
            (SOL_MINT.to_owned(), recorded_decimals(&fetched, SOL_MINT)),
        ]);
        let accounts: BTreeMap<String, RecordedAccount> = set
            .iter()
            .map(|address| {
                let account = fetched.remove(address).expect("bundle account was fetched");
                (address.clone(), account)
            })
            .collect();
        eprintln!(
            "recorder: {file} kind={} pool={pool} target={target_mint} accounts={} slot={slot}",
            kind.protocol_slug(),
            accounts.len()
        );
        inputs.push(PriceInput {
            kind: kind.protocol_slug().to_owned(),
            pool,
            target_mint,
            quote_mint: SOL_MINT.to_owned(),
            decimals,
            accounts,
        });
    }

    // Round-trip through the fixture text, then decode exactly as the snapshot test does.
    let inputs_text = pretty(&inputs);
    let reparsed = parse_inputs(&inputs_text);
    let mut failures = Vec::new();
    let mut prices = Vec::with_capacity(reparsed.len());
    for input in &reparsed {
        match decode_input(input) {
            Some(price) if price.price_native.is_finite() && price.price_native > 0.0 => {
                eprintln!(
                    "recorder: {} price_native={} native_reserves={} token_reserves={}",
                    input.kind, price.price_native, price.native_reserves, price.token_reserves
                );
                prices.push(price);
            }
            Some(price) => failures.push(format!(
                "{} pool {}: price_native {} is not finite and positive",
                input.kind, input.pool, price.price_native
            )),
            None => failures.push(format!(
                "{} pool {}: the decoder returned no price",
                input.kind, input.pool
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "entries without a usable price:\n{}",
        failures.join("\n")
    );

    let prices_text = pretty(&PriceSnapshot {
        target: current_target(),
        prices,
    });
    let dir = fixture_dir();
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    for (name, text) in [(INPUTS_FILE, &inputs_text), (PRICES_FILE, &prices_text)] {
        std::fs::write(dir.join(name), text).expect("write fixture");
        eprintln!("recorder: wrote {name} ({} bytes)", text.len());
    }
}
