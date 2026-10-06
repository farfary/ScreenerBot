// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The accounts a recorded swap case holds against the accounts production reads.
//!
//! A venue names every account it needs through `AccountReader`, so replaying a case through a
//! recording reader shows the exact set the production path asks a node for. A case that holds an
//! account no venue reads is stale, and a case that lacks one the venue requires cannot load.
//! Either way the offline swap tests would stop describing what production does.
//!
//! Venues that walk arrays (CLMM and Whirlpool tick arrays, DLMM bin arrays) request every
//! candidate address and treat an absent one as uninitialised, so for those venues a requested
//! address missing from the case is legitimate. For every other venue each requested address must
//! be in the case; [`MAY_REQUEST_ABSENT`] only shrinks.

use crate::common::pool_cases::SolanaAccount;
use crate::common::solana_pools::{
    cases_for_purpose, load_market, pubkey, CaseAccounts, RecordingAccounts, SolanaCase,
};
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::swaps::direct;
use std::collections::BTreeSet;

const SWAP_PURPOSE: &str = "direct-swap";

/// Venues whose loader requests candidate array and extension addresses that may not exist.
const MAY_REQUEST_ABSENT: [&str; 3] = ["meteora_dlmm", "orca_whirlpool", "raydium_clmm"];

/// The only venue whose loader asks which token accounts a pool owns; it recovers the curve's
/// mint that way because the account does not store it.
const LISTS_POOL_TOKEN_ACCOUNTS: &str = "pumpfun_legacy";

/// What is wrong between `case` and the accounts the production load path reads. Empty when the
/// case holds exactly what is read.
fn findings(case: &SolanaCase) -> Vec<String> {
    let pool = pubkey(&case.pool);
    let recorder = RecordingAccounts::new(CaseAccounts::new(case));
    let loaded = futures::executor::block_on(direct::load_market(&pool, &recorder));

    let mut findings = Vec::new();
    if let Err(error) = loaded {
        findings.push(format!("the pool does not load from the case: {error}"));
    }

    let reads = recorder.reads();
    if reads.first().map(Vec::as_slice) != Some(&[pool][..]) {
        findings.push("the first read is not the pool account alone".to_owned());
    }

    let requested: Vec<Pubkey> = reads.iter().flatten().copied().collect();
    let distinct: BTreeSet<Pubkey> = requested.iter().copied().collect();
    if distinct.len() != requested.len() {
        findings.push("an account is requested more than once".to_owned());
    }

    let recorded: BTreeSet<Pubkey> = case.accounts.keys().map(|a| pubkey(a)).collect();
    for address in recorded.difference(&distinct) {
        findings.push(format!("{address} is recorded but never read"));
    }
    if !MAY_REQUEST_ABSENT.contains(&case.venue.as_str()) {
        for address in distinct.difference(&recorded) {
            findings.push(format!("{address} is read but not recorded"));
        }
    }

    let owners = recorder.token_account_owners();
    let expected_owners: Vec<Pubkey> = if case.venue == LISTS_POOL_TOKEN_ACCOUNTS {
        vec![pool]
    } else {
        Vec::new()
    };
    if owners != expected_owners {
        findings.push(format!(
            "token accounts were listed for {owners:?}, expected {expected_owners:?}"
        ));
    }
    findings
}

#[test]
fn every_case_holds_exactly_the_accounts_the_production_paths_read() {
    let cases = cases_for_purpose(SWAP_PURPOSE);
    assert!(!cases.is_empty(), "no recorded {SWAP_PURPOSE} case");

    let mut failures = Vec::new();
    for (name, case) in &cases {
        for finding in findings(case) {
            failures.push(format!("{}/{name}: {finding}", case.venue));
        }
    }
    assert!(
        failures.is_empty(),
        "recorded cases drifted from the production read set:\n{}",
        failures.join("\n")
    );
}

#[test]
fn an_account_no_venue_reads_is_reported_as_stale() {
    let (name, mut case) = cases_for_purpose(SWAP_PURPOSE)
        .into_iter()
        .find(|(_, case)| case.venue == "raydium_cpmm")
        .expect("a recorded CPMM swap case");
    let stray = Pubkey::new_unique();
    case.accounts.insert(
        stray.to_string(),
        SolanaAccount {
            owner: Pubkey::new_unique().to_string(),
            lamports: 1,
            data: vec![0; 8],
        },
    );

    let findings = findings(&case);
    assert!(
        findings
            .iter()
            .any(|f| f == &format!("{stray} is recorded but never read")),
        "{name}: a stray account must be reported, got {findings:?}"
    );
}

#[test]
fn a_case_missing_an_account_the_venue_requires_is_reported() {
    let (name, mut case) = cases_for_purpose(SWAP_PURPOSE)
        .into_iter()
        .find(|(_, case)| case.venue == "raydium_cpmm")
        .expect("a recorded CPMM swap case");
    let vault = case
        .accounts
        .keys()
        .find(|address| **address != case.pool && !address.starts_with("So1111"))
        .cloned()
        .expect("the case holds accounts besides the pool");
    case.accounts.remove(&vault);

    let findings = findings(&case);
    assert!(
        findings.iter().any(|f| f.contains("does not load")),
        "{name}: dropping {vault} must stop the pool loading, got {findings:?}"
    );
}

#[test]
fn the_pool_alone_loads_through_the_same_dispatch_as_production() {
    for (name, case) in cases_for_purpose(SWAP_PURPOSE) {
        let market =
            load_market(&case).unwrap_or_else(|e| panic!("{}/{name} must load: {e}", case.venue));
        assert_eq!(
            market.pool(),
            pubkey(&case.pool),
            "{}/{name}: the market is for the case's pool",
            case.venue
        );
    }
}
