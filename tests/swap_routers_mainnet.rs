// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Every Solana swap router, every quote-asset pair, simulated against mainnet.
//!
//! Each test asks every router of `build_routers()` for a quote, has it build
//! exactly the unsigned transaction its `execute_swap` would sign, and runs that
//! transaction through the pre-send gate. Nothing is signed or submitted: the
//! node executes the real instructions against real balances and reports the
//! balance changes, and the test reads from them where the platform fee landed
//! and how much it was.
//!
//! The fee must land in the destination `revenue::fee_reference_for_pair` names
//! for the pair, in the form the router pays it: Jupiter into the reference token
//! account, the routers that build their own transfer into the fee wallet as
//! lamports when the reference is SOL. Its size must be `platform_fee_amount` of
//! the fee leg within 0.2%.
//!
//! The direct router finds pools through the pool service, which a test process
//! does not run, so it is quoted against a named deep pool of each pair instead.
//! Every other step, from the build on, is the router's own.
//!
//! Live tier: `#[ignore = "live network"]`, run with `./test.sh live`.

use crate::common;

use screenerbot::chains::solana::constants::{SOL_MINT, USDC_MINT, USDT_MINT};
use screenerbot::chains::solana::rpc::spl_token_program_id;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::solana_sdk::transaction::VersionedTransaction;
use screenerbot::chains::solana::swaps::presend::{self, Verdict};
use screenerbot::chains::solana::swaps::revenue::{
    self, fee_reference_for_pair, platform_fee_amount, FeeReference,
};
use screenerbot::chains::solana::swaps::routers::{build_routers, DirectPoolRouter, JupiterRouter};
use screenerbot::chains::{ChainId, RawAmount};
use screenerbot::swaps::{Quote, QuoteRequest, SwapMode, SwapRouter};
use std::str::FromStr;

/// JUP, an ordinary token: neither native nor a stablecoin.
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

/// Orca Whirlpool SOL/USDC, the pool the direct router is quoted against for USDC.
const DIRECT_POOL_SOL_USDC: &str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE";
/// Raydium AMM v4 SOL/USDT, the pool the direct router is quoted against for USDT.
const DIRECT_POOL_SOL_USDT: &str = "7XawhbbxtsRcQA8KTkHT9f9nc6d69UwqCDh6U5EEbEmX";
/// Orca Whirlpool JUP/SOL, the pool the direct router is quoted against for JUP.
const DIRECT_POOL_SOL_JUP: &str = "C1MgLojNLWBKADvu9BHdtgzz1oZX4dZ5zGdGcgvvW8Wz";

/// 0.01 SOL, the size of every SOL-input leg.
const SOL_INPUT_LAMPORTS: u64 = 10_000_000;
/// One whole unit of a six-decimal token (USDC, USDT, JUP), the size of every
/// token-input leg.
const ONE_TOKEN_UNIT: u64 = 1_000_000;

/// Fee tolerance: 0.2% of the expected amount, plus one raw unit for rounding.
fn fee_tolerance(expected: u64) -> u64 {
    expected / 500 + 1
}

fn pubkey(address: &str) -> Pubkey {
    Pubkey::from_str(address).expect("address constant must be a pubkey")
}

fn raw_u64(amount: RawAmount) -> u64 {
    u64::try_from(amount.raw()).expect("a Solana amount fits in u64")
}

/// The pool the direct router trades for a pair whose other leg is `token`.
fn direct_pool_for(token: &str) -> &'static str {
    match token {
        USDC_MINT => DIRECT_POOL_SOL_USDC,
        USDT_MINT => DIRECT_POOL_SOL_USDT,
        JUP_MINT => DIRECT_POOL_SOL_JUP,
        other => panic!("no direct pool is named for {other}"),
    }
}

/// A wallet that holds `amount` of `input_mint`, so the simulation spends a
/// real balance: the stand-in for SOL, or a real holder of the token.
async fn owner_for(input_mint: &str, amount: u64) -> Pubkey {
    if input_mint == SOL_MINT {
        return common::deep_pockets_owner();
    }
    let mint = pubkey(input_mint);
    let token_program = pubkey(spl_token_program_id());
    let pool = pubkey(direct_pool_for(input_mint));
    common::sell_holders(&pool, &mint, &token_program, amount)
        .await
        .into_iter()
        .find(|(_, held)| *held >= amount)
        .map(|(owner, _)| owner)
        .unwrap_or_else(|| panic!("no holder of {amount} raw units of {input_mint} was found"))
}

/// Quote the pair with `router`, through the named pool for the direct router.
async fn quote_with(router: &dyn SwapRouter, request: &QuoteRequest) -> Quote {
    let token = if request.input_mint == SOL_MINT {
        &request.output_mint
    } else {
        &request.input_mint
    };
    let quoted = if router.id() == DirectPoolRouter::new().id() {
        DirectPoolRouter::new()
            .quote_pool(request, pubkey(direct_pool_for(token)))
            .await
    } else {
        router.get_quote(request).await
    };
    quoted.unwrap_or_else(|e| {
        panic!(
            "{} could not quote {} -> {}: {e:?}",
            router.id(),
            request.input_mint,
            request.output_mint
        )
    })
}

/// Every account key the simulated transaction touches, in the order the
/// node's balance arrays index them: static keys, then the lookup tables'
/// writable and readonly loads.
fn account_keys(
    transaction: &VersionedTransaction,
    loaded: Option<(&[String], &[String])>,
) -> Vec<String> {
    let mut keys: Vec<String> = transaction
        .message
        .static_account_keys()
        .iter()
        .map(|key| key.to_string())
        .collect();
    let has_lookups = transaction
        .message
        .address_table_lookups()
        .is_some_and(|lookups| !lookups.is_empty());
    match loaded {
        Some((writable, readonly)) => {
            keys.extend(writable.iter().cloned());
            keys.extend(readonly.iter().cloned());
        }
        None => assert!(
            !has_lookups,
            "the node reported no loaded addresses for a transaction with lookup tables"
        ),
    }
    keys
}

fn token_amount(balances: &[screenerbot::chains::solana::rpc::TokenBalance], index: usize) -> u64 {
    balances
        .iter()
        .find(|balance| balance.account_index as usize == index)
        .map(|balance| {
            balance
                .ui_token_amount
                .amount
                .parse::<u64>()
                .expect("a token balance is a raw integer")
        })
        .unwrap_or(0)
}

/// Where a router pays the fee for `reference`: the fee wallet as lamports when
/// it builds its own SOL transfer, otherwise the reference token account.
fn fee_destination(router_id: &str, reference: &FeeReference) -> (&'static str, bool) {
    if reference.is_native_sol() && router_id != JupiterRouter::new().id() {
        (revenue::FEE_WALLET, true)
    } else {
        (reference.account, false)
    }
}

/// Simulate `input_amount` of `input_mint` -> `output_mint` through every
/// router and assert where and how much each pays the platform fee.
async fn every_router_pays_the_platform_fee(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
) {
    let reference = fee_reference_for_pair(input_mint, output_mint)
        .unwrap_or_else(|| panic!("{input_mint} -> {output_mint} has no fee reference"));
    let owner = owner_for(input_mint, input_amount).await;
    let owner_text = owner.to_string();
    let request = QuoteRequest {
        chain: ChainId::Solana,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        input_amount: RawAmount::from(input_amount),
        wallet_address: owner_text.clone(),
        slippage_pct: 1.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };

    for router in build_routers() {
        let label = format!("{}: {input_mint} -> {output_mint}", router.id());
        let quote = quote_with(router.as_ref(), &request).await;
        let wire = router
            .build_for_simulation(&quote, &owner_text)
            .await
            .unwrap_or_else(|e| panic!("{label}: the build failed: {e}"));
        let transaction: VersionedTransaction = bincode::deserialize(&wire)
            .unwrap_or_else(|e| panic!("{label}: the built bytes are not a transaction: {e}"));

        let outcome = match presend::gate(&transaction).await {
            Verdict::Cleared(outcome) => outcome,
            Verdict::Failed(outcome) => panic!(
                "{label}: the node rejected the swap: {}\nlogs:\n{}",
                outcome.failure_detail(),
                outcome.logs.join("\n")
            ),
            Verdict::Refused(reason) => panic!("{label}: the gate refused the swap: {reason:?}"),
            Verdict::NodeUnavailable { detail } => panic!("{label}: no node answered: {detail}"),
        };

        let loaded = outcome
            .loaded_addresses
            .as_ref()
            .map(|loaded| (loaded.writable.as_slice(), loaded.readonly.as_slice()));
        let keys = account_keys(&transaction, loaded);
        let (destination, in_lamports) = fee_destination(router.id(), &reference);
        let index = keys
            .iter()
            .position(|key| key == destination)
            .unwrap_or_else(|| {
                panic!("{label}: the fee destination {destination} is not in the transaction")
            });

        let fee_paid = if in_lamports {
            let pre = outcome
                .pre_balances
                .as_ref()
                .expect("the node returns pre balances");
            let post = outcome
                .post_balances
                .as_ref()
                .expect("the node returns post balances");
            post[index].saturating_sub(pre[index])
        } else {
            let pre = outcome
                .pre_token_balances
                .as_ref()
                .expect("the node returns pre token balances");
            let post = outcome
                .post_token_balances
                .as_ref()
                .expect("the node returns post token balances");
            token_amount(post, index).saturating_sub(token_amount(pre, index))
        };

        let expected = if reference.mint == input_mint {
            platform_fee_amount(raw_u64(quote.input_amount))
        } else if reference.is_native_sol() {
            quote
                .platform_fee_lamports
                .unwrap_or_else(|| panic!("{label}: a SOL-output quote states its fee in lamports"))
        } else {
            // A token-output fee rides on the gross output: what the owner's
            // accounts of that mint gained plus what the fee account gained.
            let pre = outcome.pre_token_balances.as_deref().unwrap_or_default();
            let post = outcome.post_token_balances.as_deref().unwrap_or_default();
            let owner_gain: u64 = post
                .iter()
                .filter(|balance| {
                    balance.mint == reference.mint
                        && balance.owner.as_deref() == Some(owner_text.as_str())
                        && balance.account_index as usize != index
                })
                .map(|balance| {
                    let at = balance.account_index as usize;
                    token_amount(post, at).saturating_sub(token_amount(pre, at))
                })
                .sum();
            platform_fee_amount(owner_gain + fee_paid)
        };

        assert!(
            expected > 0,
            "{label}: the expected platform fee must be positive"
        );
        assert!(
            fee_paid.abs_diff(expected) <= fee_tolerance(expected),
            "{label}: {destination} gained {fee_paid}, the platform fee is {expected}"
        );
        eprintln!("{label}: {fee_paid} paid into {destination} (expected {expected})");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn every_router_pays_the_platform_fee_on_sol_to_usdc() {
    let _guard = common::isolated_env();
    every_router_pays_the_platform_fee(SOL_MINT, USDC_MINT, SOL_INPUT_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn every_router_pays_the_platform_fee_on_usdc_to_sol() {
    let _guard = common::isolated_env();
    every_router_pays_the_platform_fee(USDC_MINT, SOL_MINT, ONE_TOKEN_UNIT).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn every_router_pays_the_platform_fee_on_sol_to_usdt() {
    let _guard = common::isolated_env();
    every_router_pays_the_platform_fee(SOL_MINT, USDT_MINT, SOL_INPUT_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn every_router_pays_the_platform_fee_on_usdt_to_sol() {
    let _guard = common::isolated_env();
    every_router_pays_the_platform_fee(USDT_MINT, SOL_MINT, ONE_TOKEN_UNIT).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn every_router_pays_the_platform_fee_on_sol_to_jup() {
    let _guard = common::isolated_env();
    every_router_pays_the_platform_fee(SOL_MINT, JUP_MINT, SOL_INPUT_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn every_router_pays_the_platform_fee_on_jup_to_sol() {
    let _guard = common::isolated_env();
    every_router_pays_the_platform_fee(JUP_MINT, SOL_MINT, ONE_TOKEN_UNIT).await;
}
